//! The Tower of Azora (#244, `content/azora.toml`): a Phased Dungeon with three Redridge stops,
//! each with a mini-boss and a ward stone, and the Shadow of Morganth in the top chamber.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
const HIGHGUID_UNIT: u64 = 0xF130;
/// The Phased Dungeon's id.
const AZORA: u64 = 7;

fn object(entry: u64) -> u64 {
    HIGHGUID_GAMEOBJECT << 48 | entry
}

/// The inner portals to the water, fire and shadow stops: `0x0F000000 | dungeon << 8 | stop << 1`.
const INNER_WATER: u64 = HIGHGUID_GAMEOBJECT << 48 | 0x0F00_0000 | AZORA << 8;
const INNER_FIRE: u64 = INNER_WATER | 2;
const INNER_SHADOW: u64 = INNER_WATER | 4;

const GRESH: u64 = unit_at(96034, 970015);
const THANE: u64 = unit_at(96035, 970035);
const KEPT: u64 = unit_at(96036, 970055);
const SHADOW: u64 = unit_at(96037, 970062);

const fn unit_at(entry: u64, low: u64) -> u64 {
    HIGHGUID_UNIT << 48 | entry << 24 | low
}

const WARD_WATER: u64 = 975001;
const WARD_FIRE: u64 = 975002;
const WARD_SHADOW: u64 = 975003;
const CACHE: u64 = 975004;

/// Beside each inner portal in the tower, and where each stop lands a character.
const BY_INNER_WATER: [f32; 3] = [-9558.5, -718.5, 64.74];
const BY_INNER_FIRE: [f32; 3] = [-9550.5, -709.5, 75.06];
const BY_INNER_SHADOW: [f32; 3] = [-9552.5, -707.5, 90.41];
const ARRIVAL_FIRE: [f32; 3] = [-8995.0, -2380.0, 100.0];
/// Beside each mini-boss in its stop, and each mini-boss's spot.
const BY_GRESH: [f32; 3] = [-9179.5, -2015.0, 116.06];
const GRESH_AT: [f32; 3] = [-9178.0, -2015.0, 116.06];
const BY_THANE: [f32; 3] = [-8976.5, -2375.0, 145.82];
const THANE_AT: [f32; 3] = [-8975.0, -2375.0, 145.82];
const BY_KEPT: [f32; 3] = [-9223.5, -2699.0, 88.8];
const KEPT_AT: [f32; 3] = [-9222.0, -2699.0, 88.8];
/// Beside each ward stone, and beside the ore cache.
const BY_WARD_WATER: [f32; 3] = [-9177.5, -2023.0, 119.33];
const BY_WARD_FIRE: [f32; 3] = [-8973.5, -2384.0, 151.94];
const BY_WARD_SHADOW: [f32; 3] = [-9220.5, -2705.0, 88.8];
const BY_CACHE: [f32; 3] = [-9220.5, -2701.0, 88.8];
/// In the top chamber, beside the Shadow of Morganth.
const BY_SHADOW: [f32; 3] = [-9546.5, -721.0, 99.27];
const SHADOW_AT: [f32; 3] = [-9545.0, -720.0, 99.27];

/// Five fight together: a stop's elites are too many for one.
const PLAYERS: usize = 5;
const THE_TOWER_IS_OPEN: u32 = 94601;
const APPRENTICES_SECRET: u32 = 94602;
const WARD_SHARD: u32 = 980021;

/// Level 60 warriors, the first with The Tower Is Open under way.
fn party(n: usize) -> Party {
    let mut p = Party::edited(
        Realm::with_seed(1)
            .with_creature_spells(96030, &[])
            .with_creature_spells(96036, &[]),
        &vec![60; n],
        START_ELWYNN,
        Box::new(NoGround),
        |i, blob| {
            if i == 0 {
                format!("{blob}\n[[quests]]\nid = {THE_TOWER_IS_OPEN}\nslot = 0\nkills = [0, 0, 0, 0]\n")
            } else {
                blob
            }
        },
    );
    p.run(1.0);
    p
}

fn holds(p: &Party, i: usize, guid: u64) -> bool {
    p.units_held(i).contains(&guid)
}

