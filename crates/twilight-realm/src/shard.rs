//! One copy of a place, shared by every player in it: an open-world map's mobs, which spawns are
//! in the world and which wait to come back, and the Hub's NPCs; or a Phased Dungeon's Run, its
//! creatures and its clock (ADR 0006). A player's own state (stats, auras, loot, quests) stays in
//! their session; the realm lends a session the shard's mobs while it serves that session, and
//! sends each client what comes into and leaves its view.

use std::collections::{HashMap, HashSet};

use benilla_protocol::SessionEvent;

use crate::fields::values;
use crate::looting::lootable_pair;
use crate::mobs::{self, Mob, State};
use crate::phased::{Meter, Phase, Run};
use crate::population::{Population, VIEW_YD};
use crate::rules::threat;
use crate::space::flat_distance;
use crate::Host;

/// Where a shard is: an open-world map, in the open world or one Phased Dungeon's Phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Place {
    pub(crate) map: u32,
    pub(crate) phase: Phase,
}

/// One place's shared state.
pub(crate) struct Shard {
    place: Place,
    pub(crate) mobs: Vec<Mob>,
    population: Population,
    hub: crate::hub::InView,
    /// A Phased Dungeon's Run: it starts with the shard, when the first player steps in, and
    /// resets with it, once no living player is inside.
    run: Option<Run>,
}

/// A player standing on a shard, as the realm sees them for a tick.
pub(crate) struct OnShard<S> {
    pub(crate) session: S,
    pub(crate) guid: u64,
    pub(crate) position: [f32; 3],
    pub(crate) alive: bool,
    /// The share of their health they have left.
    pub(crate) health: f32,
    /// Their create as other clients are sent it.
    pub(crate) create: SessionEvent,
    /// Their Calling's key, which other clients read in place of the class byte (#207).
    pub(crate) calling: String,
    /// Stealthed: other clients are not shown them.
    pub(crate) hidden: bool,
}

/// Whether `guid` is a player's: `HIGHGUID_PLAYER` is 0.
pub(crate) fn is_player(guid: u64) -> bool {
    guid >> 48 == 0
}

/// Whether another client may be sent a player's descriptor `field`: the object and unit block
/// short of the stats, the player's flags and looks, and what they wear in sight. Never the
/// quest log, skills, inventory, XP or money.
pub(crate) fn public_player_field(field: u16) -> bool {
    use crate::fields::{PLAYER_FLAGS, UNIT_FIELD_STAT0};
    /// `UNIT_FIELD_BYTES_2`: the sheath and the PvP flag.
    const UNIT_FIELD_BYTES_2: u16 = 164;
    /// `PLAYER_QUEST_LOG_1_1`, where the private block after the looks starts.
    const PLAYER_QUEST_LOG_1_1: u16 = 198;
    /// `PLAYER_VISIBLE_ITEM_1_CREATOR` and `PLAYER_FIELD_INV_SLOT_HEAD`: the gear in sight.
    const VISIBLE_ITEMS: std::ops::Range<u16> = 258..486;
    field < UNIT_FIELD_STAT0
        || field == UNIT_FIELD_BYTES_2
        || (PLAYER_FLAGS - 2..PLAYER_QUEST_LOG_1_1).contains(&field)
        || VISIBLE_ITEMS.contains(&field)
}

impl Shard {
    /// A fresh shard: an open world streams in around its players; a Phased Dungeon's Run starts
    /// with all its creatures at full health.
    pub(crate) fn new(place: Place) -> Self {
        let dungeon = place.phase != Phase::OpenWorld;
        let run = match place.phase {
            Phase::Dungeon(id) => Some(Run::new(id)),
            Phase::OpenWorld => None,
        };
        Self {
            place,
            mobs: if dungeon {
                crate::world::creatures(place.phase)
            } else {
                Vec::new()
            },
            population: Population::default(),
            hub: Default::default(),
            run,
        }
    }

