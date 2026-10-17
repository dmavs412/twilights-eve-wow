//! Durability (spec #1, "Gear and economy" and "Death"): what a repair costs, and what death and
//! combat wear off. vmangos is the reference (`Player::DurabilityRepair`,
//! `Player::DurabilityLossAll`, `Unit::DealDamage`); the tables are `content/durability.toml`.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::items;
use crate::rules::gear::{CLASS_ARMOR, CLASS_WEAPON};

#[derive(Deserialize)]
struct Table {
    quality: Vec<f64>,
    costs: Vec<Vec<u32>>,
}

fn table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    TABLE.get_or_init(|| {
        toml::from_str(include_str!("../../content/durability.toml"))
            .expect("content/durability.toml parses")
    })
}

/// What repairing `lost` points costs on an item of `item_level`, `quality`, `class` and
/// `subclass`, in copper: `lost × DurabilityCosts[item_level][index] × DurabilityQuality[quality]`
/// truncated, and at least 1 when anything is lost. Item levels past the table's 60 price as 60.
pub fn repair_cost_of(item_level: u8, quality: u8, class: u32, subclass: u32, lost: u32) -> u32 {
    if lost == 0 {
        return 0;
    }
    let t = table();
    let row = &t.costs[usize::from(item_level.clamp(1, t.costs.len() as u8)) - 1];
    // `ItemSubClassToDurabilityMultiplierId`.
    let index = match class {
        CLASS_WEAPON => subclass as usize,
        CLASS_ARMOR => subclass as usize + 21,
        _ => 0,
    };
    let multiplier = f64::from(row.get(index).copied().unwrap_or(0));
    let quality = t.quality.get(usize::from(quality)).copied().unwrap_or(1.0);
    // No reputation discount: the slice models none, so `cost * 1.0 + 0.5` rounds nothing away.
    let cost = (f64::from(lost) * multiplier * quality) as u32;
    cost.max(1)
}

/// What repairing item `entry`, now at `durability`, back to full costs, in copper.
pub fn repair_cost(entry: u32, durability: u32) -> u32 {
    items::template(entry).map_or(0, |t| {
        repair_cost_of(
            t.item_level,
            t.quality,
            t.class,
            t.subclass,
            t.durability.saturating_sub(durability),
        )
    })
}

/// The points an item of `max` durability loses when its wearer dies: a tenth, at least one.
pub fn death_loss(max: u32) -> u32 {
    if max == 0 {
        0
    } else {
        (max / 10).max(1)
    }
}

/// The copper death takes from `money`: a tenth (the spec's, not vanilla's).
pub fn death_money_loss(money: u32) -> u32 {
    money / 10
}

/// The roll [`wears_on_hit`] takes, `0..WEAR_ROLL`.
pub const WEAR_ROLL: u32 = 1000;

/// Whether a damaging hit the player dealt or took wears one point off a random equipment slot
/// (`DurabilityLossChance.Damage = 0.5`, half a percent).
pub fn wears_on_hit(roll: u32) -> bool {
    roll < 5
}

/// The equipment slots a wearing hit picks from: the ten Gear Slots' (neck, chest, fingers,
/// trinkets, main hand, off hand, ranged; ADR 0007), where vanilla picks from all nineteen. An
/// empty one loses nothing.
pub const WEAR_SLOTS: [u8; 9] = [1, 4, 10, 11, 12, 13, 15, 16, 17];
