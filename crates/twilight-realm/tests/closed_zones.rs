//! Closed Zones (#301, ADR 0015): the realm reads `closed_chunks.json`, no one steps onto a
//! closed chunk, living or dead, and nothing the content places stands on one.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;
use twilight_realm::{audit_rows, closed_at, closed_for_good, placed, AuditKind};

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
    assert!(closed_at(0, -10508.8, 1045.23, &[]), "Gryan Stoutmantle");
    assert!(!closed_at(0, -9465.52, 74.0069, &[]), "Marshal Dughan");
    assert!(!closed_at(0, -8933.54, -136.523, &[]), "Deputy Willem");
    assert!(
        !closed_at(1, -10508.8, 1045.23, &[]),
        "Kalimdor is all open"
    );
    assert!(!closed_at(1, -417.0, -2646.0, &[]), "the Crossroads");
}

#[test]
fn the_points_either_side_of_an_edge_are_one_open_and_one_closed() {
    assert!(!closed_at(0, EDGE_OPEN[0], EDGE_OPEN[1], &[]));
    assert!(closed_at(0, EDGE_CLOSED[0], EDGE_CLOSED[1], &[]));
    assert!(closed_at(0, FAR_CLOSED[0], FAR_CLOSED[1], &[]));
}

/// Morgan's Vigil in the Burning Steppes (Marshal Maxwell), which `steppes_open` opens.
const MORGANS_VIGIL: [f32; 3] = [-8378.5, -2748.9, 0.0];
/// The Redridge side of Blackrock Pass, and a step north onto a Burning Steppes chunk.
const PASS_OPEN: [f32; 3] = [-8690.0, -2560.0, 132.0];
const PASS_STEPPES: [f32; 3] = [-8650.0, -2560.0, 132.0];
const STEPPES_OPEN: &str = "steppes_open";

#[test]
fn the_steppes_open_with_their_milestone_and_the_four_neighbours_never_do() {
    // (x, y, where, closed before steppes_open, closed after).
    let points = [
        (
            -8378.5,
            -2748.9,
            "Morgan's Vigil, Marshal Maxwell",
            true,
            false,
        ),
        (-7495.0, -2170.9, "Flame Crest, Ragged John", true, false),
        (
            -6679.9,
            -1194.4,
            "Searing Gorge, Kalaran Windblade",
            true,
            true,
        ),
        (
            -6633.7,
            -2724.6,
            "the Badlands, Lotwil Veriatus",
            true,
            true,
        ),
        (
            -10632.3,
            -3009.4,
            "the Swamp of Sorrows, Fallen Hero of the Horde",
            true,
            true,
        ),
        (
            -3732.5,
            -797.3,
            "the Wetlands, Captain Stoutfist",
            true,
            true,
        ),
        (-10508.8, 1045.23, "Westfall, Gryan Stoutmantle", true, true),
        (
            -9219.0,
            -2222.0,
            "Lakeshire, Magistrate Solomon",
            false,
            false,
        ),
    ];
    for (x, y, place, before, after) in points {
        assert_eq!(closed_at(0, x, y, &[]), before, "{place} before");
        assert_eq!(closed_at(0, x, y, &[STEPPES_OPEN]), after, "{place} after");
    }
}

/// A party whose player 0 stands at `PASS_OPEN`, nothing pending.
fn at_the_pass(realm: twilight_realm::Realm) -> Party {
    let mut p = Party::starting(realm, &[1], START_ELWYNN, Box::new(NoGround));
    p.go(0, PASS_OPEN, 0.0);
    p.run(0.5);
    ack(&mut p);
    p
}

#[test]
fn the_steppes_are_walled_off_until_steppes_open_is_reached() {
    assert!(!closed_at(0, PASS_OPEN[0], PASS_OPEN[1], &[]));
    let mut p = at_the_pass(realm());
    assert_eq!(step(&mut p, PASS_STEPPES), (vec![PASS_OPEN], 1));
    ack(&mut p);
    p.realm.reach_milestone(STEPPES_OPEN);
    p.run(0.1);
    assert_eq!(step(&mut p, PASS_STEPPES), (vec![], 0), "once reached");
}

/// A quest GameObject on the first Burning Steppes chunk north of `PASS_OPEN`, in view of it:
/// Morgan's Vigil is farther than the view reaches from open land.
const STEPPES_OBJECT: &str = r#"
[[gameobject]]
guid = 990628
entry = 990628
display = 1
map = 0
position = [-8645.0, -2560.0, 132.0]
"#;

/// Whether `events` hold a create of the GameObject of low guid `low`.
fn creates_object(events: &[SessionEvent], low: u64) -> bool {
    events.iter().any(
        |e| matches!(e, SessionEvent::ObjectCreate { guid, .. } if guid & 0xFFFF_FFFF_FFFF == low),
    )
}

