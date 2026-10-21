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
