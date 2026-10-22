//! Forest's Edge's invisible wall (#259): no one walks across the footprint's edge, in or out,
//! except through the gap the Portal leaves.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

/// Inside the hollow by the south edge (y 530), and a step south of it, outside.
const INSIDE_SOUTH: [f32; 3] = [-9990.0, 535.0, 38.0];
const PAST_SOUTH: [f32; 3] = [-9990.0, 525.0, 38.0];
/// Outside the west edge (x -10080), and a step east of it, inside.
const OUTSIDE_WEST: [f32; 3] = [-10085.0, 600.0, 38.0];
const PAST_WEST: [f32; 3] = [-10075.0, 600.0, 38.0];
/// On either side of the east rim at the Portal (-9911, 556), where the palisade has its gap.
const IN_GAP: [f32; 3] = [-9915.0, 556.0, 38.0];
const OUT_GAP: [f32; 3] = [-9905.0, 556.0, 38.0];

fn party() -> Party {
    Party::starting(realm(), &[5], START_ELWYNN, Box::new(NoGround))
}

/// The positions player `i` was snapped to after the first `from` events.
fn snaps(p: &Party, from: usize) -> Vec<[f32; 3]> {
    p.seen(0)[from..]
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Teleport { position, .. } => Some(*position),
            _ => None,
        })
        .collect()
}

/// Player 0 stands at `start`, answers the teleport that put them there, then reports a step to
/// `to`. The snaps that came of the step.
fn step(start: [f32; 3], to: [f32; 3]) -> Vec<[f32; 3]> {
    let mut p = party();
    p.go(0, start, 0.0);
    p.run(0.5);
    ack_all(&mut p);
    let from = p.seen(0).len();
    p.go(0, to, 0.0);
    p.run(0.5);
    snaps(&p, from)
}

fn ack_all(p: &mut Party) {
    let counter = p.seen(0).iter().rev().find_map(|e| match e {
        SessionEvent::Teleport { counter, .. } => Some(*counter),
        _ => None,
    });
    if let Some(counter) = counter {
        p.cmd(0, RealmCommand::TeleportAck { counter });
    }
}

#[test]
fn a_character_inside_cannot_walk_out_through_the_south_edge() {
    assert_eq!(step(INSIDE_SOUTH, PAST_SOUTH), vec![INSIDE_SOUTH]);
}

#[test]
fn a_character_outside_cannot_walk_in_through_the_west_edge() {
    assert_eq!(step(OUTSIDE_WEST, PAST_WEST), vec![OUTSIDE_WEST]);
}

#[test]
fn the_portal_gap_lets_a_character_through_both_ways() {
    assert!(step(IN_GAP, OUT_GAP).is_empty(), "out through the gap");
    assert!(step(OUT_GAP, IN_GAP).is_empty(), "in through the gap");
}
