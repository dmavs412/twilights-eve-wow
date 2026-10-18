//! One Run per Phased Dungeon (ADR 0006): everyone inside Northshire Abbey shares its creatures
//! and its clock, and it resets once no living player is inside.

mod common;

use std::collections::HashSet;

use benilla_protocol::START_ELWYNN;
use common::party::*;
use common::*;

const HIGHGUID_UNIT: u64 = 0xF130;
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

fn the_abbeys() -> HashSet<u64> {
    let mut all = monsters();
    all.insert(BROTHER_NEALS);
    all
}

/// Two level 1 pilgrims in the yard.
fn pilgrims() -> Party {
    let mut party = Party::starting(realm(), &[1, 1], START_ELWYNN, Box::new(AbbeyFloor));
    go(&mut party, 0, OUTSIDE);
    go(&mut party, 1, OUTSIDE);
    party.run(1.0);
    party
}

fn go(party: &mut Party, i: usize, xy: [f32; 2]) {
    party.cmd(
        i,
        RealmCommand::DevTeleport {
            x: xy[0],
            y: xy[1],
            facing: None,
        },
    );
}

fn elapsed(party: &Party, i: usize) -> Option<f32> {
    party.realm.run_elapsed_as(party.sessions[i])
}

#[test]
fn two_inside_share_the_runs_creatures_and_clock() {
    let mut party = pilgrims();
    go(&mut party, 0, INSIDE);
    party.run(0.1);
    // The one outside sees none of the Run's creatures.
    assert!(party.units_held(1).is_disjoint(&monsters()));
    party.run(2.0);
    go(&mut party, 1, INSIDE);
    party.run(0.1);
    assert_eq!(party.units_held(0), the_abbeys());
    assert_eq!(party.units_held(1), the_abbeys());
    let (a, b) = (elapsed(&party, 0).unwrap(), elapsed(&party, 1).unwrap());
    assert_eq!(a, b, "one clock");
    assert!(a > 2.0, "the late one joins the Run as it stands");

    // One leaves: the Run goes on.
    go(&mut party, 0, OUTSIDE);
    party.run(1.0);
    assert_eq!(elapsed(&party, 0), None);
    assert!(elapsed(&party, 1).is_some_and(|t| t > a));
    assert!(party.units_held(0).is_disjoint(&monsters()));

    // Both out: it resets, and the next one in starts a new one.
    go(&mut party, 1, OUTSIDE);
    party.run(0.1);
    go(&mut party, 0, INSIDE);
    party.run(0.1);
    assert!(elapsed(&party, 0).is_some_and(|t| t < 1.0), "a fresh Run");
}

#[test]
fn both_dead_inside_resets_the_run() {
    let mut party = pilgrims();
    go(&mut party, 0, INSIDE);
    go(&mut party, 1, INSIDE);
    party.run(0.1);
    let brother = HIGHGUID_UNIT << 48 | 96003 << 24 | 780009;
    party.cmd(0, RealmCommand::AttackSwing { guid: brother });
    party.cmd(1, RealmCommand::AttackSwing { guid: brother });
    // The nave's level 8 Corrupted Brother against two level 1 warriors.
    let (a, b) = (party.guids[0], party.guids[1]);
    let mut dead = (false, false);
    party.run_until(120.0, |got| {
        dead.0 |= died(&got[0], a);
        dead.1 |= died(&got[1], b);
        dead.0 && dead.1
    });
    assert!(dead.0 && dead.1, "the Abbey kills both");
    party.run(0.1);
    assert_eq!(elapsed(&party, 0), None);
    assert_eq!(elapsed(&party, 1), None);

    party.cmd(0, RealmCommand::RepopRequest);
    party.run(0.1);
    go(&mut party, 0, INSIDE);
    party.run(0.1);
    assert!(elapsed(&party, 0).is_some_and(|t| t < 1.0), "a fresh Run");
}
