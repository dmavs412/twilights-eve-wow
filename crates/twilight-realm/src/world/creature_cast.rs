//! Creatures casting their Creature Spells (ADR 0008) in a fight: once a second a fighting mob
//! with a ready spell in range rolls its chance, stands still, and casts. The client sees
//! `SMSG_SPELL_START` with the cast time, then `SMSG_SPELL_GO`, then the aura on its target, as
//! vmangos sends a creature's cast. Nothing here names a spell.

use benilla_protocol::messages::{SpellDamageLog, SpellHealLog};
use benilla_protocol::SessionEvent;

use super::{cast, eye, in_front, melee_reach, Foreign, Gaze, InWorld};
use crate::clock;
use crate::combat;
use crate::creature_spells::{charge_dest, CreatureEffect, SpellTarget};
use crate::fields::{values, UNIT_FIELD_HEALTH};
use crate::mobs::{Stance, State};
use crate::rng::Rng;
use crate::rules::melee::Outcome;
use crate::rules::school::School;
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
        facing: f32,
        host: &dyn Host,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        let mut spells = self.casters.of(self.mobs[i].template.entry);
        spells.extend(self.mobs[i].extra_spells.iter().copied());
        if spells.is_empty() {
            return false;
        }
        let corpses: Vec<[f32; 3]> = self
            .mobs
            .iter()
            .filter(|m| m.state == State::Dead)
            .map(|m| m.position)
            .collect();
        let (map, player, player_alive) = (self.map, self.guid, self.player.alive());
        let mates = self.phase_mates.clone();
        let mob = &mut self.mobs[i];
        for (_, left) in &mut mob.spell_cooldowns {
            *left -= dt;
        }
        mob.spell_cooldowns.retain(|(_, left)| *left > 0.0);
        for (_, left) in &mut mob.lockouts {
            *left -= dt;
        }
        mob.lockouts.retain(|(_, left)| *left > 0.0);

        if let Some((spell, left)) = &mut mob.casting {
            let spell = *spell;
            let self_cast = spell.effect.on_caster();
            let lost = match mob.cast_at {
                // A character of another session: still in the Phase, alive and in range?
                Some(guid) => mates
                    .iter()
                    .find(|(g, _)| *g == guid)
                    .is_none_or(|(_, at)| distance(mob.position, *at) > spell.range_max),
                None => {
                    !player_alive || (!self_cast && distance(mob.position, me) > spell.range_max)
                }
            };
            if lost {
                super::drop_cast(mob, events);
                return false;
            }
            *left -= dt;
            if *left > CLOCK_EPSILON {
                return true;
            }
            mob.casting = None;
            let cooldown = spell.cooldown_ms.rolled(rng);
            mob.spell_cooldowns
                .push((spell.key.as_str(), clock::secs(cooldown)));
            let (guid, level) = (mob.guid, mob.level());
            let victim = mob.cast_at.take();
            let target = if self_cast {
                guid
            } else {
                victim.unwrap_or(player)
            };
            // A melee spell rolls the attack table as it lands, and takes the next swing's place.
            let blow = match spell.effect {
                CreatureEffect::Hit { pct_of_swing } if victim.is_none() => {
                    let stats = cast::buffed(&self.player.stats, &self.skills);
                    let from_front = in_front(facing, me, mob.position);
                    let miss = self.skills.miss_bonus(guid);
                    mob.swing_timer = clock::secs(mob.template.base_attack_time)
                        * mob.attack_time_mul
                        * self.skills.attack_time_mul(guid);
                    Some(combat::mob_strike(
                        mob,
                        pct_of_swing,
                        &stats,
                        from_front,
                        miss,
                        rng,
                    ))
                }
                _ => None,
            };
            let missed = blow.and_then(|b| cast::missed_by(b.outcome));
            events.push(SessionEvent::SpellGo {
                caster: guid,
                spell_id: spell.spell_id,
                cast_flags: 0,
                hits: if missed.is_some() {
                    Vec::new()
                } else {
                    vec![target]
                },
                misses: missed.map(|m| (target, m)).into_iter().collect(),
                target: Some(target),
                go_target: None,
                dest: None,
                ammo_display_id: None,
                item_caster: None,
            });
            self.noise_gain += spell.noise_on_cast;
            if let CreatureEffect::Heal { pct_max_health } = spell.effect {
                let max = mob.stats.max_health;
                let healed = crate::creature_spells::healed(mob.health, max, pct_max_health);
                mob.health += healed;
                events.push(SessionEvent::SpellHealLog(SpellHealLog {
                    target: guid,
                    healer: guid,
                    spell_id: spell.spell_id,
                    amount: healed,
                    critical: false,
                }));
                events.push(values(guid, &[(UNIT_FIELD_HEALTH, mob.health)]));
            } else if let CreatureEffect::Charge {
                speed_yd_s,
                bonus_pct,
            } = spell.effect
            {
                self.creature_charges(i, speed_yd_s, bonus_pct, me, facing, rng, events);
            } else if let Some(blow) = blow {
                if missed.is_none() {
                    events.push(SessionEvent::SpellDamageLog(SpellDamageLog {
                        blocked: blow.blocked,
                        ..cast::damage_log(
                            guid,
                            player,
                            spell.spell_id,
                            blow.damage,
                            spell.school,
                            blow.outcome == Outcome::Crit,
                        )
                    }));
                    self.player_takes(guid, blow.damage, spell.school, events);
                }
            } else if let CreatureEffect::Gaze { damage, radius } = spell.effect {
                self.gazes.push(Gaze {
                    caster: guid,
                    spell_id: spell.spell_id,
                    at: mob.position,
                    radius,
                    damage,
                    school: spell.school,
                    through_cover: false,
                    root_ms: 0,
                    level: Some(mob.level()),
                });
            } else if let CreatureEffect::Nova { damage, radius } = spell.effect {
                self.gazes.push(Gaze {
                    caster: guid,
                    spell_id: spell.spell_id,
                    at: mob.position,
                    radius,
                    damage,
                    school: spell.school,
                    through_cover: true,
                    root_ms: 0,
                    level: Some(mob.level()),
                });
            } else if let CreatureEffect::Root {
                radius,
                duration_ms,
            } = spell.effect
            {
                self.gazes.push(Gaze {
                    caster: guid,
                    spell_id: spell.spell_id,
                    at: mob.position,
                    radius,
                    damage: 0,
                    school: spell.school,
                    through_cover: true,
                    root_ms: duration_ms,
                    level: Some(mob.level()),
                });
            } else if let CreatureEffect::GroundPatch {
                radius,
                duration_ms,
                tick_ms,
                tick,
            } = spell.effect
            {
                // Where the target stands as the cast lands.
                let at = victim
                    .and_then(|v| mates.iter().find(|(g, _)| *g == v))
                    .map_or(me, |(_, at)| *at);
                let damage = tick.at(level, 0);
                self.put_patch(
                    guid,
                    spell.spell_id,
                    at,
                    radius,
                    duration_ms,
                    tick_ms,
                    damage,
                    spell.school,
                );
            } else if let CreatureEffect::Absorb {
                amount,
                duration_ms,
            } = spell.effect
            {
                self.mob_shield(guid, level, spell.spell_id, amount, duration_ms, events);
            } else if let CreatureEffect::Stance {
                damage_dealt_pct,
                damage_taken_pct,
                duration_ms,
            } = spell.effect
            {
                mob.stance = Some(Stance {
                    spell_id: spell.spell_id,
                    damage_dealt_pct,
                    damage_taken_pct,
                    secs_left: (duration_ms > 0).then(|| clock::secs(duration_ms)),
                });
                self.show_stance(guid, level, spell.spell_id, duration_ms, events);
            } else if let Some(target) = victim {
                self.foreign.push(Foreign {
                    target,
                    caster: guid,
                    level,
                    from: mob.position,
                    spell,
                });
            } else {
                let from = mob.position;
                self.creature_spell_lands(guid, level, from, spell, events);
            }
            return true;
        }

        mob.think_in -= dt;
        if mob.think_in > 0.0 || !player_alive {
            return false;
        }
        mob.think_in = THINK_SECS;
        let d = distance(mob.position, me);
        let (health, max_health) = (mob.health, mob.stats.max_health);
        let ready = spells.into_iter().find(|s| {
            let in_reach = s.effect.on_caster()
                || d <= s.range_max
                || (s.target == SpellTarget::Random
                    && mates
                        .iter()
                        .any(|(_, at)| distance(mob.position, *at) <= s.range_max));
            let hurt = s
                .below_health_pct
                .is_none_or(|pct| health * 100 < max_health * pct);
            let fed = s
                .corpse_within
                .is_none_or(|yd| corpse_within(&corpses, mob.position, yd));
            let begun = s
                .first_cast_ms
                .is_none_or(|ms| mob.engaged_secs >= clock::secs(ms) - CLOCK_EPSILON);
            // A stance already up is not taken again.
            let unstanced =
                !matches!(s.effect, CreatureEffect::Stance { .. }) || mob.stance.is_none();
            in_reach
                && begun
                && unstanced
                && d >= s.range_min
                && hurt
                && fed
                && !mob.spell_cooldowns.iter().any(|(k, _)| *k == s.key)
                && !mob.lockouts.iter().any(|(school, _)| *school == s.school)
        });
        let Some(spell) = ready else {
            return false;
        };
        let self_cast = spell.effect.on_caster();
        let random = spell.target == SpellTarget::Random;
        if rng.below(100) >= spell.chance
            || (!self_cast && !random && !host.line_of_sight(map, eye(mob.position), eye(me)))
        {
            return false;
        }
        // A random target: any living character in the Phase within range, the fighter included.
        // The creature keeps fighting the same character.
        let (mut chosen, mut aim) = (player, me);
        mob.cast_at = None;
        if random && !self_cast {
            let mut pool: Vec<(u64, [f32; 3])> = mates
                .iter()
                .filter(|(_, at)| distance(mob.position, *at) <= spell.range_max)
                .copied()
                .collect();
            if distance(mob.position, me) <= spell.range_max {
                pool.push((player, me));
            }
            (chosen, aim) = pool[rng.below(pool.len() as u32) as usize];
            mob.cast_at = (chosen != player).then_some(chosen);
        }
        // It stands where it is for the cast, facing its target.
        if mob.walk.is_some() {
            let stop = mob.run_to(mob.position, me);
            events.push(stop);
            mob.walk = None;
        }
        mob.orientation = (aim[1] - mob.position[1])
            .atan2(aim[0] - mob.position[0])
            .rem_euclid(std::f32::consts::TAU);
        mob.casting = Some((spell, clock::secs(spell.cast_time_ms)));
        if let Some(text) = &spell.warning {
            self.warnings.push(text.clone());
        }
        events.push(SessionEvent::SpellStart {
            caster: mob.guid,
            spell_id: spell.spell_id,
            cast_flags: 0,
            cast_time_ms: spell.cast_time_ms,
            target: Some(if self_cast { mob.guid } else { chosen }),
            ammo_display_id: None,
        });
        true
    }

    /// Mob `i`'s charge goes off: it stands `melee_reach` short of the player at `me`, then lands
    /// one blow with `bonus_pct` percent added to what a swing does.
    #[allow(clippy::too_many_arguments)]
    fn creature_charges(
        &mut self,
        i: usize,
        speed_yd_s: f32,
        bonus_pct: u32,
        me: [f32; 3],
        facing: f32,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) {
        let mob = &mut self.mobs[i];
        let dest = charge_dest(mob.position, me, melee_reach() * 0.9);
        events.push(mob.charge_to(dest, speed_yd_s, self.guid));
        mob.orientation = (me[1] - dest[1])
            .atan2(me[0] - dest[0])
            .rem_euclid(std::f32::consts::TAU);
        let stats = cast::buffed(&self.player.stats, &self.skills);
        let from_front = in_front(facing, me, dest);
        let miss = self.skills.miss_bonus(mob.guid);
        let swing = combat::mob_swing(mob, &stats, from_front, miss, rng);
        mob.swing_timer = clock::secs(mob.template.base_attack_time) * mob.attack_time_mul;
        let attacker = mob.guid;
        events.push(SessionEvent::AttackerState(
            swing.report(attacker, self.guid),
        ));
        if swing.damage > 0 {
            let damage = swing.damage * (100 + bonus_pct) / 100;
            self.player_takes(attacker, damage, School::Physical, events);
        }
    }
}

/// Whether a corpse of `corpses` lies within `yd` yards of `at`.
fn corpse_within(corpses: &[[f32; 3]], at: [f32; 3], yd: f32) -> bool {
    corpses.iter().any(|&c| distance(c, at) <= yd)
}

#[cfg(test)]
mod tests {
    use super::corpse_within;

    #[test]
    fn a_corpse_counts_only_within_range() {
        let corpses = [[20.0, 0.0, 0.0]];
        assert!(!corpse_within(&[], [0.0; 3], 10.0), "no corpse at all");
        assert!(!corpse_within(&corpses, [0.0; 3], 10.0), "20 yd off");
        assert!(corpse_within(&corpses, [12.0, 0.0, 0.0], 10.0), "8 yd off");
    }
}
