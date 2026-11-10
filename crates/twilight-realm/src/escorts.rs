//! Escorts (`content/escorts.toml`, #701, ADR 0035): a character holding a quest asks a friendly
//! creature to set off, and a private copy of it, seen by their group alone, walks a fixed path.
//! It waits for the group, fights what attacks it (a defender, #700), and meets the quest for
//! every group member holding it when it arrives. It fails when it dies or the group leaves it;
//! the quest stays in the log for another try. Nothing here names a quest.

use std::sync::OnceLock;

use benilla_protocol::messages::GossipOption;
use benilla_protocol::SessionEvent;
use serde::Deserialize;

use crate::audience::Audience;
use crate::characters::Char;
use crate::mobs::{self, Mob, State};
use crate::phased::Phase;
use crate::shard::{Place, Shard};
use crate::space::flat_distance;
use crate::Host;

/// The escort's start line in the start creature's gossip menu: clear of the hint lines (80 to
/// 89, `world::fishing`) and the talk lines ([`crate::quests::TALK_OPTION`]).
pub(crate) const ESCORT_OPTION: u32 = 90;

/// The escortee's walk, at his own walk speed.
const FULL_SPEED_PCT: u32 = 100;

fn one() -> f32 {
    1.0
}

/// One point of an escort's path.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Point {
    pub(crate) at: [f32; 2],
    #[serde(default)]
    pub(crate) wait_s: f32,
    #[serde(default)]
    pub(crate) say: Option<String>,
    #[serde(default)]
    pub(crate) run: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Escort {
    pub(crate) key: String,
    pub(crate) quest: u32,
    #[serde(default)]
    pub(crate) map: u32,
    /// The low guid of the friendly spawn row players talk to; its entry is the escortee's.
    pub(crate) spawn: u32,
    pub(crate) level: u8,
    #[serde(default = "one")]
    pub(crate) health_mul: f32,
    pub(crate) start_option: String,
    #[serde(default)]
    pub(crate) start_say: Option<String>,
    #[serde(default)]
    pub(crate) end_say: Option<String>,
    pub(crate) failed_text: String,
    pub(crate) wait_yd: f32,
    pub(crate) leash_yd: f32,
    pub(crate) leash_s: f32,
    pub(crate) reset_s: f32,
    pub(crate) linger_s: f32,
    pub(crate) point: Vec<Point>,
}

#[derive(Deserialize)]
struct File {
    #[serde(default)]
    escort: Vec<Escort>,
}

/// Only the quest ids, which the quest load reads: an escort takes a log slot of its quest.
#[derive(Deserialize)]
struct QuestsOnly {
    #[serde(default)]
    escort: Vec<QuestOnly>,
}

#[derive(Deserialize)]
struct QuestOnly {
    quest: u32,
}

const TEXT: &str = include_str!("../content/escorts.toml");

impl Escort {
    /// Where it runs: its map's open world.
    pub(crate) fn place(&self) -> Place {
        Place {
            map: self.map,
            phase: Phase::OpenWorld,
        }
    }

    /// Whether `spawn` is its start creature's row.
    fn starts_at(&self, spawn: &mobs::Spawn) -> bool {
        spawn.guid == self.spawn && spawn.map == self.map && spawn.phase == Phase::OpenWorld
    }

    /// Whether `mob` is its start creature, as the Population streamed it in.
    fn is_start(&self, mob: &Mob) -> bool {
        mob.spawn_row().is_some_and(|s| self.starts_at(s))
    }

    /// Its start creature's spawn row.
    fn start_row(&self) -> Option<&'static mobs::Spawn> {
        mobs::content().spawn.iter().find(|s| self.starts_at(s))
    }
}

pub(crate) fn content() -> &'static [Escort] {
    static ESCORTS: OnceLock<Vec<Escort>> = OnceLock::new();
    ESCORTS.get_or_init(|| {
        let all = parse(TEXT);
        check_world(&all);
        all
    })
}

/// Whether an escort names `quest`: that quest is met when its escortee arrives. Read without
/// the rest of the escorts' load, which looks quests up.
pub(crate) fn names(quest: u32) -> bool {
    static QUESTS: OnceLock<Vec<u32>> = OnceLock::new();
    QUESTS
        .get_or_init(|| {
            let file: QuestsOnly = toml::from_str(TEXT).expect("content/escorts.toml parses");
            file.escort.iter().map(|e| e.quest).collect()
        })
        .contains(&quest)
}

