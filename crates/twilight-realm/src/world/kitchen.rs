//! Kitchens (ADR 0025, #491), as Davis's Kitchen prototype plays them: a cook's gossip opens their
//! kitchen, which sends the recipe book and the pantry, predicts a pot and cooks it into a meal or
//! Hearty Mush. The first cook at a kitchen teaches its starters; an exact match teaches the
//! recipe it cooks; an item cooked with for the first time shows in every recipe that uses it.
//! Each kitchen posts three orders a character a game day, and cuts fish into bait (#494).

use std::collections::{BTreeSet, HashMap};

use benilla_protocol::messages::{self as m, ChatMessage, GossipOption};
use benilla_protocol::{KitchenIngredient, KitchenOrderRow, KitchenRecipe, SessionEvent};

use crate::characters::{Bonus, Char, DishLog, KitchenOrders, Order, Want};
use crate::cooking::{self, Family, Kind, Recipe};
use crate::equipment;
use crate::fields::values;
use crate::hub::{self, Npc};
use crate::quests::system_line;
use crate::rng::Rng;
use crate::rules::game_clock;
use crate::{KitchenAction, KitchenMode};

/// The cook's one gossip line.
const OPTION_KITCHEN: u32 = 0;
const ICON_CHAT: u8 = 0;

/// `cook_lessons`' places, as the prototype's `S.lessons`.
const LESSON_COOK: usize = 0;
const LESSON_RECIPE: usize = 1;
const LESSON_ORDER: usize = 3;
const LESSON_REVEAL: usize = 4;
const LESSON_DISCOVER: usize = 5;

/// The recipe the "Cook from your recipe book" lesson asks for.
const LESSON_RECIPE_KEY: &str = "stamina_1";

/// A cook's gossip lines: the kitchen.
pub(crate) fn kitchen_options() -> Vec<GossipOption> {
    vec![GossipOption {
        index: OPTION_KITCHEN,
        icon: ICON_CHAT,
        coded: false,
        message: "Let me use your kitchen.".into(),
    }]
}

/// A character's open kitchen: the cook's guid and the kitchen's key.
struct Open {
    npc: u64,
    kitchen: &'static str,
}

/// Every character's open kitchen, keyed by player guid. Not saved.
#[derive(Default)]
pub(crate) struct Kitchens(HashMap<u64, Open>);

impl Kitchens {
    /// Forgets `guid`'s open kitchen, as a logout does.
    pub(crate) fn close(&mut self, guid: u64) {
        self.0.remove(&guid);
    }
}

impl crate::Realm {
    /// The cook's line: their kitchen opens.
    pub(crate) fn kitchen_select(&mut self, npc: &Npc, option: u32) {
        if option != OPTION_KITCHEN || !self.near(npc) {
            return;
        }
        let Some(kitchen) = cooking::kitchen_of_cook(npc.entry) else {
            return;
        };
        let Some(guid) = self.world.as_ref().map(|w| w.guid) else {
            return;
        };
        let wall = self.wall;
        let Some(c) = self.characters.get_mut(guid) else {
            return;
        };
        let rolled = post_orders(c, kitchen, wall, &mut self.rng);
        self.events.push(SessionEvent::GossipComplete);
        self.events.push(kitchen_event(c, npc.guid(), kitchen));
        self.kitchens.0.insert(
            guid,
            Open {
                npc: npc.guid(),
                kitchen: &kitchen.key,
            },
        );
        if rolled {
            self.save(guid);
        }
    }

