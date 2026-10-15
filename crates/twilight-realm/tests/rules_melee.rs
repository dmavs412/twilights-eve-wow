//! The white-swing attack table and armour, against vmangos's values
//! (`docs/reference/vanilla-combat-rules.md` §1 and §3). Chances are basis points.

use twilight_realm::rules::melee::{
    armor_reduced, glancing_range, mob_vs_player, player_vs_mob, reach, HitTable, Outcome,
    PlayerDefense, PlayerOffense,
};

fn table(miss: u32, dodge: u32, parry: u32, glancing: u32, block: u32, crit: u32) -> HitTable {
    HitTable {
        miss,
        dodge,
        parry,
        glancing,
        block,
        crit,
        crushing: 0,
    }
}

/// A naked level-10 human warrior: skill 50, sheet crit 26 / 5.20021.
fn p10() -> PlayerOffense {
    PlayerOffense {
        level: 10,
        weapon_skill: 50,
        crit_pct: 26.0 / 5.20021,
    }
}

#[test]
fn a_player_swinging_at_a_mob_from_the_front() {
    let cases = [
        (10, table(500, 500, 500, 1000, 500, 499)),
        (11, table(550, 550, 600, 2000, 500, 399)),
        (12, table(600, 600, 700, 3000, 500, 299)),
        (13, table(800, 650, 1400, 4000, 500, 199)),
        (14, table(900, 700, 1700, 4000, 500, 99)),
        // Below level 10 a mob's avoidance shrinks by level / 10, and the player cannot glance.
        (7, table(245, 245, 140, 0, 245, 559)),
        (5, table(125, 125, 0, 0, 125, 599)),
    ];
    for (mob, want) in cases {
        assert_eq!(player_vs_mob(p10(), mob, true), want, "P10 vs M{mob}");
    }
}

#[test]
fn low_level_fights() {
    let p = |level: u8| PlayerOffense {
        level,
        weapon_skill: u16::from(level) * 5,
        crit_pct: 5.0,
    };
    assert_eq!(
        player_vs_mob(p(1), 1, true),
        table(50, 50, 50, 1000, 50, 500)
    );
    assert_eq!(player_vs_mob(p(3), 1, true), table(40, 40, 30, 0, 40, 540));
    assert_eq!(
        player_vs_mob(p(5), 5, true),
        table(250, 250, 250, 1000, 250, 500)
    );
}

#[test]
fn a_mob_three_levels_up_is_the_famous_boss_table() {
    // The sheet crit carries 0.04% per point of weapon skill over the level's maximum.
    let p60 = |weapon_skill: u16| PlayerOffense {
        level: 60,
        weapon_skill,
        crit_pct: 5.0 + f64::from(weapon_skill - 300) * 0.04,
    };
    let t = player_vs_mob(p60(300), 63, true);
    assert_eq!(
        (t.miss, t.dodge, t.parry, t.glancing, t.block, t.crit),
        (800, 650, 1400, 4000, 500, 200)
    );
    let t = player_vs_mob(p60(305), 63, true);
    assert_eq!((t.miss, t.dodge, t.crit), (600, 600, 220));
}

#[test]
fn from_behind_a_mob_cannot_parry_or_block() {
    let t = player_vs_mob(p10(), 10, false);
    assert_eq!(t, table(500, 500, 0, 1000, 0, 499));
    // miss [0,500), dodge [500,1000), glance [1000,2000), crit [2000,2499), hit [2499,10000).
    assert_eq!(t.outcome(999), Outcome::Dodge);
    assert_eq!(t.outcome(1000), Outcome::Glancing);
    assert_eq!(t.outcome(2498), Outcome::Crit);
    assert_eq!(t.outcome(2499), Outcome::Hit);
}

