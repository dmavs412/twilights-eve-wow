//! Talent Points, the respec's price, and the numbers talents change. Which talent does what is
//! data (`content/callings.toml`); these are the sums it comes to.

/// Copper in a gold piece.
const GOLD: u32 = 10_000;

/// The Talent Points a character of `level` has earned: one a level from level 2, so 9 at
/// Tier 0's cap and 29 at Tier 1's. Advancement resets the level, and the points with it.
pub fn points_at(level: u8) -> u32 {
    u32::from(level.saturating_sub(1))
}

/// What the respec after `wipes` earlier ones costs, in copper: vmangos's
/// `Player::resetTalentsCost` (1 g, 5 g, 10 g, then 5 g more a time, at most 50 g), without its
/// monthly discount, since the realm keeps no calendar.
pub fn respec_cost(wipes: u32) -> u32 {
    match wipes {
        0 => GOLD,
        1 => 5 * GOLD,
        n => n.saturating_mul(5 * GOLD).min(50 * GOLD),
    }
}

/// `amount` raised by `pct` percent, rounded half away from zero.
pub fn scaled(amount: u32, pct: u32) -> u32 {
    (f64::from(amount) * f64::from(100 + pct) / 100.0).round() as u32
}

/// One tick of Deep Wounds: `pct`% of the main hand's average swing (attack power included, as
/// vmangos reads `UNIT_FIELD_MINDAMAGE`/`MAXDAMAGE`), spread over its four ticks, at least 1.
pub fn deep_wound_tick(min_damage: f32, max_damage: f32, pct: u32) -> u32 {
    let total = f64::from(min_damage + max_damage) / 2.0 * f64::from(pct) / 100.0;
    ((total / 4.0).round() as u32).max(1)
}

/// `pct`% of `damage`, rounded half away from zero (Scent of Blood's bleed from a crit).
pub fn scaled_share(damage: u32, pct: u32) -> u32 {
    (f64::from(damage) * f64::from(pct) / 100.0).round() as u32
}

/// A melee crit on `damage` (after armour): 200%, times `bonus_pct`% more (`mod_crit_damage`, Impale)
/// of the whole crit, so +30% makes 260%.
pub fn crit_damage(damage: u32, bonus_pct: u32) -> u32 {
    crit_multiplied(damage, 200, bonus_pct)
}

/// `damage` at `crit_pct`% (200 melee, 150 spell), times `bonus_pct`% more, rounded down.
pub fn crit_multiplied(damage: u32, crit_pct: u32, bonus_pct: u32) -> u32 {
    (u64::from(damage) * u64::from(crit_pct) * u64::from(100 + bonus_pct) / 10_000) as u32
}

/// One of `ticks` heal ticks that give back `pct`% of `max_health` in all (Blood Craze), at
/// least 1.
pub fn heal_tick(max_health: u32, pct: u32, ticks: u32) -> u32 {
    (max_health * pct / 100 / ticks.max(1)).max(1)
}
