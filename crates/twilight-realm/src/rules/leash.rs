//! How far a mob follows the player before it gives up, runs home, heals and resets (the spec's
//! 40–60 yd; vmangos leashes on the distance from the creature's home).

/// The farthest a chasing mob strays from its home.
pub const LEASH_YD: f32 = 50.0;

/// Whether a mob at `at` has strayed past its leash from `home`.
pub fn beyond(home: [f32; 3], at: [f32; 3]) -> bool {
    let d2 = (home[0] - at[0]).powi(2) + (home[1] - at[1]).powi(2) + (home[2] - at[2]).powi(2);
    d2 > LEASH_YD * LEASH_YD
}
