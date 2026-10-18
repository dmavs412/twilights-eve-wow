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

/// The XP a player of level `me` earns for a kill credited to players of `levels`, theirs among
/// them (vmangos `Group::RewardGroupAtKill`): the kill's XP for the highest level it is not gray
/// to, times the group rate, halved when someone higher finds it gray, then shared by level.
/// Nothing for a player above that level; alone, exactly [`kill_xp`].
pub fn party_kill_xp(levels: &[u8], me: u8, mob: u8, elite: Elite) -> u32 {
    if levels.len() < 2 {
        return kill_xp(me, mob, elite);
    }
    let Some(top) = levels
        .iter()
        .copied()
        .filter(|&l| kill_xp(l, mob, elite) > 0)
        .max()
    else {
        return 0;
    };
    if me > top {
        return 0;
    }
    let xp = f64::from(kill_xp(top, mob, elite)) * group_rate(levels.len());
    let xp = if levels.iter().any(|&l| l > top) {
        xp / 2.0 + 1.0
    } else {
        xp
    } as u64;
    let sum: u64 = levels.iter().map(|&l| u64::from(l)).sum();
    (xp * u64::from(me) / sum.max(1)) as u32
}

/// The group's XP bonus (`MaNGOS::XP::xp_in_group_rate`, not a raid).
fn group_rate(count: usize) -> f64 {
    match count {
        0..=2 => 1.0,
        3 => 1.166,
        4 => 1.3,
        _ => 1.4,
    }
}

fn round_half_even(num: u64, den: u64) -> u64 {
    let (q, r) = (num / den, num % den);
    match (2 * r).cmp(&den) {
        std::cmp::Ordering::Less => q,
        std::cmp::Ordering::Greater => q + 1,
        std::cmp::Ordering::Equal => q + (q & 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alone_a_kill_pays_its_own_xp() {
        assert_eq!(
            party_kill_xp(&[10], 10, 10, Elite::No),
            kill_xp(10, 10, Elite::No)
        );
    }

    #[test]
    fn a_group_splits_the_kill_by_level() {
        let solo = kill_xp(10, 10, Elite::No);
        // Two equal players: no bonus, half each.
        assert_eq!(party_kill_xp(&[10, 10], 10, 10, Elite::No), solo / 2);
        // Five: 1.4 times, a fifth each.
        let five = [10; 5];
        assert_eq!(
            party_kill_xp(&five, 10, 10, Elite::No),
            (f64::from(solo) * 1.4) as u32 / 5
        );
    }

    #[test]
    fn a_player_too_high_for_the_kill_gets_nothing() {
        // The level 60 finds a level 10 mob gray; the level 10 gets half the bonus-free XP + 1,
        // shared by level.
        let low = party_kill_xp(&[10, 60], 10, 10, Elite::No);
        assert_eq!(party_kill_xp(&[10, 60], 60, 10, Elite::No), 0);
        assert_eq!(low, (kill_xp(10, 10, Elite::No) / 2 + 1) * 10 / 70);
    }
}
