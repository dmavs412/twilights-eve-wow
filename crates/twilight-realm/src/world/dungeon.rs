//! Going between maps, and the Dungeon's run: its entrance and exit triggers, the worldport
//! handshake that loads the other map, a fresh copy of the instance's mobs each time the player
//! comes in from outside, and its kills kept until they leave.

use benilla_protocol::SessionEvent;

use super::death::Respawn;
use super::InWorld;
use crate::characters::Char;
use crate::content::{EASTERN_KINGDOMS, ELWYNN_FOREST, THE_BARRENS};
use crate::dungeon::{self, Place};
use crate::equipment;
use crate::fields::*;
use crate::instance_map;
use crate::mobs::{self, Mob};
use crate::rules::xp;

impl InWorld {
    /// Whether the player is inside the Dungeon, or on an instance map (ADR 0032).
    pub(crate) fn in_dungeon(&self) -> bool {
        self.map == dungeon::content().map || instance_map::of_map(self.map).is_some()
    }

    /// Whether a worldport is waiting for the client's ack: until then the player is out of the
    /// world, as vmangos keeps them.
    pub(crate) fn transferring(&self) -> bool {
        self.transfer
    }

    /// `CMSG_AREATRIGGER`: the client walked into trigger `id`. The entrance of the Dungeon or of
    /// an instance map takes the player in, its exit out; any other trigger does nothing.
    /// Whether the player was moved.
    pub(crate) fn area_trigger(
        &mut self,
        id: u32,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        let d = dungeon::content();
        let trigger = [&d.entrance, &d.exit]
            .into_iter()
            .find(|t| t.trigger == id)
            .or_else(|| instance_map::with_trigger(id));
        let Some(trigger) = trigger else {
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
        // Going anywhere ends a Falling Sand with no snap; the snap itself comes once it is off.
        self.end_rewind(events);
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
        } else if let Some(m) = instance_map::of_map(to.map) {
            // Dying inside ends the Run, and Release Spirit brings the player out of the door.
            let out = m.exit.to;
            c.zone = m.zone;
            self.respawn = Respawn {
                map: out.map,
                position: out.position,
                orientation: out.orientation,
            };
        } else {
            c.zone = home_zone(to.map);
            self.respawn = Respawn::home(to.map);
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
    /// and inside the Dungeon a fresh copy of its mobs; outside, The Barrens stream back in. On an
    /// instance map the player stands in its open world, whose Hub NPCs stream in with the view;
    /// walking into a dungeon's footprint there steps them into its Phase, as on the continents.
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
        if self.map == dungeon::content().map {
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

/// A fresh copy of `map`'s mobs, each at a level from its range, under the Dungeon's Zone Factor.
fn instance(map: u32) -> Vec<Mob> {
    let zone = mobs::zone_factor(Some(crate::dungeon::content().zone));
    mobs::content()
        .spawn
        .iter()
        .enumerate()
        .filter(|(_, s)| s.map == map)
        .filter_map(|(i, s)| {
            let level = s.level_min + (s.guid % u32::from(s.level_max - s.level_min + 1)) as u8;
            Mob::spawn(i as u32, level, s.position[2], zone)
        })
        .collect()
}

/// Where a character saved inside the Dungeon, or on an instance map, logs back in: at its exit's
/// destination outside, since a run does not outlast the session.
pub(crate) fn come_out(c: &mut Char) {
    let d = dungeon::content();
    let out = if c.map == d.map {
        Some(d.exit.to)
    } else {
        instance_map::of_map(c.map).map(|m| m.exit.to)
    };
    if let Some(to) = out {
        c.map = to.map;
        c.zone = home_zone(to.map);
        c.position = to.position;
        c.orientation = to.orientation;
    }
}

/// Puts `c` at their bind point, listed in its map's home zone.
pub(crate) fn go_home(c: &mut Char) {
    let to = c.bind.place;
    c.map = to.map;
    c.zone = home_zone(to.map);
    c.position = to.position;
    c.orientation = to.orientation;
}

/// The zone a character on `map`'s open world is listed in: Elwynn Forest on Eastern Kingdoms,
/// The Barrens elsewhere, as a new character's.
fn home_zone(map: u32) -> u32 {
    if map == EASTERN_KINGDOMS {
        ELWYNN_FOREST
    } else {
        THE_BARRENS
    }
}