    /// An action at the open kitchen; nothing without one.
    pub(crate) fn kitchen_action(&mut self, action: KitchenAction) {
        let Some(guid) = self.world.as_ref().map(|w| w.guid) else {
            return;
        };
        let Some((npc, key)) = self.kitchens.0.get(&guid).map(|o| (o.npc, o.kitchen)) else {
            return;
        };
        let Some(kitchen) = cooking::kitchen(key) else {
            return;
        };
        let wall = self.wall;
        let Some(c) = self.characters.get_mut(guid) else {
            return;
        };
        match action {
            KitchenAction::Close => {
                self.kitchens.close(guid);
                self.events.push(SessionEvent::KitchenClosed);
            }
            KitchenAction::Predict {
                mode,
                recipe,
                slots,
            } => self.events.push(predict(c, mode, &recipe, &slots)),
            KitchenAction::Cook {
                mode,
                recipe,
                slots,
            } => match cook(c, kitchen, mode, &recipe, &slots, wall, &mut self.events) {
                Ok(cooked) => {
                    self.events.push(cooked);
                    self.events.push(kitchen_event(c, npc, kitchen));
                    self.save(guid);
                }
                Err(note) => self.events.push(SessionEvent::KitchenPrediction {
                    ready: false,
                    meal: 0,
                    note: note.to_owned(),
                    why: Vec::new(),
                }),
            },
            KitchenAction::Deliver(index) => {
                let delivered = deliver(
                    c,
                    npc,
                    kitchen,
                    usize::from(index),
                    &mut self.rng,
                    &mut self.events,
                );
                if delivered {
                    self.events.push(kitchen_event(c, npc, kitchen));
                    self.save(guid);
                }
            }
            KitchenAction::Cut(fish) => {
                if cut(c, fish, &mut self.events) {
                    self.events.push(kitchen_event(c, npc, kitchen));
                    self.save(guid);
                }
            }
        }
    }

    /// Closes the served character's kitchen when they stand beyond reach of its cook, or died.
    pub(crate) fn check_open_kitchen(&mut self) {
        let Some(w) = self.world.as_ref() else {
            return;
        };
        let Some(open) = self.kitchens.0.get(&w.guid) else {
            return;
        };
        let reach = match (hub::find(open.npc), self.characters.get(w.guid)) {
            (Some(npc), Some(c)) => c.map == npc.map && hub::within_reach(npc, c.position),
            _ => false,
        };
        if !reach || w.dead() {
            let guid = w.guid;
            self.kitchens.close(guid);
            self.events.push(SessionEvent::KitchenClosed);
        }
    }
}

/// Whether `c` knows the recipe `key`.
fn knows(c: &Char, key: &str) -> bool {
    c.cook_known.iter().any(|k| k == key)
}

/// Teaches `c` the recipe `key`, with its Dish Journal line; false when already known.
fn learn(c: &mut Char, key: &str) -> bool {
    if knows(c, key) {
        return false;
    }
    c.cook_known.push(key.to_owned());
    c.dish_log.push(DishLog {
        recipe: key.to_owned(),
        cooked: 0,
        first: String::new(),
    });
    true
}

/// The first catch (`how` "catch") or pick ("pick") of `item` teaches the recipes it teaches, with
/// a system line naming them.
pub(crate) fn learn_first(c: &mut Char, item: u32, how: &str, events: &mut Vec<SessionEvent>) {
    let taught: Vec<String> = cooking::recipes_taught_by(item)
        .into_iter()
        .filter(|r| learn(c, &r.key))
        .map(|r| meal_name(r.meal))
        .collect();
    if !taught.is_empty() {
        events.push(crate::quests::system_line(format!(
            "Your first {how} teaches you: {}.",
            taught.join(", ")
        )));
    }
}

fn meal_name(meal: u32) -> String {
    crate::items::template(meal).map_or_else(String::new, |t| t.name.clone())
}

fn kind_code(kind: Kind) -> u8 {
    match kind {
        Kind::Tier => 0,
        Kind::Dish => 1,
        Kind::Legend => 2,
        Kind::Plant => 3,
    }
}

fn capitalised(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map_or_else(String::new, |f| {
        f.to_uppercase().chain(chars).collect::<String>()
    })
}

/// An out-of-combat effect's label, as the prototype's `OOC[..].label`.
fn ooc_label(key: &str) -> &'static str {
    match key {
        "xp" => "XP from kills and quests",
        "regen" => "out-of-combat regen",
        "speed" => "move speed",
        _ => "",
    }
}

/// The prototype's `effectLabel` and `familyName` of a recipe.
fn labels(r: &Recipe) -> (String, &'static str) {
    match r.kind {
        Kind::Legend => match &r.double {
            Some(d) => (
                format!("+{} and {}", capitalised(&d.stat), ooc_label(&d.ooc)),
                "Legendary double meal",
            ),
            None => (
                "Unique legendary effect".to_owned(),
                "Legendary unique meal",
            ),
        },
        Kind::Plant => ("Legendary plant meal".to_owned(), "Legendary plant meal"),
        Kind::Tier | Kind::Dish => {
            let key = r.effect.as_deref().unwrap_or_default();
            match cooking::effect(key).map(|e| e.family) {
                Some(Family::Combat) => (format!("+{}", capitalised(key)), "Combat Well Fed"),
                Some(Family::Ooc) => {
                    let label = match key {
                        "xp" => "More XP",
                        "regen" => "Faster regen",
                        _ => "Faster movement",
                    };
                    (label.to_owned(), "Out-of-combat Well Fed")
                }
                _ => {
                    let label = match key {
                        "pull" => "Extra herb pulls",
                        "rarity" => "Rarer finds",
                        _ => "Less lure wear",
                    };
                    (label.to_owned(), "Gatherer")
                }
            }
        }
    }
}

