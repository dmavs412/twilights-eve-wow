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
        self.flask_regenerate(dt, events);
        let max = cast::buffed(&self.player.stats, &self.skills).max_health;
        let fed = self.fed_regen_pct();
        let p = &mut self.player;
        if !p.alive() || p.in_combat || p.health >= max {
            p.regen = 0.0;
            return;
        }
        p.regen += dt;
        let mut healed = false;
        while p.regen >= regen::TICK_SECS {
            p.regen -= regen::TICK_SECS;
            let pct = crate::talents::health_regen_pct(c) + fed;
            p.health = (p.health + regen::tick_heal(p.stats.base[4], pct)).min(max);
            healed = true;
        }
        if healed {
            events.push(values(self.guid, &[(UNIT_FIELD_HEALTH, p.health)]));
        }
    }

    /// A flask's `health_per_5` and `power_per_5` every 5 s, in combat and out, on top of natural
    /// regeneration.
    fn flask_regenerate(&mut self, dt: f32, events: &mut Vec<SessionEvent>) {
        let (health, power) = (
            self.player.stats.health_per_5,
            self.player.stats.power_per_5,
        );
        if (health == 0 && power == 0) || !self.player.alive() {
            self.flask_regen = 0.0;
            return;
        }
        self.flask_regen += dt;
        if self.flask_regen < FLASK_TICK_SECS {
            return;
        }
        self.flask_regen -= FLASK_TICK_SECS;
        let max = cast::buffed(&self.player.stats, &self.skills).max_health;
        let p = &mut self.player;
        if health > 0 && p.health < max {
            p.health = (p.health + health).min(max);
            events.push(values(self.guid, &[(UNIT_FIELD_HEALTH, p.health)]));
        }
        let kind = p.power.kind;
        if power > 0 && p.power.gain(kind, power) {
            events.push(values(self.guid, &[p.power.field()]));
        }
    }

    /// Sets the player's health to `health`, at most their maximum (a test seam).
    pub(crate) fn set_health(&mut self, health: u32, events: &mut Vec<SessionEvent>) {
        let max = cast::buffed(&self.player.stats, &self.skills).max_health;
        let p = &mut self.player;
        p.health = health.min(max);
        events.push(values(self.guid, &[(UNIT_FIELD_HEALTH, p.health)]));
    }

    /// Sets the player's power to `amount` as the client's field holds it, at most their
    /// maximum: a test seam.
    pub(crate) fn set_power(&mut self, amount: u32, events: &mut Vec<SessionEvent>) {
        let power = &mut self.player.power;
        power.current = amount.min(power.max);
        events.push(values(self.guid, &[power.field()]));
    }
}

/// A flask's regeneration tick.
const FLASK_TICK_SECS: f32 = 5.0;
