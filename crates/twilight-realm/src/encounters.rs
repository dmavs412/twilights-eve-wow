//! Encounters (`content/encounters.toml`): waves of creatures that come at a point in the open
//! world, or in a Phased Dungeon's Phase, when a quest step starts them, and that fail when the
//! characters fighting them leave or die. The creatures are rows in [`crate::mobs`]' spawn list
//! that no Population streams in; this module brings them into the shard's mobs, a wave at a
//! time. Nothing here names a quest.

use std::collections::HashSet;
use std::sync::OnceLock;

use benilla_protocol::messages::{ChatMessage, CHAT_MSG_RAID_WARNING, CHAT_MSG_SYSTEM};
use benilla_protocol::SessionEvent;
use serde::Deserialize;

use crate::mobs::{self, Mob, State};
use crate::phased::Phase;
use crate::shard::Place;
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
    /// The Phased Dungeon whose Phase it runs in; none runs it in the open world.
    #[serde(default)]
    pub(crate) dungeon: Option<u32>,
    /// The Milestone a win reaches, when it is not reached yet.
    #[serde(default)]
    pub(crate) records: Option<String>,
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

impl Encounter {
    /// Where it runs: the open world of Eastern Kingdoms, or its dungeon's Phase on the dungeon's
    /// map.
    pub(crate) fn place(&self) -> Place {
        match self.dungeon.and_then(crate::phased::dungeon) {
            Some(d) => Place {
                map: d.map,
                phase: Phase::Dungeon(d.id),
            },
            None => Place {
                map: crate::content::EASTERN_KINGDOMS,
                phase: Phase::OpenWorld,
            },
        }
    }
}

pub(crate) fn content() -> &'static [Encounter] {
    static ENCOUNTERS: OnceLock<Vec<Encounter>> = OnceLock::new();
    ENCOUNTERS.get_or_init(|| parse(include_str!("../content/encounters.toml")))
}

/// The encounters of `text` (the shape of `content/encounters.toml`). A repeated key, an
/// encounter with no wave, a `dungeon` that is no Phased Dungeon, an anchor outside that
/// dungeon's footprint and a `records` the Milestone catalogue lacks panic.
fn parse(text: &str) -> Vec<Encounter> {
    let file: File = toml::from_str(text).expect("content/encounters.toml parses");
    for (i, e) in file.encounter.iter().enumerate() {
        assert!(
            file.encounter[..i].iter().all(|o| o.key != e.key),
            "encounter {} is declared twice",
            e.key
        );
        assert!(!e.wave.is_empty(), "encounter {} has no wave", e.key);
        if let Some(id) = e.dungeon {
            let d = crate::phased::dungeon(id).unwrap_or_else(|| {
                panic!(
                    "encounter {} runs in dungeon {id}, which is no Phased Dungeon",
                    e.key
                )
            });
            assert!(
                d.holds(d.map, e.anchor),
                "encounter {}'s anchor {:?} is outside {}'s footprint",
                e.key,
                e.anchor,
                d.name
            );
        }
        if let Some(key) = &e.records {
            assert!(
                crate::milestones::listed(key),
                "encounter {} records Milestone {key}, which content/milestones.toml does not list",
                e.key
            );
        }
    }
    file.encounter
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
    /// Advances every encounter that runs at `place` by `dt` seconds among `players` (the
    /// characters there), bringing creatures into `mobs` (that place's shard's) and clearing them.
    /// With no one at the place, a run under way there is dropped. What to tell characters, by
    /// guid, and the encounters won this tick.
    pub(crate) fn tick(
        &mut self,
        dt: f32,
        place: Place,
        players: &[Player],
        mobs: &mut Vec<Mob>,
        host: &dyn Host,
    ) -> (Vec<(u64, SessionEvent)>, Vec<&'static Encounter>) {
        let all = content();
        self.slots.resize_with(all.len(), Slot::default);
        let mut told = Vec::new();
        let mut won = Vec::new();
        for (i, enc) in all.iter().enumerate() {
            if enc.place() != place {
                continue;
            }
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
                won.push(enc);
            }
        }
        (told, won)
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    const DUNGEON: &str = r#"
[[encounter]]
key = "anchor"
quest = 99502
start = "accepted"
dungeon = 8
records = "ilgalar_anchor_broken"
anchor = [-9288.0, -3308.0, 150.09]
start_radius = 10.0
leash = 40.0
first_wave_after_s = 10.0
next_wave_after_s = 45.0

[[encounter.wave]]
creatures = [{ entry = 97511, level = 48, count = 1, position = [-9300.0, -3300.0, 150.09] }]
"#;

    #[test]
    fn a_dungeon_encounter_that_records_a_milestone_loads() {
        let all = parse(DUNGEON);
        assert_eq!(all[0].dungeon, Some(8));
        assert_eq!(all[0].records.as_deref(), Some("ilgalar_anchor_broken"));
    }

    #[test]
    #[should_panic(expected = "which content/milestones.toml does not list")]
    fn records_naming_no_milestone_panics() {
        parse(&DUNGEON.replace("ilgalar_anchor_broken", "no_such_milestone"));
    }

    #[test]
    #[should_panic(expected = "which is no Phased Dungeon")]
    fn a_dungeon_that_is_no_phased_dungeon_panics() {
        parse(&DUNGEON.replace("dungeon = 8", "dungeon = 9999"));
    }

    #[test]
    #[should_panic(expected = "outside Tower of Ilgalar's footprint")]
    fn an_anchor_outside_the_footprint_panics() {
        parse(&DUNGEON.replace("[-9288.0, -3308.0, 150.09]", "[-9150.0, -3420.0, 100.0]"));
    }
}