/// A tier recipe's or dish's band (1, 10, 20 or 30); 0 for a legend or plant meal.
fn band(r: &Recipe) -> u8 {
    match (r.kind, r.tier) {
        (Kind::Tier | Kind::Dish, Some(t)) => cooking::tiers().bands[usize::from(t)],
        _ => 0,
    }
}

/// `r`'s page in `c`'s book: a known recipe shows everything, an unknown one only the slots of
/// items `c` has cooked with.
fn page(c: &Char, r: &Recipe) -> KitchenRecipe {
    let known = knows(c, &r.key);
    let (effect, family) = labels(r);
    KitchenRecipe {
        key: r.key.clone(),
        kind: kind_code(r.kind),
        effect,
        family: family.to_owned(),
        band: band(r),
        known,
        name: if known {
            meal_name(r.meal)
        } else {
            String::new()
        },
        meal: if known { r.meal } else { 0 },
        slots: r
            .ingredients
            .iter()
            .map(|&i| {
                if known || c.cook_tried.contains(&i) {
                    i
                } else {
                    0
                }
            })
            .collect(),
    }
}

/// Whether `item` is a legendary `[[plant]]`.
fn legendary_plant(item: u32) -> bool {
    crate::herbs::zones()
        .into_iter()
        .flat_map(|z| crate::herbs::plants_in(&z.key))
        .chain(crate::herbs::plants_in(crate::herbs::ANY))
        .any(|p| p.item == item && p.rarity == crate::herbs::Rarity::Legendary)
}

/// What a pot takes `item` as: "legend", "fish", "plant" or a cooking herb's category; `None`
/// for anything that is no ingredient.
fn category(item: u32) -> Option<String> {
    if crate::fish::legend(item).is_some() {
        Some("legend".to_owned())
    } else if crate::fish::fish(item).is_some() {
        Some("fish".to_owned())
    } else if legendary_plant(item) {
        Some("plant".to_owned())
    } else {
        cooking::category_of(item).map(str::to_owned)
    }
}

/// Every item `c` holds at least one of in the bags or Satchel, ascending.
fn held_items(c: &Char) -> Vec<u32> {
    let mut entries: Vec<u32> = c
        .inventory
        .iter()
        .map(|(_, i)| i.entry)
        .chain(c.inventory.satchel.counts.keys().copied())
        .chain(c.inventory.satchel.pieces.iter().map(|p| p.entry))
        .collect();
    entries.sort_unstable();
    entries.dedup();
    entries.retain(|&item| c.inventory.count(item) > 0);
    entries
}

/// Every ingredient `c` holds in the bags or Satchel, by rarity, then level, then entry.
fn pantry(c: &Char) -> Vec<KitchenIngredient> {
    let mut rows: Vec<(u8, KitchenIngredient)> = held_items(c)
        .into_iter()
        .filter_map(|item| {
            let category = category(item)?;
            let rarity = crate::items::template(item).map_or(0, |t| t.quality);
            let row = KitchenIngredient {
                item,
                count: c.inventory.count(item),
                level: cooking::ingredient_level(item),
                category,
                untried: !c.cook_tried.contains(&item),
            };
            Some((rarity, row))
        })
        .collect();
    rows.sort_by_key(|(rarity, r)| (*rarity, r.level, r.item));
    rows.into_iter().map(|(_, r)| r).collect()
}

/// The `Kitchen` event of `kitchen`, opened at the cook `npc`, for `c`.
fn kitchen_event(c: &Char, npc: u64, kitchen: &cooking::Kitchen) -> SessionEvent {
    let all = cooking::recipes();
    let tier = all.iter().filter(|r| r.kind == Kind::Tier);
    let others = all
        .iter()
        .filter(|r| r.kind != Kind::Tier && knows(c, &r.key));
    let held = held_items(c);
    SessionEvent::Kitchen {
        npc,
        name: kitchen.name.clone(),
        cook: cook_name(kitchen),
        book: tier.chain(others).map(|r| page(c, r)).collect(),
        pantry: pantry(c),
        lessons: c.cook_lessons,
        orders: order_rows(c, kitchen, &held),
        cuttable: held
            .iter()
            .filter_map(|&f| cooking::cut_of(f).map(|cut| (f, c.inventory.count(f), cut.bait)))
            .collect(),
    }
}

