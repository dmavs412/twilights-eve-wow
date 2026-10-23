//! Fishing's content (`content/fish.toml`, ADR 0018): the cast, the Reeling Bar's tunables, Angler
//! levels, the fish, their pools by zone, the Fishing Waters, lures and bait. The file's item rows
//! are item templates, which `crate::items` loads.

// The fishing rules (#327 on) read these; until they land nothing does.
#![allow(dead_code)]

use std::sync::OnceLock;

use serde::Deserialize;

/// The cast (`[spell]`).
#[derive(Deserialize)]
pub(crate) struct Spell {
    /// The Fishing spell, whose effect summons the bobber.
    pub(crate) id: u32,
    /// The weapon subclass the main hand needs: a fishing pole.
    pub(crate) pole_subclass: u32,
    pub(crate) bobber_entry: u32,
    pub(crate) bobber_display: u32,
    /// `[min, max]` yards ahead of the caster the bobber lands.
    pub(crate) land_yd: [f32; 2],
    /// How long the bobber floats before it is lost unanswered.
    pub(crate) cast_secs: f32,
    /// `[min, max]` seconds from landing to the bite.
    pub(crate) bite_secs: [f32; 2],
}

/// The Reeling Bar's tunables (`[bar]`); fill and zone heights are fractions of the track.
#[derive(Deserialize)]
pub(crate) struct Bar {
    pub(crate) start_fill: f32,
    pub(crate) fill_per_sec: f32,
    pub(crate) drain_per_sec: f32,
    pub(crate) timeout_secs: f32,
    pub(crate) zone_base: f32,
    pub(crate) zone_per_level: f32,
    pub(crate) zone_max: f32,
    pub(crate) difficulty_per_level_above: f32,
    pub(crate) gate_levels_above: u32,
    pub(crate) treasure_base: f32,
    pub(crate) treasure_per_level: f32,
    pub(crate) treasure_fill_secs: f32,
    pub(crate) treasure_appears_secs: [f32; 2],
}

/// Angler levels and XP (`[angler]`).
#[derive(Deserialize)]
pub(crate) struct Angler {
    pub(crate) cap: u32,
    pub(crate) xp_base: u32,
    pub(crate) xp_per_level: u32,
    pub(crate) catch_xp_base: u32,
    pub(crate) catch_xp_per_level: u32,
    pub(crate) rarity_factor: RarityFactor,
    pub(crate) perfect_factor: f32,
    pub(crate) legend_bonus_xp: u32,
}

impl Angler {
    /// The XP from Angler level `level` to the next: base + per level x level.
    pub(crate) fn xp_to_next(&self, level: u32) -> u32 {
        self.xp_base + self.xp_per_level * level
    }
}

/// A catch's XP factor by its fish's rarity.
#[derive(Deserialize)]
pub(crate) struct RarityFactor {
    pub(crate) common: f32,
    pub(crate) uncommon: f32,
    pub(crate) rare: f32,
}

impl RarityFactor {
    pub(crate) fn of(&self, rarity: Rarity) -> f32 {
        match rarity {
            Rarity::Common => self.common,
            Rarity::Uncommon => self.uncommon,
            Rarity::Rare => self.rare,
        }
    }
}

/// How a fish moves on the Reeling Bar's track.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Style {
    Smooth,
    Mixed,
    Sinker,
    Floater,
    Dart,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Rarity {
    Common,
    Uncommon,
    Rare,
}

/// One fish (`[[fish]]`), by its item.
#[derive(Deserialize)]
pub(crate) struct Fish {
    pub(crate) item: u32,
    pub(crate) style: Style,
    /// 0 to 100.
    pub(crate) difficulty: u32,
    pub(crate) rarity: Rarity,
}

/// One zone's catch (`[[pool]]`). Its fish and treasure weights sum to 100.
#[derive(Deserialize)]
pub(crate) struct Pool {
    pub(crate) key: String,
    /// The zone's `AreaTable.dbc` id.
    pub(crate) zone: u32,
    pub(crate) fish: Vec<PoolFish>,
    pub(crate) treasure: Vec<Treasure>,
    /// A treasure chest's `[min, max]` copper.
    pub(crate) copper: [u32; 2],
}

/// A fish in a pool: its weight and the level it is caught at.
#[derive(Deserialize)]
pub(crate) struct PoolFish {
    pub(crate) item: u32,
    pub(crate) weight: f32,
    pub(crate) level: u32,
}