/// Whether creature `entry` is an escort's start creature: it shows the gossip flag.
pub(crate) fn starts(entry: u32) -> bool {
    static ENTRIES: OnceLock<Vec<u32>> = OnceLock::new();
    ENTRIES
        .get_or_init(|| {
            content()
                .iter()
                .filter_map(|e| e.start_row().map(|s| s.entry))
                .collect()
        })
        .contains(&entry)
}

/// The places escorts run.
pub(crate) fn places() -> Vec<Place> {
    let mut places: Vec<Place> = content().iter().map(Escort::place).collect();
    places.sort();
    places.dedup();
    places
}

/// The escorts of `text` (the shape of `content/escorts.toml`). A repeated key, an escort with
/// fewer than two points, and two escorts sharing a quest or a spawn panic.
fn parse(text: &str) -> Vec<Escort> {
    let file: File = toml::from_str(text).expect("content/escorts.toml parses");
    for (i, e) in file.escort.iter().enumerate() {
        let before = &file.escort[..i];
        assert!(
            before.iter().all(|o| o.key != e.key),
            "escort {} is declared twice",
            e.key
        );
        assert!(
            e.point.len() >= 2,
            "escort {} has fewer than two points",
            e.key
        );
        if let Some(o) = before.iter().find(|o| o.quest == e.quest) {
            panic!("escorts {} and {} share quest {}", o.key, e.key, e.quest);
        }
        if let Some(o) = before.iter().find(|o| o.spawn == e.spawn) {
            panic!("escorts {} and {} share spawn {}", o.key, e.key, e.spawn);
        }
    }
    file.escort
}

/// The checks against the rest of the content: each `spawn` is a friendly spawn row on its
/// escort's map, and each `quest` a quest the realm has. A test's escort (a key starting
/// `test_`) has its quest added by the test, after the load.
fn check_world(all: &[Escort]) {
    for e in all {
        let row = e.start_row().unwrap_or_else(|| {
            panic!(
                "escort {}'s spawn {} is no open-world spawn row on map {}",
                e.key, e.spawn, e.map
            )
        });
        assert!(
            mobs::template(row.entry).is_some_and(|t| t.friendly),
            "escort {}'s spawn {} is no friendly creature",
            e.key,
            e.spawn
        );
        assert!(
            e.key.starts_with("test_") || crate::quests::content::quest(e.quest).is_some(),
            "escort {} names quest {}, which the realm does not have",
            e.key,
            e.quest
        );
    }
}

/// A character on an escort's map, as a tick sees them.
pub(crate) struct Player {
    pub(crate) guid: u64,
    pub(crate) position: [f32; 3],
    pub(crate) alive: bool,
    /// The quests in their log that have not failed.
    pub(crate) holding: Vec<u32>,
    /// Those of them whose escort is still to do.
    pub(crate) ready: Vec<u32>,
}

/// Where a run is.
#[derive(Clone, Copy)]
enum Stage {
    /// Made this tick; no client has his create yet.
    Fresh,
    Walking,
    /// At a point, the seconds left to stand there.
    Standing(f32),
    /// Arrived or failed: the seconds since, and whether the escortee has gone.
    Ended {
        since: f32,
        gone: bool,
    },
}

/// One escort under way for one group.
struct Run {
    escort: usize,
    /// Fixed at the start, as an Audience is.
    group: Vec<u64>,
    escortee: u64,
    /// The point he walks to next.
    next: usize,
    /// A point with `run` is behind him.
    running: bool,
    stage: Stage,
    /// Seconds no living group member has been within the leash.
    alone_s: f32,
}

/// What a tick has for the clients and the quest logs.
#[derive(Default)]
pub(crate) struct Ticked {
    /// The escortees' moves, for every client that sees them.
    pub(crate) shown: Vec<SessionEvent>,
    /// Lines for one character, by guid.
    pub(crate) told: Vec<(u64, SessionEvent)>,
    /// `(character, quest)`: the escort is met.
    pub(crate) credit: Vec<(u64, u32)>,
}

