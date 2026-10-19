//! The Barrens is populated: its real camps stream in around the player, the low camps by the
//! Crossroads cover levels 1–6 with nothing above 10 around them, and the mobs pull as camps,
//! leash home and respawn.
//!
//! Positions come from the research dumps the realm's content is generated from
//! (`data/twilights-eve/`), so a script can walk to a real camp.

mod common;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use benilla_protocol::{EntityKind, SessionEvent};
use common::*;
use serde_json::Value;
use twilight_realm::CROSSROADS;

#[test]
fn walking_the_barrens_shows_its_real_camps() {
    let (mut realm, _) = warrior_at(1);
    // Thorn Hill's Razormane, east of the Crossroads.
    let hunter = imported_spawn(3265);
    let events = teleport(&mut realm, hunter.position);

    let units = creates(&events);
    let seen = units
        .get(&hunter.guid)
        .expect("the Razormane Hunter streams in where vmangos puts it");
    assert_eq!(seen.entry, 3265);
    assert!((11..=12).contains(&seen.level), "level {}", seen.level);
    assert!(hunter.displays.contains(&seen.display_id));
    assert!(
        distance2(seen.position, hunter.position) < 0.5,
        "{:?}",
        seen.position
    );
    realm.handle(RealmCommand::CreatureQuery { entry: 3265 });
    assert_eq!(
        creature_name(&mut realm, 3265).as_deref(),
        Some("Razormane Hunter")
    );
}

#[test]
fn only_the_mobs_near_the_player_are_in_the_world() {
    let (mut realm, _) = warrior_at(1);
    let hunter = imported_spawn(3265);
    let events = teleport(&mut realm, hunter.position);
    let near = creates(&events);
    assert!(near.len() > 3, "a camp's worth: {}", near.len());
    assert!(near.len() < 120, "not the whole zone: {}", near.len());
    for (guid, u) in &near {
        let d = distance2(u.position, hunter.position);
        assert!(d <= 100.0, "{guid:#x} streamed in {d} yd away");
    }

    // Standing on a Razormane camp at level 1 starts a fight.
    let fighting: HashSet<u64> = attackers(&events).collect();
    assert!(!fighting.is_empty());

    // A long way south, Thorn Hill leaves the world and the oasis comes in; the mobs in the
    // fight stay until they have run home.
    let oasis = imported_spawn(3461);
    let events = teleport(&mut realm, oasis.position);
    let all: HashSet<u64> = near.keys().copied().collect();
    let calm: HashSet<u64> = all.difference(&fighting).copied().collect();
    assert_eq!(removed(&events), calm);
    assert!(creates(&events).contains_key(&oasis.guid));
    let events = run(&mut realm, 30.0);
    assert_eq!(removed(&events), fighting);

    // Back again: Thorn Hill streams in anew.
    let events = teleport(&mut realm, hunter.position);
    assert!(creates(&events).contains_key(&hunter.guid));
}

#[test]
fn level_1_to_10_camps_stand_near_the_crossroads() {
    let low = json("crossroads_low_camps.json");
    let mut levels = Vec::new();
    for camp in low["camps"].as_array().unwrap() {
        // A realm of its own: a camp within view of the last one is already streamed in.
        let (mut realm, _) = warrior_at(1);
        let center = [
            camp["center"][0].as_f64().unwrap() as f32,
            camp["center"][1].as_f64().unwrap() as f32,
            92.0,
        ];
        let from_hub = distance2(center, [CROSSROADS[0], CROSSROADS[1], 0.0]);
        assert!(from_hub < 600.0, "{from_hub} yd out");

        let units = creates(&teleport(&mut realm, center));
        for m in camp["members"].as_array().unwrap() {
            let guid = unit_guid(m["entry"].as_u64().unwrap(), m["guid"].as_u64().unwrap());
            let u = units.get(&guid).unwrap_or_else(|| {
                panic!(
                    "{}: member {} ({}) does not stand in its camp",
                    camp["camp_id"], m["guid"], m["name"]
                )
            });
            // A rare (rank 4) stands a few levels above the ring's top: it is the scary one.
            let top = if m["rank"] == 4 { 13 } else { 10 };
            assert!((1..=top).contains(&u.level), "level {}", u.level);
            assert_eq!(u.display_id as u64, m["display_id"].as_u64().unwrap());
            levels.push(u.level);
        }
        // The imported level-11 wanderers that stood here are gone.
        for g in camp["suppress_guids"].as_array().unwrap() {
            let low_guid = g.as_u64().unwrap();
            assert!(
                !units.keys().any(|u| u & 0xFF_FFFF == low_guid),
                "suppressed spawn {low_guid} is in the world"
            );
        }
    }
    assert_eq!(levels.iter().min(), Some(&1));
    assert!(levels.iter().max() >= Some(&5));
}

