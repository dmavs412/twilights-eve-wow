//! Kill XP and the level table (`docs/reference/vanilla-combat-rules.md` §9; kill XP follows
//! ADR 0012).

use twilight_realm::rules::xp::{gray_level, kill_xp, quest_xp_basis, xp_to_next, Elite};

#[test]
fn kill_xp_comes_from_the_mobs_level_alone() {
    // (mob, xp): mob × 5 + 45.
    let vectors = [(1, 50), (5, 70), (10, 95), (15, 120), (20, 145), (60, 345)];
    for (mob, xp) in vectors {
        assert_eq!(kill_xp(mob, Elite::No), xp, "M{mob}");
    }
}

#[test]
fn gray_level_is_vanillas() {
    assert_eq!(gray_level(5), 0);
    assert_eq!(gray_level(10), 4);
    assert_eq!(gray_level(20), 13);
    assert_eq!(gray_level(40), 31);
    assert_eq!(gray_level(60), 47);
}

#[test]
fn elites_give_double_in_the_open_world_and_more_in_a_dungeon() {
    assert_eq!(kill_xp(22, Elite::OpenWorld), 310);
    assert_eq!(kill_xp(20, Elite::Dungeon), 362); // 362.5, half to even
}

#[test]
fn the_level_table_is_vanilla_to_nine_then_steeper() {
    assert_eq!(xp_to_next(1), Some(400));
    assert_eq!(xp_to_next(9), Some(6500));
    assert_eq!(xp_to_next(10), Some(7600));
    assert_eq!(xp_to_next(20), Some(57_300));
    assert_eq!(xp_to_next(29), Some(246_700));
    let to_29: u32 = (1..=29).map(|l| xp_to_next(l).unwrap()).sum();
    assert!(to_29.abs_diff(1_600_000) < 16_000, "sum {to_29}");
    // The quest basis stays vanilla.
    assert_eq!(quest_xp_basis(1), Some(400));
    assert_eq!(quest_xp_basis(20), Some(23_200));
    assert_eq!(quest_xp_basis(30), Some(47400));
    assert_eq!(quest_xp_basis(59), Some(209_800));
    assert_eq!(quest_xp_basis(60), None);
    // 60 is the end of the game.
    assert_eq!(xp_to_next(60), None);
    assert_eq!(xp_to_next(0), None);
}

#[test]
fn a_discovery_pays_an_eighth_of_the_xp_to_the_next_level() {
    use twilight_realm::rules::xp::discovery_xp;
    // 400, 7600 and 264000 to the next level.
    assert_eq!(
        [1, 10, 30].map(discovery_xp),
        [50, 950, 33000],
        "level 1, 10 and 30"
    );
    assert_eq!(discovery_xp(60), 0, "nothing past the end of the game");
}
