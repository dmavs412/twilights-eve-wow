//! Northshire Abbey, the first Phased Dungeon (#78): a character inside its footprint sees and
//! fights only its own monsters (#94; Brother Neals stands there too, and never fights), and its
//! Run resets when the last one leaves or dies.

mod common;

use std::collections::HashSet;

use benilla_protocol::messages::AttackSwingError;
use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::*;

/// `HIGHGUID_UNIT`, in a creature guid's top 16 bits.
const HIGHGUID_UNIT: u64 = 0xF130;
/// The Abbey's survivor, a Field Giver (#85).
const BROTHER_NEALS: u64 = HIGHGUID_UNIT << 48 | 952 << 24 | 780005;
/// The Abbot at the top of the bell tower, whose bell calls two Corrupted Brothers at half health.
const ABBOT: u64 = HIGHGUID_UNIT << 48 | 96005 << 24 | 780015;
/// Where the Abbot stands, in the test's flat Abbey.
const ABBOT_AT: [f32; 2] = [-8897.0, -172.9];
/// The first low guid of a creature a call for help brings in.
const CALLED_GUID_BASE: u64 = 0x90_0000;
/// Inside the nave, and out in the yard before the front doors.
const INSIDE: [f32; 2] = [-8890.0, -172.0];
const OUTSIDE: [f32; 2] = [-8950.0, -120.0];

/// The Abbey's floor everywhere, so a teleport lands inside the footprint's height.
struct AbbeyFloor;

impl Host for AbbeyFloor {
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

/// The Abbey's monsters at a Run's start (#94): their low guids and entries.
const MONSTERS: [(u64, u64); 10] = [
    (780006, 96003),
    (780007, 96003),
    (780008, 96003),
    (780009, 96003),
    (780010, 96004),
    (780011, 96003),
    (780012, 96004),
    (780013, 96003),
    (780014, 96004),
    (780015, 96005),
];

fn monsters() -> HashSet<u64> {
    MONSTERS
        .iter()
        .map(|&(g, entry)| HIGHGUID_UNIT << 48 | entry << 24 | g)
        .collect()
}

fn is_unit(guid: u64) -> bool {
    guid >> 48 == HIGHGUID_UNIT
}

/// The realm and the creatures its client holds.
struct Abbey {
    realm: Realm,
    seen: HashSet<u64>,
    /// Each monster's health in its latest create.
    created_health: Vec<(u32, u32)>,
    /// The character.
    me: u64,
    /// Every creature a call for help has brought in, since last cleared.
    called: HashSet<u64>,
    /// Everything that has turned on the character.
    attackers: HashSet<u64>,
}

impl Abbey {
    fn new() -> Self {
        Self::at_level(1)
    }

    /// A human warrior of `level` in the yard.
    fn at_level(level: u8) -> Self {
        let mut maker = realm();
        let RealmCommand::CreateCharacter(mut req) = create_cmd("Pilgrim", HUMAN, 0, WARRIOR)
        else {
            unreachable!()
        };
        req.start = START_ELWYNN;
        maker.handle(RealmCommand::CreateCharacter(req));
        let me = roster(&mut maker)[0].guid;
        let blob = maker
            .export_character(me)
            .unwrap()
            .replace("level = 1\n", &format!("level = {level}\n"));
        let mut realm = realm();
        realm.import_character(&blob).unwrap();
        realm.handle(RealmCommand::EnterWorld { guid: me });
        let mut a = Self {
            realm,
            seen: HashSet::new(),
            created_health: Vec::new(),
            me,
            called: HashSet::new(),
            attackers: HashSet::new(),
        };
        a.go(OUTSIDE, 1.0);
        a
    }

    fn tick(&mut self, secs: f32) {
        for _ in 0..(secs * 10.0).round().max(1.0) as u32 {
            self.realm.tick(0.1, &AbbeyFloor);
            self.read();
        }
    }

