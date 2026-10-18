//! Northshire Abbey, the first Phased Dungeon (#78): a character inside its footprint sees and
//! fights only its four Defias Thugs (Brother Neals stands there too, and never fights), and its
//! Run resets when the last one leaves or dies.

mod common;

use std::collections::HashSet;

use benilla_protocol::messages::AttackSwingError;
use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::*;

/// `HIGHGUID_UNIT`, in a creature guid's top 16 bits.
const HIGHGUID_UNIT: u64 = 0xF130;
const DEFIAS_THUG: u64 = 38;
/// The Abbey's survivor, a Field Giver (#85).
const BROTHER_NEALS: u64 = HIGHGUID_UNIT << 48 | 952 << 24 | 780005;
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

fn thugs() -> HashSet<u64> {
    (780001..=780004)
        .map(|g| HIGHGUID_UNIT << 48 | DEFIAS_THUG << 24 | g)
        .collect()
}

fn is_unit(guid: u64) -> bool {
    guid >> 48 == HIGHGUID_UNIT
}

/// The realm and the creatures its client holds.
struct Abbey {
    realm: Realm,
    seen: HashSet<u64>,
    /// Each Thug's health in its latest create.
    created_health: Vec<(u32, u32)>,
}

impl Abbey {
    fn new() -> Self {
        let mut realm = realm();
        let RealmCommand::CreateCharacter(mut req) = create_cmd("Pilgrim", HUMAN, 0, WARRIOR)
        else {
            unreachable!()
        };
        req.start = START_ELWYNN;
        realm.handle(RealmCommand::CreateCharacter(req));
        let me = roster(&mut realm)[0].guid;
        realm.handle(RealmCommand::EnterWorld { guid: me });
        let mut a = Self {
            realm,
            seen: HashSet::new(),
            created_health: Vec::new(),
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
                    if thugs().contains(&guid) {
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
fn outside_sees_no_thug_and_inside_sees_only_the_abbeys_own() {
    let mut a = Abbey::new();
    assert!(a.seen.is_disjoint(&thugs()), "a Thug shows outside");
    assert!(!a.seen.is_empty(), "the open world streamed in");
    let thug = *thugs().iter().next().unwrap();
    assert!(a.refused(thug), "a Thug is targetable from outside");

    a.go(INSIDE, 0.1);
    let mut abbeys = thugs();
    abbeys.insert(BROTHER_NEALS);
    assert_eq!(
        a.seen, abbeys,
        "inside holds the four Thugs and Brother Neals, and nothing else"
    );
    assert!(!a.refused(thug), "a Thug is not attackable inside");
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
    assert!(a.seen.is_disjoint(&thugs()));
}

#[test]
fn dying_inside_ends_the_run_and_the_thugs_come_back_at_full_health() {
    let mut a = Abbey::new();
    a.go(INSIDE, 0.1);
    let thug = *thugs().iter().next().unwrap();
    a.realm.handle(RealmCommand::AttackSwing { guid: thug });
    // Four level 9 Thugs against a level 1 warrior.
    for _ in 0..600 {
        a.tick(0.1);
        if a.realm.run_elapsed().is_none() {
            break;
        }
    }
    assert_eq!(a.realm.run_elapsed(), None, "dying did not end the Run");
    assert!(a.seen.is_disjoint(&thugs()), "the dead still see the Thugs");

    a.realm.handle(RealmCommand::RepopRequest);
    a.tick(0.1);
    a.created_health.clear();
    a.go(INSIDE, 0.1);
    assert!(
        a.realm.run_elapsed().is_some(),
        "re-entering starts a new Run"
    );
    assert_eq!(a.created_health.len(), 4);
    assert!(
        a.created_health.iter().all(|(h, max)| h == max),
        "{:?}",
        a.created_health
    );
}

#[test]
fn the_abbeys_npcs_are_gone_and_mcbride_waits_outside() {
    let removed: [u64; 15] = [
        198, 375, 911, 6373, 459, 925, 915, 152, 190, 1213, 78, 11940, 196, 951, 952,
    ];
    let a = Abbey::new();
    let entries: HashSet<u64> = a.seen.iter().map(|g| g >> 24 & 0xFF_FFFF).collect();
    for entry in removed {
        assert!(!entries.contains(&entry), "{entry} still spawns");
    }
    assert!(entries.contains(&197), "Marshal McBride is gone");
}