fn shows(p: &Party, i: usize, guid: u64) -> bool {
    let mut shown = false;
    for e in p.seen(i) {
        match e {
            SessionEvent::ObjectCreate { guid: g, .. } if *g == guid => shown = true,
            SessionEvent::ObjectsRemoved(gone) if gone.contains(&guid) => shown = false,
            SessionEvent::ObjectDestroyed(g) if *g == guid => shown = false,
            _ => {}
        }
    }
    shown
}

fn teleports(p: &Party, i: usize, from: usize) -> Vec<[f32; 3]> {
    p.seen(i)[from..]
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Teleport { position, .. } => Some(*position),
            _ => None,
        })
        .collect()
}

fn ack(p: &mut Party, i: usize) {
    let counter = p
        .seen(i)
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::Teleport { counter, .. } => Some(*counter),
            _ => None,
        })
        .expect("a teleport to answer");
    p.cmd(i, RealmCommand::TeleportAck { counter });
}

fn click(p: &mut Party, i: usize, guid: u64) {
    p.cmd(i, RealmCommand::UseGameObject { guid });
    p.run(0.3);
}

/// Player `i` stands at `at` facing `toward` and fights `target`, stepping up and swinging again
/// after a fear or a root, until it dies.
fn kill(p: &mut Party, i: usize, at: [f32; 3], toward: [f32; 3], target: u64) {
    for j in 0..PLAYERS {
        p.go_facing(j, at, toward);
    }
    p.run(0.2);
    for round in 0..240 {
        // A fresh swing command restarts the swing, so it is sent only every ten rounds, to take
        // a fear's loser back into the fight. The step goes with it: a root from the last fight
        // (Gresh's Drowning Grasp) may still have held when the first one was sent.
        if round % 10 == 0 {
            for j in 0..PLAYERS {
                p.go_facing(j, at, toward);
                p.cmd(j, RealmCommand::AttackSwing { guid: target });
            }
        }
        p.run(0.5);
        if died(p.seen(i), target) {
            return;
        }
    }
    let health = p.seen(i).iter().rev().find_map(|e| match e {
        SessionEvent::ObjectValues { guid, fields, .. } if *guid == target => fields.unit_health(),
        _ => None,
    });
    let own_death = p
        .seen(i)
        .iter()
        .filter(|e| format!("{e:?}").contains("Death"))
        .count();
    let series: Vec<u32> = p
        .seen(i)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectValues { guid, fields, .. } if *guid == target => {
                fields.unit_health()
            }
            _ => None,
        })
        .collect();
    let hits = p
        .seen(i)
        .iter()
        .filter(|e| matches!(e, SessionEvent::AttackerState(s) if s.victim == target))
        .count();
    let refusals: Vec<String> = p
        .seen(i)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::AttackSwingError(r) => Some(format!("{r:?}")),
            _ => None,
        })
        .collect();
    let created: Vec<[f32; 3]> = p
        .seen(i)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate { guid, position, .. } if *guid == target => Some(*position),
            _ => None,
        })
        .collect();
    let ports: Vec<(u32, [f32; 3])> = p
        .seen(i)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Teleport {
                counter, position, ..
            } => Some((*counter, *position)),
            _ => None,
        })
        .collect();
    panic!(
        "{target:#x} did not die, last health {health:?}, {own_death} death events, {hits} blows at it,          created at {created:?} (the player stands at {at:?}), teleports {ports:?}, swings refused          {refusals:?}, health {:?}",
        &series[..series.len().min(40)]
    );
}

/// Kills the shadow stop's two Voidwalkers and two darkweavers, one at a time, and then the Kept:
/// five characters of the test's weak kind do not outlast all of them at once.
fn kill_kept(p: &mut Party) {
    for (entry, low, at) in [
        (96033, 970051, [-9230.0, -2706.0, 88.8]),
        (96033, 970052, [-9230.0, -2694.0, 88.8]),
        (96030, 970053, [-9226.0, -2686.0, 88.8]),
        (96030, 970054, [-9226.0, -2714.0, 88.8]),
    ] {
        let by = [at[0] - 1.5, at[1], at[2]];
        kill(p, 0, by, at, unit_at(entry, low));
        // Out of combat the party heals a little before the next.
        p.run(60.0);
    }
    kill(p, 0, BY_KEPT, KEPT_AT, KEPT);
}