fn cook_name(kitchen: &cooking::Kitchen) -> String {
    hub::npcs()
        .iter()
        .find(|n| n.entry == kitchen.cook)
        .map_or_else(String::new, |n| n.name.clone())
}

/// Whether every slot is an ingredient `c` holds enough of.
fn has_all(c: &Char, slots: &[u32]) -> bool {
    slots.iter().all(|&item| {
        let need = slots.iter().filter(|&&s| s == item).count() as u32;
        category(item).is_some() && c.inventory.count(item) >= need
    })
}

const NOT_ENOUGH: &str = "You don't have enough of those ingredients.";
const NOT_THIS_RECIPE: &str = "That isn't this recipe's ingredients.";

/// The prototype's `predict`: what the pot would cook.
fn predict(c: &Char, mode: KitchenMode, key: &str, slots: &[u32]) -> SessionEvent {
    let refuse = |note: &str| SessionEvent::KitchenPrediction {
        ready: false,
        meal: 0,
        note: note.to_owned(),
        why: Vec::new(),
    };
    if slots.is_empty() || slots.contains(&0) {
        return refuse(if mode == KitchenMode::Discover {
            "Fill the ??? slots with your guesses."
        } else {
            "Fill every slot to see what you would get."
        });
    }
    if !has_all(c, slots) {
        return refuse(NOT_ENOUGH);
    }
    if mode != KitchenMode::Recipe {
        return SessionEvent::KitchenPrediction {
            ready: true,
            meal: 0,
            note: String::new(),
            why: vec![
                "An exact match teaches the recipe and cooks its meal.".to_owned(),
                "If not, you get Hearty Mush and lose the ingredients.".to_owned(),
            ],
        };
    }
    let Some(r) = cooking::recipe(key).filter(|r| knows(c, key) && cooking::matches(r, slots))
    else {
        return refuse(NOT_THIS_RECIPE);
    };
    let why = match r.kind {
        Kind::Legend | Kind::Plant => format!(
            "Legendary meals have one fixed strength. {} minutes, survives death.",
            cooking::eat().legendary_minutes
        ),
        Kind::Dish => {
            let fish = r.teacher.unwrap_or_default();
            format!(
                "{} is level {}, so its dish is band {}.",
                meal_name(fish),
                cooking::ingredient_level(fish),
                band(r)
            )
        }
        Kind::Tier => format!(
            "A band {} recipe. A higher band is a different recipe.",
            band(r)
        ),
    };
    SessionEvent::KitchenPrediction {
        ready: true,
        meal: r.meal,
        note: String::new(),
        why: vec![why],
    }
}

/// A recipe's place in the prototype's `resolve`: a plant or legend before a dish before a tier.
fn priority(kind: Kind) -> u8 {
    match kind {
        Kind::Plant | Kind::Legend => 3,
        Kind::Dish => 2,
        Kind::Tier => 1,
    }
}

/// The recipe `slots` cooks: the first exact match among the tier recipes and the known ones, by
/// priority.
fn resolve(c: &Char, slots: &[u32]) -> Option<&'static Recipe> {
    let mut best: Option<&'static Recipe> = None;
    for r in cooking::recipes() {
        let open = r.kind == Kind::Tier || knows(c, &r.key);
        if open
            && cooking::matches(r, slots)
            && best.is_none_or(|b| priority(r.kind) > priority(b.kind))
        {
            best = Some(r);
        }
    }
    best
}

/// "Day 2, 08:00, Lion's Pride kitchen" at the wall clock `wall`.
fn when_and_where(wall: f64, kitchen: &cooking::Kitchen) -> String {
    let (hour, minute) = game_clock::hour_minute(wall);
    let day = game_clock::day_serial(wall) + 1;
    format!("Day {day}, {hour:02}:{minute:02}, {}", kitchen.name)
}