    fn read(&mut self) {
        for e in drain(&mut self.realm) {
            match e {
                SessionEvent::ObjectCreate { guid, fields, .. } if is_unit(guid) => {
                    self.seen.insert(guid);
                    if guid & 0xFF_FFFF >= CALLED_GUID_BASE {
                        self.called.insert(guid);
                    }
                    if monsters().contains(&guid) {
                        let h = (
                            fields.unit_health().unwrap(),
                            fields.unit_max_health().unwrap(),
                        );
                        self.created_health.push(h);
                    }
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
    }

    fn go(&mut self, xy: [f32; 2], secs: f32) {
        self.realm.handle(RealmCommand::DevTeleport {
            x: xy[0],
            y: xy[1],
            facing: None,
        });
        self.tick(secs);
    }

    /// Walks up to the Abbot and fights him until he rings his bell.
    fn fight_the_abbot(&mut self) {
        self.called.clear();
        self.attackers.clear();
        self.realm.handle(RealmCommand::DevTeleport {
            x: ABBOT_AT[0] + 1.5,
            y: ABBOT_AT[1],
            facing: Some(std::f32::consts::PI),
        });
        self.tick(0.1);
        self.realm.handle(RealmCommand::AttackSwing { guid: ABBOT });
        for _ in 0..3000 {
            self.tick(0.1);
            if !self.called.is_empty() {
                break;
            }
        }
    }

    fn refused(&mut self, guid: u64) -> bool {
        self.realm.handle(RealmCommand::AttackSwing { guid });
        drain(&mut self.realm).into_iter().any(|e| {
            matches!(
                e,
                SessionEvent::AttackSwingError(AttackSwingError::DeadOrUnattackable)
            )
        })
    }
}

#[test]
fn outside_sees_no_monster_and_inside_sees_only_the_abbeys_own() {
    let mut a = Abbey::new();
    assert!(a.seen.is_disjoint(&monsters()), "a monster shows outside");
    assert!(!a.seen.is_empty(), "the open world streamed in");
    let brother = HIGHGUID_UNIT << 48 | 96003 << 24 | 780009;
    assert!(a.refused(brother), "a monster is targetable from outside");

    a.go(INSIDE, 0.1);
    let mut abbeys = monsters();
    abbeys.insert(BROTHER_NEALS);
    assert_eq!(
        a.seen, abbeys,
        "inside holds the Abbey's monsters and Brother Neals, and nothing else"
    );
    assert!(!a.refused(brother), "a monster is not attackable inside");
}

#[test]
fn entering_starts_a_run_and_leaving_ends_it() {
    let mut a = Abbey::new();
    assert_eq!(a.realm.run_elapsed(), None);
    a.go(INSIDE, 0.5);
    assert!(a.realm.run_elapsed().is_some_and(|t| t > 0.0), "no Run");
    a.go(OUTSIDE, 0.1);
    assert_eq!(
        a.realm.run_elapsed(),
        None,
        "the Run outlived the last one out"
    );
    assert!(a.seen.is_disjoint(&monsters()));
}

#[test]
fn dying_inside_ends_the_run_and_the_monsters_come_back_at_full_health() {
    let mut a = Abbey::new();
    a.go(INSIDE, 0.1);
    let brother = HIGHGUID_UNIT << 48 | 96003 << 24 | 780009;
    a.realm.handle(RealmCommand::AttackSwing { guid: brother });
    // A level 8 Corrupted Brother against a level 1 warrior.
    for _ in 0..600 {
        a.tick(0.1);
        if a.realm.run_elapsed().is_none() {
            break;
        }
    }
    assert_eq!(a.realm.run_elapsed(), None, "dying did not end the Run");
    assert!(
        a.seen.is_disjoint(&monsters()),
        "the dead still see the monsters"
    );

    a.realm.handle(RealmCommand::RepopRequest);
    a.tick(0.1);
    a.created_health.clear();
    a.go(INSIDE, 0.1);
    assert!(
        a.realm.run_elapsed().is_some(),
        "re-entering starts a new Run"
    );
    assert_eq!(a.created_health.len(), MONSTERS.len());
    assert!(
        a.created_health.iter().all(|(h, max)| h == max),
        "{:?}",
        a.created_health
    );
}

#[test]
fn the_abbeys_npcs_are_gone_and_mcbride_waits_outside() {
    let removed: [u64; 14] = [
        198, 375, 911, 6373, 459, 925, 915, 152, 190, 1213, 78, 11940, 951, 952,
    ];
    let a = Abbey::new();
    let entries: HashSet<u64> = a.seen.iter().map(|g| g >> 24 & 0xFF_FFFF).collect();
    for entry in removed {
        assert!(!entries.contains(&entry), "{entry} still spawns");
    }
    assert!(entries.contains(&197), "Marshal McBride is gone");
}

#[test]
fn the_abbots_bell_calls_two_brothers_and_they_fall_on_his_attacker() {
    let mut a = Abbey::at_level(40);
    a.go(INSIDE, 0.1);
    assert!(a.called.is_empty(), "the adds were there from the start");
    a.fight_the_abbot();
    assert_eq!(a.called.len(), 2, "the call brought {:?}", a.called);
    assert!(
        a.called.is_subset(&a.attackers),
        "the adds are not fighting the character: {:?} of {:?}",
        a.attackers,
        a.called
    );
}

#[test]
fn a_call_is_made_once_a_life() {
    let mut a = Abbey::at_level(40);
    a.go(INSIDE, 0.1);
    a.fight_the_abbot();
    assert_eq!(a.called.len(), 2);
    // Many more blows, each past the threshold again.
    a.tick(3.0);
    assert_eq!(a.called.len(), 2, "a second call: {:?}", a.called);
}

#[test]
fn a_reset_run_lets_the_call_fire_again() {
    let mut a = Abbey::at_level(40);
    a.go(INSIDE, 0.1);
    a.fight_the_abbot();
    assert_eq!(a.called.len(), 2);

    a.go(OUTSIDE, 0.1);
    assert_eq!(a.realm.run_elapsed(), None, "the Run did not end");
    assert!(a.seen.is_disjoint(&a.called), "the adds outlived the Run");
    a.go(INSIDE, 0.1);
    assert!(
        a.seen.is_disjoint(&a.called),
        "the adds are there at the Run's start"
    );
    a.fight_the_abbot();
    assert_eq!(a.called.len(), 2, "no call in the second Run");
}
