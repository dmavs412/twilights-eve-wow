//! Durability's prices and losses: repair costs from `DurabilityCosts.dbc` and
//! `DurabilityQuality.dbc`, and what death and combat take
//! (`docs/reference/items-and-vendors.md`, "Repair" and "Durability loss").

use twilight_realm::rules::durability::{
    death_loss, death_money_loss, repair_cost, repair_cost_of, wears_on_hit, WEAR_ROLL,
};

const GNOLL_WAR_HARNESS: u32 = 1211; // green leather chest, item level 15, 65 durability
const FINE_SCIMITAR: u32 = 4560; // white sword, item level 6, 30 durability
const DULL_BLADE: u32 = 1384; // white sword, item level 3, 20 durability
const MELTED_CANDLE: u32 = 755; // grey trash, no durability

#[test]
fn the_references_worked_example_a_green_mail_chest_of_level_20() {
    // Mail is armour subclass 3, so multiplier 24 of row 20: 5. Twenty points at 1.0 cost 1s.
    assert_eq!(repair_cost_of(20, 2, 4, 3, 20), 100);
}

#[test]
fn repairing_real_items_from_broken() {
    // Leather (21 + 2) at item level 15 is 3 a point, green 1.0: 65 × 3.
    assert_eq!(repair_cost(GNOLL_WAR_HARNESS, 0), 195);
    // Swords (multiplier 7) at item level 6 are 2 a point, white 0.8: 30 × 2 × 0.8.
    assert_eq!(repair_cost(FINE_SCIMITAR, 0), 48);
    // Half broken costs half.
    assert_eq!(repair_cost(FINE_SCIMITAR, 15), 24);
}

#[test]
fn any_lost_point_costs_at_least_a_copper_and_none_costs_nothing() {
    // 1 × 1 × 0.8 truncates to 0, which vmangos raises to 1.
    assert_eq!(repair_cost(DULL_BLADE, 19), 1);
    assert_eq!(repair_cost(DULL_BLADE, 20), 0);
    assert_eq!(repair_cost(MELTED_CANDLE, 0), 0);
}

#[test]
fn death_takes_a_tenth_of_each_items_durability_at_least_one_point() {
    assert_eq!(death_loss(65), 6);
    assert_eq!(death_loss(30), 3);
    assert_eq!(death_loss(100), 10);
    assert_eq!(death_loss(9), 1);
    assert_eq!(death_loss(0), 0, "no durability, nothing to lose");
}

#[test]
fn death_takes_a_tenth_of_the_money() {
    assert_eq!(death_money_loss(1234), 123);
    assert_eq!(death_money_loss(10_000), 1_000);
    assert_eq!(death_money_loss(9), 0);
}

#[test]
fn one_hit_in_two_hundred_wears_an_item() {
    // `DurabilityLossChance.Damage = 0.5`: half a percent of each damaging hit.
    let wearing = (0..WEAR_ROLL).filter(|r| wears_on_hit(*r)).count();
    assert_eq!(WEAR_ROLL, 1000);
    assert_eq!(wearing, 5);
}
