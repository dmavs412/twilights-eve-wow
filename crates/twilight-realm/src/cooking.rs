//! Cooking's content (`content/cooking.toml`, ADR 0025 amended 2026-10-27), as Davis's Kitchen
//! prototype plays it: the eat spells, the Well Fed effects, the kitchens, herb categories, the
//! recipes, Hearty Mush, the cut baits and the order tunables. The file's item rows are item
//! templates, which `crate::items` loads.

// Cooking itself (#491 on) reads the rest.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::{OnceLock, RwLock};

use serde::Deserialize;

use crate::items::{Profession, Template};

/// Eating a meal (`[eat]`).
#[derive(Deserialize)]
pub(crate) struct Eat {
    /// The stock spell every meal row names, so the client lets it be used.
    pub(crate) use_spell: u32,
    /// The aura the client shows while Well Fed.
    pub(crate) buff_spell: u32,
    pub(crate) minutes: u32,
    pub(crate) legendary_minutes: u32,
}

/// The tier bands (`[tier]`): the level each tier starts at.
#[derive(Deserialize)]
pub(crate) struct Tier {
    pub(crate) bands: [u8; 4],
}

impl Tier {
    /// The index of the last band at or below `level`.
    pub(crate) fn band_of(&self, level: u8) -> u8 {
        self.bands.iter().rposition(|&b| b <= level).unwrap_or(0) as u8
    }
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Family {
    /// A stat.
    Combat,
    /// Off in combat.
    Ooc,
    Gather,
}

/// A Well Fed effect (`[[effect]]`).
#[derive(Deserialize)]
pub(crate) struct Effect {
    pub(crate) key: String,
    pub(crate) family: Family,
    /// The strength at each tier.
    pub(crate) values: [u32; 4],
}

/// A kitchen (`[[kitchen]]`).
#[derive(Deserialize)]
pub(crate) struct Kitchen {
    pub(crate) key: String,
    pub(crate) name: String,
    /// The Hub NPC entry whose gossip opens it.
    pub(crate) cook: u32,
    /// The tier recipe keys it starts a character with.
    pub(crate) starters: Vec<String>,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Kind {
    Tier,
    Dish,
    Legend,
    Plant,
}

/// A legendary meal's two effects: a stat and an out-of-combat one.
#[derive(Deserialize, Debug)]
pub(crate) struct Double {
    pub(crate) stat: String,
    pub(crate) stat_value: u32,
    pub(crate) ooc: String,
    pub(crate) ooc_value: u32,
}

/// A legendary meal's own effect, or one of a plant meal's specials: a key and its numbers.
#[derive(Deserialize, Debug)]
pub(crate) struct Unique {
    pub(crate) key: String,
    #[serde(flatten)]
    pub(crate) params: BTreeMap<String, u32>,
}

pub(crate) type Special = Unique;

/// A plant meal's stat.
#[derive(Deserialize, Debug)]
pub(crate) struct PlantStat {
    pub(crate) stat: String,
    pub(crate) value: u32,
}

/// A recipe (`[[recipe]]`).
#[derive(Deserialize, Debug)]
pub(crate) struct Recipe {
    pub(crate) key: String,
    pub(crate) kind: Kind,
    /// The meal item it cooks.
    pub(crate) meal: u32,
    /// Item entries, in any order.
    pub(crate) ingredients: Vec<u32>,
    /// A tier recipe's or dish's `[[effect]]` key.
    pub(crate) effect: Option<String>,
    pub(crate) tier: Option<u8>,
    /// The item whose first catch or pick teaches it.
    pub(crate) teacher: Option<u32>,
    pub(crate) double: Option<Double>,
    pub(crate) unique: Option<Unique>,
    pub(crate) stat: Option<PlantStat>,
    #[serde(default)]
    pub(crate) specials: Vec<Special>,
}

/// Hearty Mush, the meal a miss cooks (`[mush]`).
#[derive(Deserialize)]
pub(crate) struct Mush {
    pub(crate) meal: u32,
    pub(crate) regen: u32,
}

/// A cut (`[[cut]]`): one `fish` cuts into `count` of `bait` at a kitchen.
#[derive(Deserialize)]
pub(crate) struct Cut {
    pub(crate) fish: u32,
    pub(crate) bait: u32,
    pub(crate) count: u32,
}

/// A character's first Goldshire fish order.
#[derive(Deserialize)]
pub(crate) struct FirstOrder {
    pub(crate) item: u32,
    pub(crate) qty: u32,
    pub(crate) copper: u32,
}

/// The kitchen orders' tunables (`[orders]`), as the prototype's `rollOrders` reads them.
#[derive(Deserialize)]
pub(crate) struct Orders {
    pub(crate) named_fish_chance: f32,
    pub(crate) named_fish_qty: [u32; 2],
    /// `[base, per_level]`.
    pub(crate) named_fish_copper: [u32; 2],
    pub(crate) any_fish_qty: [u32; 2],
    /// Per fish.
    pub(crate) any_fish_copper: u32,
    pub(crate) meal_qty: [u32; 2],
    /// `[base, per_level]`.
    pub(crate) meal_copper: [u32; 2],
    pub(crate) herb_qty: [u32; 2],
    /// `[base, per_level]`.
    pub(crate) herb_copper: [u32; 2],
    pub(crate) bonus_chance: f32,
    pub(crate) first_order: FirstOrder,
}

/// What the load checks read of an item row.
#[derive(Deserialize)]
struct Item {
    entry: u32,
    #[serde(default)]
    use_spell: u32,
}

#[derive(Deserialize)]
struct File {
    eat: Eat,
    tier: Tier,
    effect: Vec<Effect>,
    kitchen: Vec<Kitchen>,
    /// TOML keys are strings; [`category_of`] reads them as entries.
    category: BTreeMap<String, String>,
    recipe: Vec<Recipe>,
    mush: Mush,
    cut: Vec<Cut>,
    orders: Orders,
    item: Vec<Item>,
}

/// Checks a parsed `cooking.toml`, panicking on the first row that breaks a rule.
fn check(file: &File) {
    let item = |entry: u32| file.item.iter().find(|i| i.entry == entry);
    let effect = |key: &Option<String>| {
        key.as_deref()
            .is_some_and(|k| file.effect.iter().any(|e| e.key == k))
    };
    // The shipped rows only: a test's added fish or plants are not the content's.
    let plants = crate::herbs::shipped_plants();
    let legendary_plant = |entry: u32| {
        plants
            .iter()
            .any(|p| p.item == entry && p.rarity == crate::herbs::Rarity::Legendary)
    };
    for r in &file.recipe {
        assert!(
            item(r.meal).is_some_and(|i| i.use_spell == file.eat.use_spell),
            "recipe {}'s meal {} is no cooking.toml item using [eat] use_spell",
            r.key,
            r.meal
        );
        for &i in &r.ingredients {
            assert!(
                crate::items::gathered(i),
                "recipe {}'s ingredient {i} is no fish.toml or herbs.toml item",
                r.key
            );
        }
        let tier_ok = r.tier.is_some_and(|t| (t as usize) < file.tier.bands.len());
        match r.kind {
            Kind::Tier => assert!(
                effect(&r.effect) && tier_ok,
                "tier recipe {} needs an effect and a tier",
                r.key
            ),
            Kind::Dish => {
                let teacher = r.teacher.filter(|&t| crate::fish::fish(t).is_some());
                assert!(
                    effect(&r.effect) && tier_ok && teacher.is_some(),
                    "dish {} needs an effect, a tier and a [[fish]] teacher",
                    r.key
                );
                // The shipped pools only: a test's pool may list the fish at another level.
                let fish = teacher.unwrap();
                let level = crate::fish::shipped_pools()
                    .iter()
                    .flat_map(|p| &p.fish)
                    .filter(|f| f.item == fish)
                    .map(|f| f.level)
                    .min()
                    .unwrap_or(1) as u8;
                let band = file.tier.band_of(level);
                assert!(
                    r.tier == Some(band),
                    "dish {}'s tier is not {band}, the band of its fish's level",
                    r.key
                );
            }
            Kind::Legend => assert!(
                r.teacher.is_some_and(|t| crate::fish::legend(t).is_some())
                    && r.double.is_some() != r.unique.is_some(),
                "legend recipe {} needs a [[legend]] teacher and one of double, unique",
                r.key
            ),
            Kind::Plant => assert!(
                r.teacher.is_some_and(legendary_plant),
                "plant recipe {} needs a legendary [[plant]] teacher",
                r.key
            ),
        }
    }
    for e in &file.effect {
        let mut tiers: Vec<u8> = file
            .recipe
            .iter()
            .filter(|r| r.kind == Kind::Tier && r.effect.as_deref() == Some(&e.key))
            .filter_map(|r| r.tier)
            .collect();
        tiers.sort_unstable();
        assert!(
            tiers == [0, 1, 2, 3],
            "effect {} has tier recipes {tiers:?}, not one each of 0 to 3",
            e.key
        );
    }
    let mut seen: BTreeMap<Vec<u32>, &str> = BTreeMap::new();
    for r in &file.recipe {
        let mut set = r.ingredients.clone();
        set.sort_unstable();
        if let Some(other) = seen.insert(set, &r.key) {
            panic!("recipes {other} and {} take the same ingredients", r.key);
        }
    }
    let uses = |entry: u32| {
        file.recipe
            .iter()
            .filter(|r| r.ingredients.contains(&entry))
            .count()
    };
    for f in crate::fish::shipped_fish() {
        assert!(
            uses(f.item) >= 2,
            "fish {} is in fewer than two recipes",
            f.item
        );
    }
    for p in plants {
        let need = match (p.rarity, p.use_) {
            (crate::herbs::Rarity::Legendary, _) => 1,
            (_, Some(crate::herbs::Use::Cooking)) => 2,
            _ => 0,
        };
        assert!(
            uses(p.item) >= need,
            "herb {} is in fewer than {need} recipes",
            p.item
        );
    }
    for l in crate::fish::shipped_legends() {
        assert!(uses(l.item) >= 1, "legend {} is in no recipe", l.item);
    }
    for c in &file.cut {
        assert!(
            crate::fish::fish(c.fish).is_some_and(|f| matches!(
                f.rarity,
                crate::fish::Rarity::Common | crate::fish::Rarity::Uncommon
            )),
            "cut fish {} is no common or uncommon [[fish]]",
            c.fish
        );
        assert!(
            crate::fish::bait(c.bait).is_some()
                && crate::items::template(c.bait)
                    .is_some_and(|t| t.journal == Some(Profession::Fishing)),
            "cut bait {} is no [[bait]] row and Fishing item",
            c.bait
        );
    }
    for k in &file.kitchen {
        for s in &k.starters {
            assert!(
                file.recipe
                    .iter()
                    .any(|r| &r.key == s && r.kind == Kind::Tier),
                "kitchen {}'s starter {s} is no tier recipe",
                k.key
            );
        }
    }
}

/// Parses and checks a `cooking.toml`'s text.
fn load(text: &str) -> File {
    let file: File = toml::from_str(text).expect("content/cooking.toml parses");
    check(&file);
    file
}

fn file() -> &'static File {
    static FILE: OnceLock<File> = OnceLock::new();
    FILE.get_or_init(|| load(include_str!("../content/cooking.toml")))
}

/// Recipes and items a test added, in `cooking.toml`'s shape.
#[derive(Deserialize)]
struct Added {
    #[serde(default)]
    recipe: Vec<Recipe>,
    #[serde(default)]
    item: Vec<Template>,
}

/// Each added file's text, and what it parsed to.
fn added() -> &'static RwLock<Vec<(String, &'static Added)>> {
    static ADDED: OnceLock<RwLock<Vec<(String, &'static Added)>>> = OnceLock::new();
    ADDED.get_or_init(Default::default)
}

/// Every added file, oldest first.
fn extra() -> Vec<&'static Added> {
    added().read().unwrap().iter().map(|&(_, a)| a).collect()
}

