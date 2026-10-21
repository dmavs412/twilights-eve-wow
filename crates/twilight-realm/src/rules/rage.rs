//! Rage (vmangos `Player::RewardRage`, `Player::Regenerate`). Stored in tenths: 0..=1000 is
//! 0..=100 rage, as `UNIT_FIELD_POWER2` carries it.

/// A full rage bar, in tenths.
pub const MAX: u32 = 1000;

/// Tenths lost per decay tick out of combat (vmangos: 2 rage every 2 s).
pub const DECAY_PER_TICK: u32 = 20;

/// Seconds between decay ticks (`REGEN_TIME_PLAYER_FULL`).
pub const DECAY_TICK_SECS: f32 = 2.0;

/// The global rage gain multiplier, from `data/twilights-eve/tuning.json`. Every gain is scaled
/// by it where the bar takes it in; costs and decay are not.
pub fn gain() -> f64 {
    static GAIN: std::sync::OnceLock<f64> = std::sync::OnceLock::new();
    *GAIN.get_or_init(|| {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Tuning {
            rage_gain: f64,
        }
        let tuning: Tuning =
            serde_json::from_str(include_str!("../../../../data/twilights-eve/tuning.json"))
                .expect("data/twilights-eve/tuning.json parses");
        tuning.rage_gain
    })
}

/// `tenths` of rage earned, times `gain`, truncated.
pub fn scaled(tenths: u32, gain: f64) -> u32 {
    (f64::from(tenths) * gain) as u32
}

/// The rage conversion value at `level`.
fn conversion(level: u8) -> f64 {
    let l = f64::from(level);
    0.009_110_783_6 * l * l + 3.225_598_133 * l + 4.265_291_1
}

/// Tenths of rage for dealing `damage` with a white swing at `level` (after armour).
pub fn from_damage_dealt(damage: u32, level: u8) -> u32 {
    (f64::from(damage) / conversion(level) * 7.5 * 10.0) as u32
}

/// Tenths of rage for taking `damage` at `level` (after mitigation).
pub fn from_damage_taken(damage: u32, level: u8) -> u32 {
    (f64::from(damage) / conversion(level) * 2.5 * 10.0) as u32
}

/// Tenths of rage for a white swing a dodge or parry stopped at `level`: what three quarters of
/// the damage it `would_be` would have earned (vmangos `Unit::AttackerStateUpdate`).
pub fn from_avoided(would_be: u32, level: u8) -> u32 {
    from_damage_dealt(would_be * 3 / 4, level)
}
