//! The white-swing attack table and armour. The table is flat (ADR 0012, `content/combat.toml`):
//! no level or skill enters a chance. Chances are basis points.

use twilight_realm::rules::melee::{
    armor_reduced, dual_wield_miss_bp, mob_vs_player, player_vs_mob, reach, HitTable, MobDefense,
    Outcome, PlayerDefense, PlayerOffense,
};

fn table(miss: u32, dodge: u32, parry: u32, block: u32, crit: u32) -> HitTable {
    HitTable {
        miss,
        dodge,
        parry,
        block,
        crit,
    }
}

/// A player with a 5% sheet crit.
fn player() -> PlayerOffense {
    PlayerOffense { crit_pct: 5.0 }
}

/// A mob that parries and carries a shield.
fn armed() -> MobDefense {
    MobDefense {
        parries: true,
        blocks: true,
    }
}

#[test]
fn a_mob_blocks_only_with_a_shield_and_parries_unless_its_data_says_not() {
    let bare = MobDefense {
        parries: true,
        blocks: false,
    };
    assert_eq!(
        player_vs_mob(player(), bare, true),
        table(500, 500, 500, 0, 500)
    );
    // `CREATURE_FLAG_EXTRA_NO_PARRY`: without the parry slice.
    let no_parry = MobDefense {
        parries: false,
        blocks: true,
    };
    assert_eq!(
        player_vs_mob(player(), no_parry, true),
        table(500, 500, 0, 500, 500)
    );
}

#[test]
fn a_player_swinging_at_a_mob_rolls_the_same_table_at_every_level() {
    // The mob's level is not an input: the table is the same against a level 1 and a level 25.
    assert_eq!(
        player_vs_mob(player(), armed(), true),
        table(500, 500, 500, 500, 500)
    );
    // No glancing slice exists: the rest of the roll is a hit.
    let t = player_vs_mob(player(), armed(), true);
    assert_eq!(t.outcome(1999), Outcome::Block);
    assert_eq!(t.outcome(2499), Outcome::Crit);
    assert_eq!(t.outcome(2500), Outcome::Hit);
}

#[test]
fn the_player_crit_is_the_sheet_crit() {
    let p = PlayerOffense {
        crit_pct: 26.0 / 5.20021,
    };
    assert_eq!(player_vs_mob(p, armed(), true).crit, 499);
}

#[test]
fn from_behind_a_mob_cannot_parry_or_block() {
    let t = player_vs_mob(player(), armed(), false);
    assert_eq!(t, table(500, 500, 0, 0, 500));
    // miss [0,500), dodge [500,1000), crit [1000,1500), hit [1500,10000).
    assert_eq!(t.outcome(999), Outcome::Dodge);
    assert_eq!(t.outcome(1000), Outcome::Crit);
    assert_eq!(t.outcome(1499), Outcome::Crit);
    assert_eq!(t.outcome(1500), Outcome::Hit);
}

#[test]
fn one_roll_walks_the_table_in_order() {
    let t = player_vs_mob(player(), armed(), true);
    let rolls = [
        (0, Outcome::Miss),
        (499, Outcome::Miss),
        (500, Outcome::Dodge),
        (1499, Outcome::Parry),
        (1500, Outcome::Block),
        (2000, Outcome::Crit),
        (2499, Outcome::Crit),
        (2500, Outcome::Hit),
        (9999, Outcome::Hit),
    ];
    for (roll, want) in rolls {
        assert_eq!(t.outcome(roll), want, "roll {roll}");
    }
}

#[test]
fn two_weapons_add_nineteen_points_of_miss() {
    // 5% flat plus 19% dual-wield: a white swing misses 24%.
    assert_eq!(dual_wield_miss_bp(), 1900);
    let t = player_vs_mob(player(), armed(), true);
    assert_eq!(t.miss + dual_wield_miss_bp(), 2400);
}

/// A defending player: dodge from agility, with parry and a shield.
fn defending(parry_pct: f64, block_pct: f64) -> PlayerDefense {
    PlayerDefense {
        dodge_pct: 26.0 / 5.20021,
        parry_pct,
        block_pct,
    }
}

#[test]
fn a_mob_swinging_at_a_player_has_five_percent_miss_and_crit_and_never_crushes() {
    // The mob's level is not an input, so a level 30 mob and a level 1 mob roll the same table.
    assert_eq!(
        mob_vs_player(defending(5.0, 5.0), true),
        table(500, 499, 500, 500, 500)
    );
    // No shield and no Parry skill: those slices are gone.
    assert_eq!(
        mob_vs_player(defending(0.0, 0.0), true),
        table(500, 499, 0, 0, 500)
    );
}

#[test]
fn a_player_cannot_dodge_parry_or_block_from_behind() {
    let t = mob_vs_player(defending(5.0, 5.0), false);
    assert_eq!(t, table(500, 0, 0, 0, 500));
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
