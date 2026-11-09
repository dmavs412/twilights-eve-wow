//! Raven Hill Deeps (#645, spec #637): the crypt under Raven Hill Cemetery, three encounters along
//! its first half. Commander Felstrom in the upper vaults, the Dig Overseer and his ghouls at the
//! dig face with the shoring lever that stops them, and the Grey Deacon in the grey pocket of
//! vault06, whose Stilled Dead wake at his 75%, 50% and 25% marks. The mist wall `raven_hill_grey`
//! closes the rest of the crypt.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

/// The templates (`content/raven_hill_deeps.toml`): the Dig Overseer, the Grey Deacon and the
/// Stilled Dead.
const OVERSEER: u32 = 97264;
const DEACON: u32 = 97265;
const STILLED: u32 = 97266;

/// The Phase Tester, which the dungeon no longer holds.
const PHASE_TESTER: u32 = 97100;

/// The Deacon's creature row, the shoring lever and the fallen side tunnel.
const DEACON_GUID: i64 = 974213;
const LEVER: i64 = 983051;
const FALLEN_TUNNEL: i64 = 983052;

/// The Stilled Dead that wake at the Deacon's 75% mark.
const AT_75: [u64; 2] = [974214, 974215];

/// In vault01, by the Dig Overseer: the first stop on the way down, so the long step from Elwynn
/// does not cut through the mist wall.
const IN_VAULT01: [f32; 3] = [-10265.0, 385.0, 25.25];
/// Beside the Deacon in vault06, in view of the Stilled Dead.
const BY_THE_DEACON: [f32; 3] = [-10205.0, 316.0, 20.87];
/// On vault06's floor, on the wall's open side, short of it, and a step through it.
const OPEN_SIDE: [f32; 3] = [-10185.0, 312.0, 20.87];
const SHORT_OF_THE_WALL: [f32; 3] = [-10185.0, 307.0, 20.87];
const PAST_THE_WALL: [f32; 3] = [-10185.0, 300.0, 20.87];
const REFUSED: &str = "You cannot go that way.";

/// `UNIT_FIELD_FLAGS` and `UNIT_FLAG_NOT_SELECTABLE`.
const UNIT_FIELD_FLAGS: u16 = 46;
const NOT_SELECTABLE: u32 = 0x0200_0000;

fn deeps() -> toml::Table {
    include_str!("../content/raven_hill_deeps.toml")
        .parse()
        .unwrap()
}

fn rows<'a>(file: &'a toml::Table, kind: &str) -> &'a [toml::Value] {
    file.get(kind)
        .and_then(|v| v.as_array())
        .map_or(&[][..], |a| a.as_slice())
}

fn num(x: &toml::Value) -> f32 {
    x.as_float().or(x.as_integer().map(|i| i as f64)).unwrap() as f32
}

fn point(v: &toml::Value) -> [f32; 3] {
    let a = v.as_array().unwrap();
    [num(&a[0]), num(&a[1]), num(&a[2])]
}

fn pair(v: &toml::Value) -> [f32; 2] {
    let a = v.as_array().unwrap();
    [num(&a[0]), num(&a[1])]
}

fn int(v: &toml::Value, key: &str) -> Option<i64> {
    v.get(key).and_then(|x| x.as_integer())
}

fn entry(c: &toml::Value) -> u32 {
    int(c, "entry").unwrap() as u32
}

