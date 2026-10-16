//! **Rested XP**, in the world: standing in an inn raises `PLAYER_FLAGS_RESTING` and fills the
//! pool ([`crate::rest`]); stepping out lowers it.

use benilla_protocol::SessionEvent;

use super::InWorld;
use crate::characters::Char;
use crate::fields::values;

impl InWorld {
    /// The resting flag as the player stands, and `dt` seconds of rest while it is up.
    pub(super) fn rest(&mut self, dt: f32, c: &mut Char, events: &mut Vec<SessionEvent>) {
        let resting = crate::rest::in_inn(self.map, c.position);
        if resting != self.resting {
            self.resting = resting;
            events.push(values(self.guid, &[crate::rest::flags(resting)]));
        }
        if !resting {
            return;
        }
        let shown = c.rest.pool as u32;
        crate::rest::rest_in_inn(c, dt);
        if c.rest.pool as u32 != shown {
            events.push(values(self.guid, &crate::rest::pairs(c)));
        }
    }
}
