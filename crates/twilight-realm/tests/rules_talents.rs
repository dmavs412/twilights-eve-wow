//! The talent rules as exact values: when Talent Points come, what a respec costs, and the
//! numbers the Basic Warrior's talents change (`data/twilights-eve/callings.json`, vmangos).

use twilight_realm::rules::talents::{deep_wound_tick, points_at, respec_cost, scaled};

#[test]
fn a_point_comes_with_every_level_from_two() {
    assert_eq!(points_at(1), 0);
    assert_eq!(points_at(2), 1);
    assert_eq!(points_at(5), 4);
    // The Tier Caps: 9 for a Basic Warrior at 10, 29 for a Tier 1 Warrior at 30.
    assert_eq!(points_at(10), 9);
    assert_eq!(points_at(30), 29);
}

#[test]
fn a_respec_costs_vanillas_escalating_gold() {
    // vmangos `Player::resetTalentsCost`: 1 g, 5 g, 10 g, then 5 g more each time, up to 50 g.
    assert_eq!(respec_cost(0), 10_000);
    assert_eq!(respec_cost(1), 50_000);
    assert_eq!(respec_cost(2), 100_000);
    assert_eq!(respec_cost(3), 150_000);
    assert_eq!(respec_cost(10), 500_000);
    assert_eq!(respec_cost(11), 500_000);
    assert_eq!(respec_cost(1000), 500_000);
}

#[test]
fn a_percent_bonus_rounds_half_away_from_zero() {
    // Improved Rend 3/3 on a 5-damage tick: 6.75.
    assert_eq!(scaled(5, 35), 7);
    // Improved Battle Shout 1/3 on 15 attack power: 15.75.
    assert_eq!(scaled(15, 5), 16);
    assert_eq!(scaled(4, 15), 5);
    assert_eq!(scaled(10, 25), 13);
    assert_eq!(scaled(7, 0), 7);
}

#[test]
fn deep_wounds_bleeds_a_share_of_the_average_swing_over_four_ticks() {
    // A 2-4 damage swing averages 3: 60% is 1.8 over four ticks, never under one a tick.
    assert_eq!(deep_wound_tick(2.0, 4.0, 60), 1);
    // A 20-30 weapon averages 25: 20% is 5, 1.25 a tick; 60% is 15, 3.75 a tick.
    assert_eq!(deep_wound_tick(20.0, 30.0, 20), 1);
    assert_eq!(deep_wound_tick(20.0, 30.0, 60), 4);
    assert_eq!(deep_wound_tick(50.0, 70.0, 40), 6);
}

#[test]
fn impale_adds_a_share_of_the_crits_bonus() {
    use twilight_realm::rules::talents::crit_damage;
    // A plain crit doubles; Impale 2/2 adds a fifth of the bonus: 2.2×.
    assert_eq!(crit_damage(100, 0), 200);
    assert_eq!(crit_damage(100, 10), 210);
    assert_eq!(crit_damage(100, 20), 220);
    assert_eq!(crit_damage(33, 20), 72);
}

#[test]
fn blood_craze_spreads_its_share_of_the_health_over_its_ticks() {
    use twilight_realm::rules::talents::heal_tick;
    // 3% of 1000 health over six ticks: 5 a tick; never under 1.
    assert_eq!(heal_tick(1000, 3, 6), 5);
    assert_eq!(heal_tick(400, 1, 6), 1);
    assert_eq!(heal_tick(50, 1, 6), 1);
}
