//! Melee haste, as pure rules (ADR 0002): a faster swing time, and the per-hit cut that keeps
//! the haste alone from adding damage a second.

use twilight_realm::rules::haste::{haste_cut_damage, hasted_attack_time};

/// 40% faster swings (Seal of the Crusader): the swing time over 1.4, to the millisecond.
#[test]
fn forty_percent_haste_divides_the_swing_time_by_one_point_four() {
    assert_eq!(hasted_attack_time(2800, 40), 2000);
    assert_eq!(hasted_attack_time(2000, 40), 1429, "1428.6");
    assert_eq!(hasted_attack_time(3600, 40), 2571, "2571.4");
    assert_eq!(hasted_attack_time(2000, 0), 2000);
}

/// "...but deals less damage with each attack": each hit over 1.4, so the faster swings alone
/// deal what the slower ones did a second.
#[test]
fn the_haste_cut_divides_each_hit_by_the_same_factor() {
    assert_eq!(haste_cut_damage(140.0, 40), 100.0);
    assert_eq!(haste_cut_damage(70.0, 40), 50.0);
    assert_eq!(haste_cut_damage(10.0, 0), 10.0);
}