#[test]
fn walking_around_the_crossroads_meets_nothing_above_level_10() {
    // Rings around the town, out to where what streams in still stands inside the inner ring.
    const FRIENDLY: u32 = 35;
    let (mut realm, _) = warrior_at(10);
    let mut world = HashMap::new();
    let mut met = HashSet::new();
    for radius in [150.0_f32, 300.0, 440.0] {
        let steps = (std::f32::consts::TAU * radius / 16.0).ceil() as usize;
        for i in 0..steps {
            let a = std::f32::consts::TAU * i as f32 / steps as f32;
            let at = [
                CROSSROADS[0] + radius * a.cos(),
                CROSSROADS[1] + radius * a.sin(),
                92.0,
            ];
            realm.handle(RealmCommand::Move {
                motion: Default::default(),
                position: at,
                orientation: a,
            });
            see(&mut world, &run(&mut realm, 0.2));
            for (guid, u) in world.iter().filter(|(_, u)| u.faction != FRIENDLY) {
                assert!(
                    u.level <= 10,
                    "a level {} {} at {:?}, {radius} yd out",
                    u.level,
                    u.entry,
                    u.position
                );
                met.insert(*guid);
            }
        }
    }
    assert!(
        met.len() > 20,
        "the low camps are on the way: {}",
        met.len()
    );
}

/// How far above a low camp's top level a mob is red to the camp's players (vanilla's con colour:
/// five levels up).
const RED_ABOVE: u32 = 5;
/// How near the way from the Crossroads to a low camp no red mob stands.
const ROUTE_CLEAR_YD: f32 = 40.0;

#[test]
fn at_a_low_camp_no_red_mob_is_in_view() {
    let low = json("crossroads_low_camps.json");
    for camp in low["camps"].as_array().unwrap() {
        let (mut realm, _) = warrior_at(1);
        let mut world = HashMap::new();
        see(&mut world, &teleport(&mut realm, camp_center(camp)));
        see(&mut world, &run(&mut realm, 2.0));
        let red = red_to(camp);
        // The Hub's townsfolk (`FactionTemplate.dbc` 35) are friendly to every race.
        let reds: Vec<_> = world
            .values()
            .filter(|u| u.level >= red && u.faction != 35)
            .collect();
        assert!(
            reds.is_empty(),
            "{}: level {red}+ mobs in view: {reds:?}",
            camp["camp_id"]
        );
    }
}

#[test]
fn the_way_from_the_crossroads_to_a_low_camp_passes_no_red_mob() {
    let low = json("crossroads_low_camps.json");
    for camp in low["camps"].as_array().unwrap() {
        let (mut realm, _) = warrior_at(1);
        let mut world = HashMap::new();
        let from = [CROSSROADS[0], CROSSROADS[1], 92.0];
        let to = camp_center(camp);
        let red = red_to(camp);
        let steps = (distance2(from, to) / 8.0).ceil() as usize;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let at = [
                from[0] + (to[0] - from[0]) * t,
                from[1] + (to[1] - from[1]) * t,
                92.0,
            ];
            realm.handle(RealmCommand::Move {
                motion: Default::default(),
                position: at,
                orientation: 0.0,
            });
            see(&mut world, &run(&mut realm, 1.0));
            // The Hub's townsfolk (`FactionTemplate.dbc` 35) are friendly to every race.
            for u in world.values().filter(|u| u.level >= red && u.faction != 35) {
                let d = distance2(u.position, at);
                assert!(
                    d >= ROUTE_CLEAR_YD,
                    "{}: a level {} {} stands {d} yd from the way",
                    camp["camp_id"],
                    u.level,
                    u.entry
                );
            }
        }
    }
}