#[test]
fn a_quest_object_on_the_steppes_shows_only_once_they_open() {
    assert!(
        closed_at(0, -8645.0, -2560.0, &[]),
        "the object stands on the Steppes"
    );
    let mut p = at_the_pass(realm().with_quests(STEPPES_OBJECT));
    p.run(2.0);
    assert!(
        !creates_object(p.seen(0), 990628),
        "sent before the Steppes open"
    );
    let from = p.seen(0).len();
    p.realm.reach_milestone(STEPPES_OPEN);
    p.run(1.0);
    assert!(
        creates_object(&p.seen(0)[from..], 990628),
        "the next look sends it"
    );
}

/// Each `position = [x, y, z]` line of a save, in order.
fn save_positions(blob: &str) -> Vec<(usize, [f32; 3])> {
    blob.lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let list = line.strip_prefix("position = [")?.strip_suffix(']')?;
            let v: Vec<f32> = list.split(',').map(|n| n.trim().parse().unwrap()).collect();
            Some((i, [v[0], v[1], v[2]]))
        })
        .collect()
}

#[test]
fn a_character_saved_on_the_steppes_enters_the_world_at_their_bind_point() {
    let at_vigil = |_: usize, blob: String| {
        let (line, _) = save_positions(&blob)[0];
        blob.lines()
            .enumerate()
            .map(|(i, l)| {
                if i == line {
                    format!(
                        "position = [{}, {}, {}]",
                        MORGANS_VIGIL[0], MORGANS_VIGIL[1], MORGANS_VIGIL[2]
                    )
                } else {
                    l.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    };
    let mut p = Party::edited(realm(), &[1], START_ELWYNN, Box::new(NoGround), at_vigil);
    let blob = p.realm.export_character(p.guids[0]).unwrap();
    let bind_line = blob.lines().position(|l| l == "[bind]").expect("a bind");
    let positions = save_positions(&blob);
    let here = positions[0].1;
    let bind = positions
        .iter()
        .find(|(i, _)| *i > bind_line)
        .expect("the bind's position")
        .1;
    assert!(
        flat_distance(here, bind) < 1.0,
        "at the bind {bind:?}: {here:?}"
    );
    assert!(flat_distance(here, MORGANS_VIGIL) > 1000.0);
    let log = p.realm.drain_log();
    assert!(
        log.iter()
            .any(|l| l.contains("Varian") && l.contains("closed land")),
        "{log:?}"
    );
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

/// Content on land a Milestone opens is fine: it shows once the land opens.
#[test]
fn nothing_the_content_places_stands_on_a_chunk_closed_for_good() {
    let mut offenders: Vec<String> = Vec::new();
    for p in placed() {
        let [x, y] = p.position;
        if closed_for_good(p.map, x, y) && !offenders.iter().any(|o| o.starts_with(&p.what)) {
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

#[test]
fn audit_rows_hold_only_our_units() {
    let rows = audit_rows();
    // The imported spawns' guids, from the generated rows of `elwynn.toml` and `barrens.toml`.
    let imported: std::collections::HashSet<String> = [
        include_str!("../content/elwynn.toml"),
        include_str!("../content/barrens.toml"),
    ]
    .iter()
    .flat_map(|file| file.lines())
    .filter_map(|line| {
        let row = line.trim().strip_prefix('[')?;
        let guid = row.split(',').next()?.trim();
        guid.parse::<u32>().ok().map(|g| format!("creature {g} "))
    })
    .collect();
    assert!(!imported.is_empty(), "the imported rows are read");
    // A `[[moved]]` row stands an imported spawn where we put it, which makes it ours.
    let overrides = include_str!("../content/elwynn_overrides.toml");
    let moved: Vec<String> = overrides
        .split("[[moved]]")
        .skip(1)
        .filter_map(|table| {
            let line = table
                .lines()
                .find(|l| l.trim_start().starts_with("entry"))?;
            Some(format!("(entry {})", line.split('=').nth(1)?.trim()))
        })
        .collect();
    for r in rows
        .iter()
        .filter(|r| !moved.iter().any(|m| r.what.ends_with(m)))
    {
        assert!(
            !imported.iter().any(|g| r.what.starts_with(g.as_str())),
            "{} is an imported spawn",
            r.what
        );
    }
    let walk = placed();
    for p in walk.iter().filter(|p| p.what.starts_with("Rumour ")) {
        if p.what.contains(" point") {
            assert!(
                rows.iter().any(|r| r.what == p.what && r.map == p.map),
                "{} is audited",
                p.what
            );
        }
    }
    let kinds = |pick: fn(&AuditKind) -> bool| rows.iter().any(|r| pick(&r.kind));
    assert!(kinds(|k| *k == AuditKind::Creature));
    assert!(kinds(|k| *k == AuditKind::HubNpc));
    assert!(kinds(|k| *k == AuditKind::GameObject));
    assert!(kinds(|k| matches!(k, AuditKind::RumourGiver { .. })));
    assert!(kinds(|k| matches!(k, AuditKind::RumourStep { .. })));
}
