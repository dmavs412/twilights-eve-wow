//! When a killed mob comes back: about a minute after the player has left its area (the spec
//! replaces vanilla's 5–7 minute timers, which ran whether or not anyone was there).

/// Seconds the player must be away before a dead mob respawns.
pub const SECS: f32 = 60.0;

/// Seconds for a rare (`CreatureEliteType` 4): a named creature that is worth hunting for stays
/// down ten times as long.
pub const RARE_SECS: f32 = 600.0;

/// The seconds the player must be away before a dead mob of `rank` respawns.
pub fn wait(rank: u32) -> f32 {
    if rank == 4 {
        RARE_SECS
    } else {
        SECS
    }
}

/// How far from a mob's home the player must be to count as away: past the widest aggro radius
/// (45 yd), so a mob never comes back on top of the player who killed it.
pub const AWAY_YD: f32 = 60.0;

/// The seconds a dead mob still waits after `dt` more seconds with the player `player_distance`
/// yards from its home: the clock runs only while the player is away, and starts over at `wait`
/// each time they come back.
pub fn countdown(remaining: f32, wait: f32, dt: f32, player_distance: f32) -> f32 {
    if player_distance > AWAY_YD {
        remaining - dt
    } else {
        wait
    }
}
