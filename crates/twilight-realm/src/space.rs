//! Distances in the world, and how near the player must stand to deal with something.

/// How near an NPC or a corpse the player must stand to deal with it (`INTERACTION_DISTANCE`,
/// with slack for the bodies' reach and the movement reports' lag), measured on the ground.
pub(crate) const INTERACTION_YD: f32 = 10.0;

/// The straight-line distance between `a` and `b`.
pub(crate) fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// The distance between `a` and `b` on the ground, heights aside.
pub(crate) fn flat_distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// Whether the player at `me` stands near enough to `at` to deal with what is there.
pub(crate) fn within_interaction(at: [f32; 3], me: [f32; 3]) -> bool {
    flat_distance(at, me) <= INTERACTION_YD
}
