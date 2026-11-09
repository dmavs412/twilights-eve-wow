//! Mistmantle Manor (#641, spec #637): one open-air box over the farm, Forest's Edge's form, and
//! three plain fights in it. The yard's farmhands and the steward at the front door, the lady and
//! her servants in the walled garden, and Stalvan in the house behind two stacks of crates.

mod common;

use std::collections::BTreeMap;

use common::*;

/// The templates (`content/mistmantle_manor.toml`): the Risen Farmhand, the Silent Servant,
/// Steward Aldric, Lady Corrine Mistmantle and Stalvan Mistmantle.
const FARMHAND: u32 = 97225;
const SERVANT: u32 = 97226;
const STEWARD: u32 = 97227;
const LADY: u32 = 97228;
const STALVAN: u32 = 97229;

/// The Phase Tester, which the dungeon no longer holds.
const PHASE_TESTER: u32 = 97100;

/// The house's front door.
const DOOR: [f32; 2] = [-10362.6, -1257.0];

fn manor() -> toml::Table {
    include_str!("../content/mistmantle_manor.toml")
        .parse()
        .unwrap()
}

fn rows<'a>(file: &'a toml::Table, kind: &str) -> &'a [toml::Value] {
    file.get(kind)
        .and_then(|v| v.as_array())
        .map_or(&[][..], |a| a.as_slice())
}

fn point(v: &toml::Value) -> [f32; 3] {
    let a = v.as_array().unwrap();
    let f = |x: &toml::Value| x.as_float().or(x.as_integer().map(|i| i as f64)).unwrap() as f32;
    [f(&a[0]), f(&a[1]), f(&a[2])]
}

fn int(v: &toml::Value, key: &str) -> Option<i64> {
    v.get(key).and_then(|x| x.as_integer())
}

fn entry(c: &toml::Value) -> u32 {
    int(c, "entry").unwrap() as u32
}

/// The one footprint box: `[lo, hi]` on x, y and z.
fn footprint(file: &toml::Table) -> [[f32; 2]; 3] {
    let boxes = rows(file, "footprint");
    assert_eq!(boxes.len(), 1, "one box");
    let pair = |v: &toml::Value| {
        let a = v.as_array().unwrap();
        [
            a[0].as_float().unwrap() as f32,
            a[1].as_float().unwrap() as f32,
        ]
    };
    let b = &boxes[0];
    [pair(&b["x"]), pair(&b["y"]), pair(&b["z"])]
}

/// Whether the box holds `p` on x and y; an outdoor row's Z is ground_check.py's to write.
fn holds(b: &[[f32; 2]; 3], p: [f32; 3]) -> bool {
    (0..2).all(|i| (b[i][0]..=b[i][1]).contains(&p[i]))
}

