//! A rooted player (#240): `SMSG_FORCE_MOVE_ROOT` holds them in place for the spell's duration and
//! `SMSG_FORCE_MOVE_UNROOT` lets them go. They can still cast and swing, and damage does not break
//! it. The realm also drops any move report that leaves the spot, as it does under a fear.

use benilla_protocol::messages::MoveMode;
use benilla_protocol::SessionEvent;

use super::InWorld;
use crate::clock;

/// Seconds of float drift a root's clock forgives.
const CLOCK_EPSILON: f32 = 1e-3;

/// The root a player is held by.
pub(super) struct Root {
    /// Seconds of rooting left.
    left: f32,
}

impl InWorld {
    /// The player can't move for `duration_ms`, less under a Well Fed `cc_half`; a root already
    /// holding is lengthened, never shortened.
    pub(super) fn start_root(
        &mut self,
        spell_id: u32,
        duration_ms: u32,
        events: &mut Vec<SessionEvent>,
    ) {
        if !self.player.alive() {
            return;
        }
        let duration_ms = self.fed_cc_ms(duration_ms, false);
        let secs = clock::secs(duration_ms);
        self.show_root(spell_id, duration_ms, events);
        if let Some(root) = &mut self.root {
            root.left = root.left.max(secs);
            return;
        }
        self.root = Some(Root { left: secs });
        self.move_modes += 1;
        events.push(SessionEvent::MoveMode {
            guid: self.guid,
            counter: self.move_modes,
            mode: MoveMode::Root,
            apply: true,
        });
    }

    /// Whether the player is held in place.
    pub(super) fn is_rooted(&self) -> bool {
        self.root.is_some()
    }

    /// Advances a root by `dt`: it ends when its time is up or the player dies.
    pub(super) fn root_tick(&mut self, dt: f32, events: &mut Vec<SessionEvent>) {
        let Some(root) = &mut self.root else {
            return;
        };
        root.left -= dt;
        let alive = self.player.alive();
        if root.left > CLOCK_EPSILON && alive {
            return;
        }
        self.root = None;
        if alive {
            self.move_modes += 1;
            events.push(SessionEvent::MoveMode {
                guid: self.guid,
                counter: self.move_modes,
                mode: MoveMode::Root,
                apply: false,
            });
        }
    }
}