/// A treasure roll's item and weight.
#[derive(Deserialize)]
pub(crate) struct Treasure {
    pub(crate) item: u32,
    pub(crate) weight: f32,
}

/// A Fishing Water (`[[water]]`): a named stretch of water, by area or by circle.
#[derive(Deserialize)]
pub(crate) struct Water {
    pub(crate) key: String,
    pub(crate) name: String,
    /// The `AreaTable.dbc` id it covers.
    #[serde(default)]
    pub(crate) area: Option<u32>,
    /// `[map, x, y, radius]`.
    #[serde(default)]
    pub(crate) circle: Option<[f32; 4]>,
    /// Its pool's key; `None` fishes its zone's pool.
    #[serde(default)]
    pub(crate) pool: Option<String>,
}

/// What a lure or a bait does to a cast; each key 0 when not set.
#[derive(Deserialize, Default)]
pub(crate) struct Effect {
    /// Counts as this many Angler levels more.
    #[serde(default)]
    pub(crate) angler_levels: u32,
    /// The bite comes this percent sooner (negative) or later.
    #[serde(default)]
    pub(crate) bite_wait_pct: i32,
    /// The catch zone is this percent taller.
    #[serde(default)]
    pub(crate) zone_pct: i32,
    /// Percentage points more treasure chance.
    #[serde(default)]
    pub(crate) treasure_points: u32,
}

/// A stock lure (`[[lure]]`): a buff on the pole.
#[derive(Deserialize)]
pub(crate) struct Lure {
    pub(crate) item: u32,
    pub(crate) duration_secs: u32,
    #[serde(flatten)]
    pub(crate) effect: Effect,
}

/// A custom bait (`[[bait]]`), used up one per cast.
#[derive(Deserialize)]
pub(crate) struct Bait {
    pub(crate) item: u32,
    #[serde(flatten)]
    pub(crate) effect: Effect,
}

#[derive(Deserialize)]
struct File {
    spell: Spell,
    bar: Bar,
    angler: Angler,
    fish: Vec<Fish>,
    pool: Vec<Pool>,
    water: Vec<Water>,
    lure: Vec<Lure>,
    bait: Vec<Bait>,
}

fn file() -> &'static File {
    static FILE: OnceLock<File> = OnceLock::new();
    FILE.get_or_init(|| {
        toml::from_str(include_str!("../content/fish.toml")).expect("content/fish.toml parses")
    })
}

pub(crate) fn spell() -> &'static Spell {
    &file().spell
}

pub(crate) fn bar() -> &'static Bar {
    &file().bar
}

pub(crate) fn angler() -> &'static Angler {
    &file().angler
}

/// The fish caught as `item`.
pub(crate) fn fish(item: u32) -> Option<&'static Fish> {
    file().fish.iter().find(|f| f.item == item)
}

/// The pool of the zone `zone` (`AreaTable.dbc` id).
pub(crate) fn pool_for_zone(zone: u32) -> Option<&'static Pool> {
    file().pool.iter().find(|p| p.zone == zone)
}

/// The pool named `key`.
pub(crate) fn pool(key: &str) -> Option<&'static Pool> {
    file().pool.iter().find(|p| p.key == key)
}

/// The Fishing Water named `key`.
pub(crate) fn water(key: &str) -> Option<&'static Water> {
    file().water.iter().find(|w| w.key == key)
}

/// Every Fishing Water.
pub(crate) fn waters() -> &'static [Water] {
    &file().water
}

/// The lure that is item `item`.
pub(crate) fn lure(item: u32) -> Option<&'static Lure> {
    file().lure.iter().find(|l| l.item == item)
}

/// The bait that is item `item`.
pub(crate) fn bait(item: u32) -> Option<&'static Bait> {
    file().bait.iter().find(|b| b.item == item)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_loads_and_its_accessors_answer() {
        assert_eq!(spell().id, 7620);
        assert_eq!(angler().xp_to_next(12), 940);
        assert_eq!(fish(6310).map(|f| f.style), Some(Style::Dart));
        assert_eq!(pool_for_zone(12).map(|p| p.key.as_str()), Some("elwynn"));
        assert!(water("cathedral_canal").is_some_and(|w| w.circle.is_some()));
        assert_eq!(lure(6530).map(|l| l.effect.bite_wait_pct), Some(-30));
        assert_eq!(bait(960102).map(|b| b.effect.treasure_points), Some(10));
        assert_eq!(bait(960105).map(|b| b.effect.angler_levels), Some(0));
    }
}