    /// The stops whose inner portal is open in the Run; none with no Run.
    pub(crate) fn portals_open(&self) -> Vec<String> {
        self.run
            .as_ref()
            .map_or(Vec::new(), |r| r.portals_open().to_vec())
    }

    /// Opens or closes the inner portal of stop `key` in the Run.
    pub(crate) fn set_portal(&mut self, key: &str, open: bool) {
        if let Some(run) = &mut self.run {
            run.set_portal(key, open);
        }
    }

    /// The stacks on the Run's meter; 0 with none.
    pub(crate) fn noise(&self) -> u32 {
        self.run.as_ref().map_or(0, |r| r.noise)
    }

    /// The Run's meter hears what happened this tick: the creatures first hurt or dead since it
    /// last looked, `gain` more from casts and `drain` from hiding, and the clock.
    pub(crate) fn hear_noise(&mut self, meter: &Meter, gain: u32, drain: u32) {
        let Some(run) = &mut self.run else {
            return;
        };
        let mut gain = gain;
        for m in &mut self.mobs {
            if !m.noised && m.health < m.stats.max_health {
                m.noised = true;
                gain += meter.on_first_damage;
            }
            if !m.death_noised && m.state == State::Dead {
                m.death_noised = true;
                gain += m.template.noise_on_death.unwrap_or(meter.on_death);
            }
        }
        run.hear(meter, gain, drain);
    }

    /// Seconds into its Run, for a Phased Dungeon's.
    pub(crate) fn run_elapsed(&self) -> Option<f32> {
        self.run.as_ref().map(|r| r.elapsed)
    }

    /// Streams spawns and Hub NPCs in around `players` and out once none is near, brings back
    /// the dead whose time is up, and walks the Hub's wanderers. The destroys and moves it makes;
    /// the creates go to each client as its view takes them in.
    pub(crate) fn update(
        &mut self,
        dt: f32,
        players: &[[f32; 3]],
        host: &dyn Host,
    ) -> Vec<SessionEvent> {
        let mut events = Vec::new();
        if let Some(run) = &mut self.run {
            run.elapsed += dt;
            return events;
        }
        let map = self.place.map;
        self.population
            .update(dt, players, map, host, &mut self.mobs, &mut events);
        self.hub.update(dt, players, map, host, &mut events);
        events.retain(|e| !matches!(e, SessionEvent::ObjectCreate { .. }));
        events
    }

    /// Every mob hunting a player who is not on this shard gives up and leaves: its owner went
    /// into a Phase or a Dungeon, or out of the world.
    pub(crate) fn drop_hunts(&mut self, present: &[u64]) {
        self.mobs
            .retain(|m| m.hunts.is_none_or(|owner| present.contains(&owner)));
    }

    /// A mob of `spawn` died: it comes back once every player has been away `wait` seconds.
    pub(crate) fn died(&mut self, spawn: u32, wait: f32) {
        // An Encounter's creatures never come back by themselves.
        if mobs::content()
            .spawn
            .get(spawn as usize)
            .is_some_and(|s| s.encounter.is_some())
        {
            return;
        }
        self.population.died(spawn, wait);
    }

    /// Everything a player at `me` sees: each mob and Hub NPC within view, and its create.
    pub(crate) fn in_view(&self, me: [f32; 3]) -> Vec<(u64, SessionEvent)> {
        // A Run is there all at once.
        let dungeon = self.run.is_some();
        let near = |m: &&Mob| {
            dungeon
                || flat_distance(m.position, me) <= VIEW_YD
                || flat_distance(m.home, me) <= VIEW_YD
        };
        let mut seen: Vec<(u64, SessionEvent)> = self
            .mobs
            .iter()
            .filter(near)
            .map(|m| (m.guid, m.create()))
            .collect();
        seen.extend(self.hub.in_view(me));
        seen
    }

