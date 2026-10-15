//! The Hearthstone's spell: back to the Hub. The cast itself is an item cast ([`super::cast`]);
//! this is what it does when it goes off, and the teleport handshake that follows.

use benilla_protocol::SessionEvent;

use super::InWorld;
use crate::characters::Char;
use crate::content::CROSSROADS;

/// `Spell.dbc` 8690, "Hearthstone".
const HEARTH_SPELL: u32 = 8690;
/// Its cast time (`CastingTimeIndex` 16).
const HEARTH_CAST_MS: u32 = 10_000;

/// The cast time of an item spell the realm runs; `None` for one it does not.
pub(super) fn cast_time_ms(spell_id: u32) -> Option<u32> {
    (spell_id == HEARTH_SPELL).then_some(HEARTH_CAST_MS)
}

/// The same-map teleports sent and not yet acknowledged (`MSG_MOVE_TELEPORT_ACK`). Until the
/// client acks, its movement reports were sent from where it stood before, and are ignored, as
/// vmangos ignores them.
#[derive(Default)]
pub(super) struct Teleports {
    /// The last counter sent.
    counter: u32,
    /// Waiting for the ack of `counter`.
    pending: bool,
}

impl InWorld {
    /// What an item spell does when it goes off.
    pub(super) fn item_spell_effect(
        &mut self,
        spell_id: u32,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) {
        if spell_id == HEARTH_SPELL {
            // From inside a Dungeon, a worldport.
            let [x, y, z, facing] = CROSSROADS;
            let hub = crate::dungeon::Place {
                map: crate::content::KALIMDOR,
                position: [x, y, z],
                orientation: facing,
            };
            self.move_to(c, hub, events);
        }
    }

    /// Moves the player on its own map: the fight ends, and the client snaps there.
    pub(super) fn teleport(
        &mut self,
        c: &mut Char,
        position: [f32; 3],
        orientation: f32,
        events: &mut Vec<SessionEvent>,
    ) {
        self.stop_attack(events);
        c.position = position;
        c.orientation = orientation;
        let t = &mut self.teleports;
        t.counter += 1;
        t.pending = true;
        events.push(SessionEvent::Teleport {
            guid: self.guid,
            counter: t.counter,
            position,
            orientation,
        });
    }

    /// `MSG_MOVE_TELEPORT_ACK`: the client stands where it was sent.
    pub(crate) fn teleport_ack(&mut self, counter: u32) {
        if counter == self.teleports.counter {
            self.teleports.pending = false;
        }
    }

    /// The player's own movement report: ignored while a teleport is unacknowledged, and any
    /// step off the spot interrupts a timed cast. Whether it stands.
    pub(crate) fn moved(
        &mut self,
        from: [f32; 3],
        to: [f32; 3],
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        // Nor while a worldport is unacknowledged: the report is from the map left behind.
        if self.teleports.pending || self.transferring() {
            return false;
        }
        if crate::space::distance(from, to) > MOVE_EPSILON {
            self.interrupt_cast(events);
        }
        true
    }
}

/// How far a report may drift before it counts as a step, in yards.
const MOVE_EPSILON: f32 = 0.05;
