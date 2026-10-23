//! Bounty Hunters (#177): one comes on the Heat Level's timer of open-world play, an elite a
//! level above its owner; it never leashes, and it gives up when its owner reaches the Hub or a
//! Phase, which starts the timer over.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::*;
use twilight_realm::rules::notoriety::{heat_level, hunter_secs};
use twilight_realm::{Host, CROSSROADS};

/// Notoriety that stays at Heat 4 through every drain here.
const HOT: f32 = 5000.0;
/// The lowest creature entry a hunter's name takes (`notoriety.toml`'s `entry_base`).
const HUNTER_ENTRIES: std::ops::Range<u32> = 980_000..990_000;
/// Out in the Barrens, well clear of the Hub.
const OUT: [f32; 2] = [CROSSROADS[0], CROSSROADS[1] + 300.0];
/// Just outside Northshire Abbey, and inside its Phase.
const ABBEY_OUTSIDE: [f32; 2] = [-8950.0, -120.0];
const ABBEY_INSIDE: [f32; 2] = [-8890.0, -172.0];

/// Flat ground at the Abbey's floor height, open sight everywhere.
struct Floor;

impl Host for Floor {
    fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
        Some(82.0)
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
}

/// Ground rising half a yard per yard north of `OUT`, open sight everywhere.
struct Slope;

impl Slope {
    fn ground(x: f32) -> f32 {
        82.0 + 0.5 * (x - OUT[0])
    }
}

impl Host for Slope {
    fn ground_height(&self, _: u32, x: f32, _: f32, _: f32) -> Option<f32> {
        Some(Self::ground(x))
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
}

/// Flat ground where no body fits anywhere.
struct Crowded;

impl Host for Crowded {
    fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
        Some(82.0)
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
    fn body_fits(&self, _: u32, _: [f32; 3]) -> bool {
        false
    }
}

/// A level 40 warrior with some Notoriety, and what their client has been told.
struct Hunted {
    realm: Realm,
    me: u64,
    host: Box<dyn Host>,
    /// Every hunter created for this client, `(guid, entry, level)`, in order.
    hunters: Vec<(u64, u32, u32)>,
    /// Where each of `hunters` was created.
    at: Vec<[f32; 3]>,
    removed: Vec<u64>,
    events: Vec<SessionEvent>,
}

impl Hunted {
    fn new(notoriety: f32, elwynn: bool) -> Self {
        Self::on(notoriety, elwynn, 0, Box::new(Floor))
    }

    /// As [`Self::new`], at Bounty Rank `rank`, over `host`.
    fn on(notoriety: f32, elwynn: bool, rank: u32, host: Box<dyn Host>) -> Self {
        let mut maker = realm();
        let RealmCommand::CreateCharacter(mut req) = create_cmd("Quarry", HUMAN, 0, WARRIOR) else {
            unreachable!()
        };
        if elwynn {
            req.start = benilla_protocol::START_ELWYNN;
        }
        maker.handle(RealmCommand::CreateCharacter(req));
        let me = roster(&mut maker)[0].guid;
        let blob = maker
            .export_character(me)
            .unwrap()
            .replace("level = 1\n", "level = 40\n")
            .replacen(
                "xp = 0\n",
                &format!("xp = 0\nnotoriety = {notoriety:?}\nbounty_rank = {rank}\n"),
                1,
            );
        let mut realm = realm();
        realm.import_character(&blob).unwrap();
        realm.handle(RealmCommand::EnterWorld { guid: me });
        let mut h = Self {
            realm,
            me,
            host,
            hunters: Vec::new(),
            at: Vec::new(),
            removed: Vec::new(),
            events: Vec::new(),
        };
        h.run(0.1);
        h
    }

    fn run(&mut self, secs: f32) {
        for _ in 0..(secs * 10.0).round().max(1.0) as u32 {
            self.realm.tick(0.1, self.host.as_ref());
            for e in drain(&mut self.realm) {
                match &e {
                    SessionEvent::ObjectCreate {
                        guid,
                        kind: EntityKind::Unit,
                        fields,
                        position,
                        ..
                    } if fields
                        .object_entry()
                        .is_some_and(|e| HUNTER_ENTRIES.contains(&e)) =>
                    {
                        self.hunters.push((
                            *guid,
                            fields.object_entry().unwrap(),
                            fields.unit_level().unwrap(),
                        ));
                        self.at.push(*position);
                    }
                    SessionEvent::ObjectsRemoved(gone) => self.removed.extend(gone),
                    SessionEvent::ObjectDestroyed(g) => self.removed.push(*g),
                    _ => {}
                }
                self.events.push(e);
            }
        }
    }

