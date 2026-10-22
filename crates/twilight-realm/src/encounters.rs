//! Encounters (`content/encounters.toml`): waves of creatures that come at a point in the open
//! world when a quest step starts them, and that fail when the characters fighting them leave or
//! die. The creatures are rows in [`crate::mobs`]' spawn list that no Population streams in; this
//! module brings them into the shard's mobs, a wave at a time. Nothing here names a quest.

use std::collections::HashSet;
use std::sync::OnceLock;

use benilla_protocol::messages::{ChatMessage, CHAT_MSG_RAID_WARNING, CHAT_MSG_SYSTEM};
use benilla_protocol::SessionEvent;
use serde::Deserialize;

use crate::mobs::{self, Mob, State};
use crate::space::flat_distance;
use crate::Host;

/// The first low guid a run gives the creatures it brings in; each gets one of its own, above
/// every spawn row's.
const RUN_GUID_BASE: u32 = 0xC0_0000;
const RUN_GUIDS: u32 = 0x3F_0000;

/// Said to the characters near the anchor when an encounter fails.
const ABANDONED: &str = "You have abandoned the defense.";

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Start {
    Accepted,
    UseDone,
}

#[derive(Deserialize)]
pub(crate) struct Creature {
    pub(crate) entry: u32,
    pub(crate) level: u8,
    pub(crate) count: u32,
    pub(crate) position: [f32; 3],
}

#[derive(Deserialize)]
pub(crate) struct Wave {
    pub(crate) creatures: Vec<Creature>,
}

#[derive(Deserialize)]
pub(crate) struct Encounter {
    pub(crate) key: String,
    pub(crate) quest: u32,
    pub(crate) start: Start,
    pub(crate) anchor: [f32; 3],
    pub(crate) start_radius: f32,
    pub(crate) leash: f32,
    #[serde(default)]
    pub(crate) warning: Option<String>,
    pub(crate) first_wave_after_s: f32,
    pub(crate) next_wave_after_s: f32,
    pub(crate) wave: Vec<Wave>,
}

#[derive(Deserialize)]
struct File {
    encounter: Vec<Encounter>,
}

pub(crate) fn content() -> &'static [Encounter] {
    static ENCOUNTERS: OnceLock<Vec<Encounter>> = OnceLock::new();
    ENCOUNTERS.get_or_init(|| {
        let file: File = toml::from_str(include_str!("../content/encounters.toml"))
            .expect("content/encounters.toml parses");
        for (i, e) in file.encounter.iter().enumerate() {
            assert!(
                file.encounter[..i].iter().all(|o| o.key != e.key),
                "encounter {} is declared twice",
                e.key
            );
            assert!(!e.wave.is_empty(), "encounter {} has no wave", e.key);
        }
        file.encounter
    })
}

/// A character on the encounters' map, as a tick sees them.
pub(crate) struct Player {
    pub(crate) guid: u64,
    pub(crate) position: [f32; 3],
    pub(crate) alive: bool,
    /// The quests in their log: id, and whether every use objective of it is met.
    pub(crate) quests: Vec<(u32, bool)>,
}

impl Player {
    fn holds(&self, quest: u32) -> bool {
        self.quests.iter().any(|&(id, _)| id == quest)
    }

    fn uses_done(&self, quest: u32) -> bool {
        self.quests.iter().any(|&(id, done)| id == quest && done)
    }
}

/// One run of an encounter.
struct Run {
    /// The next wave to come.
    next_wave: usize,
    /// Seconds until it comes, unless the wave before it is dead sooner.
    next_in: f32,
    /// The guids of each wave that has come.
    waves: Vec<Vec<u64>>,
    /// Creatures that came this tick or last, whose fight has yet to be announced to the
    /// character they turn on.
    pending: Vec<u64>,
}

/// One encounter's state.
#[derive(Default)]
struct Slot {
    run: Option<Run>,
    /// Every creature of the last or current run, to clear before the next one.
    brought: Vec<u64>,
    /// The characters holding the quest who stood within the start radius last tick.
    inside: HashSet<u64>,
    /// The characters whose use objectives were all met last tick.
    used: HashSet<u64>,
}

/// Every encounter's state.
#[derive(Default)]
pub(crate) struct Encounters {
    slots: Vec<Slot>,
    /// Creatures brought in so far, for their guids.
    brought: u32,
}

/// The spawn rows of encounter `e`'s wave `w`.
fn rows(e: usize, w: usize) -> impl Iterator<Item = u32> {
    mobs::content()
        .spawn
        .iter()
        .enumerate()
        .filter(move |(_, s)| s.encounter == Some((e, w)))
        .map(|(i, _)| i as u32)
}

fn say(chat_type: u8, text: &str) -> SessionEvent {
    SessionEvent::Chat(ChatMessage {
        chat_type,
        language: 0,
        sender_guid: 0,
        target_guid: 0,
        sender_name: None,
        channel: None,
        text: text.to_string(),
        chat_tag: 0,
    })
}

