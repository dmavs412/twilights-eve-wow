//! What a kill drops (spec #1, "Loot"), and some copper. Above level 10, or off Eastern
//! Kingdoms: real 1.12 world-drop items near the mob's level, mostly grey and white with a green
//! now and then. The item pools are the reference's (`docs/reference/items-and-vendors.md`):
//! every world-drop item, grouped by the level it drops at and its quality; the Gear rows of
//! pool level 12 and under have left them. On Eastern Kingdoms at level 10 and under, the
//! Northshire roll instead (spec #272, `content/northshire_loot.toml`): 8% a Token white, 4% a
//! World green within a level of the mob's, nothing else and no grey. The spread and the money
//! are this slice's tuning, not vanilla's.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::items;
use crate::rng::Rng;
use crate::tables::MAX_LEVEL;

/// An item's quality (`ItemQualities`), which colours its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Quality {
    Poor = 0,
    Common = 1,
    Uncommon = 2,
    Rare = 3,
}

impl Quality {
    fn from_u8(q: u8) -> Option<Self> {
        match q {
            0 => Some(Self::Poor),
            1 => Some(Self::Common),
            2 => Some(Self::Uncommon),
            3 => Some(Self::Rare),
            _ => None,
        }
    }

    /// The next quality down, which a pool falls back to when there is none of this one.
    fn lower(self) -> Option<Self> {
        match self {
            Self::Poor => None,
            Self::Common => Some(Self::Poor),
            Self::Uncommon => Some(Self::Common),
            Self::Rare => Some(Self::Uncommon),
        }
    }
}

/// A creature's loot roll as data: `chance` percent to drop one entry of `one_of`.
#[derive(Debug, Clone, Deserialize)]
pub struct Roll {
    pub chance: u8,
    pub one_of: Vec<u32>,
}

impl Roll {
    /// Panics when the roll is not content-valid: `chance` 1 to 100, `one_of` not empty and
    /// every entry an item. `what` names the owner in the message.
    pub(crate) fn check(&self, what: &str) {
        assert!(
            (1..=100).contains(&self.chance),
            "{what}: a roll's chance is {}, not 1 to 100",
            self.chance
        );
        assert!(!self.one_of.is_empty(), "{what}: a roll has no items");
        for &item in &self.one_of {
            assert!(
                items::template(item).is_some(),
                "{what}: a roll names item {item}, which the content does not have"
            );
        }
    }
}

/// The item of the first roll in `rolls` that succeeds. Each roll in order draws `0..100`; a draw
/// under its `chance` returns one of its `one_of` drawn evenly, and none succeeding gives `None`.
pub(crate) fn first_roll(rolls: &[Roll], rng: &mut Rng) -> Option<u32> {
    rolls.iter().find_map(|roll| {
        (rng.below(100) < u32::from(roll.chance))
            .then(|| roll.one_of[rng.below(roll.one_of.len() as u32) as usize])
    })
}

/// The quality roll is out of this.
pub const QUALITY_ROLL: u32 = 100;

/// The drop's quality for a roll in `0..QUALITY_ROLL`: 60 grey, 30 white, 10 green.
pub fn quality(roll: u32) -> Quality {
    match roll {
        0..60 => Quality::Poor,
        60..90 => Quality::Common,
        _ => Quality::Uncommon,
    }
}

/// How far either side of the mob's level its drops come from.
const WINDOW: u8 = 2;
/// How much further a window may widen looking for an item of the rolled quality: up to 6 levels.
const MAX_WIDEN: u8 = 4;

/// The items a mob of `level` drops at `quality`: every one whose pool level is within 2 of the
/// mob's. Where there are none (vanilla has no greens under level 7, and few whites at 16-20),
/// the window widens a level each side until it finds some, and a quality with none at all near
/// the level falls back to the next one down.
pub fn pool(level: u8, quality: Quality) -> &'static [u32] {
    static POOLS: OnceLock<HashMap<(u8, Quality), Vec<u32>>> = OnceLock::new();
    let pools = POOLS.get_or_init(|| {
        let mut pools = HashMap::new();
        for level in 1..=MAX_LEVEL {
            for q in [
                Quality::Poor,
                Quality::Common,
                Quality::Uncommon,
                Quality::Rare,
            ] {
                pools.insert((level, q), build_pool(level, q));
            }
        }
        pools
    });
    pools
        .get(&(level.clamp(1, MAX_LEVEL), quality))
        .map_or(&[], Vec::as_slice)
}