#[test]
fn a_mob_notices_a_player_by_level_difference_and_brings_its_camp() {
    // The Prairie Wolves (5–6, hostile, 18 yd detection), south-east of the Crossroads.
    let (mut realm, me) = warrior_at(1);
    let wolves = camp_units(&mut realm, "low-6");
    let wolf = northernmost(&wolves);
    // vmangos's radius: the detection range, plus a yard per level the wolf is above the player.
    let r = 18.0 + (wolf.1.level as f32 - 1.0);

    let events = teleport(&mut realm, north_of(wolf.1.position, r + 1.0));
    let events = [events, run(&mut realm, 1.0)].concat();
    assert_eq!(
        attackers(&events).count(),
        0,
        "out of reach at {} yd",
        r + 1.0
    );

    let events = teleport(&mut realm, north_of(wolf.1.position, r - 1.0));
    let pulled: HashSet<u64> = attackers(&events).collect();
    assert!(pulled.contains(&wolf.0), "noticed at {} yd", r - 1.0);
    assert_eq!(
        pulled,
        HashSet::from([wolf.0]),
        "the campmates wait to hear"
    );
    // Within three seconds the campmates in earshot join, and only they.
    let later: HashSet<u64> = attackers(&run(&mut realm, 3.0)).collect();
    let near: HashSet<u64> = wolves
        .iter()
        .filter(|(_, u)| distance2(u.position, wolf.1.position) <= 8.0)
        .map(|(g, _)| *g)
        .collect();
    assert_eq!(later, &near - &pulled, "the campmates within 8 yd come");
    assert!(events.iter().all(|e| !matches!(
        e,
        SessionEvent::AttackStart { victim, .. } if *victim != me
    )));

    // A level-10 player is noticed only much closer: 18 − (10 − level) yd.
    let (mut realm, _) = warrior_at(10);
    let wolves = camp_units(&mut realm, "low-6");
    let wolf = northernmost(&wolves);
    let events = teleport(&mut realm, north_of(wolf.1.position, r - 1.0));
    let events = [events, run(&mut realm, 1.0)].concat();
    assert_eq!(attackers(&events).count(), 0);
    let r10 = 18.0 - (10.0 - wolf.1.level as f32);
    let events = teleport(&mut realm, north_of(wolf.1.position, r10 - 0.5));
    assert!(attackers(&events).any(|a| a == wolf.0));
}

#[test]
fn a_neutral_mob_waits_to_be_attacked_and_leaves_its_campmates_idle() {
    // The Plainstriders (1–2, neutral).
    let (mut realm, me) = warrior_at(1);
    let striders = camp_units(&mut realm, "low-1");
    let (guid, strider) = northernmost(&striders);
    let at = north_of(strider.position, 2.0);
    let events = teleport(&mut realm, at);
    let events = [events, run(&mut realm, 2.0)].concat();
    assert_eq!(
        attackers(&events).count(),
        0,
        "neutral creatures leave you be"
    );

    face(&mut realm, at, strider.position);
    realm.handle(RealmCommand::AttackSwing { guid });
    let events = run(&mut realm, 0.5);
    let pulled: HashSet<u64> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::AttackStart { attacker, victim } if *victim == me => Some(*attacker),
            _ => None,
        })
        .collect();
    assert!(striders.len() > 1, "a camp of several");
    assert_eq!(
        pulled,
        HashSet::from([guid]),
        "only the strider attacked fights; its campmates graze on"
    );
}