impl Encounters {
    /// Advances every encounter by `dt` seconds among `players`, bringing creatures into `mobs`
    /// (the open world's) and clearing them. What to tell characters, by guid.
    pub(crate) fn tick(
        &mut self,
        dt: f32,
        players: &[Player],
        mobs: &mut Vec<Mob>,
        host: &dyn Host,
    ) -> Vec<(u64, SessionEvent)> {
        let all = content();
        self.slots.resize_with(all.len(), Slot::default);
        let mut told = Vec::new();
        for (i, enc) in all.iter().enumerate() {
            let questers: Vec<&Player> = players.iter().filter(|p| p.holds(enc.quest)).collect();
            let alive: Vec<&Player> = questers.iter().copied().filter(|p| p.alive).collect();
            let slot = &mut self.slots[i];
            let near = |p: &Player, yards: f32| flat_distance(p.position, enc.anchor) <= yards;

            // Who has come within the start radius, and whose last use objective just completed.
            let inside: HashSet<u64> = alive
                .iter()
                .filter(|p| near(p, enc.start_radius))
                .map(|p| p.guid)
                .collect();
            let came = inside.iter().any(|g| !slot.inside.contains(g));
            slot.inside = inside;
            let used: HashSet<u64> = questers
                .iter()
                .filter(|p| p.uses_done(enc.quest))
                .map(|p| p.guid)
                .collect();
            let finished = used.iter().any(|g| !slot.used.contains(g));
            slot.used = used;

            if slot.run.is_none() {
                let go = match enc.start {
                    Start::Accepted => came,
                    Start::UseDone => finished,
                };
                if go {
                    mobs.retain(|m| !slot.brought.contains(&m.guid));
                    slot.brought.clear();
                    slot.run = Some(Run {
                        next_wave: 0,
                        next_in: enc.first_wave_after_s,
                        waves: Vec::new(),
                        pending: Vec::new(),
                    });
                    if let Some(text) = &enc.warning {
                        for p in players.iter().filter(|p| near(p, enc.leash)) {
                            told.push((p.guid, say(CHAT_MSG_RAID_WARNING, text)));
                        }
                    }
                }
            }
            let Some(run) = &mut slot.run else {
                continue;
            };

            // It fails when no living character holding the quest is within the leash.
            if !alive.iter().any(|p| near(p, enc.leash)) {
                mobs.retain(|m| !slot.brought.contains(&m.guid));
                slot.brought.clear();
                slot.run = None;
                for p in players.iter().filter(|p| near(p, enc.leash)) {
                    told.push((p.guid, say(CHAT_MSG_SYSTEM, ABANDONED)));
                }
                continue;
            }

            // The creatures that came last tick, now in every client's view, turn on the nearest
            // character holding the quest.
            for guid in std::mem::take(&mut run.pending) {
                let Some(mob) = mobs.iter_mut().find(|m| m.guid == guid) else {
                    continue;
                };
                if mob.state != State::Idle {
                    continue;
                }
                let Some(target) = alive.iter().min_by(|a, b| {
                    flat_distance(a.position, mob.position)
                        .total_cmp(&flat_distance(b.position, mob.position))
                }) else {
                    continue;
                };
                let mut events = Vec::new();
                crate::world::engage(mob, target.guid, &mut events);
                told.extend(events.into_iter().map(|e| (target.guid, e)));
            }

            // The next wave comes when the one before it is dead, or its time is up.
            let dead = |guid: &u64| {
                mobs.iter()
                    .find(|m| m.guid == *guid)
                    .is_none_or(|m| m.state == State::Dead)
            };
            if run.next_wave < enc.wave.len() {
                run.next_in -= dt;
                if run.waves.last().is_some_and(|w| w.iter().all(dead)) {
                    run.next_in = 0.0;
                }
                if run.next_in <= 0.0 {
                    let mut came = Vec::new();
                    for row in rows(i, run.next_wave) {
                        let Some(spawn) = mobs::content().spawn.get(row as usize) else {
                            continue;
                        };
                        let z = crate::ground::grounded_z(
                            host,
                            spawn.map,
                            spawn.position,
                            spawn.grounded,
                        );
                        let zone =
                            mobs::zone_factor_at(host, spawn.phase, spawn.map, spawn.position);
                        let Some(mut mob) = Mob::spawn(row, spawn.level_min, z, zone) else {
                            continue;
                        };
                        mob.guid =
                            mobs::unit_guid(spawn.entry, RUN_GUID_BASE + self.brought % RUN_GUIDS);
                        self.brought = self.brought.wrapping_add(1);
                        came.push(mob.guid);
                        mobs.push(mob);
                    }
                    slot.brought.extend(&came);
                    run.pending.extend(&came);
                    run.waves.push(came);
                    run.next_wave += 1;
                    run.next_in = enc.next_wave_after_s;
                }
            } else if run.waves.iter().flatten().all(dead) {
                // The last wave is dead; its corpses stay until the next run clears them.
                slot.run = None;
            }
        }
        told
    }
}
