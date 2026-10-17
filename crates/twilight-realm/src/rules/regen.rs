//! Natural health regeneration: out of combat, a tick every two seconds heals a share of Spirit
//! (vanilla's `Player::RegenerateHealth` reads `gtRegenHPPerSpt`; the realm has no game data, so
//! it uses one flat ratio, `HP_PER_SPIRIT`).

/// Seconds between regeneration ticks.
pub const TICK_SECS: f32 = 2.0;

/// Health a tick heals per point of Spirit, before any percent.
const HP_PER_SPIRIT: f32 = 0.25;

/// What one tick heals at `spirit`, with `regen_pct` percent more (a health-regen-percent
/// effect). At least one point.
pub fn tick_heal(spirit: u32, regen_pct: u32) -> u32 {
    let raw = spirit as f32 * HP_PER_SPIRIT * (100 + regen_pct) as f32 / 100.0;
    (raw.round() as u32).max(1)
}
