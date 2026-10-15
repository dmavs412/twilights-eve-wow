//! The dev multipliers: testing aids the host reads from its boot flags (`?speed=` / `?xp=`). They
//! belong to the realm's run, never to a character, so no save carries them.

use benilla_protocol::MoveSpeeds;

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
    /// The multipliers from the boot flags' raw values (`?speed=` / `WOW_TWILIGHT_SPEED`,
    /// `?xp=` / `WOW_TWILIGHT_XP`). A flag that is missing, or not a positive finite number, is 1.
    pub fn from_flags(speed: Option<&str>, xp: Option<&str>) -> Self {
        fn factor(raw: Option<&str>) -> f32 {
            raw.and_then(|v| v.trim().parse::<f32>().ok())
                .filter(|f| f.is_finite() && *f > 0.0)
                .unwrap_or(1.0)
        }
        Self {
            speed: factor(speed),
            xp: factor(xp),
        }
    }

    /// `speeds` with every movement speed scaled by the speed multiplier; turning is not movement.
    pub(crate) fn speeds(&self, speeds: MoveSpeeds) -> MoveSpeeds {
        MoveSpeeds {
            walk: speeds.walk * self.speed,
            run: speeds.run * self.speed,
            run_back: speeds.run_back * self.speed,
            swim: speeds.swim * self.speed,
            swim_back: speeds.swim_back * self.speed,
            turn_rate: speeds.turn_rate,
        }
    }

    /// `amount` XP scaled by the XP multiplier, to the nearest whole point.
    pub(crate) fn xp(&self, amount: u32) -> u32 {
        (amount as f32 * self.xp).round() as u32
    }
}