#[test]
fn one_roll_walks_the_table_in_vmangos_order() {
    let t = player_vs_mob(p10(), 10, true);
    let rolls = [
        (0, Outcome::Miss),
        (499, Outcome::Miss),
        (500, Outcome::Dodge),
        (1499, Outcome::Parry),
        (1500, Outcome::Glancing),
        (2499, Outcome::Glancing),
        (2500, Outcome::Block),
        (3000, Outcome::Crit),
        (3498, Outcome::Crit),
        (3499, Outcome::Hit),
        (9999, Outcome::Hit),
    ];
    for (roll, want) in rolls {
        assert_eq!(t.outcome(roll), want, "roll {roll}");
    }
    // P10 vs M13's worked example: 7548 is the last crit.
    let t = player_vs_mob(p10(), 13, true);
    assert_eq!(t.outcome(7548), Outcome::Crit);
    assert_eq!(t.outcome(7549), Outcome::Hit);
}

/// A naked level-10 human warrior defending: defence 50, dodge from agility, with parry and a
/// shield.
fn p10_defending(parry_pct: f64, block_pct: f64) -> PlayerDefense {
    PlayerDefense {
        level: 10,
        defense: 50,
        dodge_pct: 26.0 / 5.20021,
        parry_pct,
        block_pct,
    }
}

#[test]
fn a_mob_swinging_at_a_player() {
    let full = p10_defending(5.0, 5.0);
    let cases = [
        (8, [540, 539, 540, 540, 460, 0]),
        (10, [500, 499, 500, 500, 500, 0]),
        (12, [460, 459, 460, 460, 540, 0]),
        // Crushing blows start three levels up.
        (13, [440, 439, 440, 440, 560, 1500]),
        (14, [420, 419, 420, 420, 580, 2500]),
        (15, [400, 399, 400, 400, 600, 3500]),
    ];
    for (mob, [miss, dodge, parry, block, crit, crushing]) in cases {
        let want = HitTable {
            miss,
            dodge,
            parry,
            glancing: 0,
            block,
            crit,
            crushing,
        };
        assert_eq!(mob_vs_player(mob, full, true), want, "M{mob} vs P10");
    }
    // No shield and no Parry skill: those slices are gone.
    let bare = mob_vs_player(10, p10_defending(0.0, 0.0), true);
    assert_eq!(bare, table(500, 499, 0, 0, 0, 500));
}

#[test]
fn a_player_cannot_dodge_parry_or_block_from_behind() {
    let t = mob_vs_player(10, p10_defending(5.0, 5.0), false);
    assert_eq!(t, table(500, 0, 0, 0, 0, 500));
}

#[test]
fn glancing_blows_lose_more_the_higher_the_defence() {
    let close =
        |(a, b): (f32, f32), (c, d): (f32, f32)| (a - c).abs() < 1e-4 && (b - d).abs() < 1e-4;
    assert!(close(glancing_range(50, 50), (0.91, 0.99)));
    assert!(close(glancing_range(55, 50), (0.91, 0.99)));
    assert!(close(glancing_range(60, 50), (0.80, 0.90)));
    assert!(close(glancing_range(65, 50), (0.55, 0.75)));
}

#[test]
fn armour_mitigates_by_the_attackers_level() {
    let cases = [
        (0, 10, 100),
        (52, 10, 96),
        (25, 3, 96),
        (100, 1, 82),
        (492, 10, 71),
        (492, 12, 74),
        (822, 20, 71),
        (1000, 10, 55),
        (3000, 30, 49),
        (4000, 60, 57),
        (17265, 60, 25), // the 75% cap
    ];
    for (armor, level, want) in cases {
        assert_eq!(
            armor_reduced(100, armor, level),
            want,
            "{armor} armour vs L{level}"
        );
    }
    // A hit never mitigates below 1.
    assert_eq!(armor_reduced(1, 17265, 60), 1);
}

#[test]
fn melee_reach_is_both_reaches_and_four_thirds_but_never_under_five_yards() {
    // Two 1.5 yd bodies come to 4.33 yd: the 5 yd floor (`NOMINAL_MELEE_RANGE`).
    assert_eq!(reach(1.5, 1.5), 5.0);
    // A big mob (4 yd) is reached from 1.5 + 4 + 4/3.
    assert!((reach(1.5, 4.0) - 6.833_333).abs() < 1e-5);
}