/// Adds the rows in `toml` alongside the shipped ones, for the rest of the process. Parallel tests
/// add the same text; it is added once.
pub(crate) fn add(toml: &str) {
    let parsed: Added = toml::from_str(toml).expect("the added cooking content parses");
    let mut all = added().write().unwrap();
    if all.iter().all(|(text, _)| text != toml) {
        all.push((toml.to_owned(), Box::leak(Box::new(parsed))));
    }
}

/// The item row `entry` a test added, for `crate::items::template`.
pub(crate) fn added_item(entry: u32) -> Option<&'static Template> {
    extra()
        .into_iter()
        .flat_map(|a| &a.item)
        .find(|t| t.entry == entry)
}

pub(crate) fn eat() -> &'static Eat {
    &file().eat
}

pub(crate) fn tiers() -> &'static Tier {
    &file().tier
}

pub(crate) fn effects() -> &'static [Effect] {
    &file().effect
}

pub(crate) fn effect(key: &str) -> Option<&'static Effect> {
    effects().iter().find(|e| e.key == key)
}

pub(crate) fn kitchens() -> &'static [Kitchen] {
    &file().kitchen
}

pub(crate) fn kitchen(key: &str) -> Option<&'static Kitchen> {
    kitchens().iter().find(|k| k.key == key)
}

