//! Crossing between Phases: stepping into a Phased Dungeon's footprint swaps the open world's
//! mobs for a fresh set of the dungeon's and starts its Run; stepping out, or dying, ends the
//! Run and swaps them back.

use benilla_protocol::SessionEvent;

use super::InWorld;
use crate::characters::Char;
use crate::mobs::{self, Mob};
use crate::phased::{self, Phase, Run};

impl InWorld {
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
        // fight and every aura on them.
        self.stop_attack(events);
        let units: Vec<u64> = self.mobs.iter().map(|m| m.guid).collect();
        for &unit in &units {
            self.auras_die_with(unit, events);
        }
        if !units.is_empty() {
            events.push(SessionEvent::ObjectsRemoved(units));
        }
        self.mobs.clear();
        self.population.forget_view();
        // One character to a realm: whoever leaves is the last one out, and the Run ends.
        self.run = None;
        self.phase = now;
        if let Phase::Dungeon(_) = now {
            self.run = Some(Run { elapsed: 0.0 });
            self.mobs = creatures(now);
            events.extend(self.mobs.iter().map(Mob::create));
        }
    }

    /// Seconds into the Run of the dungeon the player is in.
    pub(crate) fn run_elapsed(&self) -> Option<f32> {
        self.run.as_ref().map(|r| r.elapsed)
    }
}

/// A fresh set of `phase`'s creatures, at full health where their content file puts them.
fn creatures(phase: Phase) -> Vec<Mob> {
    mobs::content()
        .spawn
        .iter()
        .enumerate()
        .filter(|(_, s)| s.phase == phase)
        .filter_map(|(i, s)| {
            // The authored height: the ground lookup finds the terrain under a building's floor.
            Mob::spawn(i as u32, s.level_min, s.position[2])
        })
        .collect()
}
