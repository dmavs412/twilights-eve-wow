//! Herbalism's content (`content/herbs.toml`, ADR 0024): the Herb Patch, the Forager's Grid's
//! tunables, Herbalist levels, the plants of each zone, the tools, the consumables and the perks.
//! The file's item rows are item templates, which `crate::items` loads.

// The Herbalist, the patches and the grid (#438 to #440) read the rest.
#![allow(dead_code)]

use std::sync::{OnceLock, RwLock};

use serde::Deserialize;

use crate::fish::Block;

/// The two spells (`[spell]`).
#[derive(Deserialize)]
pub(crate) struct Spell {
    /// The spell teaching Herbalism adds to the spellbook (Herb Gathering).
    pub(crate) learn: u32,
    /// Find Herbs, which toggles herb tracking.
    pub(crate) find: u32,
}

/// The Herb Patch GameObject (`[patch]`).
#[derive(Deserialize)]
pub(crate) struct Patch {
    pub(crate) entry: u32,
    pub(crate) display: u32,
    pub(crate) name: String,
    /// Real seconds before a finished patch is replaced.
    pub(crate) respawn_secs: f32,
}

/// The Forager's Grid's tunables (`[grid]`).
#[derive(Deserialize)]
pub(crate) struct Grid {
    pub(crate) size: u32,
    pub(crate) pulls: u32,
    /// `[min, max]` plants rolled into a patch.
    pub(crate) plants: [u32; 2],
    pub(crate) thorns: u32,
    /// Extra pulls a pulled thorn costs.
    pub(crate) thorn_cost: u32,
    /// The chance per patch that the zone's legendary replaces a rolled plant, in its Bloom.
    pub(crate) legendary_chance: f32,
    /// A plant in its Bloom weighs and yields this much more.
    pub(crate) bloom_factor: f32,
    /// Weight added per Herbalist level above 1, as a fraction of the base weight.
    pub(crate) uncommon_boost: f32,
    pub(crate) rare_boost: f32,
    pub(crate) sweep_cost: u32,
    pub(crate) sweep_length: u32,
    pub(crate) probe_cost: u32,
    pub(crate) snip_cost: u32,
    pub(crate) snip_bonus: u32,
}

/// A colour band: the factor for a plant at most `up_to` levels below the Herbalist.
#[derive(Deserialize)]
pub(crate) struct Colour {
    pub(crate) up_to: u32,
    pub(crate) factor: f32,
}

/// A title band: held from Herbalist level `from` until the next band.
#[derive(Deserialize)]
pub(crate) struct Title {
    pub(crate) from: u32,
    pub(crate) title: String,
}

/// Herbalist levels and XP (`[herbalist]`).
#[derive(Deserialize)]
pub(crate) struct Herbalist {
    pub(crate) cap: u32,
    pub(crate) xp_base: u32,
    pub(crate) xp_per_level: u32,
    pub(crate) plant_xp_base: u32,
    pub(crate) plant_xp_per_level: u32,
    pub(crate) weed_xp: u32,
    pub(crate) thorn_xp: u32,
    /// Rising by `up_to`; a gap past the last band gives 0.
    pub(crate) colours: Vec<Colour>,
    /// One perk slot per this many Herbalist levels.
    pub(crate) perk_every: u32,
    /// The pick counts that unlock a plant's three tiers of facts.
    pub(crate) reveal_at: [u32; 3],
    /// What the first perk reset costs; it doubles with each reset.
    pub(crate) reset_base_copper: u32,
    /// Title bands, rising by `from`.
    pub(crate) titles: Vec<Title>,
}

impl Herbalist {
    /// The title of the last band whose `from` is at or below `level`.
    pub(crate) fn title(&self, level: u32) -> &str {
        self.titles
            .iter()
            .rev()
            .find(|t| t.from <= level)
            .map_or("", |t| t.title.as_str())
    }

    /// The XP from Herbalist level `level` to the next: base + per level x level.
    pub(crate) fn xp_to_next(&self, level: u32) -> u32 {
        self.xp_base + self.xp_per_level * level
    }

    /// The factor on a plant's XP when the Herbalist is `above` levels over it.
    pub(crate) fn colour_factor(&self, above: u32) -> f32 {
        self.colours
            .iter()
            .find(|c| above <= c.up_to)
            .map_or(0.0, |c| c.factor)
    }
}

/// A herb zone (`[[zone]]`).
#[derive(Deserialize)]
pub(crate) struct Zone {
    pub(crate) key: String,
    /// The zone's `AreaTable.dbc` id.
    pub(crate) zone: u32,
    /// Patches kept live per character.
    pub(crate) live: u32,
    /// Thorns in this zone's patches in place of `[grid]`'s; a test zone sets 0.
    #[serde(default)]
    pub(crate) thorns: Option<u32>,
    /// The Herb Spots.
    pub(crate) points: Vec<Point>,
}

/// A Herb Spot (`{ x = .., y = .., z = .. }`). Its Z is the terrain's, so a patch grown where the
/// host has no ground loaded still stands on the grass (#477).
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Point {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) z: f32,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Rarity {
    Common,
    Uncommon,
    Rare,
    Legendary,
}

