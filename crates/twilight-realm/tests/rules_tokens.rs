//! The Token vendor's prices and level bands (`docs/reference/items-and-vendors.md`, "Token
//! vendor"): the 1 to 10 band is the Token whites at spec #272's prices (Chest and two-hander
//! 3, every other weapon 2, amulet and ring 1); from 11 up, greens priced
//! `max(1, round(item_level / 4 * slot_mod))` Tokens. Each band is listed to the buyer whose
//! level falls in it.

use twilight_realm::rules::tokens::{band, price, stock};

const NOVICES_WOOLEN_VEST: u32 = 900200; // a Token white Chest: 3
const MILITIA_CLAYMORE: u32 = 900204; // a Token white two-hander: 3
const MILITIA_SHORTSWORD: u32 = 900205; // a Token white one-hander: 2
const PENDANT_OF_BRAWN: u32 = 900212; // a Token white amulet: 1
const SHADOWHIDE_TWO_HANDED_SWORD: u32 = 1460; // two-hander, item level 20: 20 / 4 * 1.0 = 5
const BURNISHED_TUNIC: u32 = 2989; // chest, item level 21: 21 / 4 * 0.9 = 4.725
const LIGHT_BOW: u32 = 4576; // ranged, item level 21: 21 / 4 * 0.5 = 2.625
const TUNNEL_PICK: u32 = 756; // two-hander, item level 29: 29 / 4 * 1.0 = 7.25
const GLIMMERING_MAIL_BREASTPLATE: u32 = 4071; // chest, item level 31: 31 / 4 * 0.9 = 6.975
const TOUGH_JERKY: u32 = 117; // the goods vendor's

#[test]
fn a_token_white_costs_what_the_spec_says_for_its_slot() {
    assert_eq!(price(NOVICES_WOOLEN_VEST), Some(3));
    assert_eq!(price(MILITIA_CLAYMORE), Some(3));
    assert_eq!(price(MILITIA_SHORTSWORD), Some(2));
    assert_eq!(price(PENDANT_OF_BRAWN), Some(1));
}

#[test]
fn a_green_costs_its_item_level_over_four_scaled_by_its_slot() {
    assert_eq!(price(SHADOWHIDE_TWO_HANDED_SWORD), Some(5));
    assert_eq!(price(BURNISHED_TUNIC), Some(5));
    assert_eq!(price(LIGHT_BOW), Some(3));
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
    assert_eq!(first.len(), 22, "the Token whites");
    assert_eq!(first[0], (NOVICES_WOOLEN_VEST, 3));
    assert_eq!(stock(10), first);
    assert_eq!(stock(15).len(), 12);
    assert_eq!(stock(25).len(), 13);
    assert!(stock(25).contains(&(GLIMMERING_MAIL_BREASTPLATE, 7)));
    assert!(!stock(25).iter().any(|&(e, _)| e == NOVICES_WOOLEN_VEST));
}
