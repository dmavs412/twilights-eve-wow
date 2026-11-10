//! Defenders (#700): a friendly creature that hostile creatures fight, that fights back and that
//! players heal. A hostile creature notices a defender as it notices a player; a blow between two
//! creatures is rolled and landed here; and an ally skill's heals land on a defender the caster
//! sees. Nothing here names a creature.

use benilla_protocol::messages::{PeriodicAuraLog, PeriodicTick, SpellHealLog};
use benilla_protocol::SessionEvent;

use super::{drop_cast, engage, eye, in_front, wound, InWorld, Ticked};
use crate::characters::Char;
use crate::combat;
use crate::fields::*;
use crate::mobs::State;
use crate::rng::Rng;
use crate::rules::aggro;
use crate::rules::school::School;
use crate::space::distance;
use crate::Host;

impl InWorld {
    /// Each hostile idle creature this session runs notices the living defenders it may fight
    /// within its aggro radius for the defender's level, in its line of sight, and is pulled onto
    /// the nearest: it engages it and brings its camp, as a player's pull does. Cover, stealth and
    /// a cart seat hide only players.
    pub(super) fn notice_defenders(&mut self, host: &dyn Host, events: &mut Vec<SessionEvent>) {
        if !self.mobs.iter().any(|m| m.defender && m.alive()) {
            return;
        }
        for i in 0..self.mobs.len() {
            let mob = &self.mobs[i];
            if mob.state != State::Idle
                || mob.defender
                || mob.asleep
                || mob.frozen
                || !mob.template.hostile
                || mob.hunts.is_some()
                || !self.runs(mob.guid)
                || self.skills.incapacitated(mob.guid)
            {
                continue;
            }
            let near = |d: &&crate::mobs::Mob| {
                let r = aggro::radius(mob.template.detection_range, d.level(), mob.level());
                distance(mob.position, d.position) <= r
            };
            let seen = self
                .mobs
                .iter()
                .filter(|d| d.defender && mob.may_fight(d))
                .filter(near)
                .filter(|d| host.line_of_sight(self.map, eye(mob.position), eye(d.position)))
                .min_by(|a, b| {
                    distance(mob.position, a.position)
                        .total_cmp(&distance(mob.position, b.position))
                })
                .map(|d| d.guid);
            if let Some(defender) = seen {
                self.pull_onto(i, defender, Some(host), events);
            }
        }
    }

    /// The living creatures mob `i` fights or hates and may fight: each guid and where it stands.
    /// None while it is out of a fight.
    pub(super) fn creature_foes(&self, i: usize) -> Vec<(u64, [f32; 3])> {
        let mob = &self.mobs[i];
        if mob.state != State::Engaged {
            return Vec::new();
        }
        self.mobs
            .iter()
            .filter(|m| m.guid == mob.target || mob.threat.contains(m.guid))
            .filter(|m| mob.may_fight(m))
            .map(|m| (m.guid, m.position))
            .collect()
    }

    /// Mob `i`'s white swing at the creature `victim`: rolled on the creature-on-creature table,
    /// reported as an `AttackerState` as a swing at a player is, and landed.
    pub(super) fn swing_at_creature(
        &mut self,
        i: usize,
        victim: u64,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let Some(v) = self.mobs.iter().position(|m| m.guid == victim) else {
            return;
        };
        let (attacker, target) = (&self.mobs[i], &self.mobs[v]);
        let from_front = in_front(target.orientation, target.position, attacker.position);
        let swing = combat::mob_swing_at_mob(attacker, target, from_front, rng);
        events.push(SessionEvent::AttackerState(
            swing.report(attacker.guid, victim),
        ));
        self.creature_struck(i, v, swing.damage, c, rng, events, ticked);
    }

