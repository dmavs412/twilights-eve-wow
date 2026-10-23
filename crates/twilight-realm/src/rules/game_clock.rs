//! The game clock (ADR 0019): the realm's wall clock times [`SPEED`], taken modulo a day. One game
//! day is 2 real hours. It reads nothing but the wall clock, so it keeps running while every
//! character is logged out and needs no save. `wall_secs` is the realm's wall clock in seconds
//! since the Unix epoch, the same `wall` the world's tick takes.

/// Game seconds per real second.
pub const SPEED: f64 = 12.0;

/// Game minutes per real second, the timescale `SMSG_LOGIN_SETTIMESPEED` sends the client.
pub const TIMESCALE: f32 = 0.2;

/// Minutes in a game day.
const MINUTES_PER_DAY: u64 = 24 * 60;

/// The game minute of the day (0 to 1439) at the wall clock's `wall_secs`.
pub fn minute_of_day(wall_secs: f64) -> u32 {
    ((wall_secs * SPEED / 60.0).floor() as u64 % MINUTES_PER_DAY) as u32
}

/// The game hour (0 to 23) and minute (0 to 59) at the wall clock's `wall_secs`.
pub fn hour_minute(wall_secs: f64) -> (u8, u8) {
    let m = minute_of_day(wall_secs);
    ((m / 60) as u8, (m % 60) as u8)
}

/// The number of whole game days since the epoch at the wall clock's `wall_secs`, so the moon
/// phase moves with the clock.
pub fn day_serial(wall_secs: f64) -> u32 {
    (wall_secs * SPEED / 86_400.0).floor() as u32
}