#[test]
fn a_neutral_mob_can_be_attacked_and_killed() {
    // A Plainstrider (1-2) under the neutral beast faction the client shows yellow.
    const NEUTRAL_BEAST: u32 = 189;
    let (mut realm, me) = warrior_at(5);
    let striders = camp_units(&mut realm, "low-1");
    let (guid, strider) = northernmost(&striders);
    assert_eq!(strider.faction, NEUTRAL_BEAST);
    let at = north_of(strider.position, 2.0);
    teleport(&mut realm, at);
    face(&mut realm, at, strider.position);

    realm.handle(RealmCommand::AttackSwing { guid });
    let events = run(&mut realm, 60.0);
    assert!(
        events.iter().any(
            |e| matches!(e, SessionEvent::AttackStart { attacker, victim } if *attacker == me && *victim == guid)
        ),
        "the realm takes the attack"
    );
    assert!(
        attackers(&events).any(|a| a == guid),
        "the strider fights back"
    );
    assert_eq!(last_health(&events, guid), Some(0), "the strider dies");
}

#[test]
fn a_mob_dragged_past_its_leash_runs_home_and_heals() {
    let (mut realm, me) = warrior_at(10);
    let wolves = camp_units(&mut realm, "low-6");
    let (guid, wolf) = northernmost(&wolves);
    let home = wolf.position;
    let at = north_of(home, 2.0);
    teleport(&mut realm, at);
    face(&mut realm, at, home);
    realm.handle(RealmCommand::AttackSwing { guid });
    let mut events = run(&mut realm, 3.0);
    let hurt = last_health(&events, guid).expect("the wolf took a hit");
    assert!(hurt > 0 && hurt < wolf.max_health);
    realm.handle(RealmCommand::AttackStop);

    // Run north faster than it can follow, 20 yd/s, to 90 yd from its home.
    let mut d = 2.0;
    while d < 90.0 {
        d += 2.0;
        realm.handle(RealmCommand::Move {
            motion: Default::default(),
            position: north_of(home, d),
            orientation: 0.0,
        });
        realm.tick(0.1, &NoGround);
        events.extend(drain(&mut realm));
    }
    events.extend(run(&mut realm, 15.0));

    let stop = events
        .iter()
        .position(|e| matches!(e, SessionEvent::AttackStop { attacker, victim } if *attacker == guid && *victim == me))
        .expect("the wolf gives up the fight");
    let (start, end) = events[stop..]
        .iter()
        .find_map(|e| match e {
            SessionEvent::MonsterMove { guid: g, path, .. } if *g == guid => {
                Some((path[0], *path.last().unwrap()))
            }
            _ => None,
        })
        .expect("and runs");
    let strayed = distance2(start, home);
    assert!((45.0..60.0).contains(&strayed), "leashed {strayed} yd out");
    assert!(distance2(end, home) < 0.1, "home: {end:?}");
    assert_eq!(last_health(&events, guid), Some(wolf.max_health));
    assert!(
        !attackers(&events[stop..]).any(|a| a == guid),
        "and it stays reset"
    );
}

#[test]
fn a_killed_mob_respawns_a_minute_after_the_player_leaves() {
    let (mut realm, _) = warrior_at(20);
    // The lone Vile Familiar in front of the Crossroads inn, seen again after a trip away.
    let hub = [CROSSROADS[0], CROSSROADS[1], 95.9];
    teleport(&mut realm, north_of(hub, 500.0));
    let units = creates(&teleport(&mut realm, hub));
    let (&guid, imp) = units
        .iter()
        .find(|(_, u)| u.entry == 3101)
        .expect("the familiar");
    let at = north_of(imp.position, 2.0);
    teleport(&mut realm, at);
    face(&mut realm, at, imp.position);
    realm.handle(RealmCommand::AttackSwing { guid });
    let mut dead = false;
    for _ in 0..300 {
        if last_health(&run(&mut realm, 0.1), guid) == Some(0) {
            dead = true;
            break;
        }
    }
    assert!(dead, "the familiar dies");

    // Standing over the corpse, it stays dead.
    assert!(!creates(&run(&mut realm, 120.0)).contains_key(&guid));

    // 70 yd off, still in view: half a minute, back, and off again; the clock starts over.
    teleport(&mut realm, north_of(imp.position, 70.0));
    assert!(!creates(&run(&mut realm, 30.0)).contains_key(&guid));
    teleport(&mut realm, at);
    teleport(&mut realm, north_of(imp.position, 70.0));
    assert!(!creates(&run(&mut realm, 59.0)).contains_key(&guid));
    let events = run(&mut realm, 2.0);
    let destroyed = events
        .iter()
        .position(|e| matches!(e, SessionEvent::ObjectDestroyed(g) if *g == guid))
        .expect("the corpse goes");
    let fresh = creates(&events[destroyed..]);
    let fresh = fresh.get(&guid).expect("and the familiar is back");
    assert_eq!(fresh.health, fresh.max_health);
    assert!(distance2(fresh.position, imp.position) < 0.1);
}

