//! When a hostile mob notices a player (vmangos `Creature::GetAttackDistance`).

/// How close a player of `player_level` may come before a mob of `mob_level` with the template's
/// `detection` range attacks: a yard more per level the mob is above (at most 25), a yard less
/// per level below, never under 5 yd (or `detection`, if smaller). 0 when it never aggroes on
/// sight.
pub fn radius(detection: f32, player_level: u8, mob_level: u8) -> f32 {
    if detection < 1.0 {
        return 0.0;
    }
    let diff = (i32::from(player_level) - i32::from(mob_level)).max(-25);
    (detection - diff as f32).max(detection.min(5.0))
}

/// How near a pulled mob its campmates must stand to hear it (social aggro).
pub const SOCIAL_RANGE_YD: f32 = 8.0;

/// The fastest and slowest a campmate answers: the nearest comes first, the farthest last.
pub const SOCIAL_DELAY_SECS: (f32, f32) = (0.5, 2.0);

/// Seconds a campmate `d` yards from the one pulled waits before it joins.
pub fn social_delay(d: f32) -> f32 {
    let (fast, slow) = SOCIAL_DELAY_SECS;
    fast + (slow - fast) * (d / SOCIAL_RANGE_YD).clamp(0.0, 1.0)
}
