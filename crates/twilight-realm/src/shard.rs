//! One copy of an open-world map, shared by every player on it: its mobs, which spawns are in the
//! world and which wait to come back, and the Hub's NPCs. A player's own state (stats, auras,
//! loot, quests) stays in their session; the realm lends a session the shard's mobs while it
//! serves that session, and sends each client what comes into and leaves its view.

use std::collections::{HashMap, HashSet};

use benilla_protocol::SessionEvent;

use crate::fields::values;
use crate::looting::lootable_pair;
use crate::mobs::{Mob, State};
use crate::population::{Population, VIEW_YD};
use crate::space::flat_distance;
use crate::Host;

/// One open-world map's shared state.
#[derive(Default)]
pub(crate) struct Shard {
    pub(crate) mobs: Vec<Mob>,
    population: Population,
    hub: crate::hub::InView,
}

/// A player standing on a shard, as the realm sees them for a tick.
pub(crate) struct OnShard<S> {
    pub(crate) session: S,
    pub(crate) guid: u64,
    pub(crate) position: [f32; 3],
}

impl Shard {
    /// Streams spawns and Hub NPCs in around `players` and out once none is near, brings back
    /// the dead whose time is up, and walks the Hub's wanderers. The destroys and moves it makes;
    /// the creates go to each client as its view takes them in.
    pub(crate) fn update(
        &mut self,
        dt: f32,
        players: &[[f32; 3]],
        map: u32,
        host: &dyn Host,
    ) -> Vec<SessionEvent> {
        let mut events = Vec::new();
        self.population
            .update(dt, players, map, host, &mut self.mobs, &mut events);
        self.hub.update(dt, players, map, host, &mut events);
        events.retain(|e| !matches!(e, SessionEvent::ObjectCreate { .. }));
        events
    }

    /// A mob of `spawn` died: it comes back once every player has been away `wait` seconds.
    pub(crate) fn died(&mut self, spawn: u32, wait: f32) {
        self.population.died(spawn, wait);
    }

    /// Everything a player at `me` sees: each mob and Hub NPC within view, and its create.
    pub(crate) fn in_view(&self, me: [f32; 3]) -> Vec<(u64, SessionEvent)> {
        let near = |m: &&Mob| {
            flat_distance(m.position, me) <= VIEW_YD || flat_distance(m.home, me) <= VIEW_YD
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
            let fighting = matches!(m.state, State::Engaged | State::Dazed | State::Evading);
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

/// The units an event shows doing or suffering something, for the other clients that see them;
/// empty for what only its own client is told.
pub(crate) fn shown_units(e: &SessionEvent) -> Vec<u64> {
    use SessionEvent as E;
    match e {
        E::ObjectValues { guid, .. }
        | E::MonsterMove { guid, .. }
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

/// `e` as another client sees it: a corpse is lootable only by its killer.
pub(crate) fn as_seen_by_others(e: &SessionEvent) -> SessionEvent {
    match e {
        SessionEvent::ObjectValues { guid, fields } => {
            let (dynamic, _) = lootable_pair(false);
            let pairs: Vec<(u16, u32)> = fields
                .raw_fields()
                .map(|(field, v)| {
                    if field == dynamic {
                        lootable_pair(false)
                    } else {
                        (field, v)
                    }
                })
                .collect();
            values(*guid, &pairs)
        }
        other => other.clone(),
    }
}
