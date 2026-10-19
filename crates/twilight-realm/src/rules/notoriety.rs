//! **Notoriety** and its **Heat Levels** (`GLOSSARY.md`, spec #175, `content/notoriety.toml`):
//! what each achievement adds, how fast the Hub drains it, and what each Heat Level multiplies XP
//! by.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::content::{CROSSROADS, KALIMDOR};
use crate::rules::xp::{self, Elite};

#[derive(Deserialize)]
struct Gain {
    kill: f32,
    kill_per_level: f32,
    kill_min: f32,
    elite: f32,
    boss: f32,
    dungeon_clear: f32,
    quest: f32,
    rare_drop: f32,
}

#[derive(Deserialize)]
struct Drain {
    per_second: f32,
    hub_radius: f32,
}

#[derive(Deserialize)]
struct Heat {
    at: f32,
    xp: f32,
    aura: u32,
    hunter_minutes: f32,
}

#[derive(Deserialize)]
struct Rank {
    step: f32,
    affix_every: u32,
}

#[derive(Deserialize)]
struct Coin {
    item: u32,
    base: u32,
    per_rank: u32,
}

#[derive(Deserialize)]
struct CoinRow {
    entry: u32,
    coins: u32,
}

#[derive(Deserialize)]
struct File {
    gain: Gain,
    drain: Drain,
    heat: Vec<Heat>,
    rank: Rank,
    coin: Coin,
    coin_stock: Vec<CoinRow>,
}

fn file() -> &'static File {
    static FILE: OnceLock<File> = OnceLock::new();
    FILE.get_or_init(|| {
        toml::from_str(include_str!("../../content/notoriety.toml"))
            .expect("content/notoriety.toml parses")
    })
}

/// The Heat Level `notoriety` stands at: 0 below the first threshold, else how many it has
/// reached.
pub fn heat_level(notoriety: f32) -> u8 {
    file().heat.iter().filter(|h| notoriety >= h.at).count() as u8
}

/// How many Heat Levels there are.
pub fn heat_levels() -> u8 {
    file().heat.len() as u8
}

/// The Notoriety Heat Level `level` starts at; 0 for level 0.
pub fn threshold(level: u8) -> f32 {
    level
        .checked_sub(1)
        .and_then(|i| file().heat.get(usize::from(i)))
        .map_or(0.0, |h| h.at)
}

/// What Heat Level `level` multiplies every XP award by: ×1 at level 0.
pub fn xp_multiplier(level: u8) -> f32 {
    level
        .checked_sub(1)
        .and_then(|i| file().heat.get(usize::from(i)))
        .map_or(1.0, |h| h.xp)
}

/// An award of `xp` at Heat Level `level`, rounded to the nearest point.
pub fn with_heat(xp: u32, level: u8) -> u32 {
    (xp as f32 * xp_multiplier(level)).round() as u32
}

/// The client spell Heat Level `level` shows as an aura; `None` at level 0.
pub fn heat_aura(level: u8) -> Option<u32> {
    level
        .checked_sub(1)
        .and_then(|i| file().heat.get(usize::from(i)))
        .map(|h| h.aura)
}

/// The seconds of open-world play between Bounty Hunters at Heat Level `level`; `None` at
/// level 0, which sends none.
pub fn hunter_secs(level: u8) -> Option<f32> {
    level
        .checked_sub(1)
        .and_then(|i| file().heat.get(usize::from(i)))
        .map(|h| h.hunter_minutes * 60.0)
}

/// The Notoriety a `player`-level character earns killing a `mob`-level creature: nothing for a
/// gray one, more for each level above them, more again for an elite or a Dungeon boss, and a
/// boss's Dungeon clear on top.
pub fn kill_gain(player: u8, mob: u8, elite: Elite, boss: bool) -> f32 {
    if mob <= xp::gray_level(player) {
        return 0.0;
    }
    let g = &file().gain;
    let levels = f32::from(mob) - f32::from(player);
    let mut gain = (g.kill + g.kill_per_level * levels).max(g.kill_min);
    if elite != Elite::No {
        gain *= g.elite;
    }
    if boss {
        gain = gain * g.boss + g.dungeon_clear;
    }
    gain
}

/// The Notoriety a quest hand-in earns.
pub fn quest_gain() -> f32 {
    file().gain.quest
}

/// The Notoriety an item of `quality` in a corpse's loot earns: only rare (blue) or better.
pub fn drop_gain(quality: u8) -> f32 {
    if quality >= RARE {
        file().gain.rare_drop
    } else {
        0.0
    }
}

/// `ITEM_QUALITY_RARE`.
const RARE: u8 = 3;

/// `notoriety` after `secs` resting in the Hub: lower by the drain rate, never below 0.
pub fn drained(notoriety: f32, secs: f32) -> f32 {
    (notoriety - file().drain.per_second * secs).max(0.0)
}

/// `notoriety` after Advancement: halved, rounded down.
pub fn advanced(notoriety: f32) -> f32 {
    (notoriety / 2.0).floor()
}

/// Whether `position` on `map` lies in the Hub, where Notoriety drains.
pub fn in_hub(map: u32, position: [f32; 3]) -> bool {
    map == KALIMDOR
        && (position[0] - CROSSROADS[0]).hypot(position[1] - CROSSROADS[1])
            <= file().drain.hub_radius
}

/// What a Bounty Hunter's health and damage are multiplied by when its owner's Bounty Rank is
/// `rank`.
pub fn rank_multiplier(rank: u32) -> f32 {
    1.0 + file().rank.step * rank as f32
}

/// How many affixes a Bounty Hunter carries at Bounty Rank `rank`, before the pool's size caps
/// it.
pub fn affix_count(rank: u32) -> usize {
    (rank / file().rank.affix_every.max(1)) as usize
}

/// The Blood Coin item.
pub fn blood_coin() -> u32 {
    file().coin.item
}

/// The Blood Coin a hunter of Bounty Rank `rank` drops into each credited player's loot.
pub fn coin_drop(rank: u32) -> u32 {
    let c = &file().coin;
    c.base + c.per_rank * rank
}

/// The Blood Coin vendor's stock, `(item entry, Blood Coin)`, in the merchant window's order.
pub fn coin_stock() -> Vec<(u32, u32)> {
    file()
        .coin_stock
        .iter()
        .map(|r| (r.entry, r.coins))
        .collect()
}

/// The Blood Coin price of `entry`, if the Blood Coin vendor sells it.
pub fn coin_price(entry: u32) -> Option<u32> {
    file()
        .coin_stock
        .iter()
        .find(|r| r.entry == entry)
        .map(|r| r.coins)
}