    /// Whether the client that sees `guid` keeps it out of view: a mob in a fight or running home
    /// stays until it is done, as it stays in the world.
    pub(crate) fn keeps(&self, guid: u64) -> bool {
        self.mobs
            .iter()
            .any(|m| m.guid == guid && !matches!(m.state, State::Idle | State::Dead))
    }

    /// Which player runs each mob's fight and AI this tick: the one it fights, while that player
    /// is here; otherwise the nearest.
    pub(crate) fn owners<S: Copy + Eq + std::hash::Hash>(
        &self,
        players: &[OnShard<S>],
    ) -> HashMap<S, HashSet<u64>> {
        let mut owners: HashMap<S, HashSet<u64>> = HashMap::new();
        for m in &self.mobs {
            let fighting = matches!(m.state, State::Engaged | State::Evading);
            let owner = players
                .iter()
                .find(|p| fighting && p.guid == m.target)
                .or_else(|| {
                    players.iter().min_by(|a, b| {
                        flat_distance(a.position, m.position)
                            .total_cmp(&flat_distance(b.position, m.position))
                    })
                });
            if let Some(p) = owner {
                owners.entry(p.session).or_default().insert(m.guid);
            }
        }
        owners
    }
}

/// Adds the threat `events` show `player` making (#147): their damage to each mob 1:1, and
/// their heals at half, split across the living mobs that hate the one healed.
pub(crate) fn note_threat(mobs: &mut [Mob], player: u64, events: &[SessionEvent]) {
    use benilla_protocol::messages::PeriodicTick;
    for e in events {
        let (victim, damage, healed, who) = match e {
            SessionEvent::AttackerState(s) if s.attacker == player => {
                (s.victim, s.damage, 0, player)
            }
            SessionEvent::SpellDamageLog(l) if l.attacker == player => {
                (l.target, l.damage, 0, player)
            }
            SessionEvent::PeriodicAuraLog(l) if l.caster == player => {
                let (mut damage, mut healed) = (0, 0);
                for t in &l.ticks {
                    match t {
                        PeriodicTick::Damage { amount, .. } => damage += amount,
                        PeriodicTick::Heal { amount } => healed += amount,
                        _ => {}
                    }
                }
                (l.target, damage, healed, player)
            }
            // A heal from another player is shown in the healed one's session.
            SessionEvent::SpellHealLog(l) => (l.target, 0, l.amount, l.healer),
            _ => continue,
        };
        if let Some(m) = mobs.iter_mut().find(|m| m.guid == victim) {
            // Even a miss puts the attacker on the table.
            m.threat.add(player, damage as f32);
            continue;
        }
        if healed > 0 {
            let hating = |m: &Mob| m.alive() && m.threat.contains(victim);
            let share = threat::heal_share(healed, mobs.iter().filter(|m| hating(m)).count());
            for m in mobs.iter_mut().filter(|m| hating(m)) {
                m.threat.add(who, share);
            }
        }
    }
}

/// The units an event shows doing or suffering something, for the other clients that see them;
/// empty for what only its own client is told.
pub(crate) fn shown_units(e: &SessionEvent) -> Vec<u64> {
    use SessionEvent as E;
    match e {
        E::ObjectValues { guid, .. }
        | E::MonsterMove { guid, .. }
        | E::UnitMove { guid, .. }
        | E::ObjectMove { guid, .. }
        | E::Emote { guid, .. }
        | E::PlaySpellVisual { unit: guid, .. } => vec![*guid],
        E::AttackStart { attacker, victim } | E::AttackStop { attacker, victim } => {
            vec![*attacker, *victim]
        }
        E::AttackerState(s) => vec![s.attacker, s.victim],
        E::SpellStart { caster, target, .. } => {
            [Some(*caster), *target].into_iter().flatten().collect()
        }
        E::SpellGo { caster, hits, .. } => std::iter::once(*caster)
            .chain(hits.iter().copied())
            .collect(),
        E::SpellDamageLog(l) => vec![l.target, l.attacker],
        E::PeriodicAuraLog(l) => vec![l.target, l.caster],
        E::SpellHealLog(l) => vec![l.target, l.healer],
        E::SpellLogMiss(l) => vec![l.caster],
        _ => Vec::new(),
    }
}

