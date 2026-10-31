//! One Run per Phased Dungeon (ADR 0006): everyone inside Northshire Abbey shares its creatures
//! and its clock, and it resets once no living player is inside.

mod common;

use std::collections::HashSet;

use benilla_protocol::messages::CHAT_MSG_SYSTEM;
use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

const HIGHGUID_UNIT: u64 = 0xF130;
const BROTHER_NEALS: u64 = HIGHGUID_UNIT << 48 | 952 << 24 | 780005;
/// Inside the nave, and out in the yard before the front doors.
const INSIDE: [f32; 2] = [-8890.0, -172.0];
const OUTSIDE: [f32; 2] = [-8950.0, -120.0];

/// The Abbey's floor everywhere, so a teleport lands inside the footprint's height, and an
/// upper storey under a creature placed on one (#253): a flat 82 would put the Abbot in the nave
/// beside the pilgrims.
struct AbbeyFloor;

impl Host for AbbeyFloor {
    fn ground_height(&self, _: u32, _: f32, _: f32, z_hint: f32) -> Option<f32> {
        Some(if (100.0..130.0).contains(&z_hint) {
            z_hint - 2.0
        } else {
            82.0
        })
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

/// The max health of `guid` in each `UNIT_FIELD_MAXHEALTH` player `i`'s client has been sent,
/// creates and values updates alike, in order.
fn max_healths(party: &Party, i: usize, guid: u64) -> Vec<u32> {
    party
        .seen(i)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid: g, fields, ..
            }
            | SessionEvent::ObjectValues { guid: g, fields }
                if *g == guid =>
            {
                fields.unit_max_health()
            }
            _ => None,
        })
        .collect()
}

/// The max health `guid` was created with on player `i`'s client.
fn created_max_health(party: &Party, i: usize, guid: u64) -> u32 {
    party
        .seen(i)
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid: g, fields, ..
            } if *g == guid => fields.unit_max_health(),
            _ => None,
        })
        .expect("a create")
}

/// Whether player `i`'s client was sent a values update of `guid`'s max health at `max`.
fn sent_max_health(party: &Party, i: usize, guid: u64, max: u32) -> bool {
    party.seen(i).iter().any(|e| {
        matches!(e, SessionEvent::ObjectValues { guid: g, fields }
            if *g == guid && fields.unit_max_health() == Some(max))
    })
}

/// Half again as much, rounded: a creature's health at a Head Count of 2 (ADR 0029).
fn half_again(health: u32) -> u32 {
    (health as f32 * 1.5).round() as u32
}

/// The chat lines of `chat_type` player `i` has been sent.
fn said(party: &Party, i: usize, chat_type: u8) -> Vec<String> {
    party
        .seen(i)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Chat(m) if m.chat_type == chat_type => Some(m.text.clone()),
            _ => None,
        })
        .collect()
}

/// The "tuned for" lines player `i` has been told.
fn tuned(party: &Party, i: usize) -> Vec<String> {
    said(party, i, CHAT_MSG_SYSTEM)
        .into_iter()
        .filter(|t| t.contains("tuned for"))
        .collect()
}

#[test]
fn a_second_player_inside_scales_the_idle_creatures() {
    let mut party = pilgrims();
    go(&mut party, 0, INSIDE);
    party.run(0.5);
    let created: Vec<(u64, u32)> = monsters()
        .into_iter()
        .map(|m| (m, created_max_health(&party, 0, m)))
        .collect();
    go(&mut party, 1, INSIDE);
    party.run(0.5);
    // The one inside is sent the new max health; the one joining is created with it.
    let scaled: Vec<(u64, u32)> = created
        .iter()
        .copied()
        .filter(|&(m, made)| sent_max_health(&party, 0, m, half_again(made)))
        .collect();
    assert!(!scaled.is_empty(), "no monster scaled to two");
    for &(m, made) in &scaled {
        assert_eq!(max_healths(&party, 1, m).last(), Some(&half_again(made)));
    }

    go(&mut party, 1, OUTSIDE);
    party.run(0.5);
    for &(m, made) in &scaled {
        assert_eq!(max_healths(&party, 0, m).last(), Some(&made), "back to one");
    }
}

#[test]
fn a_pulled_creature_keeps_its_scale_when_a_player_leaves() {
    let mut party = pilgrims();
    go(&mut party, 0, INSIDE);
    party.run(0.5);
    go(&mut party, 1, INSIDE);
    party.run(0.5);
    // A ground-floor monster scaled to two, and the first pilgrim beside it.
    let (brother, at) = unit_creates(party.seen(0))
        .into_iter()
        .filter(|(m, _, at)| monsters().contains(m) && at[2] < 100.0)
        .find(|&(m, _, _)| {
            sent_max_health(&party, 0, m, half_again(created_max_health(&party, 0, m)))
        })
        .map(|(m, _, at)| (m, at))
        .expect("a scaled monster on the ground floor");
    go(&mut party, 0, [at[0] + 1.0, at[1]]);
    party.run(0.1);
    party.cmd(0, RealmCommand::AttackSwing { guid: brother });
    let (a, mut fought) = (party.guids[0], false);
    party.run_until(10.0, |got| {
        fought |= swung_at(&got[0], brother).contains(&a);
        fought
    });
    assert!(fought, "the brother fights");

    let from = party.seen(0).len();
    go(&mut party, 1, OUTSIDE);
    party.run(1.0);
    let since = &party.seen(0)[from..];
    assert!(
        !since
            .iter()
            .any(|e| matches!(e, SessionEvent::ObjectValues { guid, fields }
            if *guid == brother && fields.unit_max_health().is_some())),
        "the fought brother keeps the scale it was pulled at"
    );
    let back = monsters().into_iter().filter(|&m| m != brother).any(|m| {
        let made = created_max_health(&party, 0, m);
        since.iter().any(|e| {
            matches!(e, SessionEvent::ObjectValues { guid, fields }
                    if *guid == m && fields.unit_max_health() == Some(made))
        })
    });
    assert!(back, "an idle monster goes back to its created health");
}

#[test]
fn the_phase_is_told_when_its_head_count_changes() {
    let mut party = pilgrims();
    go(&mut party, 0, INSIDE);
    party.run(0.5);
    assert_eq!(tuned(&party, 0), Vec::<String>::new(), "the first one in");
    go(&mut party, 1, INSIDE);
    party.run(0.5);
    let two = "Northshire Abbey is now tuned for 2 players.";
    assert_eq!(tuned(&party, 0), [two]);
    assert_eq!(tuned(&party, 1), [two]);
    go(&mut party, 1, OUTSIDE);
    party.run(0.5);
    assert_eq!(
        tuned(&party, 0),
        [two, "Northshire Abbey is now tuned for 1 player."]
    );
    assert_eq!(
        tuned(&party, 1),
        [two],
        "the one outside is not in the Phase"
    );
}
