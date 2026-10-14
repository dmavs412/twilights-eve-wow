//! Static content: where characters start, and what each race looks like and fights for.

/// Kalimdor, the continent The Barrens is on.
pub const KALIMDOR: u32 = 1;
/// `AreaTable.dbc`'s The Barrens.
pub const THE_BARRENS: u32 = 17;

/// The Hub: the Crossroads, in front of the inn, facing the road north. `[x, y, z, facing]`.
pub const CROSSROADS: [f32; 4] = [-441.8, -2648.6, 95.9, 1.6];

/// `ChrRaces.dbc` ids 1..=8: Human … Troll.
pub(crate) fn valid_race(race: u8) -> bool {
    (1..=8).contains(&race)
}

/// A race's `(male, female)` native `CreatureDisplayInfo` ids (`ChrRaces.dbc`).
pub(crate) fn display_id(race: u8, gender: u8) -> u32 {
    let (male, female) = match race {
        1 => (49, 50),
        2 => (51, 52),
        3 => (53, 54),
        4 => (55, 56),
        5 => (57, 58),
        6 => (59, 60),
        7 => (1563, 1564),
        _ => (1478, 1479),
    };
    if gender == 0 {
        male
    } else {
        female
    }
}

/// A race's player `FactionTemplate.dbc` id (`ChrRaces.dbc`).
pub(crate) fn faction_template(race: u8) -> u32 {
    match race {
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        5 => 5,
        6 => 6,
        7 => 115,
        _ => 116,
    }
}

/// A Warrior's level-1 base stats by race, `[str, agi, sta, int, spi]` (`player_levelstats`).
pub(crate) fn base_stats(race: u8) -> [u32; 5] {
    match race {
        1 => [23, 20, 22, 20, 21],
        2 => [26, 17, 24, 17, 23],
        3 => [25, 16, 25, 19, 19],
        4 => [20, 25, 21, 20, 20],
        5 => [22, 18, 23, 18, 25],
        6 => [28, 15, 24, 15, 22],
        7 => [18, 23, 21, 24, 20],
        _ => [24, 22, 23, 16, 21],
    }
}

/// A level-1 Warrior's base health (`player_classlevelstats`).
pub(crate) const BASE_HEALTH: u32 = 20;