/// The prototype's `cook`: the `KitchenCooked` of a pot, or the reason it was refused.
fn cook(
    c: &mut Char,
    kitchen: &cooking::Kitchen,
    mode: KitchenMode,
    key: &str,
    slots: &[u32],
    wall: f64,
    events: &mut Vec<SessionEvent>,
) -> Result<SessionEvent, &'static str> {
    if slots.is_empty() || slots.contains(&0) {
        return Err("Fill every slot first.");
    }
    if !has_all(c, slots) {
        return Err(NOT_ENOUGH);
    }
    let chosen = cooking::recipe(key);
    if mode == KitchenMode::Recipe
        && !chosen.is_some_and(|r| knows(c, key) && cooking::matches(r, slots))
    {
        return Err(NOT_THIS_RECIPE);
    }
    let mut learned = Vec::new();
    let mut lines = Vec::new();
    if !c.cook_visited.iter().any(|k| *k == kitchen.key) {
        c.cook_visited.push(kitchen.key.clone());
        lines.push("First time in my kitchen? Here, these are mine.");
        for s in &kitchen.starters {
            if learn(c, s) {
                learned.push(s.clone());
            }
        }
    }
    let recipe = match mode {
        KitchenMode::Recipe => chosen,
        _ => resolve(c, slots),
    };
    if let Some(r) = recipe.filter(|r| learn(c, &r.key)) {
        learned.push(r.key.clone());
        c.cook_lessons[LESSON_DISCOVER] = true;
    }
    let close = recipe.is_none()
        && cooking::recipes().into_iter().any(|r| {
            (r.kind == Kind::Tier || knows(c, &r.key)) && cooking::misses(r, slots) == Some(1)
        });
    let mut distinct = slots.to_vec();
    distinct.sort_unstable();
    distinct.dedup();
    let mut revealed = Vec::new();
    for &item in &distinct {
        if c.cook_tried.contains(&item) {
            continue;
        }
        let at = c.cook_tried.partition_point(|&t| t < item);
        c.cook_tried.insert(at, item);
        let count = cooking::recipes()
            .into_iter()
            .filter(|r| r.kind == Kind::Tier && !knows(c, &r.key) && r.ingredients.contains(&item))
            .count() as u32;
        if count > 0 {
            c.cook_lessons[LESSON_REVEAL] = true;
        }
        revealed.push((item, count));
    }
    for &item in &distinct {
        let n = slots.iter().filter(|&&s| s == item).count() as u32;
        equipment::take(c, item, n, events);
    }
    let meal = match recipe {
        Some(r) => {
            if let Some(log) = c.dish_log.iter_mut().find(|l| l.recipe == r.key) {
                log.cooked += 1;
                if log.first.is_empty() {
                    log.first = when_and_where(wall, kitchen);
                }
            }
            if r.key == LESSON_RECIPE_KEY && mode == KitchenMode::Recipe {
                c.cook_lessons[LESSON_RECIPE] = true;
            }
            r.meal
        }
        None => {
            lines.push(if close {
                "Close. One thing's wrong, I'd say."
            } else {
                "Well. It's food."
            });
            cooking::mush().meal
        }
    };
    c.cook_lessons[LESSON_COOK] = true;
    if let Err(reason) = equipment::receive(c, meal, 1, events) {
        events.push(equipment::inventory_failure(reason, 0));
    }
    Ok(SessionEvent::KitchenCooked {
        meal,
        mush: recipe.is_none(),
        close,
        learned,
        revealed,
        line: lines.join(" "),
    })
}

/// An order's kind.
const ORDER_FISH: u8 = 0;
const ORDER_MEAL: u8 = 1;
const ORDER_HERB: u8 = 2;

/// The game hour the kitchens post new orders at.
const DAWN_HOUR: f64 = 6.0;

/// The dawn-based game day at the wall clock `wall`: it turns at [`DAWN_HOUR`].
fn order_day(wall: f64) -> u32 {
    game_clock::day_serial(wall - DAWN_HOUR * 3600.0 / game_clock::SPEED)
}

/// Rolls `kitchen`'s orders for `c` when they were not rolled on today's order day; true when
/// it rolled.
fn post_orders(c: &mut Char, kitchen: &cooking::Kitchen, wall: f64, rng: &mut Rng) -> bool {
    let day = order_day(wall);
    if c.kitchen_orders
        .iter()
        .any(|k| k.kitchen == kitchen.key && k.day == day)
    {
        return false;
    }
    let orders = roll_orders(c, kitchen, rng);
    c.kitchen_orders.retain(|k| k.kitchen != kitchen.key);
    c.kitchen_orders.push(KitchenOrders {
        kitchen: kitchen.key.clone(),
        day,
        orders,
    });
    true
}

