//! Several players in one realm, each in their own session: the multiplayer scripts' harness.

use std::collections::HashMap;

use benilla_protocol::{EntityKind, SessionEvent};
use twilight_realm::{Host, Realm, RealmCommand, SessionId};

use super::{create, roster, CHAR_CREATE_SUCCESS, HUMAN};

/// A host with no ground loaded anywhere: the realm keeps each spawn's own height.
pub struct NoGround;

impl Host for NoGround {
    fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
        None
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
}

pub struct Party {
    pub realm: Realm,
    pub sessions: Vec<SessionId>,
    pub guids: Vec<u64>,
    /// Every event so far, by session.
    pub log: HashMap<SessionId, Vec<SessionEvent>>,
}

const NAMES: [&str; 5] = ["Varian", "Anduin", "Jaina", "Rexxar", "Thrall"];

impl Party {
    /// One warrior per entry of `levels`, each in their own session, in the world.
    pub fn new(mut realm: Realm, levels: &[u8]) -> Self {
        let mut maker = super::realm();
        for name in &NAMES[..levels.len()] {
            assert_eq!(create(&mut maker, name, HUMAN, 0), CHAR_CREATE_SUCCESS);
        }
        let mut guids = Vec::new();
        for (name, level) in NAMES.iter().zip(levels) {
            let guid = roster(&mut maker)
                .into_iter()
                .find(|c| c.name == *name)
                .unwrap()
                .guid;
            let blob = maker
                .export_character(guid)
                .unwrap()
                .replace("level = 1\n", &format!("level = {level}\n"));
            realm.import_character(&blob).unwrap();
            guids.push(guid);
        }
        let sessions: Vec<SessionId> = guids
            .iter()
            .map(|&guid| {
                let s = realm.connect();
                realm.handle_as(s, RealmCommand::EnterWorld { guid });
                s
            })
            .collect();
        let mut party = Self {
            realm,
            sessions,
            guids,
            log: HashMap::new(),
        };
        party.run(0.0);
        party
    }

    /// Puts player `i` at `at`, facing `facing`.
    pub fn go(&mut self, i: usize, at: [f32; 3], facing: f32) {
        self.cmd(
            i,
            RealmCommand::Move {
                position: at,
                orientation: facing,
            },
        );
    }

    /// Puts player `i` at `at`, facing `toward`.
    pub fn go_facing(&mut self, i: usize, at: [f32; 3], toward: [f32; 3]) {
        self.go(i, at, (toward[1] - at[1]).atan2(toward[0] - at[0]));
    }

    pub fn cmd(&mut self, i: usize, command: RealmCommand) {
        self.realm.handle_as(self.sessions[i], command);
        self.collect();
    }

    fn collect(&mut self) -> HashMap<SessionId, Vec<SessionEvent>> {
        let mut got: HashMap<SessionId, Vec<SessionEvent>> = HashMap::new();
        for (s, e) in self.realm.drain_session_events() {
            got.entry(s).or_default().push(e.clone());
            self.log.entry(s).or_default().push(e);
        }
        got
    }

    /// Ticks `secs` in 0.1 s steps (the first at once); the events each player got, by index.
    pub fn run(&mut self, secs: f32) -> Vec<Vec<SessionEvent>> {
        let mut got = vec![Vec::new(); self.sessions.len()];
        let mut t = -0.1;
        while t < secs - 1e-3 {
            let dt = if t < 0.0 { 0.0 } else { 0.1 };
            self.realm.tick(dt, &NoGround);
            for (s, events) in self.collect() {
                if let Some(i) = self.sessions.iter().position(|&x| x == s) {
                    got[i].extend(events);
                }
            }
            t += 0.1;
        }
        got
    }

    /// Ticks in 0.1 s steps until `done` holds for what came, at most `secs`; whether it did.
    pub fn run_until(
        &mut self,
        secs: f32,
        mut done: impl FnMut(&[Vec<SessionEvent>]) -> bool,
    ) -> bool {
        let mut t = 0.0;
        while t < secs {
            let got = self.run(0.1);
            if done(&got) {
                return true;
            }
            t += 0.1;
        }
        false
    }

    /// Everything player `i`'s client has been sent.
    pub fn seen(&self, i: usize) -> &[SessionEvent] {
        self.log
            .get(&self.sessions[i])
            .map_or(&[][..], |e| e.as_slice())
    }

    /// The unit of `entry` whose create player `i` saw nearest `near`: its guid and position.
    pub fn unit(&self, i: usize, entry: u32, near: [f32; 3]) -> (u64, [f32; 3]) {
        unit_creates(self.seen(i))
            .into_iter()
            .filter(|(_, e, _)| *e == entry)
            .min_by(|a, b| flat(a.2, near).total_cmp(&flat(b.2, near)))
            .map(|(g, _, at)| (g, at))
            .expect("a unit of the entry")
    }
}

pub fn flat_distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    flat(a, b)
}

fn flat(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// The units created: guid, entry, position.
pub fn unit_creates(events: &[SessionEvent]) -> Vec<(u64, u32, [f32; 3])> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                fields,
                ..
            } => Some((*guid, fields.object_entry().unwrap_or(0), *position)),
            _ => None,
        })
        .collect()
}

/// Whether `events` show `guid`'s health at 0.
pub fn died(events: &[SessionEvent], guid: u64) -> bool {
    events.iter().any(|e| {
        matches!(e, SessionEvent::ObjectValues { guid: g, fields }
            if *g == guid && fields.unit_health() == Some(0))
    })
}

/// The victims of `attacker`'s melee swings in `events`.
pub fn swung_at(events: &[SessionEvent], attacker: u64) -> Vec<u64> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::AttackerState(s) if s.attacker == attacker => Some(s.victim),
            _ => None,
        })
        .collect()
}