fn build_pool(level: u8, quality: Quality) -> Vec<u32> {
    let mut q = Some(quality);
    while let Some(quality) = q {
        for widen in 0..=MAX_WIDEN {
            let reach = WINDOW + widen;
            let pool = window(
                level.saturating_sub(reach),
                level.saturating_add(reach),
                quality,
            );
            if !pool.is_empty() {
                return pool;
            }
        }
        q = quality.lower();
    }
    Vec::new()
}

/// What the Northshire roll picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    /// One of the 22 Token whites.
    White,
    /// A World green within a level of the mob's.
    Green,
}

/// The Northshire roll is out of this.
pub const NORTHSHIRE_ROLL: u32 = 100;
/// The highest mob level the Northshire roll runs at; above it, the pools.
pub const NORTHSHIRE_MAX_LEVEL: u8 = 10;
/// The share of kills that drop a Token white, in hundredths.
const WHITE_PCT: u32 = 8;
/// The share of kills that drop a World green, in hundredths.
const GREEN_PCT: u32 = 4;
/// The World greens run from level 5 to 8: a mob's level is clamped into that before the
/// within-one match.
const GREEN_LEVELS: (u8, u8) = (5, 8);

/// The Northshire roll for a mob of `level` and a roll in `0..NORTHSHIRE_ROLL`: 8% a white,
/// 4% a green, else nothing; nothing at all above level 10, where the pools roll.
pub fn northshire_roll(level: u8, roll: u32) -> Option<Pick> {
    if level > NORTHSHIRE_MAX_LEVEL {
        return None;
    }
    if roll < WHITE_PCT {
        Some(Pick::White)
    } else if roll < WHITE_PCT + GREEN_PCT {
        Some(Pick::Green)
    } else {
        None
    }
}

#[derive(Deserialize)]
struct NorthshireFile {
    white: Vec<u32>,
    green: Vec<GreenRow>,
}

#[derive(Deserialize)]
struct GreenRow {
    entry: u32,
    level: u8,
}

fn northshire() -> &'static NorthshireFile {
    static FILE: OnceLock<NorthshireFile> = OnceLock::new();
    FILE.get_or_init(|| {
        toml::from_str(include_str!("../../content/northshire_loot.toml"))
            .expect("content/northshire_loot.toml parses")
    })
}

/// The 22 Token whites the Northshire roll draws from.
pub fn northshire_whites() -> &'static [u32] {
    &northshire().white
}

/// Every World green with its level, as the content lists them.
pub fn northshire_green_rows() -> Vec<(u32, u8)> {
    northshire()
        .green
        .iter()
        .map(|g| (g.entry, g.level))
        .collect()
}

/// The World greens a mob of `level` may drop: those within one level of the mob's, the mob's
/// level first clamped to the greens' 5 to 8.
pub fn northshire_greens(level: u8) -> &'static [u32] {
    static BY_LEVEL: OnceLock<Vec<Vec<u32>>> = OnceLock::new();
    let by_level = BY_LEVEL.get_or_init(|| {
        (0..=NORTHSHIRE_MAX_LEVEL)
            .map(|level| {
                let l = level.clamp(GREEN_LEVELS.0, GREEN_LEVELS.1);
                northshire()
                    .green
                    .iter()
                    .filter(|g| g.level.abs_diff(l) <= 1)
                    .map(|g| g.entry)
                    .collect()
            })
            .collect()
    });
    by_level.get(usize::from(level)).map_or(&[], Vec::as_slice)
}

/// The level an item drops at, for the items any pool holds.
pub fn pool_level(entry: u32) -> Option<u8> {
    items::template(entry)
        .filter(|t| Quality::from_u8(t.quality).is_some())
        .map(|t| t.pool_level)
}

/// A mob's copper, `(min, max)`: `L + LÂ²/4`, up to twice that.
pub fn money(level: u8) -> (u32, u32) {
    let l = u32::from(level);
    let min = l + l * l / 4;
    (min, 2 * min)
}

/// The items with a pool level in `lo..=hi` at `quality`, sorted by entry.
fn window(lo: u8, hi: u8, quality: Quality) -> Vec<u32> {
    let mut entries: Vec<u32> = items::all()
        .values()
        .filter(|t| {
            Quality::from_u8(t.quality) == Some(quality) && (lo..=hi).contains(&t.pool_level)
        })
        .map(|t| t.entry)
        .collect();
    entries.sort_unstable();
    entries
}

/// How many levels a Dungeon boss's band spans: the runner's level and the four below it.
const BOSS_BAND: u8 = 5;