/// A fish's or herb's rarity as a number: common 0, uncommon 1, rare 2, legendary 3.
fn rarity_value(item: u32) -> u32 {
    if let Some(f) = crate::fish::fish(item) {
        return f.rarity as u32;
    }
    crate::herbs::zones()
        .into_iter()
        .flat_map(|z| crate::herbs::plants_in(&z.key))
        .chain(crate::herbs::plants_in(crate::herbs::ANY))
        .find(|p| p.item == item)
        .map_or(0, |p| p.rarity as u32)
}

const RARE: u32 = 2;

/// "a mushroom", "an allium", "greens": the prototype's article rule.
fn with_article(category: &str) -> String {
    if category == "greens" {
        category.to_owned()
    } else if category.starts_with(['a', 'e', 'i', 'o', 'u']) {
        format!("an {category}")
    } else {
        format!("a {category}")
    }
}

/// "mushrooms", "greens", "fruit": the prototype's `CAT[..].many`.
fn plural(category: &str) -> String {
    match category {
        "greens" | "fruit" | "fish" => category.to_owned(),
        _ => format!("{category}s"),
    }
}

fn pick<T: Copy>(list: &[T], rng: &mut Rng) -> Option<T> {
    (!list.is_empty()).then(|| list[rng.below(list.len() as u32) as usize])
}

/// The prototype's `rollOrders` for one kitchen: a fish, a meal and a herb order, each with a
/// chance of a bonus. The first Goldshire roll posts the first fish order and sets
/// `first_order_given`.
pub(crate) fn roll_orders(c: &mut Char, kitchen: &cooking::Kitchen, rng: &mut Rng) -> Vec<Order> {
    let t = cooking::orders();
    let level = c.level;
    let fish: Vec<u32> = crate::fish::fish_items()
        .filter(|&f| crate::fish::legend(f).is_none() && cooking::ingredient_level(f) <= level)
        .collect();
    let herbs: Vec<(u32, &str)> = cooking::cooking_herbs()
        .filter(|&(h, _)| cooking::ingredient_level(h) <= level)
        .collect();
    let order = |kind: u8, label: String, want: Want, qty: u32, copper: u32| Order {
        kind,
        label,
        want,
        qty,
        copper,
        bonus: None,
        done: false,
    };
    let mut list = Vec::new();

    let first = &t.first_order;
    let named = pick(&fish, rng).filter(|_| rng.frand(0.0, 1.0) < t.named_fish_chance);
    list.push(if !c.first_order_given && kitchen.key == first.kitchen {
        c.first_order_given = true;
        order(
            ORDER_FISH,
            format!("{} {}", first.qty, meal_name(first.item)),
            Want::Item(first.item),
            first.qty,
            first.copper,
        )
    } else if let Some(f) = named {
        let rarity = rarity_value(f);
        let [lo, hi] = if rarity == RARE {
            t.rare_fish_qty
        } else {
            t.named_fish_qty
        };
        let qty = rng.urand(lo, hi);
        let [base, per_level] = t.named_fish_copper;
        let fish_level = u32::from(cooking::ingredient_level(f));
        order(
            ORDER_FISH,
            format!("{qty} {}", meal_name(f)),
            Want::Item(f),
            qty,
            qty * (base + per_level * fish_level) * (1 + rarity),
        )
    } else {
        let qty = rng.urand(t.any_fish_qty[0], t.any_fish_qty[1]);
        order(
            ORDER_FISH,
            format!("{qty} fish of any kind"),
            Want::AnyFish,
            qty,
            qty * t.any_fish_copper,
        )
    });

    // The meal: a herb category from the recipes they know, else the kitchen's starters.
    let keys: Vec<&String> = if c.cook_known.is_empty() {
        kitchen.starters.iter().collect()
    } else {
        c.cook_known.iter().collect()
    };
    let categories: BTreeSet<&str> = keys
        .into_iter()
        .filter_map(|k| cooking::recipe(k))
        .filter(|r| matches!(r.kind, Kind::Tier | Kind::Dish))
        .flat_map(|r| {
            r.ingredients
                .iter()
                .filter_map(|&i| cooking::category_of(i))
        })
        .filter(|cat| herbs.iter().any(|(_, h)| h == cat))
        .collect();
    let categories: Vec<&str> = categories.into_iter().collect();
    let category = pick(&categories, rng).unwrap_or("fish");
    let bands = cooking::tiers().bands;
    let high = level >= bands[1] && rng.below(2) == 0;
    let qty = rng.urand(t.meal_qty[0], t.meal_qty[1]);
    let [base, per_level] = t.meal_copper;
    let band_text = if high {
        format!("band {} or higher ", bands[1])
    } else {
        String::new()
    };
    list.push(order(
        ORDER_MEAL,
        format!(
            "{qty} {band_text}dishes with {} in them",
            with_article(category)
        ),
        Want::MealWith {
            category: category.to_owned(),
            min_band: if high { bands[1] } else { bands[0] },
        },
        qty,
        qty * (base + per_level * u32::from(level)) * (1 + u32::from(high)),
    ));

    // The herb: a category of a cooking herb they can reach.
    let herb_categories: BTreeSet<&str> = herbs.iter().map(|&(_, cat)| cat).collect();
    let herb_categories: Vec<&str> = herb_categories.into_iter().collect();
    if let Some(category) = pick(&herb_categories, rng) {
        let qty = rng.urand(t.herb_qty[0], t.herb_qty[1]);
        let [base, per_level] = t.herb_copper;
        list.push(order(
            ORDER_HERB,
            format!("{qty} raw {}", plural(category)),
            Want::HerbOf(category.to_owned()),
            qty,
            qty * (base + per_level * u32::from(level)),
        ));
    }

    let rares: Vec<u32> = herbs
        .iter()
        .map(|&(h, _)| h)
        .chain(fish.iter().copied())
        .filter(|&i| rarity_value(i) == RARE)
        .collect();
    for o in &mut list {
        if rng.frand(0.0, 1.0) < t.bonus_chance {
            o.bonus = Some(match pick(&rares, rng).filter(|_| rng.below(2) == 0) {
                Some(item) => Bonus::Item(item),
                None => Bonus::Rumour,
            });
        }
    }
    list
}

