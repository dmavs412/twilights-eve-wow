//! The Token vendor's stock (`content/token_vendor.toml`): the 1 to 10 band is the 22 Token
//! whites of spec #272, hand-authored rows of the gear sheet; the bands from 11 up are checked
//! against the research they come from, `data/twilights-eve/token_vendor.json`
//! (`docs/reference/items-and-vendors.md`), less the greens for a body part outside the ten
//! Gear Slots (ADR 0007).

use serde_json::Value;
use twilight_realm::rules::gear::is_gear_slot;
use twilight_realm::rules::tokens::stock;

fn reference() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/twilights-eve/token_vendor.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn each_band_from_11_up_sells_the_researched_greens_at_the_researched_price() {
    let reference = reference();
    for (band, level) in [("11-20", 11), ("21-30", 21)] {
        let want: Vec<(u32, u32)> = reference["bands"][band]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| is_gear_slot(row["item"]["inventory_type"].as_u64().unwrap() as u32))
            .map(|row| {
                (
                    row["entry"].as_u64().unwrap() as u32,
                    row["token_cost"].as_u64().unwrap() as u32,
                )
            })
            .collect();
        assert_eq!(stock(level), want, "band {band}");
    }
}

#[test]
fn every_researched_green_is_inside_its_band_by_required_level() {
    let reference = reference();
    for (band, low, high) in [("11-20", 11, 20), ("21-30", 21, 30)] {
        for row in reference["bands"][band].as_array().unwrap() {
            let required = row["required_level"].as_u64().unwrap();
            assert!((low..=high).contains(&required), "{row}");
            assert_eq!(row["item"]["quality"], 2, "a green: {row}");
        }
    }
}

/// The 1 to 10 band is the 22 Token whites and nothing else: each a row of the gear sheet
/// (`content/gear.toml`, generated from it) of quality Common at required level 1, priced 1 to
/// 3 Tokens, and the same at every level of the band.
#[test]
fn the_first_band_is_the_twenty_two_token_whites_of_the_gear_sheet() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("content/gear.toml");
    let gear: toml::Value = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let rows = gear["item"].as_array().unwrap();
    let band = stock(1);
    assert_eq!(band.len(), 22);
    assert_eq!(stock(5), band);
    assert_eq!(stock(10), band);
    for (entry, tokens) in band {
        let row = rows
            .iter()
            .find(|r| r["entry"].as_integer() == Some(i64::from(entry)))
            .unwrap_or_else(|| panic!("{entry} is a row of the gear sheet"));
        assert_eq!(row["quality"].as_integer(), Some(1), "{entry} is Common");
        assert_eq!(row["required_level"].as_integer(), Some(1), "{entry}");
        assert!((1..=3).contains(&tokens), "{entry} costs {tokens}");
    }
}