fn flat(a: [f32; 3], b: [f32; 2]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

#[test]
fn everything_stands_in_the_box_and_the_portal_on_its_edge_behind_a_wall() {
    let file = manor();
    let b = footprint(&file);
    let creatures = rows(&file, "creature");
    assert_eq!(creatures.len(), 14, "14 creatures");
    for c in creatures {
        let guid = int(c, "guid").unwrap();
        assert!((974001..=974014).contains(&guid), "guid {guid}");
        let at = point(&c["position"]);
        assert!(holds(&b, at), "creature {guid} at {at:?}");
        for call in rows(c.as_table().unwrap(), "on_health") {
            let at = point(&call["position"]);
            assert!(holds(&b, at), "creature {guid}'s call at {at:?}");
        }
    }
    let objects = rows(&file, "gameobject");
    assert_eq!(objects.len(), 5, "the ore, two crates and two candles");
    for o in objects {
        let at = point(&o["position"]);
        assert!(holds(&b, at), "GameObject {:?} at {at:?}", o["guid"]);
    }
    let portal = point(&file["portal"]["position"]);
    let edge = [
        portal[0] - b[0][0],
        b[0][1] - portal[0],
        portal[1] - b[1][0],
        b[1][1] - portal[1],
    ];
    let off = edge.iter().copied().fold(f32::MAX, f32::min);
    assert!(holds(&b, portal), "the Portal at {portal:?} is outside");
    assert!(off <= 5.0, "the Portal is {off} yd from the edge");
    assert!(file.contains_key("wall"), "no [wall]");
}

#[test]
fn the_phase_tester_is_gone_and_stalvan_alone_raises_the_boss_bar() {
    let file = manor();
    let creatures = rows(&file, "creature");
    assert!(creatures.iter().all(|c| entry(c) != PHASE_TESTER));
    let bosses: Vec<u32> = creatures
        .iter()
        .map(entry)
        .filter(|&e| mob_template(e).get("boss").and_then(|b| b.as_bool()) == Some(true))
        .collect();
    assert_eq!(bosses, [STALVAN]);
}

#[test]
fn the_steward_calls_two_servants_out_of_the_front_door() {
    let file = manor();
    let stewards: Vec<&toml::Value> = rows(&file, "creature")
        .iter()
        .filter(|c| entry(c) == STEWARD)
        .collect();
    assert_eq!(stewards.len(), 1);
    let calls = rows(stewards[0].as_table().unwrap(), "on_health");
    assert_eq!(calls.len(), 1);
    assert_eq!(int(&calls[0], "entry"), Some(SERVANT.into()));
    assert_eq!(int(&calls[0], "count"), Some(2));
    let at = point(&calls[0]["position"]);
    assert!(flat(at, DOOR) < 0.5, "the call comes in at {at:?}");
}

#[test]
fn each_cover_pillar_stands_near_stalvan_between_him_and_the_door() {
    let file = manor();
    let stalvan = rows(&file, "creature")
        .iter()
        .find(|c| entry(c) == STALVAN)
        .map(|c| point(&c["position"]))
        .expect("Stalvan");
    let pillars = rows(&file, "cover");
    assert_eq!(pillars.len(), 2);
    let (dx, dy) = (DOOR[0] - stalvan[0], DOOR[1] - stalvan[1]);
    for p in pillars {
        let at = point(&p["position"]);
        assert!(flat(at, [stalvan[0], stalvan[1]]) <= 8.0, "{at:?}");
        // Its projection onto the line from Stalvan to the door falls between the two.
        let t = ((at[0] - stalvan[0]) * dx + (at[1] - stalvan[1]) * dy) / (dx * dx + dy * dy);
        assert!(t > 0.0 && t < 1.0, "{at:?} is not between them: {t}");
        // A crate stack shows each pillar.
        let shown = rows(&file, "gameobject")
            .iter()
            .any(|o| point(&o["position"]) == at);
        assert!(shown, "no GameObject at {at:?}");
    }
}

#[test]
fn only_stalvan_has_an_aggro_line() {
    for e in [FARMHAND, SERVANT, STEWARD, LADY, STALVAN] {
        let aggro = mob_template(e).get("say").and_then(|s| s.get("aggro"));
        assert_eq!(aggro.is_some(), e == STALVAN, "{e}: {aggro:?}");
    }
}

#[test]
fn three_camps_of_three_farmhands_and_the_garden_s_three_share_one() {
    let file = manor();
    let mut camps: BTreeMap<i64, Vec<u32>> = BTreeMap::new();
    for c in rows(&file, "creature") {
        if let Some(camp) = int(c, "camp") {
            camps.entry(camp).or_default().push(entry(c));
        }
    }
    let farmhand_camps: Vec<&Vec<u32>> = camps
        .values()
        .filter(|members| members.contains(&FARMHAND))
        .collect();
    assert_eq!(farmhand_camps.len(), 3, "{camps:?}");
    for members in farmhand_camps {
        assert_eq!(members, &[FARMHAND; 3], "{camps:?}");
    }
    let garden: Vec<&Vec<u32>> = camps
        .values()
        .filter(|members| members.contains(&LADY))
        .collect();
    assert_eq!(garden.len(), 1, "{camps:?}");
    let mut garden = garden[0].clone();
    garden.sort_unstable();
    assert_eq!(garden, [SERVANT, SERVANT, LADY]);
    assert_eq!(camps.len(), 4, "{camps:?}");
}