/// Whether an order wanting `want` takes `item`. A meal counts when its tier recipe or dish holds
/// an ingredient of the category and its tier is at least `min_band`'s.
fn accepts(want: &Want, item: u32) -> bool {
    match want {
        Want::Item(wanted) => item == *wanted,
        Want::AnyFish => crate::fish::fish(item).is_some() && crate::fish::legend(item).is_none(),
        Want::HerbOf(cat) => cooking::category_of(item) == Some(cat.as_str()),
        Want::MealWith {
            category: cat,
            min_band,
        } => cooking::recipe_of_meal(item).is_some_and(|r| {
            matches!(r.kind, Kind::Tier | Kind::Dish)
                && r.tier
                    .is_some_and(|t| t >= cooking::tiers().band_of(*min_band))
                && r.ingredients
                    .iter()
                    .any(|&i| category(i).as_deref() == Some(cat.as_str()))
        }),
    }
}

/// The prototype's `valueOf`: a delivery takes the lowest first. A meal is its tier plus one; a
/// fish or herb its rarity times 100 plus its level.
fn value(item: u32) -> u32 {
    match cooking::recipe_of_meal(item) {
        Some(r) => r.tier.map_or(5, |t| u32::from(t) + 1),
        None => rarity_value(item) * 100 + u32::from(cooking::ingredient_level(item)),
    }
}

/// `kitchen`'s orders for `c` as the window shows them, `held` being [`held_items`].
fn order_rows(c: &Char, kitchen: &cooking::Kitchen, held: &[u32]) -> Vec<KitchenOrderRow> {
    let Some(k) = c.kitchen_orders.iter().find(|k| k.kitchen == kitchen.key) else {
        return Vec::new();
    };
    k.orders
        .iter()
        .map(|o| {
            let have: u32 = held
                .iter()
                .filter(|&&i| accepts(&o.want, i))
                .map(|&i| c.inventory.count(i))
                .sum();
            KitchenOrderRow {
                label: o.label.clone(),
                kind: o.kind,
                copper: o.copper,
                qty: o.qty,
                have: have.min(o.qty),
                done: o.done,
                can_deliver: !o.done && have >= o.qty,
                bonus: match o.bonus {
                    Some(Bonus::Rumour) => "a recipe Rumour".to_owned(),
                    Some(Bonus::Item(item)) => format!("1 {}", meal_name(item)),
                    None => String::new(),
                },
                bonus_rare: matches!(o.bonus, Some(Bonus::Item(_))),
            }
        })
        .collect()
}

