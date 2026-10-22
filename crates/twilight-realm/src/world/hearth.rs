//! The Hearthstone's spell: back to the bind point ([`crate::inn`]). The cast itself is an item
//! cast ([`super::cast`]); this is what it does when it goes off, and the teleport handshake that
//! follows.

use benilla_protocol::SessionEvent;

use super::InWorld;
use crate::characters::Char;

/// `Spell.dbc` 8690, "Hearthstone": the Hearthstone's `use_spell` in `content/given_items.toml`,
/// which `tests/content_spell_ids.rs` checks against the client.
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
            // From another map, a worldport. A stop's pull does not hold a character who hearths.
            self.in_stop = None;
            let home = c.bind.place;
            self.move_to(c, home, events);
        }
    }

    /// A teleport scroll goes off: the carried item of guid `item` is used up and the reader is
    /// teleported to `[x, y, z, facing]`. A stop's pull does not hold a reader.
    pub(super) fn read_scroll(
        &mut self,
        item: u64,
        [x, y, z, facing]: [f32; 4],
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) {
        let slot = c
            .inventory
            .iter()
            .find_map(|(slot, i)| (i.guid == item).then_some(slot));
        if let Some(slot) = slot {
            super::food::use_up_one(slot, c, events);
        }
        self.in_stop = None;
        self.teleport(c, [x, y, z], facing, events);
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
        self.snap_client(position, orientation, events);
    }

    /// Tells the client to snap to `position` and ignore its own reports until it answers.
    pub(super) fn snap_client(
        &mut self,
        position: [f32; 3],
        orientation: f32,
        events: &mut Vec<SessionEvent>,
    ) {
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

    /// Whether a teleport is waiting for the client's answer.
    pub(super) fn teleport_pending(&self) -> bool {
        self.teleports.pending
    }

    /// A dev teleport (`WOW_PROBE_GOTO`): to the spot on this map, on the ground there, or
    /// refused with a notification where the host has no ground.
    pub(crate) fn dev_teleport(
        &mut self,
        to: crate::dev::DevTeleport,
        host: &dyn crate::Host,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) {
        let crate::dev::DevTeleport { x, y, facing } = to;
        match host.ground_height(self.map, x, y, crate::dev::DEV_TELEPORT_Z_HINT) {
            Some(z) => {
                self.in_stop = None;
                let facing = facing.unwrap_or(c.orientation);
                self.teleport(c, [x, y, z], facing, events);
            }
            None => events.push(SessionEvent::Notification {
                text: format!("Dev teleport refused: no ground at ({x}, {y})"),
            }),
        }
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
        // The client has no control while feared; the realm moves the player.
        if self.fear.is_some() {
            return false;
        }
        let stepped = crate::space::distance(from, to) > MOVE_EPSILON;
        // A rooted client sends turns but no steps.
        if stepped && self.is_rooted() {
            return false;
        }
        if stepped {
            self.interrupt_cast(events);
            self.stepped(events);
        }
        true
    }
}

/// How far a report may drift before it counts as a step, in yards.
const MOVE_EPSILON: f32 = 0.05;

#[cfg(test)]
mod tests {
    /// The realm's own copy of the id agrees with the content the spell-id test checks.
    #[test]
    fn the_hearth_spell_is_the_hearthstones_use_spell() {
        let stone = crate::items::template(crate::content::HEARTHSTONE).expect("the Hearthstone");
        assert_eq!(super::HEARTH_SPELL, stone.use_spell);
    }
}
