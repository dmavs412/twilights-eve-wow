//! The leash and the respawn clock, against the spec's numbers (#1: leash at about 40–60 yd from
//! home, respawn about 60 s after the player leaves).

use twilight_realm::rules::{leash, respawn};

#[test]
fn a_mob_leashes_past_fifty_yards_from_home() {
    let home = [100.0, -200.0, 50.0];
    assert!(!leash::beyond(home, home));
    assert!(!leash::beyond(home, [149.9, -200.0, 50.0]));
    assert!(leash::beyond(home, [150.1, -200.0, 50.0]));
    // Height counts: 30 across and 40 up is 50.
    assert!(!leash::beyond(home, [130.0, -200.0, 89.9]));
    assert!(leash::beyond(home, [130.0, -200.0, 90.1]));
}

#[test]
fn the_respawn_clock_runs_only_while_the_player_is_away() {
    assert_eq!(respawn::SECS, 60.0);
    // Away (past 60 yd): the clock runs.
    assert_eq!(respawn::countdown(60.0, 0.5, 60.1), 59.5);
    assert_eq!(respawn::countdown(0.25, 0.5, 200.0), -0.25);
    // Near: it starts over.
    assert_eq!(respawn::countdown(12.0, 0.5, 60.0), 60.0);
    assert_eq!(respawn::countdown(12.0, 0.5, 0.0), 60.0);
}