impl Ticked {
    pub(crate) fn extend(&mut self, other: Ticked) {
        self.shown.extend(other.shown);
        self.told.extend(other.told);
        self.credit.extend(other.credit);
    }
}

/// Every escort under way, and the starts asked for since the last tick.
#[derive(Default)]
pub(crate) struct Escorts {
    runs: Vec<Run>,
    /// `(character, escort)`: the start line was chosen.
    asked: Vec<(u64, usize)>,
}

impl Escorts {
    /// Whether character `guid` is in a running escort's group, or one ended whose start creature
    /// they do not see yet.
    pub(crate) fn in_group(&self, guid: u64) -> bool {
        self.runs.iter().any(|r| r.group.contains(&guid))
    }

    /// The escort whose start line the start creature of `spawn` offers `c`: they hold its quest,
    /// not failed, its escort is still to do, and they are in no running escort's group.
    pub(crate) fn offered(&self, c: &Char, spawn: &mobs::Spawn) -> Option<usize> {
        content().iter().position(|e| {
            e.starts_at(spawn) && crate::quests::escort_due(c, e.quest) && !self.in_group(c.guid)
        })
    }

    /// The start line of escort `i`, at gossip index [`ESCORT_OPTION`].
    pub(crate) fn option(i: usize) -> GossipOption {
        GossipOption {
            index: ESCORT_OPTION,
            icon: 0,
            coded: false,
            message: content()[i].start_option.clone(),
        }
    }

    /// Character `guid` chose escort `i`'s start line: it starts on the next tick, if it still
    /// may.
    pub(crate) fn ask(&mut self, guid: u64, i: usize) {
        self.asked.push((guid, i));
    }

    /// Every escortee under way, which a Rumour ending leaves alone.
    pub(crate) fn escortees(&self) -> Vec<u64> {
        self.runs.iter().map(|r| r.escortee).collect()
    }

    /// Hides each start creature at `place` from the groups of its escorts under way or waiting
    /// to reset: one the Population streamed back in is a new one, hidden from no one. Called
    /// after the stream, before any client is shown it, and by every tick.
    pub(crate) fn hide_starts(&self, place: Place, shard: &mut Shard) {
        let all = content();
        for r in self.runs.iter().filter(|r| all[r.escort].place() == place) {
            let e = &all[r.escort];
            let starts: Vec<u64> = shard
                .mobs
                .iter()
                .filter(|m| e.is_start(m))
                .map(|m| m.guid)
                .collect();
            for g in starts {
                shard.hide_from(g, &r.group);
            }
        }
    }

    /// No shard stands at `place`: its escortees went with it, so its runs and asks are dropped.
    pub(crate) fn drop_at(&mut self, place: Place) {
        let all = content();
        self.runs.retain(|r| all[r.escort].place() != place);
        self.asked.retain(|&(_, i)| all[i].place() != place);
    }

    /// Advances every escort at `place` by `dt` seconds among `players` (the characters there),
    /// on that place's `shard`, then starts the ones asked for there.
    pub(crate) fn tick(
        &mut self,
        dt: f32,
        place: Place,
        players: &[Player],
        shard: &mut Shard,
        host: &dyn Host,
    ) -> Ticked {
        let all = content();
        let mut out = Ticked::default();
        self.hide_starts(place, shard);
        let mut runs = std::mem::take(&mut self.runs);
        runs.retain_mut(|r| {
            let e = &all[r.escort];
            if e.place() != place {
                return true;
            }
            let lives = r.tick(e, dt, players, shard, host, &mut out);
            if !lives {
                let starts: Vec<u64> = shard
                    .mobs
                    .iter()
                    .filter(|m| e.is_start(m))
                    .map(|m| m.guid)
                    .collect();
                for g in starts {
                    shard.unhide(g, &r.group);
                }
            }
            lives
        });
        self.runs = runs;
        // After the runs, so a new escortee's first step waits for the tick that shows him.
        let (here, elsewhere): (Vec<_>, Vec<_>) = std::mem::take(&mut self.asked)
            .into_iter()
            .partition(|&(_, i)| all[i].place() == place);
        self.asked = elsewhere;
        for (guid, i) in here {
            self.start(i, guid, players, shard, host);
        }
        out
    }

