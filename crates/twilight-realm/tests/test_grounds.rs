//! The Test Grounds (#199): a Phased Dungeon of training dummies. The probe's start puts a
//! character inside it, the dummies take hits but never fight back or die, and a dummy is whole
//! again `INERT_RESET_SECS` after its last hit.

mod common;

use std::collections::HashSet;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::fight::View;
use common::*;

/// `HIGHGUID_UNIT`, in a creature guid's top 16 bits.
const HIGHGUID_UNIT: u64 = 0xF130;
/// The dummies at a Run's start: low guid and entry, as `test_grounds.toml` places them.
const DUMMIES: [(u64, u64); 4] = [
    (790001, 97001),
    (790002, 97001),
    (790003, 97001),
    (790004, 97002),
];
/// The level-5 dummy, and the spot in front of it (it stands at (-9640, 264)).
const LOW_DUMMY: u64 = HIGHGUID_UNIT << 48 | 97001 << 24 | 790001;
const BY_LOW_DUMMY: [f32; 2] = [-9641.5, 264.0];
/// Out in Elwynn, well clear of the box.
const OUTSIDE: [f32; 2] = [-9720.0, 255.0];

/// The same ground everywhere, so every point lands inside the box's height.
struct Ground;

impl Host for Ground {
    fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
        Some(47.2)
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
}

fn dummies() -> HashSet<u64> {
    DUMMIES
        .iter()
        .map(|&(g, entry)| HIGHGUID_UNIT << 48 | entry << 24 | g)
        .collect()
}

struct Grounds {
    realm: Realm,
    view: View,
    seen: HashSet<u64>,
    me: u64,
    /// Everything that has turned on the character.
    attackers: HashSet<u64>,
    /// Every unit that died.
    dead: HashSet<u64>,
}

impl Grounds {
    /// A level-30 human warrior, out in Elwynn.
    fn new() -> Self {
        let mut maker = realm();
        let RealmCommand::CreateCharacter(mut req) = create_cmd("Tester", HUMAN, 0, WARRIOR) else {
            unreachable!()
        };
        req.start = START_ELWYNN;
        maker.handle(RealmCommand::CreateCharacter(req));
        let me = roster(&mut maker)[0].guid;
        let blob = maker
            .export_character(me)
            .unwrap()
            .replace("level = 1\n", "level = 30\n");
        let mut realm = realm();
        realm.import_character(&blob).unwrap();
        realm.handle(RealmCommand::EnterWorld { guid: me });
        let mut g = Self {
            realm,
            view: View::default(),
            seen: HashSet::new(),
            me,
            attackers: HashSet::new(),
            dead: HashSet::new(),
        };
        g.go(OUTSIDE, None, 1.0);
        g
    }

    fn tick(&mut self, secs: f32) {
        for _ in 0..(secs * 10.0).round().max(1.0) as u32 {
            self.realm.tick(0.1, &Ground);
            let events = self.view.drain(&mut self.realm);
            for e in events {
                match e {
                    SessionEvent::ObjectCreate { guid, .. } if guid >> 48 == HIGHGUID_UNIT => {
                        self.seen.insert(guid);
                    }
                    SessionEvent::ObjectsRemoved(gone) => gone.iter().for_each(|g| {
                        self.seen.remove(g);
                    }),
                    SessionEvent::ObjectDestroyed(g) => {
                        self.seen.remove(&g);
                    }
                    SessionEvent::AttackStart { attacker, victim } if victim == self.me => {
                        self.attackers.insert(attacker);
                    }
                    _ => {}
                }
            }
            for &d in &dummies() {
                if self.view.objects.contains_key(&d) && self.view.health(d) == 0 {
                    self.dead.insert(d);
                }
            }
        }
    }

    fn go(&mut self, xy: [f32; 2], facing: Option<f32>, secs: f32) {
        self.realm.handle(RealmCommand::DevTeleport {
            x: xy[0],
            y: xy[1],
            facing,
        });
        self.tick(secs);
    }

    fn max_health(&self, guid: u64) -> u32 {
        self.view.fields(guid).unit_max_health().unwrap()
    }
}

#[test]
fn the_probe_start_is_inside_the_phase_with_every_dummy() {
    let mut g = Grounds::new();
    assert!(
        g.seen.is_disjoint(&dummies()),
        "a dummy shows outside the Test Grounds"
    );

    let [x, y, facing] = twilight_realm::test_grounds_start();
    g.go([x, y], Some(facing), 0.5);
    assert!(
        dummies().is_subset(&g.seen),
        "missing dummies at the start: {:?}",
        dummies().difference(&g.seen).collect::<Vec<_>>()
    );
}

#[test]
fn a_dummy_takes_hits_never_fights_back_and_never_dies() {
    let mut g = Grounds::new();
    g.go(BY_LOW_DUMMY, Some(0.0), 0.5);
    let full = g.max_health(LOW_DUMMY);
    g.realm
        .handle(RealmCommand::AttackSwing { guid: LOW_DUMMY });
    let mut lowest = full;
    for _ in 0..600 {
        g.tick(0.1);
        lowest = lowest.min(g.view.health(LOW_DUMMY));
    }
    assert!(
        lowest < full,
        "a minute of swings did no damage ({lowest}/{full})"
    );
    assert_eq!(
        lowest, 1,
        "a level-30 warrior did not wear a level-5 dummy down to its floor"
    );
    assert!(g.dead.is_empty(), "a dummy died: {:?}", g.dead);
    assert!(
        g.attackers.is_empty(),
        "a dummy turned on the character: {:?}",
        g.attackers
    );
}

#[test]
fn a_dummy_is_whole_again_ten_seconds_after_its_last_hit() {
    let mut g = Grounds::new();
    g.go(BY_LOW_DUMMY, Some(0.0), 0.5);
    let full = g.max_health(LOW_DUMMY);
    g.realm
        .handle(RealmCommand::AttackSwing { guid: LOW_DUMMY });
    for _ in 0..300 {
        g.tick(0.1);
        if g.view.health(LOW_DUMMY) < full {
            break;
        }
    }
    assert!(g.view.health(LOW_DUMMY) < full, "no hit landed");
    g.realm.handle(RealmCommand::AttackStop);
    g.tick(5.0);
    assert!(
        g.view.health(LOW_DUMMY) < full,
        "healed before its 10 s were up"
    );
    g.tick(6.0);
    assert_eq!(
        g.view.health(LOW_DUMMY),
        full,
        "not whole 11 s after the last hit"
    );
}
