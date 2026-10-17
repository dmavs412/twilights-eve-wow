//! Kill XP and the level table (vmangos `Formulas.h` `MaNGOS::XP`, `player_xp_for_level`).

use crate::tables::{self, MAX_LEVEL};

/// Whether a kill is an elite's, and where: vmangos doubles an elite's XP, and gives 2.5× inside
/// a (non-raid) dungeon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Elite {
    No,
    OpenWorld,
    Dungeon,
}

/// XP from `level` to the next; `None` at the end of the game (or for level 0).
pub fn xp_to_next(level: u8) -> Option<u32> {
    if level == 0 || level >= MAX_LEVEL {
        return None;
    }
    tables::levels()
        .xp_to_next
        .get(usize::from(level) - 1)
        .copied()
}

/// What discovering an area pays a character of `level` before the Tier XP Rate: an eighth of the
/// XP to the next level; nothing at the end of the game.
pub fn discovery_xp(level: u8) -> u32 {
    xp_to_next(level).map_or(0, |next| next / 8)
}

/// The Tier XP Rate (`GLOSSARY.md`): what every XP award a character of `tier` earns is
/// multiplied by, before the dev multiplier. ×1 at Tier 0, ×3 at Tier 1, ×5 at Tier 2, ×7 at
/// Tier 3.
pub fn tier_rate(tier: u8) -> u32 {
    2 * u32::from(tier) + 1
}

/// The highest mob level that is gray (worth nothing) to a player of `level` (`GetGrayLevel`).
pub fn gray_level(level: u8) -> u8 {
    match level {
        0..=5 => 0,
        6..=39 => level - 5 - level / 10,
        _ => level - 1 - level / 5,
    }
}

/// `GetZeroDifference`: how many levels below the player a mob's XP reaches zero.
fn zero_difference(level: u8) -> u32 {
    match level {
        0..=7 => 5,
        8..=9 => 6,
        10..=11 => 7,
        12..=15 => 8,
        16..=19 => 9,
        20..=29 => 11,
        30..=39 => 12,
        40..=44 => 13,
        45..=49 => 14,
        50..=54 => 15,
        55..=59 => 16,
        _ => 17,
    }
}

/// The XP a player of level `player` earns for killing a mob of level `mob` (`MaNGOS::XP::Gain`):
/// 5% more per level above (up to +4), shrinking to nothing at the gray level. Rounded half to
/// even, as `nearbyint` does, from exact integer arithmetic.
pub fn kill_xp(player: u8, mob: u8, elite: Elite) -> u32 {
    let base = u64::from(player) * 5 + 45;
    // xp = num / den, exactly.
    let (mut num, mut den) = if mob >= player {
        (base * (100 + 5 * u64::from((mob - player).min(4))), 100)
    } else if mob > gray_level(player) {
        let zd = u64::from(zero_difference(player));
        (base * (zd - u64::from(player - mob)), zd)
    } else {
        return 0;
    };
    match elite {
        Elite::No => {}
        Elite::OpenWorld => num *= 2,
        Elite::Dungeon => {
            num *= 5;
            den *= 2;
        }
    }
    round_half_even(num, den) as u32
}

fn round_half_even(num: u64, den: u64) -> u64 {
    let (q, r) = (num / den, num % den);
    match (2 * r).cmp(&den) {
        std::cmp::Ordering::Less => q,
        std::cmp::Ordering::Greater => q + 1,
        std::cmp::Ordering::Equal => q + (q & 1),
    }
}
