//! The Seals' numbers: how often a Seal's swing proc goes off, and what it deals.

use super::melee::ROLL;

/// A procs-per-minute chance on one swing of `attack_time_ms`, in basis points of [`ROLL`]
/// (`Unit::GetPPMProcChance`: `ppm × speed / 60`).
pub fn ppm_chance(ppm: f32, attack_time_ms: u32) -> u32 {
    let chance = f64::from(ppm) * f64::from(attack_time_ms) / 60_000.0;
    (chance * f64::from(ROLL)).round() as u32
}

/// `pct`% of a weapon `roll`, rounded (`SPELL_EFFECT_WEAPON_PERCENT_DAMAGE`, Seal of Command's
/// 70%).
pub fn weapon_pct_damage(roll: f32, pct: u32) -> u32 {
    (f64::from(roll) * f64::from(pct) / 100.0).round().max(0.0) as u32
}

/// A swing time of `attack_time_ms` made `haste_pct`% faster (Seal of the Crusader's 40%): the
/// time over `1 + haste`, rounded to the millisecond.
pub fn hasted_attack_time(attack_time_ms: u32, haste_pct: u32) -> u32 {
    (f64::from(attack_time_ms) * 100.0 / f64::from(100 + haste_pct)).round() as u32
}

/// A hit's `damage` cut to match `haste_pct`% faster swings (Seal of the Crusader's "deals less
/// damage with each attack"): over `1 + haste`, so the haste alone adds no damage a second.
pub fn haste_cut_damage(damage: f32, haste_pct: u32) -> f32 {
    damage * 100.0 / (100 + haste_pct) as f32
}

/// `damage` of holy at a target that takes `pct`% more of it (Judgement of the Crusader),
/// rounded.
pub fn holy_taken(damage: u32, pct: u32) -> u32 {
    (f64::from(damage) * f64::from(100 + pct) / 100.0).round() as u32
}
