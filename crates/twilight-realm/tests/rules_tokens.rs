//! The Token vendor's prices and level bands (`docs/reference/items-and-vendors.md`, "Token
//! vendor"): greens priced `max(1, round(item_level / 4 * slot_mod))` Tokens, listed by the band
//! of required levels the buyer's level falls in.

use twilight_realm::rules::tokens::{band, price, stock};

const WARRIORS_TUNIC: u32 = 2965; // chest, item level 11: 11 / 4 * 0.9 = 2.475
const HUNTING_BOW: u32 = 8180; // ranged, item level 11: 11 / 4 * 0.5 = 1.375
const CORAL_CLAYMORE: u32 = 3188; // two-hander, item level 15: 15 / 4 * 1.0 = 3.75
const TUNNEL_PICK: u32 = 756; // two-hander, item level 29: 29 / 4 * 1.0 = 7.25
const GLIMMERING_MAIL_BREASTPLATE: u32 = 4071; // chest, item level 31: 31 / 4 * 0.9 = 6.975
const TOUGH_JERKY: u32 = 117; // the goods vendor's

#[test]
fn a_green_costs_its_item_level_over_four_scaled_by_its_slot() {
    assert_eq!(price(WARRIORS_TUNIC), Some(2));
    assert_eq!(price(HUNTING_BOW), Some(1));
    assert_eq!(price(CORAL_CLAYMORE), Some(4));
    assert_eq!(price(TUNNEL_PICK), Some(7));
    assert_eq!(price(GLIMMERING_MAIL_BREASTPLATE), Some(7));
}

#[test]
fn what_the_token_vendor_does_not_sell_has_no_token_price() {
    assert_eq!(price(TOUGH_JERKY), None);
    assert_eq!(price(0), None);
}

#[test]
fn levels_fall_in_bands_of_ten_and_the_last_band_takes_everything_above() {
    assert_eq!(band(1), (1, 10));
    assert_eq!(band(10), (1, 10));
    assert_eq!(band(11), (11, 20));
    assert_eq!(band(20), (11, 20));
    assert_eq!(band(21), (21, 30));
    assert_eq!(band(30), (21, 30));
    assert_eq!(band(60), (21, 30));
}

#[test]
fn the_stock_is_the_buyers_band() {
    let first = stock(1);
    assert_eq!(first.len(), 8);
    assert_eq!(first[0], (WARRIORS_TUNIC, 2));
    assert_eq!(stock(10), first);
    assert_eq!(stock(15).len(), 12);
    assert_eq!(stock(25).len(), 13);
    assert!(stock(25).contains(&(GLIMMERING_MAIL_BREASTPLATE, 7)));
    assert!(!stock(25).iter().any(|&(e, _)| e == WARRIORS_TUNIC));
}
