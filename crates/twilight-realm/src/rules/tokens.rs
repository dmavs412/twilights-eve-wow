//! The Token vendors' prices and stock (`content/token_vendor.toml`,
//! `docs/reference/items-and-vendors.md` "Token vendor"): greens priced in Tokens. The Crossroads
//! and Northshire vendors offer the band of required levels the buyer's level falls in; the
//! Goldshire vendor sells one flat list at any level.

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

/// The Goldshire vendor's list, sold at any level.
#[derive(Deserialize)]
struct Flat {
    item: Vec<Row>,
}

#[derive(Deserialize)]
struct File {
    band: Vec<Band>,
    flat: Flat,
}

fn file() -> &'static File {
    static FILE: OnceLock<File> = OnceLock::new();
    FILE.get_or_init(|| {
        toml::from_str(include_str!("../../content/token_vendor.toml"))
            .expect("content/token_vendor.toml parses")
    })
}

fn bands() -> &'static [Band] {
    &file().band
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

/// What the Goldshire Token vendor offers at any level: `(item entry, Tokens)`, in the merchant
/// window's order.
pub fn flat_stock() -> Vec<(u32, u32)> {
    file()
        .flat
        .item
        .iter()
        .map(|r| (r.entry, r.tokens))
        .collect()
}

/// The Token price of `entry` at the Goldshire Token vendor, if it sells it.
pub fn flat_price(entry: u32) -> Option<u32> {
    file()
        .flat
        .item
        .iter()
        .find(|r| r.entry == entry)
        .map(|r| r.tokens)
}
