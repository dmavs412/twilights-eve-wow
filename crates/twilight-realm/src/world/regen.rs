//! Natural health regeneration: out of combat, a living player heals a Spirit-based share every
//! tick ([`crate::rules::regen`]), more with a health-regen-percent effect.

use benilla_protocol::SessionEvent;

use super::{cast, InWorld};
use crate::characters::Char;
use crate::fields::*;
use crate::rules::regen;

impl InWorld {
    /// Regeneration goes on by `dt`. A fight, or a full bar, restarts the tick.
    pub(super) fn regenerate(&mut self, dt: f32, c: &Char, events: &mut Vec<SessionEvent>) {
        let max = cast::buffed(&self.player.stats, &self.skills).max_health;
        let p = &mut self.player;
        if !p.alive() || p.in_combat || p.health >= max {
            p.regen = 0.0;
            return;
        }
        p.regen += dt;
        let mut healed = false;
        while p.regen >= regen::TICK_SECS {
            p.regen -= regen::TICK_SECS;
            let fed = crate::cooking::fed(c).map_or(0, |f| f.regen_pct);
            let pct = crate::talents::health_regen_pct(c) + fed;
            p.health = (p.health + regen::tick_heal(p.stats.base[4], pct)).min(max);
            healed = true;
        }
        if healed {
            events.push(values(self.guid, &[(UNIT_FIELD_HEALTH, p.health)]));
        }
    }
}