    fn go(&mut self, xy: [f32; 2]) {
        self.realm.handle(RealmCommand::DevTeleport {
            x: xy[0],
            y: xy[1],
            facing: None,
        });
        self.run(0.5);
    }

    /// The hunters sent, by guid, without the same one twice.
    fn sent(&self) -> Vec<u64> {
        let mut guids: Vec<u64> = self.hunters.iter().map(|h| h.0).collect();
        guids.dedup();
        guids
    }

    /// Runs until just short of the wait at `HOT`, then past it.
    fn wait_out_the_timer(&mut self) {
        let wait = hunter_secs(heat_level(HOT)).unwrap();
        let before = self.sent().len();
        self.run(wait - 5.0);
        assert_eq!(self.sent().len(), before, "none before the timer");
        self.run(10.0);
        assert_eq!(self.sent().len(), before + 1, "one when it runs out");
    }
}

#[test]
fn no_hunter_comes_at_heat_0() {
    let mut h = Hunted::new(0.0, false);
    h.go(OUT);
    h.run(20.0 * 60.0);
    assert!(h.hunters.is_empty());
}

#[test]
fn one_hunter_comes_on_the_timer_an_elite_a_level_up_and_never_a_second() {
    let mut h = Hunted::new(HOT, false);
    h.go(OUT);
    h.wait_out_the_timer();
    let (_, entry, level) = h.hunters[0];
    assert_eq!(level, 41, "the character's level + 1");

    h.realm.handle(RealmCommand::CreatureQuery { entry });
    let rank = drain(&mut h.realm).into_iter().find_map(|e| match e {
        SessionEvent::CreatureName { name, rank, .. } => Some((name, rank)),
        _ => None,
    });
    let (name, rank) = rank.expect("the client learns its name");
    assert_eq!(rank, 1, "an elite");
    assert!(name.is_some_and(|n| n.contains(' ')), "a generated name");

    h.run(hunter_secs(heat_level(HOT)).unwrap() + 30.0);
    assert_eq!(h.sent().len(), 1, "only one at a time");
}

#[test]
fn a_hunter_never_leashes() {
    let mut h = Hunted::new(HOT, false);
    h.go(OUT);
    h.wait_out_the_timer();
    let hunter = h.hunters[0].0;
    // Far past any leash from where it came in.
    h.go([OUT[0] + 150.0, OUT[1] + 150.0]);
    let from = h.events.len();
    h.run(60.0);
    let swung = h.events[from..].iter().any(
        |e| matches!(e, SessionEvent::AttackerState(s) if s.attacker == hunter && s.victim == h.me),
    );
    assert!(swung, "it caught up and fought on");
    assert!(!h.removed.contains(&hunter));
}

#[test]
fn a_hunter_gives_up_in_the_hub_and_the_timer_starts_over() {
    let mut h = Hunted::new(HOT, false);
    h.go(OUT);
    h.wait_out_the_timer();
    let hunter = h.hunters[0].0;
    h.go([CROSSROADS[0], CROSSROADS[1]]);
    assert!(h.removed.contains(&hunter), "it left the world");
    h.go(OUT);
    h.wait_out_the_timer();
}

#[test]
fn the_timer_does_not_run_in_the_hub() {
    let mut h = Hunted::new(HOT, false);
    h.run(hunter_secs(heat_level(HOT)).unwrap() * 2.0);
    assert!(h.hunters.is_empty());
    h.go(OUT);
    h.wait_out_the_timer();
}

#[test]
fn a_hunter_gives_up_when_its_owner_enters_a_phase() {
    let mut h = Hunted::new(HOT, true);
    h.go(ABBEY_OUTSIDE);
    h.wait_out_the_timer();
    let hunter = h.hunters[0].0;
    h.go(ABBEY_INSIDE);
    assert!(h.removed.contains(&hunter), "it left the world");
}

#[test]
fn each_add_stands_on_its_own_ground_not_the_hunters_height() {
    // Rank 8 rolls every affix, the adds one among them.
    let mut h = Hunted::on(HOT, false, 8, Box::new(Slope));
    h.go(OUT);
    h.run(hunter_secs(heat_level(HOT)).unwrap() + 10.0);
    assert_eq!(h.sent().len(), 3, "the hunter and its two adds");
    for at in &h.at {
        assert!(
            (at[2] - Slope::ground(at[0])).abs() < 1e-3,
            "{at:?} stands on the slope"
        );
    }
}

#[test]
fn no_hunter_comes_where_no_body_fits() {
    let mut h = Hunted::on(HOT, false, 0, Box::new(Crowded));
    h.go(OUT);
    h.run(hunter_secs(heat_level(HOT)).unwrap() + 30.0);
    assert!(h.hunters.is_empty());
}