/// The blues a Dungeon boss drops for a runner of `level`, out of `blues` (`(entry, required
/// level)`), in their order: those of the run's level band, the five levels up to the runner's,
/// which they can wear now. Below every band it is the lowest band there is, above every band the
/// highest.
pub fn boss_band(level: u8, blues: &[(u32, u8)]) -> Vec<u32> {
    let within = |lo: u8, hi: u8| -> Vec<u32> {
        blues
            .iter()
            .filter(|(_, required)| (lo..=hi).contains(required))
            .map(|(entry, _)| *entry)
            .collect()
    };
    let band = within(level.saturating_sub(BOSS_BAND - 1), level);
    if !band.is_empty() {
        return band;
    }
    let required = || blues.iter().map(|(_, r)| *r);
    if let Some(lowest) = required().filter(|&r| r > level).min() {
        return within(lowest, lowest.saturating_add(BOSS_BAND - 1));
    }
    required().max().map_or_else(Vec::new, |highest| {
        within(highest.saturating_sub(BOSS_BAND - 1), highest)
    })
}

#[cfg(test)]
mod roll_tests {
    use super::*;

    fn creature_rolls(guid: u32) -> &'static [Roll] {
        crate::phased::rolls(crate::content::EASTERN_KINGDOMS, guid)
    }

    #[test]
    fn a_chance_of_100_always_drops_one_of_its_items_and_none_drops_nothing() {
        let sure = [Roll {
            chance: 100,
            one_of: vec![7, 8],
        }];
        let mut rng = Rng::new(1);
        assert!((0..50).all(|_| matches!(first_roll(&sure, &mut rng), Some(7 | 8))));
        assert_eq!(first_roll(&[], &mut rng), None);
    }

    #[test]
    fn the_abbots_200_kills_each_drop_one_of_22_and_the_epic_comes_3_to_25_times() {
        let rolls = creature_rolls(780015);
        let epic = 900599;
        let mut epics = 0;
        for seed in 0..200 {
            let item = first_roll(rolls, &mut Rng::new(seed)).expect("the Abbot always drops");
            assert!(
                item == epic || rolls[1].one_of.contains(&item),
                "item {item} is not among his 22"
            );
            epics += usize::from(item == epic);
        }
        assert_eq!(rolls.iter().flat_map(|r| &r.one_of).count(), 22);
        assert!((3..=25).contains(&epics), "{epics} epics in 200 kills");
    }

    #[test]
    fn a_disciple_drops_one_of_the_21_about_1_percent_of_the_time() {
        let rolls = creature_rolls(780010);
        assert_eq!(rolls.iter().flat_map(|r| &r.one_of).count(), 21);
        let drops = (0..1000)
            .filter_map(|seed| first_roll(rolls, &mut Rng::new(seed)))
            .inspect(|item| assert!(rolls[0].one_of.contains(item)))
            .count();
        assert!((3..=25).contains(&drops), "{drops} drops in 1000 kills");
    }

    #[test]
    fn padfoot_drops_one_of_his_five_half_the_time_and_never_two() {
        let rolls = &crate::mobs::template(103).expect("Padfoot").rolls;
        let five = [900404, 900405, 900406, 900407, 900408];
        let drops = (0..200)
            .filter_map(|seed| first_roll(rolls, &mut Rng::new(seed)))
            .inspect(|item| assert!(five.contains(item)))
            .count();
        assert!((70..=130).contains(&drops), "{drops} drops in 200 kills");
    }

    #[test]
    fn vurx_rolls_one_of_his_five() {
        let rolls = &crate::mobs::template(96002).expect("Vurx").rolls;
        assert_eq!(rolls.len(), 1);
        assert_eq!(rolls[0].one_of, [900409, 900410, 900411, 900412, 900413]);
    }

    #[test]
    fn princess_rolls_her_elite_pool_then_her_bands_greens() {
        let rolls = &crate::mobs::template(330).expect("Princess").rolls;
        assert_eq!(rolls.len(), 2);
        let pool = 904000..=904005;
        let greens = 902000..=902019;
        let drops: Vec<u32> = (0..400)
            .filter_map(|seed| first_roll(rolls, &mut Rng::new(seed)))
            .collect();
        assert!(drops.iter().any(|i| pool.contains(i)), "no elite blue");
        assert!(drops.iter().any(|i| greens.contains(i)), "no green");
        assert!(drops.iter().all(|i| pool.contains(i) || greens.contains(i)));
    }
}