fn flat(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// Whether a footprint box holds the world point `p`, in the building's frame (`origin`, `yaw`).
fn holds(file: &toml::Table, p: [f32; 3]) -> bool {
    let [ox, oy, oz] = point(&file["origin"]);
    let (sin, cos) = num(&file["yaw"]).to_radians().sin_cos();
    let (dx, dy) = (p[0] - ox, p[1] - oy);
    let local = [dx * cos + dy * sin, dy * cos - dx * sin, p[2] - oz];
    rows(file, "footprint").iter().any(|b| {
        ["x", "y", "z"].iter().zip(local).all(|(k, v)| {
            let [lo, hi] = pair(&b[*k]);
            (lo..=hi).contains(&v)
        })
    })
}

/// The mist wall `raven_hill_grey` from `data/twilights-eve/mist_walls.json`.
fn grey_wall() -> serde_json::Value {
    let file: serde_json::Value =
        serde_json::from_str(include_str!("../../../data/twilights-eve/mist_walls.json")).unwrap();
    file["walls"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["key"] == "raven_hill_grey")
        .cloned()
        .expect("no raven_hill_grey wall")
}

fn json_pair(v: &serde_json::Value) -> [f32; 2] {
    [v[0].as_f64().unwrap() as f32, v[1].as_f64().unwrap() as f32]
}

/// Which side of the wall's line `p` is on: positive on one side, negative on the other.
fn side_of(wall: &serde_json::Value, p: [f32; 2]) -> f32 {
    let (a, b) = (json_pair(&wall["from"]), json_pair(&wall["to"]));
    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
}

fn deacon_at(file: &toml::Table) -> [f32; 3] {
    rows(file, "creature")
        .iter()
        .find(|c| entry(c) == DEACON)
        .map(|c| point(&c["position"]))
        .expect("the Deacon")
}

#[test]
fn everything_stands_in_the_footprint_and_the_phase_tester_is_gone() {
    let file = deeps();
    let creatures = rows(&file, "creature");
    assert_eq!(creatures.len(), 19, "19 creatures");
    for c in creatures {
        let guid = int(c, "guid").unwrap();
        assert!((974201..=974219).contains(&guid), "guid {guid}");
        assert_ne!(
            entry(c),
            PHASE_TESTER,
            "creature {guid} is the Phase Tester"
        );
        let at = point(&c["position"]);
        assert!(holds(&file, at), "creature {guid} at {at:?}");
        for call in rows(c.as_table().unwrap(), "every") {
            let at = point(&call["position"]);
            assert!(holds(&file, at), "creature {guid}'s call at {at:?}");
        }
    }
    let objects = rows(&file, "gameobject");
    assert_eq!(
        objects.len(),
        5,
        "the lever, the fallen tunnel and three rubble heaps"
    );
    for o in objects {
        let at = point(&o["position"]);
        assert!(holds(&file, at), "GameObject {:?} at {at:?}", o["guid"]);
    }
    for p in rows(&file, "cover") {
        let at = point(&p["position"]);
        assert!(holds(&file, at), "cover at {at:?}");
    }
}

#[test]
fn the_deacon_alone_raises_the_boss_bar_and_the_lever_stops_the_overseer_s_ghouls() {
    let file = deeps();
    let creatures = rows(&file, "creature");
    let bosses: Vec<u32> = creatures
        .iter()
        .map(entry)
        .filter(|&e| mob_template(e).get("boss").and_then(|b| b.as_bool()) == Some(true))
        .collect();
    assert_eq!(bosses, [DEACON]);

    let overseers: Vec<&toml::Value> = creatures.iter().filter(|c| entry(c) == OVERSEER).collect();
    assert_eq!(overseers.len(), 1);
    let calls = rows(overseers[0].as_table().unwrap(), "every");
    assert_eq!(calls.len(), 1);
    assert_eq!(int(&calls[0], "until_used"), Some(LEVER));

    let object = |guid: i64| {
        rows(&file, "gameobject")
            .iter()
            .find(|o| int(o, "guid") == Some(guid))
            .unwrap_or_else(|| panic!("no GameObject {guid}"))
    };
    assert_eq!(
        object(LEVER).get("use").and_then(|u| u.as_str()),
        Some("lever")
    );
    assert_eq!(int(object(FALLEN_TUNNEL), "after_used"), Some(LEVER));
}

#[test]
fn the_six_stilled_dead_wait_on_the_deacon_two_at_each_mark() {
    let file = deeps();
    let mut marks: Vec<i64> = rows(&file, "creature")
        .iter()
        .filter(|c| entry(c) == STILLED)
        .map(|c| {
            let frozen = c.get("frozen_until").expect("a Stilled Dead not frozen");
            assert_eq!(int(frozen, "guid"), Some(DEACON_GUID), "{c:?}");
            int(frozen, "pct").unwrap()
        })
        .collect();
    marks.sort_unstable();
    assert_eq!(marks, [25, 25, 50, 50, 75, 75]);
}

#[test]
fn the_three_cover_pillars_stand_near_the_deacon_on_his_side_of_the_mist_wall() {
    let file = deeps();
    let deacon = deacon_at(&file);
    let wall = grey_wall();
    let his_side = side_of(&wall, [deacon[0], deacon[1]]).signum();
    let pillars = rows(&file, "cover");
    assert_eq!(pillars.len(), 3);
    for p in pillars {
        let at = point(&p["position"]);
        assert!(flat(at, deacon) <= 25.0, "{at:?} is far from the Deacon");
        assert_eq!(
            side_of(&wall, [at[0], at[1]]).signum(),
            his_side,
            "{at:?} is past the wall"
        );
        // A rubble heap shows each pillar.
        let shown = rows(&file, "gameobject")
            .iter()
            .any(|o| point(&o["position"]) == at);
        assert!(shown, "no GameObject at {at:?}");
    }
}

/// A level-60 character in vault01, come from Elwynn as the Ward Test's.
fn party() -> Party {
    let mut p = Party::edited(
        Realm::with_seed(1),
        &[60],
        START_ELWYNN,
        Box::new(NoGround),
        |_, blob| blob,
    );
    p.run(1.0);
    p.go(0, IN_VAULT01, 0.0);
    p.run(0.5);
    p
}

/// The notifications saying `said` among `events`.
fn notes(events: &[SessionEvent], said: &str) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, SessionEvent::Notification { text } if text == said))
        .count()
}

