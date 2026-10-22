//! Creatures calling for help: the first time a fighting creature's health falls to a call's
//! threshold, its adds appear at the call's point and fall on the player; and a creature with
//! timed calls brings its adds on a clock while it fights. Nothing here names a creature.

use std::sync::atomic::{AtomicU32, Ordering};

use benilla_protocol::SessionEvent;

use super::{engage, InWorld};
use crate::ground::grounded_z;
use crate::mobs::{self, Mob, State};
use crate::Host;

/// Counts the creatures timed calls have brought in, for a guid of each.
static TIMED_SEQ: AtomicU32 = AtomicU32::new(0);

impl InWorld {
    /// Makes each timed call that a fighting creature's clock has come to, and takes away the
    /// creatures of the calls of one that died, left combat, or is gone. The clock is the time
    /// since the pull, so a creature that fights again starts it over.
    pub(super) fn timed_calls(&mut self, host: &dyn Host, events: &mut Vec<SessionEvent>) {
        let mut made = Vec::new();
        let mut ended = Vec::new();
        for mob in &mut self.mobs {
            let Some(spawn) = mob.spawn_row().filter(|s| !s.every.is_empty()) else {
                continue;
            };
            if mob.state != State::Engaged {
                if mob.timed_made.iter().any(|&n| n > 0) {
                    mob.timed_made.fill(0);
                    ended.push(mob.guid);
                }
                continue;
            }
            for (k, call) in spawn.every.iter().enumerate() {
                let due = (mob.engaged_secs / call.every_s) as u32;
                while mob.timed_made[k] < due {
                    mob.timed_made[k] += 1;
                    made.push((mob.guid, mob.target, spawn, k));
                }
            }
        }
        // Their callers may have left the world altogether.
        let callers: Vec<u64> = self.mobs.iter().map(|m| m.guid).collect();
        let gone: Vec<u64> = self
            .mobs
            .iter()
            .filter(|m| {
                m.brought_by
                    .is_some_and(|b| ended.contains(&b) || !callers.contains(&b))
            })
            .map(|m| m.guid)
            .collect();
        self.remove_units(&gone, events);
        let content = mobs::content();
        for (caller, target, caller_spawn, k) in made {
            let call = &caller_spawn.every[k];
            if let Some(spell_id) = call.client_spell_id {
                events.push(SessionEvent::SpellGo {
                    caster: caller,
                    spell_id,
                    cast_flags: 0,
                    hits: Vec::new(),
                    misses: Vec::new(),
                    target: None,
                    go_target: None,
                    dest: None,
                    ammo_display_id: None,
                    item_caster: None,
                });
            }
            let adds = content
                .spawn
                .iter()
                .enumerate()
                .filter(|(_, s)| s.timed_by == Some((caller_spawn.guid, k)));
            for (i, s) in adds {
                let wave = TIMED_SEQ.fetch_add(1, Ordering::Relaxed);
                let z = grounded_z(host, s.map, s.position, s.grounded);
                let zone = crate::mobs::zone_factor_at(host, s.phase, s.map, s.position);
                let Some(mut add) = Mob::timed(i as u32, call.level, z, zone, wave, caller) else {
                    continue;
                };
                add.audience = self.audience_of(caller);
                events.push(add.create());
                engage(&mut add, target, events);
                self.mobs.push(add);
            }
        }
    }

    /// The Audience of the creature `guid`, which a creature it calls in inherits.
    fn audience_of(&self, guid: u64) -> Option<crate::audience::Audience> {
        self.mobs
            .iter()
            .find(|m| m.guid == guid)
            .and_then(|m| m.audience.clone())
    }

    /// Makes each call for help that a creature's health has just reached, once per life: it
    /// lives on the [`Mob`], which a Run's reset replaces.
    pub(super) fn call_for_help(&mut self, host: &dyn Host, events: &mut Vec<SessionEvent>) {
        let content = mobs::content();
        let mut made = Vec::new();
        for mob in &mut self.mobs {
            if mob.state != State::Engaged {
                continue;
            }
            let Some(spawn) = mob.spawn_row() else {
                continue;
            };
            for (k, call) in spawn.on_health.iter().enumerate() {
                if !mob.called[k] && call.reached(mob.health, mob.stats.max_health) {
                    mob.called[k] = true;
                    made.push((mob.guid, spawn, k));
                }
            }
        }
        for (caller, caller_spawn, k) in made {
            let call = &caller_spawn.on_health[k];
            if let Some(spell_id) = call.client_spell_id {
                events.push(SessionEvent::SpellGo {
                    caster: caller,
                    spell_id,
                    cast_flags: 0,
                    hits: Vec::new(),
                    misses: Vec::new(),
                    target: None,
                    go_target: None,
                    dest: None,
                    ammo_display_id: None,
                    item_caster: None,
                });
            }
            let adds = content
                .spawn
                .iter()
                .enumerate()
                .filter(|(_, s)| s.called_by == Some((caller_spawn.guid, k)));
            for (i, s) in adds {
                let level = crate::mobs::banded(call.entry, caller_spawn.map, call.position)
                    .map_or(call.level, |range| crate::bands::pick(i as u32, range));
                let z = grounded_z(host, s.map, s.position, s.grounded);
                let zone = crate::mobs::zone_factor_at(host, s.phase, s.map, s.position);
                let Some(mut add) = Mob::spawn(i as u32, level, z, zone) else {
                    continue;
                };
                add.audience = self.audience_of(caller);
                events.push(add.create());
                engage(&mut add, self.guid, events);
                self.mobs.push(add);
            }
        }
    }
}