// ── Helpers ───────────────────────────────────────────────────────────────────────────────────

const SEED: u64 = 7;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn json(name: &str) -> Value {
    let path = repo().join("data/twilights-eve").join(name);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// A spawn out of a research dump.
struct Spawn {
    guid: u64,
    position: [f32; 3],
    displays: Vec<u32>,
}

/// The creature guid the realm gives a spawn: `HIGHGUID_UNIT`, the entry, the spawn's low guid.
fn unit_guid(entry: u64, low: u64) -> u64 {
    0xF130 << 48 | entry << 24 | low
}

/// The first imported spawn of `entry` in `barrens_spawns.json` that the realm's rings keep.
fn imported_spawn(entry: u64) -> Spawn {
    let dump = json("barrens_spawns.json");
    let kept: toml::Value = toml::from_str(include_str!("../content/barrens.toml")).unwrap();
    let kept: HashSet<i64> = kept["spawns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r[0].as_integer().unwrap())
        .collect();
    let s = dump["spawns"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["entry"].as_u64() == Some(entry) && kept.contains(&s["guid"].as_i64().unwrap()))
        .expect("a kept imported spawn of the entry");
    let displays = dump["templates"][entry.to_string()]["displays"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["display_id"].as_u64().unwrap() as u32)
        .collect();
    Spawn {
        guid: unit_guid(entry, s["guid"].as_u64().unwrap()),
        position: ["x", "y", "z"].map(|k| s[k].as_f64().unwrap() as f32),
        displays,
    }
}

/// A human warrior of `level` in the world at the Crossroads, on flat ground at the dump's
/// heights (the host has no ground there).
fn warrior_at(level: u8) -> (Realm, u64) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"));
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &NoGround);
    drain(&mut realm);
    (realm, guid)
}

/// A host with no ground loaded anywhere: the realm keeps each spawn's own height.
struct NoGround;

impl Host for NoGround {
    fn ground_height(&self, _map: u32, _x: f32, _y: f32, _z_hint: f32) -> Option<f32> {
        None
    }
    fn line_of_sight(&self, _map: u32, _from: [f32; 3], _to: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _map: u32, _x: f32, _y: f32) -> Option<u32> {
        None
    }
}

/// Moves the player to `at` (facing north) and ticks once.
fn teleport(realm: &mut Realm, at: [f32; 3]) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: at,
        orientation: 0.0,
    });
    run(realm, 0.0)
}

/// Ticks `secs` in 0.1 s steps (the first tick at once), collecting the events.
fn run(realm: &mut Realm, secs: f32) -> Vec<SessionEvent> {
    let mut events = Vec::new();
    realm.tick(0.0, &NoGround);
    events.extend(drain(realm));
    let mut t = 0.0;
    while t < secs - 1e-3 {
        realm.tick(0.1, &NoGround);
        events.extend(drain(realm));
        t += 0.1;
    }
    events
}

/// A unit's create, as the client sees it.
#[derive(Debug, Clone)]
struct Unit {
    entry: u32,
    level: u32,
    display_id: u32,
    health: u32,
    max_health: u32,
    faction: u32,
    position: [f32; 3],
}