/// The cook of `kitchen`, whose guid is `npc`, says `text`.
fn cook_says(npc: u64, kitchen: &cooking::Kitchen, text: &str) -> SessionEvent {
    SessionEvent::Chat(ChatMessage {
        chat_type: m::CHAT_MSG_MONSTER_SAY,
        language: 0,
        sender_guid: npc,
        target_guid: 0,
        sender_name: Some(cook_name(kitchen)),
        channel: None,
        text: text.to_owned(),
        chat_tag: 0,
    })
}

/// A Rumour: teaches a random tier recipe `c` does not know.
fn rumour(c: &mut Char, rng: &mut Rng, events: &mut Vec<SessionEvent>) {
    let pool: Vec<&Recipe> = cooking::recipes()
        .into_iter()
        .filter(|r| r.kind == Kind::Tier && !knows(c, &r.key))
        .collect();
    let Some(r) = pick(&pool, rng) else {
        events.push(system_line(
            "You already know every recipe a Rumour can teach.".to_owned(),
        ));
        return;
    };
    learn(c, &r.key);
    events.push(system_line(format!(
        "The bonus Rumour teaches you: {}.",
        meal_name(r.meal)
    )));
}

/// The prototype's `deliver`: fills `kitchen`'s order `index` from what `c` holds, the cheapest
/// first, pays it and its bonus; false, with a line, when it is closed or `c` holds too few.
fn deliver(
    c: &mut Char,
    npc: u64,
    kitchen: &cooking::Kitchen,
    index: usize,
    rng: &mut Rng,
    events: &mut Vec<SessionEvent>,
) -> bool {
    let Some(order) = c
        .kitchen_orders
        .iter()
        .find(|k| k.kitchen == kitchen.key)
        .and_then(|k| k.orders.get(index))
        .filter(|o| !o.done)
        .cloned()
    else {
        events.push(system_line("That order is closed.".to_owned()));
        return false;
    };
    let mut items: Vec<u32> = held_items(c)
        .into_iter()
        .filter(|&i| accepts(&order.want, i))
        .collect();
    items.sort_by_key(|&i| (value(i), i));
    let have: u32 = items.iter().map(|&i| c.inventory.count(i)).sum();
    if have < order.qty {
        events.push(system_line(format!("You need {}.", order.label)));
        return false;
    }
    let mut left = order.qty;
    for item in items {
        let n = left.min(c.inventory.count(item));
        equipment::take(c, item, n, events);
        left -= n;
        if left == 0 {
            break;
        }
    }
    if let Some(o) = c
        .kitchen_orders
        .iter_mut()
        .find(|k| k.kitchen == kitchen.key)
        .and_then(|k| k.orders.get_mut(index))
    {
        o.done = true;
    }
    c.inventory.money += order.copper;
    events.push(values(c.guid, &[equipment::money_pair(&c.inventory)]));
    events.push(cook_says(npc, kitchen, "Just what I needed."));
    match order.bonus {
        Some(Bonus::Item(item)) => match equipment::receive(c, item, 1, events) {
            Ok(_) => events.push(system_line(format!("Bonus: 1 {}.", meal_name(item)))),
            Err(reason) => events.push(equipment::inventory_failure(reason, 0)),
        },
        Some(Bonus::Rumour) => rumour(c, rng, events),
        None => {}
    }
    c.cook_lessons[LESSON_ORDER] = true;
    true
}

/// The prototype's `cutBait`: one `fish` becomes its `[[cut]]` bait, which `receive` sends to the
/// Fishing Satchel; false, with a line, when it does not cut or `c` holds none.
fn cut(c: &mut Char, fish: u32, events: &mut Vec<SessionEvent>) -> bool {
    let Some(cut) = cooking::cut_of(fish).filter(|_| c.inventory.count(fish) > 0) else {
        events.push(system_line("You can't cut that.".to_owned()));
        return false;
    };
    if let Err(reason) = equipment::receive(c, cut.bait, cut.count, events) {
        events.push(equipment::inventory_failure(reason, 0));
        return false;
    }
    equipment::take(c, fish, 1, events);
    events.push(system_line(format!(
        "You cut {} into {} {}.",
        meal_name(fish),
        cut.count,
        meal_name(cut.bait)
    )));
    true
}