/// `e` as another client sees it: a corpse is lootable only by its killer, and a player shows
/// only their public fields; `None` when nothing of it is for others.
pub(crate) fn as_seen_by_others(e: &SessionEvent) -> Option<SessionEvent> {
    match e {
        SessionEvent::ObjectValues { guid, fields } => {
            let (dynamic, _) = lootable_pair(false);
            let player = is_player(*guid);
            let pairs: Vec<(u16, u32)> = fields
                .raw_fields()
                .filter(|&(field, _)| !player || public_player_field(field))
                .map(|(field, v)| {
                    if field == dynamic && !player {
                        lootable_pair(false)
                    } else {
                        (field, v)
                    }
                })
                .collect();
            (!pairs.is_empty()).then(|| values(*guid, &pairs))
        }
        other => Some(other.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mobs::Template;
    use crate::rules::stats::UnitClass;

    fn meter() -> Meter {
        Meter {
            name: "Noise".into(),
            max: 20,
            client_spell: 1,
            clock_stack_every_s: 30.0,
            on_first_damage: 1,
            on_death: 1,
            hide_drain_every_s: 2.0,
            wake_base_yd: 3.0,
            wake_per_stack_yd: 1.5,
        }
    }

    /// The Abbey's Run, a Phased Dungeon with creatures in it.
    fn run() -> Shard {
        let shard = Shard::new(Place {
            map: 0,
            phase: Phase::Dungeon(1),
        });
        assert!(shard.mobs.len() >= 2, "the Abbey holds creatures");
        shard
    }

    #[test]
    fn damage_to_a_sleeper_adds_one_stack_once() {
        let mut shard = run();
        shard.hear_noise(&meter(), 0, 0);
        assert_eq!(shard.noise(), 0, "an untouched Run is quiet");
        shard.mobs[0].health -= 1;
        shard.hear_noise(&meter(), 0, 0);
        assert_eq!(shard.noise(), 1);
        shard.mobs[0].health -= 1;
        shard.hear_noise(&meter(), 0, 0);
        assert_eq!(shard.noise(), 1, "a second blow to the same creature");
        shard.mobs[1].health -= 1;
        shard.hear_noise(&meter(), 0, 0);
        assert_eq!(shard.noise(), 2, "the next creature's first blow");
    }

    #[test]
    fn a_death_adds_the_meters_stack_or_the_creatures_own() {
        let mut shard = run();
        let mut runt = Template::hunter(
            1,
            "Runt".into(),
            String::new(),
            1,
            1,
            UnitClass::Warrior,
            2000,
            1.0,
            Vec::new(),
            Vec::new(),
        );
        runt.noise_on_death = Some(2);
        shard.mobs[0].template = Box::leak(Box::new(runt));
        for i in [0, 1] {
            shard.mobs[i].noised = true;
            shard.mobs[i].health = 0;
            shard.mobs[i].state = State::Dead;
        }
        shard.hear_noise(&meter(), 0, 0);
        assert_eq!(shard.noise(), 3, "the Runt's 2 and the meter's 1");
        shard.hear_noise(&meter(), 0, 0);
        assert_eq!(shard.noise(), 3, "a death is heard once");
    }

    #[test]
    fn the_meter_clamps_at_its_max_and_drains_to_nothing() {
        let mut shard = run();
        shard.hear_noise(&meter(), 50, 0);
        assert_eq!(shard.noise(), 20);
        shard.hear_noise(&meter(), 0, 3);
        assert_eq!(shard.noise(), 17);
        shard.hear_noise(&meter(), 0, 99);
        assert_eq!(shard.noise(), 0);
    }
}
