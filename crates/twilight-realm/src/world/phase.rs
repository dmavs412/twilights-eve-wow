//! Crossing between Phases: stepping into a Phased Dungeon's footprint swaps the open world's
//! mobs for a fresh set of the dungeon's and starts its Run; stepping out, or dying, ends the
//! Run and swaps them back.

use benilla_protocol::SessionEvent;

use super::InWorld;
use crate::characters::Char;
use crate::mobs::{self, Mob};
use crate::phased::{self, Phase};
use crate::population::VIEW_YD;
use crate::space::flat_distance;

impl InWorld {
    /// Puts the player in the Phase where they stand, unless a worldport is under way.
    pub(crate) fn step_into_phase(&mut self, c: &Char, events: &mut Vec<SessionEvent>) {
        if !self.transfer {
            self.cross_phase(c, events);
        }
    }

    /// Puts the player in the Phase where they stand; the dead are in none of a dungeon's.
    pub(super) fn cross_phase(&mut self, c: &Char, events: &mut Vec<SessionEvent>) {
        let now = if self.player.alive() {
            phased::at(self.map, c.position)
        } else {
            Phase::OpenWorld
        };
        if now == self.phase {
            return;
        }
        // The client is sent only the creatures of its own Phase: the old ones go, with the
        // fight and every aura on them. A Bounty Hunter gives up the chase.
        self.recall_hunter(events);
        self.stop_attack(events);
        let units: Vec<u64> = self.mobs.iter().map(|m| m.guid).collect();
        for &unit in &units {
            self.auras_die_with(unit, events);
        }
        // The new Phase's creatures are its Run's, in its shard: they come with the view.
        self.drop_view(events);
        self.phase = now;
    }

    /// Streams each Phased Dungeon's doorway portal in and out with the view, in every Phase.
    pub(super) fn show_portals(&mut self, me: [f32; 3], events: &mut Vec<SessionEvent>) {
        for d in phased::content() {
            let d_yd = if d.map == self.map {
                flat_distance(d.portal.position, me)
            } else {
                f32::INFINITY
            };
            let shown = self.portals.contains(&d.id);
            if !shown && d_yd <= VIEW_YD {
                self.portals.push(d.id);
                events.push(d.portal_create());
            } else if shown && d_yd > VIEW_YD + 10.0 {
                self.portals.retain(|&id| id != d.id);
                events.push(SessionEvent::ObjectsRemoved(vec![d.portal_guid()]));
            }
        }
    }
}

/// A fresh set of `phase`'s creatures, at full health where their content file puts them.
pub(crate) fn creatures(phase: Phase) -> Vec<Mob> {
    mobs::content()
        .spawn
        .iter()
        .enumerate()
        .filter(|(_, s)| s.phase == phase && s.called_by.is_none())
        .filter_map(|(i, s)| {
            // The authored height: the ground lookup finds the terrain under a building's floor.
            Mob::spawn(i as u32, s.level_min, s.position[2])
        })
        .collect()
}
