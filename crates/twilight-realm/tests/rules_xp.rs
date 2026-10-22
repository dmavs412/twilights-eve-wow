//! Kill XP and the level table (`docs/reference/vanilla-combat-rules.md` §9; kill XP follows
//! ADR 0012).

use twilight_realm::rules::xp::{gray_level, kill_xp, xp_to_next, Elite};

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
fn the_level_table_is_vanillas() {
    assert_eq!(xp_to_next(1), Some(400));
    assert_eq!(xp_to_next(10), Some(7600));
    assert_eq!(xp_to_next(30), Some(47400));
    assert_eq!(xp_to_next(59), Some(209_800));
    // 60 is the end of the game.
    assert_eq!(xp_to_next(60), None);
    assert_eq!(xp_to_next(0), None);
}

#[test]
fn a_discovery_pays_an_eighth_of_the_xp_to_the_next_level() {
    use twilight_realm::rules::xp::discovery_xp;
    // 400, 7600 and 47400 to the next level.
    assert_eq!(
        [1, 10, 30].map(discovery_xp),
        [50, 950, 5925],
        "level 1, 10 and 30"
    );
    assert_eq!(discovery_xp(60), 0, "nothing past the end of the game");
}
