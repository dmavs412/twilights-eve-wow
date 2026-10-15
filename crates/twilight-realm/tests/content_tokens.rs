//! The Token vendor's stock (`content/token_vendor.toml`) against the research it comes from,
//! `data/twilights-eve/token_vendor.json` (`docs/reference/items-and-vendors.md`).

use serde_json::Value;
use twilight_realm::rules::tokens::stock;

fn reference() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/twilights-eve/token_vendor.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn each_band_sells_the_researched_greens_at_the_researched_price() {
    let reference = reference();
    for (band, level) in [("1-10", 1), ("11-20", 11), ("21-30", 21)] {
        let want: Vec<(u32, u32)> = reference["bands"][band]
            .as_array()
            .unwrap()
            .iter()
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
fn every_green_is_inside_its_band_by_required_level() {
    let reference = reference();
    for (band, low, high) in [("1-10", 1, 10), ("11-20", 11, 20), ("21-30", 21, 30)] {
        for row in reference["bands"][band].as_array().unwrap() {
            let required = row["required_level"].as_u64().unwrap();
            assert!((low..=high).contains(&required), "{row}");
            assert_eq!(row["item"]["quality"], 2, "a green: {row}");
        }
    }
}