/// The kitchen whose cook is the Hub NPC `entry`.
pub(crate) fn kitchen_of_cook(entry: u32) -> Option<&'static Kitchen> {
    kitchens().iter().find(|k| k.cook == entry)
}

/// Every recipe, the shipped ones first.
pub(crate) fn recipes() -> Vec<&'static Recipe> {
    file()
        .recipe
        .iter()
        .chain(extra().into_iter().flat_map(|a| &a.recipe))
        .collect()
}

pub(crate) fn recipe(key: &str) -> Option<&'static Recipe> {
    recipes().into_iter().find(|r| r.key == key)
}

/// The recipe that cooks `item`.
pub(crate) fn recipe_of_meal(item: u32) -> Option<&'static Recipe> {
    recipes().into_iter().find(|r| r.meal == item)
}

/// The recipes the first catch or pick of `item` teaches.
pub(crate) fn recipes_taught_by(item: u32) -> Vec<&'static Recipe> {
    recipes()
        .into_iter()
        .filter(|r| r.teacher == Some(item))
        .collect()
}

pub(crate) fn mush() -> &'static Mush {
    &file().mush
}

/// The cut of fish `fish`, if it cuts into a bait.
pub(crate) fn cut_of(fish: u32) -> Option<&'static Cut> {
    file().cut.iter().find(|c| c.fish == fish)
}

