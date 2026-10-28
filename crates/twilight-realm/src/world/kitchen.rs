//! Kitchens (ADR 0025, #491), as Davis's Kitchen prototype plays them: a cook's gossip opens their
//! kitchen, which sends the recipe book and the pantry, predicts a pot and cooks it into a meal or
//! Hearty Mush. The first cook at a kitchen teaches its starters; an exact match teaches the
//! recipe it cooks; an item cooked with for the first time shows in every recipe that uses it.

use std::collections::HashMap;

use benilla_protocol::messages::GossipOption;
use benilla_protocol::{KitchenIngredient, KitchenRecipe, SessionEvent};

use crate::characters::{Char, DishLog};
use crate::cooking::{self, Family, Kind, Recipe};
use crate::equipment;
use crate::hub::{self, Npc};
use crate::rules::game_clock;
use crate::{KitchenAction, KitchenMode};

/// The cook's one gossip line.
const OPTION_KITCHEN: u32 = 0;
const ICON_CHAT: u8 = 0;

/// `cook_lessons`' places, as the prototype's `S.lessons`.
const LESSON_COOK: usize = 0;
const LESSON_RECIPE: usize = 1;
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
        let Some(c) = self.characters.get(guid) else {
            return;
        };
        self.events.push(SessionEvent::GossipComplete);
        self.events.push(kitchen_event(c, npc.guid(), kitchen));
        self.kitchens.0.insert(
            guid,
            Open {
                npc: npc.guid(),
                kitchen: &kitchen.key,
            },
        );
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

/// Every ingredient `c` holds in the bags or Satchel, by rarity, then level, then entry.
fn pantry(c: &Char) -> Vec<KitchenIngredient> {
    let mut entries: Vec<u32> = c
        .inventory
        .iter()
        .map(|(_, i)| i.entry)
        .chain(c.inventory.satchel.counts.keys().copied())
        .chain(c.inventory.satchel.pieces.iter().map(|p| p.entry))
        .collect();
    entries.sort_unstable();
    entries.dedup();
    let mut rows: Vec<(u8, KitchenIngredient)> = entries
        .into_iter()
        .filter_map(|item| {
            let category = category(item)?;
            let count = c.inventory.count(item);
            (count > 0).then(|| {
                let rarity = crate::items::template(item).map_or(0, |t| t.quality);
                let row = KitchenIngredient {
                    item,
                    count,
                    level: cooking::ingredient_level(item),
                    category,
                    untried: !c.cook_tried.contains(&item),
                };
                (rarity, row)
            })
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
    let cook = hub::npcs()
        .iter()
        .find(|n| n.entry == kitchen.cook)
        .map_or_else(String::new, |n| n.name.clone());
    SessionEvent::Kitchen {
        npc,
        name: kitchen.name.clone(),
        cook,
        book: tier.chain(others).map(|r| page(c, r)).collect(),
        pantry: pantry(c),
        lessons: c.cook_lessons,
    }
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