/// What a plant is for.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Use {
    Cooking,
    Alchemy,
}

/// The zone key of the finds every zone shares.
pub(crate) const ANY: &str = "any";

/// One pool row (`[[plant]]`): a plant as one zone grows it.
#[derive(Deserialize)]
pub(crate) struct Plant {
    /// A `[[zone]]` key, or [`ANY`].
    pub(crate) zone: String,
    pub(crate) item: u32,
    pub(crate) rarity: Rarity,
    /// The Herbalist level it needs.
    pub(crate) level: u32,
    pub(crate) bloom: Option<Block>,
    /// It grows only in its Bloom.
    #[serde(default)]
    pub(crate) timed: bool,
    pub(crate) weight: u32,
    #[serde(rename = "use")]
    pub(crate) use_: Option<Use>,
    /// The Lost Coin Purse: `[min, max]` copper it pays at once.
    pub(crate) money: Option<[u32; 2]>,
}

/// What a tool gives (`[[tool]]`).
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Skill {
    Sweep,
    Probe,
    Snip,
}

#[derive(Deserialize)]
pub(crate) struct Tool {
    pub(crate) item: u32,
    pub(crate) skill: Skill,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Effect {
    Tea,
    Sight,
    Salve,
    Draught,
    Twine,
    Charm,
    Moonwater,
}

/// A consumable (`[[consumable]]`), one used per patch.
#[derive(Deserialize)]
pub(crate) struct Consumable {
    pub(crate) item: u32,
    pub(crate) effect: Effect,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum PerkKind {
    Boon,
    Pact,
}

/// A perk (`[[perk]]`).
#[derive(Deserialize)]
pub(crate) struct Perk {
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) kind: PerkKind,
    pub(crate) text: String,
}

#[derive(Deserialize)]
struct File {
    spell: Spell,
    patch: Patch,
    grid: Grid,
    herbalist: Herbalist,
    zone: Vec<Zone>,
    plant: Vec<Plant>,
    tool: Vec<Tool>,
    consumable: Vec<Consumable>,
    perk: Vec<Perk>,
}

/// Checks a parsed `herbs.toml`, panicking on the first row that breaks a rule.
fn check(file: &File) {
    for p in &file.plant {
        assert!(
            crate::items::template(p.item).is_some(),
            "plant {} has no item row",
            p.item
        );
        assert!(
            p.zone == ANY || file.zone.iter().any(|z| z.key == p.zone),
            "plant {} names zone {:?}, which no [[zone]] names",
            p.item,
            p.zone
        );
        assert!(
            p.rarity != Rarity::Legendary || (p.timed && p.zone != ANY),
            "legendary {} must be timed and grow in one zone",
            p.item
        );
    }
    for z in &file.zone {
        let legendaries = file
            .plant
            .iter()
            .filter(|p| p.zone == z.key && p.rarity == Rarity::Legendary)
            .count();
        assert!(
            legendaries == 1,
            "zone {} has {legendaries} legendaries, not one",
            z.key
        );
        assert!(
            z.points.len() >= z.live as usize,
            "zone {} has {} points, fewer than its {} live patches",
            z.key,
            z.points.len(),
            z.live
        );
    }
    let titles = &file.herbalist.titles;
    assert!(
        titles.first().is_some_and(|t| t.from == 1)
            && titles.windows(2).all(|w| w[0].from < w[1].from),
        "[herbalist] titles must start at level 1 and rise"
    );
}

/// Parses and checks a `herbs.toml`'s text.
fn load(text: &str) -> File {
    let file: File = toml::from_str(text).expect("content/herbs.toml parses");
    check(&file);
    file
}

fn file() -> &'static File {
    static FILE: OnceLock<File> = OnceLock::new();
    FILE.get_or_init(|| load(include_str!("../content/herbs.toml")))
}

/// Zones, plants, tools, consumables and perks a test added (`Realm::with_herbs`), in
/// `herbs.toml`'s shape.
#[derive(Deserialize)]
struct Added {
    #[serde(default)]
    zone: Vec<Zone>,
    #[serde(default)]
    plant: Vec<Plant>,
    #[serde(default)]
    tool: Vec<Tool>,
    #[serde(default)]
    consumable: Vec<Consumable>,
    #[serde(default)]
    perk: Vec<Perk>,
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
    let parsed: Added = toml::from_str(toml).expect("the added herbalism content parses");
    let mut all = added().write().unwrap();
    if all.iter().all(|(text, _)| text != toml) {
        all.push((toml.to_owned(), Box::leak(Box::new(parsed))));
    }
}

pub(crate) fn spell() -> &'static Spell {
    &file().spell
}

pub(crate) fn patch() -> &'static Patch {
    &file().patch
}

pub(crate) fn grid() -> &'static Grid {
    &file().grid
}

pub(crate) fn herbalist() -> &'static Herbalist {
    &file().herbalist
}

/// Every herb zone, the shipped ones first.
pub(crate) fn zones() -> Vec<&'static Zone> {
    file()
        .zone
        .iter()
        .chain(extra().into_iter().flat_map(|a| &a.zone))
        .collect()
}

