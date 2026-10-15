//! The Token vendor's prices and level bands (`content/token_vendor.toml`,
//! `docs/reference/items-and-vendors.md` "Token vendor"): greens priced in Tokens, offered by the
//! band of required levels the buyer's level falls in.

use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Deserialize)]
struct Band {
    /// The lowest and highest required level it sells.
    levels: (u8, u8),
    item: Vec<Row>,
}

#[derive(Deserialize)]
struct Row {
    entry: u32,
    tokens: u32,
}

#[derive(Deserialize)]
struct File {
    band: Vec<Band>,
}

fn bands() -> &'static [Band] {
    static BANDS: OnceLock<Vec<Band>> = OnceLock::new();
    BANDS.get_or_init(|| {
        let file: File = toml::from_str(include_str!("../../content/token_vendor.toml"))
            .expect("content/token_vendor.toml parses");
        file.band
    })
}

/// The band a character of `level` is offered; the last band serves every level above it.
fn band_of(level: u8) -> &'static Band {
    let all = bands();
    all.iter()
        .find(|b| level <= b.levels.1)
        .unwrap_or_else(|| all.last().expect("the Token vendor has a band"))
}

/// The band of required levels a character of `level` is offered, `(lowest, highest)`.
pub fn band(level: u8) -> (u8, u8) {
    band_of(level).levels
}

/// What the Token vendor offers a character of `level`: `(item entry, Tokens)`, in the merchant
/// window's order.
pub fn stock(level: u8) -> Vec<(u32, u32)> {
    band_of(level)
        .item
        .iter()
        .map(|r| (r.entry, r.tokens))
        .collect()
}

/// The Token price of `entry`, if the Token vendor sells it in any band.
pub fn price(entry: u32) -> Option<u32> {
    bands()
        .iter()
        .flat_map(|b| &b.item)
        .find(|r| r.entry == entry)
        .map(|r| r.tokens)
}
