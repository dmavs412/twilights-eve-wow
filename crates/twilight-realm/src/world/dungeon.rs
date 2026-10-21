//! Going between maps, and the Dungeon's run: its entrance and exit triggers, the worldport
//! handshake that loads the other map, a fresh copy of the instance's mobs each time the player
//! comes in from outside, and its kills kept until they leave.

use benilla_protocol::SessionEvent;

use super::death::Respawn;
use super::InWorld;
use crate::characters::Char;
use crate::content::THE_BARRENS;
use crate::dungeon::{self, Place};
use crate::equipment;
use crate::fields::*;
use crate::mobs::{self, Mob};
use crate::rules::xp;

impl InWorld {
    /// Whether the player is inside the Dungeon.
    pub(crate) fn in_dungeon(&self) -> bool {
        self.map == dungeon::content().map
    }

    /// Whether a worldport is waiting for the client's ack: until then the player is out of the
    /// world, as vmangos keeps them.
    pub(crate) fn transferring(&self) -> bool {
        self.transfer
    }

    /// `CMSG_AREATRIGGER`: the client walked into trigger `id`. The Dungeon's entrance takes the
    /// player in, its exit out; any other trigger does nothing. Whether the player was moved.
    pub(crate) fn area_trigger(
        &mut self,
        id: u32,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        let d = dungeon::content();
        let Some(trigger) = [&d.entrance, &d.exit].into_iter().find(|t| t.trigger == id) else {
            return false;
        };
        if self.transfer || self.dead() || !trigger.holds(self.map, c.position) {
            return false;
        }
        self.move_to(c, trigger.to, events);
        true
    }

    /// Sends the player to `to`: a teleport on this map, a worldport to another.
    pub(super) fn move_to(&mut self, c: &mut Char, to: Place, events: &mut Vec<SessionEvent>) {
        if to.map == self.map {
            self.teleport(c, to.position, to.orientation, events);
            return;
        }
        // The client drops every object at the worldport: the fight, the mobs and their corpses,
        // and every aura with them. Leaving the Dungeon ends its run.
        self.stop_attack(events);
        self.interrupt_cast(events);
        let units: Vec<u64> = self.mobs.iter().map(|m| m.guid).collect();
        for unit in units.into_iter().chain([self.guid]) {
            self.auras_die_with(unit, events);
        }
        self.drop_view(&mut Vec::new());
        self.in_stop = None;
        self.map = to.map;
        c.map = to.map;
        c.position = to.position;
        c.orientation = to.orientation;
        let d = dungeon::content();
        if to.map == d.map {
            c.zone = d.zone;
            self.respawn = Respawn {
                map: d.map,
                position: d.entrance.to.position,
                orientation: d.entrance.to.orientation,
            };
        } else {
            c.zone = THE_BARRENS;
            self.respawn = Respawn::HUB;
        }
        self.transfer = true;
        events.push(SessionEvent::TransferPending {
            map_id: to.map,
            transport_entry: None,
        });
        events.push(SessionEvent::Worldport {
            map_id: to.map,
            position: to.position,
            orientation: to.orientation,
            needs_ack: true,
        });
    }

    /// `MSG_MOVE_WORLDPORT_ACK`: the client has loaded the new map. The player is created again,
    /// and inside the Dungeon a fresh copy of its mobs; outside, The Barrens stream back in.
    pub(crate) fn worldport_ack(&mut self, c: &Char, events: &mut Vec<SessionEvent>) {
        if !self.transfer {
            return;
        }
        self.transfer = false;
        equipment::login(c, events);
        events.push(self.player_create(c));
        let p = &self.player;
        events.push(values(
            self.guid,
            &[(UNIT_FIELD_HEALTH, p.health), p.power.field()],
        ));
        if self.in_dungeon() {
            self.mobs = instance(self.map);
            events.extend(self.mobs.iter().map(Mob::create));
        }
    }

    /// Whether `mob` is the Dungeon's boss.
    pub(super) fn is_boss(mob: &Mob) -> bool {
        mob.spawn_row().is_some_and(|spawn| {
            spawn.map == dungeon::content().map && spawn.guid == dungeon::content().boss
        })
    }

    /// What an elite's kill XP counts as here.
    pub(super) fn elite_kind(&self) -> xp::Elite {
        if self.in_dungeon() {
            xp::Elite::Dungeon
        } else {
            xp::Elite::OpenWorld
        }
    }
}

/// A fresh copy of `map`'s mobs, each at a level from its range.
fn instance(map: u32) -> Vec<Mob> {
    mobs::content()
        .spawn
        .iter()
        .enumerate()
        .filter(|(_, s)| s.map == map)
        .filter_map(|(i, s)| {
            let level = s.level_min + (s.guid % u32::from(s.level_max - s.level_min + 1)) as u8;
            Mob::spawn(i as u32, level, s.position[2])
        })
        .collect()
}

/// Where a character saved inside the Dungeon logs back in: outside its entrance, since a run
/// does not outlast the session.
pub(crate) fn come_out(c: &mut Char) {
    let d = dungeon::content();
    if c.map == d.map {
        let to = d.exit.to;
        c.map = to.map;
        c.zone = THE_BARRENS;
        c.position = to.position;
        c.orientation = to.orientation;
    }
}