pub(crate) fn orders() -> &'static Orders {
    &file().orders
}

/// A cooking herb's category (`[category]`).
pub(crate) fn category_of(item: u32) -> Option<&'static str> {
    file().category.get(&item.to_string()).map(String::as_str)
}

/// How many of `list`'s items a best ordering lines up with `recipe`'s ingredients: the size of
/// the two multisets' overlap.
fn overlap(recipe: &Recipe, list: &[u32]) -> usize {
    let mut left = recipe.ingredients.clone();
    list.iter()
        .filter(|i| {
            let at = left.iter().position(|x| x == *i);
            at.map(|at| left.swap_remove(at)).is_some()
        })
        .count()
}

/// Whether `list` is `recipe`'s ingredients, in any order.
pub(crate) fn matches(recipe: &Recipe, list: &[u32]) -> bool {
    misses(recipe, list) == Some(0)
}

/// How many of `list`'s items are not `recipe`'s, at the best ordering; None for a list of
/// another length (the prototype's `distance`).
pub(crate) fn misses(recipe: &Recipe, list: &[u32]) -> Option<usize> {
    (list.len() == recipe.ingredients.len()).then(|| list.len() - overlap(recipe, list))
}

/// The level of an ingredient: the lowest level a fishing pool lists the fish at, a `[[plant]]`
/// row lists the herb at, or a Legendary Fish's own; 1 for a shared find or an item none lists.
pub(crate) fn ingredient_level(item: u32) -> u8 {
    let pools = crate::fish::pools()
        .flat_map(|p| &p.fish)
        .filter(|f| f.item == item)
        .map(|f| f.level);
    let plants = crate::herbs::zones()
        .into_iter()
        .flat_map(|z| crate::herbs::plants_in(&z.key))
        .chain(crate::herbs::plants_in(crate::herbs::ANY))
        .filter(|p| p.item == item)
        .map(|p| {
            if p.zone == crate::herbs::ANY {
                1
            } else {
                p.level
            }
        });
    let legend = crate::fish::legend(item).map(|l| l.level);
    pools.chain(plants).chain(legend).min().unwrap_or(1) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_file_loads() {
        // The shipped rows: a parallel test may have added one.
        let shipped = &file().recipe;
        assert_eq!(shipped.len(), 93);
        let tier = shipped.iter().filter(|r| r.kind == Kind::Tier).count();
        assert_eq!(tier, 44);
        assert_eq!(effects().len(), 11);
        assert_eq!(kitchens().len(), 2);
        assert_eq!(eat().use_spell, 25695);
        assert_eq!(mush().meal, 962400);
        assert_eq!(cut_of(960401).map(|c| c.bait), Some(960502));
        assert_eq!(category_of(961026), Some("egg"));
        assert_eq!(orders().first_order.item, 960401);
    }

    #[test]
    fn matches_ignores_order() {
        let stamina = recipe("stamina_1").unwrap();
        assert!(matches(stamina, &[961002, 960401, 961008]));
        assert!(!matches(stamina, &[961002, 960401, 961001]));
    }

    #[test]
    fn misses_counts_the_wrong_items_at_the_best_order() {
        let stamina = recipe("stamina_1").unwrap();
        assert_eq!(misses(stamina, &[960401, 961002, 961001]), Some(1));
        assert_eq!(misses(stamina, &[960401, 960401, 960401]), Some(2));
        assert_eq!(misses(stamina, &[960401, 961002]), None);
    }

    #[test]
    fn an_ingredient_takes_its_lowest_level() {
        assert_eq!(ingredient_level(960423), 24);
        assert_eq!(ingredient_level(961026), 1);
    }

    #[test]
    fn a_meal_and_a_teacher_find_their_recipes() {
        assert_eq!(
            recipe_of_meal(962001).map(|r| r.key.as_str()),
            Some("stamina_1")
        );
        let taught: Vec<&str> = recipes_taught_by(960409)
            .iter()
            .map(|r| r.key.as_str())
            .collect();
        assert_eq!(taught, ["dish_gilded_koi"]);
    }

    #[test]
    fn the_meals_and_cut_baits_are_items() {
        let meal = crate::items::template(962001).unwrap();
        assert_eq!(meal.use_spell, 25695);
        let bait = crate::items::template(960501).unwrap();
        assert_eq!(bait.journal, Some(Profession::Fishing));
    }

    #[test]
    fn an_added_recipe_and_item_are_found() {
        add(r#"
            item = [{ entry = 969999, name = "Test Meal", use_spell = 25695 }]

            [[recipe]]
            key = "test_meal"
            kind = "tier"
            meal = 969999
            ingredients = [960401]
            effect = "stamina"
            tier = 0
        "#);
        assert_eq!(
            recipe_of_meal(969999).map(|r| r.key.as_str()),
            Some("test_meal")
        );
        assert!(crate::items::template(969999).is_some());
    }

    #[test]
    #[should_panic(expected = "take the same ingredients")]
    fn two_recipes_with_one_multiset_are_refused() {
        let text = include_str!("../content/cooking.toml").replacen(
            "ingredients = [960411, 961010, 961020]",
            "ingredients = [961002, 960401, 961008]",
            1,
        );
        load(&text);
    }

    #[test]
    #[should_panic(expected = "not one each of 0 to 3")]
    fn an_effect_missing_a_tier_is_refused() {
        let text = include_str!("../content/cooking.toml").replacen(
            "effect = \"stamina\"\ntier = 1",
            "effect = \"stamina\"\ntier = 2",
            1,
        );
        load(&text);
    }
}
