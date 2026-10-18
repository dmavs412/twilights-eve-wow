//! Creatures casting their Creature Spells (ADR 0008) in a fight: once a second a fighting mob
//! with a ready spell in range rolls its chance, stands still, and casts. The client sees
//! `SMSG_SPELL_START` with the cast time, then `SMSG_SPELL_GO`, then the aura on its target, as
//! vmangos sends a creature's cast. Nothing here names a spell.

use benilla_protocol::SessionEvent;

use super::{eye, InWorld};
use crate::clock;
use crate::rng::Rng;
use crate::space::distance;
use crate::Host;

/// Seconds between one decision whether to cast and the next.
const THINK_SECS: f32 = 1.0;
/// Seconds of float drift a cast's clock forgives.
const CLOCK_EPSILON: f32 = 1e-3;

impl InWorld {
    /// Mob `i`'s Creature Spells, in a fight with the player at `me`: the cast under way runs on
    /// or breaks, and one with none under way may start one. Whether it is casting, when it
    /// neither swings nor chases.
    pub(super) fn mob_cast(
        &mut self,
        i: usize,
        dt: f32,
        me: [f32; 3],
        host: &dyn Host,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        let spells = self.casters.of(self.mobs[i].template.entry);
        if spells.is_empty() {
            return false;
        }
        let (map, player, player_alive) = (self.map, self.guid, self.player.alive());
        let mob = &mut self.mobs[i];
        for (_, left) in &mut mob.spell_cooldowns {
            *left -= dt;
        }
        mob.spell_cooldowns.retain(|(_, left)| *left > 0.0);

        if let Some((spell, left)) = &mut mob.casting {
            let spell = *spell;
            if !player_alive || distance(mob.position, me) > spell.range_max {
                super::drop_cast(mob, events);
                return false;
            }
            *left -= dt;
            if *left > CLOCK_EPSILON {
                return true;
            }
            mob.casting = None;
            mob.spell_cooldowns
                .push((spell.key.as_str(), clock::secs(spell.cooldown_ms)));
            let (guid, level) = (mob.guid, mob.level());
            events.push(SessionEvent::SpellGo {
                caster: guid,
                spell_id: spell.client_spell_id,
                cast_flags: 0,
                hits: vec![player],
                misses: Vec::new(),
                target: Some(player),
                go_target: None,
                dest: None,
                ammo_display_id: None,
                item_caster: None,
            });
            self.creature_spell_lands(guid, level, spell, events);
            return true;
        }

        mob.think_in -= dt;
        if mob.think_in > 0.0 || !player_alive {
            return false;
        }
        mob.think_in = THINK_SECS;
        let d = distance(mob.position, me);
        let ready = spells
            .into_iter()
            .find(|s| d <= s.range_max && !mob.spell_cooldowns.iter().any(|(k, _)| *k == s.key));
        let Some(spell) = ready else {
            return false;
        };
        if rng.below(100) >= spell.chance || !host.line_of_sight(map, eye(mob.position), eye(me)) {
            return false;
        }
        // It stands where it is for the cast, facing its target.
        if mob.walk.is_some() {
            let stop = mob.run_to(mob.position, me);
            events.push(stop);
            mob.walk = None;
        }
        mob.orientation = (me[1] - mob.position[1])
            .atan2(me[0] - mob.position[0])
            .rem_euclid(std::f32::consts::TAU);
        mob.casting = Some((spell, clock::secs(spell.cast_time_ms)));
        events.push(SessionEvent::SpellStart {
            caster: mob.guid,
            spell_id: spell.client_spell_id,
            cast_flags: 0,
            cast_time_ms: spell.cast_time_ms,
            target: Some(player),
            ammo_display_id: None,
        });
        true
    }
}