    /// Starts escort `i` for character `guid` and those near its start creature, when they still
    /// may.
    fn start(
        &mut self,
        i: usize,
        guid: u64,
        players: &[Player],
        shard: &mut Shard,
        host: &dyn Host,
    ) {
        let e = &content()[i];
        let Some(starter) = players.iter().find(|p| p.guid == guid) else {
            return;
        };
        if !starter.ready.contains(&e.quest) || self.in_group(guid) {
            return;
        }
        let Some(start) = shard.mobs.iter().find(|m| e.is_start(m) && m.alive()) else {
            return;
        };
        let (start, template, at, facing) = (
            start.guid,
            start.template,
            start.position,
            start.orientation,
        );
        let range = crate::group::get().credit_range_yd;
        let mut group = vec![guid];
        group.extend(
            players
                .iter()
                .filter(|p| p.guid != guid && !self.in_group(p.guid))
                .filter(|p| flat_distance(p.position, at) <= range)
                .map(|p| p.guid),
        );
        let audience = Audience::of(&group);
        let Some(escortee) = shard.spawn_for(host, template, e.level, at, audience, true) else {
            return;
        };
        if let Some(m) = shard.mobs.iter_mut().find(|m| m.guid == escortee) {
            m.stats.max_health = ((m.stats.max_health as f32 * e.health_mul).round() as u32).max(1);
            m.health = m.stats.max_health;
            // Made as a hunter is, he never strolls; a defender never leashes.
            m.orientation = facing;
            m.home_orientation = facing;
        }
        shard.hide_from(start, &group);
        self.runs.push(Run {
            escort: i,
            group,
            escortee,
            next: 0,
            running: false,
            stage: Stage::Fresh,
            alone_s: 0.0,
        });
    }
}

impl Run {
    /// One tick at its escort's place. Whether it is still under way or waiting to reset.
    fn tick(
        &mut self,
        e: &Escort,
        dt: f32,
        players: &[Player],
        shard: &mut Shard,
        host: &dyn Host,
        out: &mut Ticked,
    ) -> bool {
        if let Stage::Ended { since, gone } = &mut self.stage {
            *since += dt;
            if !*gone && *since >= e.linger_s {
                *gone = true;
                shard.mobs.retain(|m| m.guid != self.escortee);
            }
            return !(*gone && *since >= e.reset_s);
        }
        let group: Vec<&Player> = players
            .iter()
            .filter(|p| self.group.contains(&p.guid))
            .collect();
        let Some(i) = shard
            .mobs
            .iter()
            .position(|m| m.guid == self.escortee && m.alive())
        else {
            self.fail(e, shard, out);
            return true;
        };
        let at = shard.mobs[i].position;
        let near = |yd: f32| {
            group
                .iter()
                .any(|p| p.alive && flat_distance(p.position, at) <= yd)
        };
        if near(e.leash_yd) {
            self.alone_s = 0.0;
        } else {
            self.alone_s += dt;
            if self.alone_s >= e.leash_s {
                self.fail(e, shard, out);
                return true;
            }
        }
        let mob = &mut shard.mobs[i];
        // In a fight the walk waits; the fight ends where he stands, and he goes on from there.
        if mob.state != State::Idle {
            return true;
        }
        if let Stage::Fresh = self.stage {
            self.stage = Stage::Walking;
            if let Some(text) = &e.start_say {
                say(mob, text, &group, out);
            }
        }
        if !near(e.wait_yd) {
            out.shown.extend(mob.halt());
            return true;
        }
        if let Stage::Standing(left) = &mut self.stage {
            *left -= dt;
            if *left > 0.0 {
                return true;
            }
            self.stage = Stage::Walking;
        }
        let point = &e.point[self.next];
        let [x, y] = point.at;
        let z = crate::ground::ground_under(host, e.map, [x, y, mob.position[2]], true)
            .unwrap_or(mob.position[2]);
        let to = [x, y, z];
        if !mob.there(to) {
            let step = if self.running {
                mob.run_toward(to)
            } else {
                mob.walk_toward(to, FULL_SPEED_PCT)
            };
            out.shown.extend(step);
            return true;
        }
        // A point reached: it is his home from now on, so nothing sends him back to the start.
        mob.home = to;
        if let Some(text) = &point.say {
            say(mob, text, &group, out);
        }
        self.running |= point.run;
        if self.next + 1 < e.point.len() {
            self.next += 1;
            self.stage = Stage::Standing(point.wait_s);
            return true;
        }
        // The end: every group member on the map holding the quest has it met, wherever they
        // stand, alive or dead.
        for p in &group {
            if p.holding.contains(&e.quest) {
                out.credit.push((p.guid, e.quest));
            }
        }
        if let Some(text) = &e.end_say {
            say(mob, text, &group, out);
        }
        self.stage = Stage::Ended {
            since: 0.0,
            gone: false,
        };
        true
    }

