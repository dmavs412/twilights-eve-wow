//! Closed Zones (#301, ADR 0015): the realm reads `closed_chunks.json`, no one steps onto a
//! closed chunk, living or dead, and nothing the content places stands on one.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;
use twilight_realm::{closed_at, placed};

/// Elwynn's side of the Westfall edge at y 866.67 (chunk 486, 803), and 2 yards west of it, in
/// Westfall (chunk 485, 803).
const EDGE_OPEN: [f32; 3] = [-9700.0, 865.67, 40.0];
const EDGE_CLOSED: [f32; 3] = [-9700.0, 867.67, 40.0];
/// 60 yards west of `EDGE_OPEN`, deeper in Westfall: farther than a walker's step.
const FAR_CLOSED: [f32; 3] = [-9700.0, 925.67, 40.0];
/// Young Wolf, the Northshire creature that kills a level 1 warrior who does not fight back.
const YOUNG_WOLF: u32 = 299;
const TOLD: &str = "You cannot go that way.";

fn party() -> Party {
    Party::starting(realm(), &[1], START_ELWYNN, Box::new(NoGround))
}

/// Answers the last teleport player 0 was sent, so their reports count again.
fn ack(p: &mut Party) {
    let counter = p.seen(0).iter().rev().find_map(|e| match e {
        SessionEvent::Teleport { counter, .. } => Some(*counter),
        _ => None,
    });
    if let Some(counter) = counter {
        p.cmd(0, RealmCommand::TeleportAck { counter });
    }
}

/// Player 0 reports a step to `to`; the positions they were snapped to and how many times they
/// were told, from what that step brought.
fn step(p: &mut Party, to: [f32; 3]) -> (Vec<[f32; 3]>, usize) {
    let from = p.seen(0).len();
    p.go(0, to, 0.0);
    p.run(0.1);
    let new = &p.seen(0)[from..];
    let snaps = new
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Teleport { position, .. } => Some(*position),
            _ => None,
        })
        .collect();
    let told = new
        .iter()
        .filter(|e| matches!(e, SessionEvent::Notification { text } if text == TOLD))
        .count();
    (snaps, told)
}

/// A party whose player 0 stands at `EDGE_OPEN`, nothing pending.
fn at_the_edge() -> Party {
    let mut p = party();
    p.go(0, EDGE_OPEN, 0.0);
    p.run(0.5);
    ack(&mut p);
    p
}

#[test]
fn sentinel_hill_is_closed_and_goldshire_and_northshire_are_open() {
    assert!(closed_at(0, -10508.8, 1045.23), "Gryan Stoutmantle");
    assert!(!closed_at(0, -9465.52, 74.0069), "Marshal Dughan");
    assert!(!closed_at(0, -8933.54, -136.523), "Deputy Willem");
    assert!(!closed_at(1, -10508.8, 1045.23), "Kalimdor is all open");
    assert!(!closed_at(1, -417.0, -2646.0), "the Crossroads");
}

#[test]
fn the_points_either_side_of_an_edge_are_one_open_and_one_closed() {
    assert!(!closed_at(0, EDGE_OPEN[0], EDGE_OPEN[1]));
    assert!(closed_at(0, EDGE_CLOSED[0], EDGE_CLOSED[1]));
    assert!(closed_at(0, FAR_CLOSED[0], FAR_CLOSED[1]));
}

#[test]
fn a_living_character_is_snapped_back_and_told_once_in_three_seconds() {
    let mut p = at_the_edge();
    assert_eq!(step(&mut p, EDGE_CLOSED), (vec![EDGE_OPEN], 1));
    ack(&mut p);
    assert_eq!(
        step(&mut p, EDGE_CLOSED),
        (vec![EDGE_OPEN], 0),
        "within 3 s"
    );
    ack(&mut p);
    p.run(3.0);
    assert_eq!(step(&mut p, EDGE_CLOSED), (vec![EDGE_OPEN], 1), "after 3 s");
}

#[test]
fn a_step_along_the_open_side_goes_through() {
    let mut p = at_the_edge();
    let along = [EDGE_OPEN[0] + 2.0, EDGE_OPEN[1], EDGE_OPEN[2]];
    assert_eq!(step(&mut p, along), (vec![], 0));
}

#[test]
fn a_report_sixty_yards_into_a_closed_zone_is_refused() {
    let mut p = at_the_edge();
    assert_eq!(step(&mut p, FAR_CLOSED), (vec![EDGE_OPEN], 1));
}

#[test]
fn a_dead_character_is_snapped_back_too() {
    let mut p = party();
    p.run(0.5);
    let me = p.guids[0];
    let (wolf, at) = p.unit(0, YOUNG_WOLF, [-8949.95, -132.493, 83.5]);
    let lying = [at[0] + 1.0, at[1], at[2]];
    p.go_facing(0, lying, at);
    p.run(0.5);
    ack(&mut p);
    // One swing to start the fight, then the warrior lets the wolf win.
    p.cmd(0, RealmCommand::AttackSwing { guid: wolf });
    p.run(0.5);
    p.cmd(0, RealmCommand::AttackStop);
    let mut dead = false;
    for _ in 0..1200 {
        p.run(1.0);
        if died(p.seen(0), me) {
            dead = true;
            break;
        }
    }
    assert!(dead, "the wolf killed the warrior within twenty minutes");
    ack(&mut p);
    let (snaps, told) = step(&mut p, EDGE_CLOSED);
    assert_eq!(snaps.len(), 1, "snapped back");
    assert!(
        flat_distance(snaps[0], EDGE_CLOSED) > 1000.0 && flat_distance(snaps[0], lying) < 30.0,
        "back where the body was, not in Westfall: {snaps:?}"
    );
    assert_eq!(told, 1);
}

#[test]
fn nothing_the_content_places_stands_on_a_closed_chunk() {
    let mut offenders: Vec<String> = Vec::new();
    for p in placed() {
        let [x, y] = p.position;
        if closed_at(p.map, x, y) && !offenders.iter().any(|o| o.starts_with(&p.what)) {
            offenders.push(format!("{} at ({x}, {y}) on map {}", p.what, p.map));
        }
    }
    assert!(
        offenders.is_empty(),
        "placed on a closed chunk:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn the_content_walk_reaches_every_kind_it_names() {
    let all = placed();
    for kind in [
        "creature ",
        "Hub NPC ",
        "quest GameObject ",
        "explore spot",
        "Rumour ",
        "Encounter ",
        "footprint",
        " portal",
        "Dungeon entrance",
        "respawn",
        " bind",
    ] {
        assert!(
            all.iter().any(|p| p.what.contains(kind)),
            "no {kind:?} in the walk"
        );
    }
    assert!(
        all.iter()
            .any(|p| p.what.starts_with("Rumour ") && p.what.contains(" step ")),
        "no Rumour step point in the walk"
    );
}
