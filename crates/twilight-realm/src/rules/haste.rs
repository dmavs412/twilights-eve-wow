//! Melee haste: a faster swing, and the per-hit cut that goes with it when the haste is not
//! meant to add damage.

/// A swing time of `attack_time_ms` made `haste_pct`% faster (Seal of the Crusader's 40%): the
/// time over `1 + haste`, rounded to the millisecond.
pub fn hasted_attack_time(attack_time_ms: u32, haste_pct: u32) -> u32 {
    (f64::from(attack_time_ms) * 100.0 / f64::from(100 + haste_pct)).round() as u32
}

/// A white swing's `damage` cut to match `haste_pct`% faster swings (Seal of the Crusader's "deals less
/// damage with each attack"): over `1 + haste`, so the haste alone adds no damage a second.
pub fn haste_cut_damage(damage: f32, haste_pct: u32) -> f32 {
    damage * 100.0 / (100 + haste_pct) as f32
}