/// Kills the stop's mini-boss, pins its ward stone and takes the stop's way back to the tower.
fn pin(
    p: &mut Party,
    inner: u64,
    by_inner: [f32; 3],
    boss: (u64, [f32; 3], [f32; 3]),
    ward: (u64, [f32; 3]),
) {
    p.go(0, by_inner, 0.0);
    p.run(0.3);
    click(p, 0, inner);
    ack(p, 0);
    kill(p, 0, boss.1, boss.2, boss.0);
    p.go(0, ward.1, 0.0);
    p.run(0.2);
    click(p, 0, object(ward.0));
    ack(p, 0);
}

fn offered(p: &Party, i: usize, from: usize, quest: u32) -> bool {
    p.seen(i)[from..].iter().any(|e| match e {
        SessionEvent::QuestDetail(d) => d.quest_id == quest,
        SessionEvent::QuestGreeting(list) => list.quests.iter().any(|q| q.quest_id == quest),
        _ => false,
    })
}

#[test]
fn the_water_portal_is_open_and_the_fire_and_shadow_portals_are_closed() {
    let mut p = party(PLAYERS);
    p.go(0, BY_INNER_WATER, 0.0);
    p.run(0.3);
    let from = p.seen(0).len();
    click(&mut p, 0, INNER_FIRE);
    click(&mut p, 0, INNER_SHADOW);
    assert!(
        teleports(&p, 0, from).is_empty(),
        "a closed portal took them"
    );
    click(&mut p, 0, INNER_WATER);
    assert_eq!(
        teleports(&p, 0, from),
        [[-9200.0, -2020.0, 43.0]],
        "the water portal is closed"
    );
}

/// Terrain 2.3 yd under the tower's ground floor, as the game's ground lookup finds it there.
struct Terrain;

impl Host for Terrain {
    fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
        Some(62.4)
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
}

#[test]
fn the_water_portal_stands_on_the_floor_not_the_terrain_under_the_tower() {
    let mut p = Party::starting(Realm::with_seed(1), &[60], START_ELWYNN, Box::new(Terrain));
    p.run(1.0);
    p.go(0, BY_INNER_WATER, 0.0);
    p.run(0.5);
    let z = p.seen(0).iter().find_map(|e| match e {
        SessionEvent::ObjectCreate { guid, position, .. } if *guid == INNER_WATER => {
            Some(position[2])
        }
        _ => None,
    });
    assert_eq!(z, Some(64.74), "the portal is buried or never shown");
    let from = p.seen(0).len();
    click(&mut p, 0, INNER_WATER);
    assert_eq!(teleports(&p, 0, from), [[-9200.0, -2020.0, 43.0]]);
}

#[test]
fn pinning_water_opens_fire_and_pinning_shadow_brings_the_shadow_of_morganth() {
    let mut p = party(PLAYERS);
    p.go(0, BY_INNER_WATER, 0.0);
    p.run(0.5);
    assert!(
        !holds(&p, 0, SHADOW),
        "the Shadow is in the chamber at the start"
    );

    pin(
        &mut p,
        INNER_WATER,
        BY_INNER_WATER,
        (GRESH, BY_GRESH, GRESH_AT),
        (WARD_WATER, BY_WARD_WATER),
    );
    p.go(0, BY_INNER_FIRE, 0.0);
    p.run(0.3);
    let from = p.seen(0).len();
    click(&mut p, 0, INNER_FIRE);
    assert_eq!(teleports(&p, 0, from), [ARRIVAL_FIRE], "fire did not open");
    ack(&mut p, 0);

    kill(&mut p, 0, BY_THANE, THANE_AT, THANE);
    p.go(0, BY_WARD_FIRE, 0.0);
    p.run(0.2);
    click(&mut p, 0, object(WARD_FIRE));
    ack(&mut p, 0);

    p.go(0, BY_INNER_SHADOW, 0.0);
    p.run(0.3);
    click(&mut p, 0, INNER_SHADOW);
    ack(&mut p, 0);
    kill_kept(&mut p);
    p.go(0, BY_WARD_SHADOW, 0.0);
    p.run(0.2);
    click(&mut p, 0, object(WARD_SHADOW));
    ack(&mut p, 0);

    p.go(0, BY_SHADOW, 0.0);
    p.run(1.0);
    assert!(holds(&p, 0, SHADOW), "the Shadow did not come");
}

