//! The aggro radius against vmangos's values (`docs/reference/vanilla-combat-rules.md` §11.1).

use twilight_realm::rules::aggro::radius;

#[test]
fn a_mob_notices_a_player_by_level_difference() {
    // (detection range, player, mob, radius)
    let cases = [
        (18.0, 10, 10, 18.0),
        (18.0, 10, 13, 21.0),
        (18.0, 10, 7, 15.0),
        (18.0, 10, 1, 9.0),
        (18.0, 30, 10, 5.0), // the 5 yd floor
        (18.0, 1, 30, 43.0), // the difference is clamped at 25 levels
        (10.0, 1, 3, 12.0),
    ];
    for (detection, player, mob, want) in cases {
        assert_eq!(
            radius(detection, player, mob),
            want,
            "P{player} vs M{mob} at {detection}"
        );
    }
}

#[test]
fn a_mob_without_detection_never_aggroes_on_sight() {
    assert_eq!(radius(0.0, 1, 10), 0.0);
}