    /// The escort fails: the escortee goes, and each group member is told. Their quests are
    /// untouched.
    fn fail(&mut self, e: &Escort, shard: &mut Shard, out: &mut Ticked) {
        shard.mobs.retain(|m| m.guid != self.escortee);
        for &g in &self.group {
            out.told
                .push((g, crate::quests::system_line(e.failed_text.clone())));
        }
        self.stage = Stage::Ended {
            since: 0.0,
            gone: true,
        };
    }
}

/// `mob` says `text` to the group members within a creature's line of it.
fn say(mob: &Mob, text: &str, group: &[&Player], out: &mut Ticked) {
    let line = crate::chat::creature_say(mob, text.to_string());
    for p in group {
        if flat_distance(p.position, mob.position) <= crate::chat::creature_say_yd() {
            out.told.push((p.guid, line.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO: &str = r#"
[[escort]]
key = "a"
quest = 1
spawn = 10
level = 30
start_option = "Go."
failed_text = "Failed."
wait_yd = 20.0
leash_yd = 60.0
leash_s = 10.0
reset_s = 5.0
linger_s = 3.0

[[escort.point]]
at = [0.0, 0.0]

[[escort.point]]
at = [20.0, 0.0]
wait_s = 2.0
say = "Here."
run = true
"#;

    #[test]
    fn an_escort_loads_with_its_defaults() {
        let all = parse(TWO);
        assert_eq!(all[0].map, 0);
        assert_eq!(all[0].health_mul, 1.0);
        assert!(all[0].start_say.is_none());
        assert!(all[0].point[1].run && !all[0].point[0].run);
        assert_eq!(all[0].point[1].wait_s, 2.0);
    }

    #[test]
    #[should_panic(expected = "is declared twice")]
    fn a_repeated_key_panics() {
        let second = TWO
            .replace("quest = 1", "quest = 2")
            .replace("spawn = 10", "spawn = 11");
        parse(&format!("{TWO}{second}"));
    }

    #[test]
    #[should_panic(expected = "fewer than two points")]
    fn one_point_panics() {
        parse(TWO.split("[[escort.point]]\nat = [20.0").next().unwrap());
    }

    #[test]
    #[should_panic(expected = "share quest 1")]
    fn two_escorts_of_one_quest_panic() {
        let second = TWO
            .replace("key = \"a\"", "key = \"b\"")
            .replace("spawn = 10", "spawn = 11");
        parse(&format!("{TWO}{second}"));
    }

    #[test]
    #[should_panic(expected = "share spawn 10")]
    fn two_escorts_of_one_spawn_panic() {
        let second = TWO
            .replace("key = \"a\"", "key = \"b\"")
            .replace("quest = 1", "quest = 2");
        parse(&format!("{TWO}{second}"));
    }

    #[test]
    #[should_panic(expected = "is no open-world spawn row on map 0")]
    fn a_spawn_with_no_row_panics() {
        check_world(&parse(&TWO.replace("spawn = 10", "spawn = 4000000000")));
    }

    #[test]
    #[should_panic(expected = "which the realm does not have")]
    fn a_quest_the_realm_lacks_panics() {
        let real = parse(TEXT);
        let row = format!("spawn = {}", real[0].spawn);
        check_world(&parse(
            &TWO.replace("spawn = 10", &row)
                .replace("quest = 1", "quest = 3999999"),
        ));
    }

    #[test]
    fn the_shipped_escorts_load_and_their_start_creatures_gossip() {
        let all = content();
        assert!(all.iter().any(|e| e.key == "test_escort"));
        for e in all {
            assert!(starts(e.start_row().unwrap().entry));
            assert!(names(e.quest));
        }
    }
}