    /// Creature `v` loses `damage` to creature `i`'s blow. It dies of one that leaves it no
    /// health; else the blow is its threat on `v`, 1:1, and an idle `v` turns on `i`.
    #[allow(clippy::too_many_arguments)]
    fn creature_struck(
        &mut self,
        i: usize,
        v: usize,
        damage: u32,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let attacker = self.mobs[i].guid;
        let victim = &mut self.mobs[v];
        wound(
            victim,
            damage,
            attacker,
            School::Physical,
            &mut self.harms,
            events,
        );
        if victim.health == 0 && victim.alive() {
            if victim.defender {
                self.defender_died(v, events);
            } else {
                // Credited as a player's kill, to everyone in range of the corpse.
                let (guid, _) = self.fall(v, c, rng, events, ticked);
                self.wearer_died(guid, c, rng, events, ticked);
                self.auras_die_with(guid, events);
            }
            return;
        }
        if victim.state == State::Idle {
            engage(victim, attacker, events);
        }
        victim.threat.add(attacker, damage as f32);
    }

    /// Defender `v` dies: no kill, so no experience, loot, money or respawn. Its fight ends, and
    /// those that fought it drop it from their tables. Its corpse stays until whatever made it
    /// takes it away.
    fn defender_died(&mut self, v: usize, events: &mut Vec<SessionEvent>) {
        let mob = &mut self.mobs[v];
        let engaged = mob.state == State::Engaged;
        mob.state = State::Dead;
        mob.walk = None;
        mob.threat.clear();
        drop_cast(mob, events);
        if engaged {
            events.push(SessionEvent::AttackStop {
                attacker: mob.guid,
                victim: mob.target,
            });
        }
        let mut pairs = vec![(UNIT_FIELD_HEALTH, 0), (UNIT_FIELD_FLAGS, 0)];
        pairs.extend(guid_pairs(UNIT_FIELD_TARGET, 0));
        events.push(values(mob.guid, &pairs));
        let guid = mob.guid;
        self.auras_die_with(guid, events);
    }

    /// The living defender `guid` the player sees, by its index; `None` for any other unit.
    pub(super) fn defender(&self, guid: u64) -> Option<usize> {
        self.mobs
            .iter()
            .position(|m| m.guid == guid && m.defender && m.alive() && m.admits(self.guid))
    }

    /// The living defenders the player sees, each with where it stands and the share of its
    /// health it has left, as [`Self::friendly_players`] lists players.
    pub(super) fn friendly_defenders(&self) -> Vec<(u64, [f32; 3], f32)> {
        self.mobs
            .iter()
            .filter(|m| m.defender && m.alive() && m.admits(self.guid))
            .map(|m| {
                let share = m.health as f32 / m.stats.max_health.max(1) as f32;
                (m.guid, m.position, share)
            })
            .collect()
    }

    /// Heals the defender `guid` `amount` of the player's `spell_id`, up to its maximum health:
    /// its new health and a `SpellHealLog` naming it. What was healed, and what would have been
    /// with no maximum to stop it; `None` when `guid` is no living defender the player sees.
    pub(super) fn heal_defender(
        &mut self,
        guid: u64,
        spell_id: u32,
        amount: u32,
        events: &mut Vec<SessionEvent>,
    ) -> Option<(u32, u32)> {
        let i = self.defender(guid)?;
        let mob = &mut self.mobs[i];
        let healed = amount.min(mob.stats.max_health.saturating_sub(mob.health));
        mob.health += healed;
        events.push(SessionEvent::SpellHealLog(SpellHealLog {
            target: guid,
            healer: self.guid,
            spell_id,
            amount: healed,
            critical: false,
        }));
        events.push(values(guid, &[(UNIT_FIELD_HEALTH, mob.health)]));
        Some((healed, amount))
    }

    /// One tick of the player's heal over time `spell_id` on the defender `guid`, up to its
    /// maximum health. Nothing once it is dead or gone.
    pub(super) fn heal_tick_defender(
        &mut self,
        guid: u64,
        spell_id: u32,
        per_tick: u32,
        events: &mut Vec<SessionEvent>,
    ) {
        let Some(i) = self.defender(guid) else {
            return;
        };
        let mob = &mut self.mobs[i];
        mob.health = (mob.health + per_tick).min(mob.stats.max_health);
        events.push(SessionEvent::PeriodicAuraLog(PeriodicAuraLog {
            target: guid,
            caster: self.guid,
            spell_id,
            ticks: vec![PeriodicTick::Heal { amount: per_tick }],
        }));
        events.push(values(guid, &[(UNIT_FIELD_HEALTH, mob.health)]));
    }
}