/// The pool of the zone keyed `zone_key`: its own rows, then the rows every zone shares.
pub(crate) fn plants_in(zone_key: &str) -> Vec<&'static Plant> {
    file()
        .plant
        .iter()
        .chain(extra().into_iter().flat_map(|a| &a.plant))
        .filter(|p| p.zone == zone_key)
        .chain(
            file()
                .plant
                .iter()
                .chain(extra().into_iter().flat_map(|a| &a.plant))
                .filter(|p| p.zone == ANY),
        )
        .collect()
}

/// The shipped `[[plant]]` rows, without any a test added: what a content check reads.
pub(crate) fn shipped_plants() -> &'static [Plant] {
    &file().plant
}

/// The tool whose item is `item`.
pub(crate) fn tool(item: u32) -> Option<&'static Tool> {
    file()
        .tool
        .iter()
        .chain(extra().into_iter().flat_map(|a| &a.tool))
        .find(|t| t.item == item)
}

/// The consumable whose item is `item`.
pub(crate) fn consumable(item: u32) -> Option<&'static Consumable> {
    file()
        .consumable
        .iter()
        .chain(extra().into_iter().flat_map(|a| &a.consumable))
        .find(|c| c.item == item)
}

/// Every perk, the shipped ones first.
pub(crate) fn perks() -> Vec<&'static Perk> {
    file()
        .perk
        .iter()
        .chain(extra().into_iter().flat_map(|a| &a.perk))
        .collect()
}

/// The perk keyed `key`.
pub(crate) fn perk(key: &str) -> Option<&'static Perk> {
    perks().into_iter().find(|p| p.key == key)
}

/// The plant part keys an item row's `part` may hold.
pub(crate) const PARTS: [&str; 10] = [
    "flower",
    "leaf",
    "root",
    "berry",
    "mushroom",
    "nest",
    "nut",
    "honeycomb",
    "sap",
    "purse",
];

/// The plant part of `item`'s row, "" for none.
pub(crate) fn part_of(item: u32) -> &'static str {
    crate::items::template(item)
        .and_then(|t| t.part.as_deref())
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_file_loads() {
        assert_eq!(zones().len(), 3);
        assert_eq!(perks().len(), 9);
        assert_eq!(spell().find, 2383);
        assert_eq!(patch().entry, 981001);
    }

    #[test]
    fn a_zone_pool_holds_its_own_rows_and_the_shared_finds() {
        let elwynn = plants_in("elwynn");
        assert_eq!(elwynn.len(), 15 + 4);
        assert_eq!(plants_in("duskwood").len(), 12 + 4);
    }

    #[test]
    fn every_plant_goes_to_the_herbalism_satchel() {
        for z in zones() {
            for p in plants_in(&z.key) {
                assert_eq!(
                    crate::items::template(p.item).and_then(|t| t.journal),
                    Some(crate::items::Profession::Herbalism),
                    "plant item {} in zone {}",
                    p.item,
                    z.key
                );
            }
        }
    }

    #[test]
    fn every_plant_has_a_part() {
        for z in zones() {
            for p in plants_in(&z.key) {
                assert!(
                    PARTS.contains(&part_of(p.item)),
                    "plant item {} in zone {} has part {:?}",
                    p.item,
                    z.key,
                    part_of(p.item)
                );
            }
        }
        assert_eq!(part_of(961027), "purse");
        assert_eq!(part_of(961101), "");
    }

    #[test]
    fn the_colour_bands_fall_to_grey() {
        let h = herbalist();
        assert_eq!(h.colour_factor(2), 1.0);
        assert_eq!(h.colour_factor(3), 0.6);
        assert_eq!(h.colour_factor(9), 0.25);
        assert_eq!(h.colour_factor(10), 0.0);
        assert_eq!(h.title(12), "Herb Gatherer");
        assert_eq!(h.xp_to_next(3), 100);
    }

    #[test]
    fn the_tools_and_consumables_are_found_by_item() {
        assert_eq!(tool(961102).map(|t| t.skill), Some(Skill::Probe));
        assert_eq!(
            consumable(961207).map(|c| c.effect),
            Some(Effect::Moonwater)
        );
        assert!(perk("nightowl").is_some_and(|p| p.kind == PerkKind::Pact));
    }

    #[test]
    #[should_panic(expected = "which no [[zone]] names")]
    fn a_plant_in_an_unknown_zone_is_refused() {
        let text = include_str!("../content/herbs.toml").replacen(
            "zone = \"elwynn\"\nitem = 2447",
            "zone = \"nowhere\"\nitem = 2447",
            1,
        );
        load(&text);
    }

    #[test]
    #[should_panic(expected = "must be timed")]
    fn an_untimed_legendary_is_refused() {
        let text = include_str!("../content/herbs.toml").replacen(
            "item = 961009\nrarity = \"legendary\"\nlevel = 14\nbloom = \"night\"\ntimed = true",
            "item = 961009\nrarity = \"legendary\"\nlevel = 14\nbloom = \"night\"",
            1,
        );
        load(&text);
    }
}
