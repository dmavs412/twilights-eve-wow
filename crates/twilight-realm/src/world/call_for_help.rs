//! Creatures calling for help: the first time a fighting creature's health falls to a call's
//! threshold, its adds appear at the call's point and fall on the player. Nothing here names a
//! creature.

use benilla_protocol::SessionEvent;

use super::{engage, InWorld};
use crate::mobs::{self, Mob, State};

impl InWorld {
    /// Makes each call for help that a creature's health has just reached, once per life: it
    /// lives on the [`Mob`], which a Run's reset replaces.
    pub(super) fn call_for_help(&mut self, events: &mut Vec<SessionEvent>) {
        let content = mobs::content();
        let mut made = Vec::new();
        for mob in &mut self.mobs {
            if !matches!(mob.state, State::Engaged | State::Dazed) {
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
            for (i, _) in adds {
                let Some(mut add) = Mob::spawn(i as u32, call.level, call.position[2]) else {
                    continue;
                };
                events.push(add.create());
                engage(&mut add, self.guid, events);
                self.mobs.push(add);
            }
        }
    }
}