/// Every unit created in `events`, by guid.
fn creates(events: &[SessionEvent]) -> HashMap<u64, Unit> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                fields,
                ..
            } => Some((
                *guid,
                Unit {
                    entry: fields.object_entry().unwrap(),
                    level: fields.unit_level().unwrap(),
                    display_id: fields.unit_displayid().unwrap() as u32,
                    health: fields.unit_health().unwrap(),
                    max_health: fields.unit_max_health().unwrap(),
                    faction: fields.unit_faction_template().unwrap(),
                    position: *position,
                },
            )),
            _ => None,
        })
        .collect()
}

/// Keeps `world` to the units the client holds after `events`: created in, removed out.
fn see(world: &mut HashMap<u64, Unit>, events: &[SessionEvent]) {
    for e in events {
        match e {
            SessionEvent::ObjectsRemoved(guids) => {
                for g in guids {
                    world.remove(g);
                }
            }
            SessionEvent::ObjectDestroyed(g) => {
                world.remove(g);
            }
            _ => world.extend(creates(std::slice::from_ref(e))),
        }
    }
}

/// A low camp's centre, at the dump's ground height.
fn camp_center(camp: &Value) -> [f32; 3] {
    [
        camp["center"][0].as_f64().unwrap() as f32,
        camp["center"][1].as_f64().unwrap() as f32,
        92.0,
    ]
}

/// The lowest level that is red to a player at the top of a low camp's band.
fn red_to(camp: &Value) -> u32 {
    camp["level_max"].as_u64().unwrap() as u32 + RED_ABOVE
}

/// The members of a low camp (`crossroads_low_camps.json`'s `camp_id`) as the realm streams them
/// in, seen from 60 yd south of its centre.
fn camp_units(realm: &mut Realm, camp_id: &str) -> HashMap<u64, Unit> {
    let low = json("crossroads_low_camps.json");
    let camp = low["camps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["camp_id"] == camp_id)
        .expect("the camp");
    let guids: HashSet<u64> = camp["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| unit_guid(m["entry"].as_u64().unwrap(), m["guid"].as_u64().unwrap()))
        .collect();
    let center = [
        camp["center"][0].as_f64().unwrap() as f32 - 60.0,
        camp["center"][1].as_f64().unwrap() as f32,
        92.0,
    ];
    let mut units = creates(&teleport(realm, center));
    units.retain(|g, _| guids.contains(g));
    assert_eq!(units.len(), guids.len());
    units
}

/// The unit furthest north: nothing else in its camp is nearer a point north of it.
fn northernmost(units: &HashMap<u64, Unit>) -> (u64, Unit) {
    units
        .iter()
        .max_by(|a, b| a.1.position[0].total_cmp(&b.1.position[0]))
        .map(|(g, u)| (*g, u.clone()))
        .unwrap()
}

/// The point `yards` north (+x) of `p`.
fn north_of(p: [f32; 3], yards: f32) -> [f32; 3] {
    [p[0] + yards, p[1], p[2]]
}

/// Turns the player standing at `at` toward `target`.
fn face(realm: &mut Realm, at: [f32; 3], target: [f32; 3]) {
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: at,
        orientation: (target[1] - at[1]).atan2(target[0] - at[0]),
    });
}

fn last_health(events: &[SessionEvent], guid: u64) -> Option<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectValues { guid: g, fields } if *g == guid => fields.unit_health(),
            _ => None,
        })
        .next_back()
}

/// Every guid that left the world in `events`.
fn removed(events: &[SessionEvent]) -> HashSet<u64> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectsRemoved(guids) => Some(guids.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// The mobs that started attacking someone in `events`.
fn attackers(events: &[SessionEvent]) -> impl Iterator<Item = u64> + '_ {
    events.iter().filter_map(|e| match e {
        SessionEvent::AttackStart { attacker, .. } if attacker >> 48 == 0xF130 => Some(*attacker),
        _ => None,
    })
}

fn creature_name(realm: &mut Realm, entry: u32) -> Option<String> {
    drain(realm).into_iter().find_map(|e| match e {
        SessionEvent::CreatureName { entry: e, name, .. } if e == entry => name,
        _ => None,
    })
}

fn distance2(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}
