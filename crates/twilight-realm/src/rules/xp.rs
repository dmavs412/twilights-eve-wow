//! Kill XP and the level table (`player_xp_for_level`; kill XP is the Power Level rule, ADR 0012).

use crate::tables::{self, MAX_LEVEL};

/// Whether a kill is an elite's, and where: an elite pays double, and 2.5× inside a (non-raid)
/// dungeon.
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

/// The XP a kill of a mob of level `mob` pays, whoever kills it: `mob × 5 + 45`, doubled for an
/// open-world elite and ×2.5 in a dungeon. Rounded half to even, as `nearbyint` does, from exact
/// integer arithmetic.
pub fn kill_xp(mob: u8, elite: Elite) -> u32 {
    let base = u64::from(mob) * 5 + 45;
    let (num, den) = match elite {
        Elite::No => (base, 1),
        Elite::OpenWorld => (base * 2, 1),
        Elite::Dungeon => (base * 5, 2),
    };
    round_half_even(num, den) as u32
}

/// The XP each of `count` credited players earns for a kill: the kill's XP times the group rate,
/// shared equally. Alone, exactly [`kill_xp`].
pub fn party_kill_xp(count: usize, mob: u8, elite: Elite) -> u32 {
    let xp = kill_xp(mob, elite);
    if count < 2 {
        return xp;
    }
    (f64::from(xp) * group_rate(count) / count as f64) as u32
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
    fn the_mobs_level_sets_the_kill() {
        // A level 1 character killing a level 15 mob, and a level 40 killing a level 5 one.
        assert_eq!(kill_xp(15, Elite::No), 120);
        assert_eq!(kill_xp(5, Elite::No), 70);
        assert_eq!(kill_xp(1, Elite::No), 50);
    }

    #[test]
    fn elites_pay_double_in_the_open_world_and_more_in_a_dungeon() {
        assert_eq!(kill_xp(22, Elite::OpenWorld), 310);
        assert_eq!(kill_xp(20, Elite::Dungeon), 362); // 362.5, half to even
    }

    #[test]
    fn alone_a_kill_pays_its_own_xp() {
        assert_eq!(party_kill_xp(1, 10, Elite::No), kill_xp(10, Elite::No));
    }

    #[test]
    fn a_group_splits_the_kill_equally() {
        let solo = kill_xp(10, Elite::No);
        // Two players: no bonus, half each.
        assert_eq!(party_kill_xp(2, 10, Elite::No), solo / 2);
        // Five: 1.4 times, a fifth each.
        assert_eq!(
            party_kill_xp(5, 10, Elite::No),
            (f64::from(solo) * 1.4) as u32 / 5
        );
    }
}