/// Walks to `at` and lets the realm tick; the events that came.
fn walk(p: &mut Party, at: [f32; 3]) -> Vec<SessionEvent> {
    let from = p.seen(0).len();
    p.go(0, at, 0.0);
    p.run(0.5);
    p.seen(0)[from..].to_vec()
}

#[test]
fn the_mist_wall_loads_its_open_side_is_the_dungeon_s_and_a_step_through_it_is_refused() {
    let file = deeps();
    let wall = grey_wall();
    assert_eq!(wall["map"], 0);
    let [x, y] = json_pair(&wall["open_side"]);
    let floor = wall["floor"].as_f64().unwrap() as f32;
    assert!(
        holds(&file, [x, y, floor]),
        "the open side is outside the footprint"
    );
    let deacon = deacon_at(&file);
    assert_eq!(
        side_of(&wall, [x, y]).signum(),
        side_of(&wall, [deacon[0], deacon[1]]).signum(),
        "the Deacon stands past the wall"
    );

    let mut p = party();
    let events = walk(&mut p, OPEN_SIDE);
    assert_eq!(notes(&events, REFUSED), 0, "{events:?}");
    let events = walk(&mut p, SHORT_OF_THE_WALL);
    assert_eq!(notes(&events, REFUSED), 0, "a step short of the wall");
    let events = walk(&mut p, PAST_THE_WALL);
    assert_eq!(notes(&events, REFUSED), 1, "{events:?}");
}

fn unit(entry: u32, low: u64) -> u64 {
    0xF130 << 48 | u64::from(entry) << 24 | low
}

/// Whether player 0's client was last shown `guid` as not selectable: by its create, or by a
/// later change of its flags.
fn frozen(p: &Party, guid: u64) -> bool {
    let mut frozen = false;
    for e in p.seen(0) {
        match e {
            SessionEvent::ObjectCreate {
                guid: g, fields, ..
            } if *g == guid => frozen = fields.unit_flags() & NOT_SELECTABLE != 0,
            SessionEvent::ObjectValues { guid: g, fields } if *g == guid => {
                if let Some((_, v)) = fields.raw_fields().find(|(f, _)| *f == UNIT_FIELD_FLAGS) {
                    frozen = v & NOT_SELECTABLE != 0;
                }
            }
            _ => {}
        }
    }
    frozen
}

#[test]
fn in_a_run_the_deacon_at_70_percent_wakes_exactly_two_stilled_dead() {
    let file = deeps();
    let stilled: Vec<u64> = rows(&file, "creature")
        .iter()
        .filter(|c| entry(c) == STILLED)
        .map(|c| int(c, "guid").unwrap() as u64)
        .collect();
    let mut p = party();
    p.go_facing(0, BY_THE_DEACON, deacon_at(&file));
    p.run(0.5);
    for &g in &stilled {
        assert!(
            p.units_held(0).contains(&unit(STILLED, g)),
            "{g} not in view"
        );
        assert!(frozen(&p, unit(STILLED, g)), "{g} starts awake");
    }
    p.realm
        .set_creature_health_pct(unit(DEACON, DEACON_GUID as u64), 70);
    p.run(0.5);
    let awake: Vec<u64> = stilled
        .iter()
        .copied()
        .filter(|&g| !frozen(&p, unit(STILLED, g)))
        .collect();
    assert_eq!(awake, AT_75);
}
