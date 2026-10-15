//! The realm's two units of time: content and packets count milliseconds, the clocks that tick
//! count seconds.

/// `ms` milliseconds, in seconds.
pub(crate) fn secs(ms: u32) -> f32 {
    ms as f32 / 1000.0
}

/// `secs` seconds, in whole milliseconds (rounded; never below 0).
pub(crate) fn ms(secs: f32) -> u32 {
    (secs * 1000.0).round().max(0.0) as u32
}
