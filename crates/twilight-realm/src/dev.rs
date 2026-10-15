//! The dev multipliers: testing aids the host reads from its boot flags (`?speed=` / `?xp=`). They
//! belong to the realm's run, never to a character, so no save carries them.

/// How much faster the player runs and how much more XP they earn. Both default to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DevMultipliers {
    /// Scales the player's movement speeds.
    pub speed: f32,
    /// Scales every XP gain.
    pub xp: f32,
}

impl Default for DevMultipliers {
    fn default() -> Self {
        Self { speed: 1.0, xp: 1.0 }
    }
}

impl DevMultipliers {
    /// `amount` XP scaled by the XP multiplier, to the nearest whole point.
    pub(crate) fn xp(&self, amount: u32) -> u32 {
        (amount as f32 * self.xp).round() as u32
    }
}
