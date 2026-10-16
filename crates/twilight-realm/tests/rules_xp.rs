//! Kill XP and the level table against vmangos's values (`docs/reference/vanilla-combat-rules.md`
//! §8 and §9).

use twilight_realm::rules::xp::{gray_level, kill_xp, xp_to_next, Elite};

#[test]
fn kill_xp_matches_vmangos_by_level_difference() {
    // (player, mob, xp), §8's vector table.
    let vectors = [
        (1, 1, 50),
        (1, 2, 52), // 52.5 rounds half to even
        (1, 3, 55),
        (1, 5, 60),
        (1, 6, 60), // capped at +4
        (5, 3, 42),
        (5, 1, 14), // gray level 0: still green
        (6, 2, 15),
        (10, 10, 95),
        (10, 8, 68),
        (10, 5, 27),
        (10, 14, 114),
        (10, 20, 114),
        (20, 20, 145),
        (20, 15, 79),
        (25, 19, 77),
        (30, 30, 195),
        (30, 23, 81),
        (60, 60, 345),
        (60, 48, 101),
    ];
    for (player, mob, xp) in vectors {
        assert_eq!(kill_xp(player, mob, Elite::No), xp, "P{player} vs M{mob}");
    }
}

#[test]
fn gray_mobs_give_no_xp() {
    for (player, mob) in [(6, 1), (10, 4), (20, 13), (25, 18), (60, 47)] {
        assert_eq!(kill_xp(player, mob, Elite::No), 0, "P{player} vs M{mob}");
    }
    assert_eq!(gray_level(5), 0);
    assert_eq!(gray_level(10), 4);
    assert_eq!(gray_level(20), 13);
    assert_eq!(gray_level(40), 31);
    assert_eq!(gray_level(60), 47);
}

#[test]
fn elites_give_double_in_the_open_world_and_more_in_a_dungeon() {
    assert_eq!(kill_xp(20, 22, Elite::OpenWorld), 319);
    assert_eq!(kill_xp(20, 20, Elite::Dungeon), 362); // 362.5, half to even
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