#[test]
fn the_ore_cache_appears_after_the_kept_dies_and_offers_the_apprentices_secret() {
    let mut p = party(PLAYERS);
    p.go(0, BY_INNER_WATER, 0.0);
    p.run(0.5);
    pin(
        &mut p,
        INNER_WATER,
        BY_INNER_WATER,
        (GRESH, BY_GRESH, GRESH_AT),
        (WARD_WATER, BY_WARD_WATER),
    );
    p.go(0, BY_INNER_FIRE, 0.0);
    p.run(0.3);
    click(&mut p, 0, INNER_FIRE);
    ack(&mut p, 0);
    kill(&mut p, 0, BY_THANE, THANE_AT, THANE);
    p.go(0, BY_WARD_FIRE, 0.0);
    p.run(0.2);
    click(&mut p, 0, object(WARD_FIRE));
    ack(&mut p, 0);
    p.go(0, BY_INNER_SHADOW, 0.0);
    p.run(0.3);
    click(&mut p, 0, INNER_SHADOW);
    ack(&mut p, 0);

    p.run(0.5);
    assert!(
        !shows(&p, 0, object(CACHE)),
        "the cache stands before the Kept dies"
    );
    kill_kept(&mut p);
    p.go(0, BY_CACHE, 0.0);
    p.run(0.5);
    assert!(shows(&p, 0, object(CACHE)), "the cache did not appear");
    let from = p.seen(0).len();
    click(&mut p, 0, object(CACHE));
    assert!(
        offered(&p, 0, from, APPRENTICES_SECRET),
        "the cache did not offer the quest to a character with The Tower Is Open"
    );
}

/// A fight with the Shadow outlasted the test characters' health, so the corpse itself is not
/// looted: the file's drop list for its guid is checked instead.
#[test]
fn the_shadow_of_morganths_corpse_holds_three_ward_shards() {
    let file: toml::Table = include_str!("../content/azora.toml").parse().unwrap();
    let shadow = file["creature"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["guid"].as_integer() == Some(970062))
        .expect("the Shadow of Morganth is in the file");
    let drop: Vec<i64> = shadow["drop"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i.as_integer().unwrap())
        .collect();
    let shards = drop.iter().filter(|&&i| i == i64::from(WARD_SHARD)).count();
    assert_eq!(shards, 3, "the Shadow's drop list {drop:?}");
}

fn floats(v: &toml::Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_float().unwrap())
        .collect()
}

/// Two gnolls, two acolytes, one Voidwalker and Theocritus stand in the tower at the start of a
/// Run (#262). The Shadow of Morganth, held back by its ward, is not one of them.
#[test]
fn six_creatures_stand_in_the_tower_at_the_start_of_a_run() {
    let file: toml::Table = include_str!("../content/azora.toml").parse().unwrap();
    let origin = floats(&file["origin"]);
    let (sin, cos) = file["yaw"].as_float().unwrap().to_radians().sin_cos();
    let boxes: Vec<[Vec<f64>; 3]> = file["footprint"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| [floats(&b["x"]), floats(&b["y"]), floats(&b["z"])])
        .collect();
    let inside = |p: Vec<f64>| {
        let (dx, dy) = (p[0] - origin[0], p[1] - origin[1]);
        let local = [dx * cos + dy * sin, dy * cos - dx * sin, p[2] - origin[2]];
        boxes
            .iter()
            .any(|b| (0..3).all(|k| b[k][0] <= local[k] && local[k] < b[k][1]))
    };
    let standing: Vec<i64> = file["creature"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| inside(floats(&c["position"])))
        .filter(|c| c.get("after_pinned").is_none())
        .map(|c| c["guid"].as_integer().unwrap())
        .collect();
    assert_eq!(
        standing,
        [970001, 970002, 970021, 970022, 970041, 970061],
        "the tower's creatures at the start"
    );
}
