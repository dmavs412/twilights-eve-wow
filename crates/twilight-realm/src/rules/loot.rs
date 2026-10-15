//! What a kill drops (spec #1, "Loot"): real 1.12 world-drop items near the mob's level, mostly
//! grey and white with a green now and then, and some copper. The item pools are the reference's
//! (`docs/reference/items-and-vendors.md`): every world-drop item, grouped by the level it drops
//! at and its quality. The spread and the money are this slice's tuning, not vanilla's.

use std::collections::HashMap;
use std::sync::OnceLock;

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
