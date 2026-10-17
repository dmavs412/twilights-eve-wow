//! The inner ring (levels 1-10, within 600 yd of the Crossroads) against its two givers' quests:
//! every Camp type there is a Hunting Quest target, every quest's targets lie in its giver's
//! Hunting Ground, each Hunting Ground has one find quest, no spawn there is above 10, and the
//! quests carry a character from 1 to 10.

use std::collections::{HashMap, HashSet};

const CROSSROADS: [f64; 2] = [-441.0, -2596.0];
const INNER_RADIUS: f64 = 600.0;

/// A Hunting Ground: its name, its giver, and the band of distance from the Crossroads it holds.
struct Ground {
    name: &'static str,
    giver: u32,
    from: f64,
    to: f64,
}

const GROUNDS: [Ground; 2] = [
    Ground {
        name: "The Near Plains",
        giver: 90101,
        from: 0.0,
        to: 380.0,
    },
    Ground {
        name: "The Far Fields",
        giver: 90102,
        from: 380.0,
        to: INNER_RADIUS,
    },
];

fn toml_file(text: &str) -> toml::Value {
    toml::from_str(text).expect("content parses")
}

fn int(v: &toml::Value, k: &str) -> i64 {
    v.get(k).and_then(|x| x.as_integer()).unwrap_or(0)
}

fn from_crossroads(p: [f64; 2]) -> f64 {
    ((p[0] - CROSSROADS[0]).powi(2) + (p[1] - CROSSROADS[1]).powi(2)).sqrt()
}

/// Every creature a quest asks to be killed or drops for it.
fn targets(q: &toml::Value) -> Vec<u32> {
    let mut t = Vec::new();
    for k in q
        .get("kill")
        .and_then(|k| k.as_array())
        .into_iter()
        .flatten()
    {
        t.push(k[0].as_integer().unwrap() as u32);
    }
    for c in q
        .get("collect")
        .and_then(|c| c.as_array())
        .into_iter()
        .flatten()
    {
        for f in c["from"].as_array().unwrap() {
            t.push(f.as_integer().unwrap() as u32);
        }
    }
    t
}

/// Every Barrens spawn: entry -> (x, y, level_max).
fn spawns() -> HashMap<u32, Vec<([f64; 2], i64)>> {
    let b = toml_file(include_str!("../content/barrens.toml"));
    let mut spawns: HashMap<u32, Vec<([f64; 2], i64)>> = HashMap::new();
    for s in b["spawns"].as_array().unwrap() {
        let f = |i: usize| s[i].as_float().unwrap();
        spawns
            .entry(s[1].as_integer().unwrap() as u32)
            .or_default()
            .push(([f(4), f(5)], s[3].as_integer().unwrap()));
    }
    spawns
}

fn inner_quests() -> Vec<toml::Value> {
    let givers: HashSet<u32> = GROUNDS.iter().map(|g| g.giver).collect();
    toml_file(include_str!("../content/quests.toml"))["quest"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|q| givers.contains(&(int(q, "giver") as u32)))
        .cloned()
        .collect()
}

/// The creature types with a spawn in the inner ring.
fn inner_ring() -> Vec<u32> {
    let mut entries: Vec<u32> = spawns()
        .into_iter()
        .filter(|(_, at)| at.iter().any(|(p, _)| from_crossroads(*p) <= INNER_RADIUS))
        .map(|(e, _)| e)
        .collect();
    entries.sort();
    entries
}

#[test]
fn no_inner_ring_spawn_is_above_level_ten() {
    for (e, at) in spawns() {
        for (p, level_max) in at {
            if from_crossroads(p) <= INNER_RADIUS {
                assert!(level_max <= 10, "{e} at {p:?} is level {level_max}");
            }
        }
    }
}

#[test]
fn every_inner_ring_camp_type_is_a_hunting_quest_target() {
    let hunted: HashSet<u32> = inner_quests().iter().flat_map(targets).collect();
    let inner = inner_ring();
    assert!(inner.len() >= 15, "the inner ring: {inner:?}");
    for e in inner {
        assert!(
            hunted.contains(&e),
            "{e} is no inner Hunting Quest's target"
        );
    }
}

#[test]
fn every_inner_quests_targets_lie_in_its_givers_hunting_ground() {
    let spawns = spawns();
    let quests = inner_quests();
    let inside = |g: &Ground, p: [f64; 2]| (g.from..=g.to).contains(&from_crossroads(p));
    for g in &GROUNDS {
        let mine: Vec<&toml::Value> = quests
            .iter()
            .filter(|q| int(q, "giver") as u32 == g.giver)
            .collect();
        assert!(mine.len() >= 5, "{}: {} quests", g.name, mine.len());
        for q in mine {
            let id = int(q, "id");
            for e in targets(q) {
                for (p, _) in &spawns[&e] {
                    assert!(
                        inside(g, *p),
                        "quest {id}: {e} spawns at {p:?}, outside {}",
                        g.name
                    );
                }
            }
            if let Some(x) = q.get("explore") {
                let at = x["position"].as_array().unwrap();
                let p = [at[0].as_float().unwrap(), at[1].as_float().unwrap()];
                assert!(inside(g, p), "quest {id}: its spot lies outside {}", g.name);
            }
        }
    }
}

#[test]
fn each_inner_hunting_ground_has_one_find_quest() {
    let quests = inner_quests();
    for g in &GROUNDS {
        let finds = quests
            .iter()
            .filter(|q| int(q, "giver") as u32 == g.giver)
            .filter(|q| q.get("explore").is_some() || q.get("ender").is_some())
            .count();
        assert_eq!(finds, 1, "{}", g.name);
    }
}

#[test]
fn the_inner_rings_hunting_quests_carry_a_character_from_one_to_ten() {
    let quests = inner_quests();
    for level in 1..=10 {
        let offered = quests
            .iter()
            .filter(|q| q["kind"].as_str() == Some("hunting"))
            .filter(|q| {
                let l = int(q, "level");
                let min = q
                    .get("min_level")
                    .map_or((l - 4).max(1), |m| m.as_integer().unwrap());
                min <= level && l >= level
            })
            .count();
        assert!(
            offered >= 2,
            "at level {level} the inner ring offers {offered} quests"
        );
    }
}
