//! The outer ring (levels 20-30, the southern Barrens) against Camp Taurajo's quests: every Camp
//! type there is a Hunting Quest target, every kept rare has a "Wanted" quest, every quest's
//! targets lie in its giver's Hunting Ground, each Hunting Ground has one find quest, and the
//! quests carry a character from 20 to 30.

use std::collections::{HashMap, HashSet};

/// South of this (WoW x, yards) is the outer ring; Camp Taurajo stands just past it.
const OUTER_EDGE_X: f64 = -2250.0;

/// A Hunting Ground: its name, the givers whose quests it holds, its middle and its radius.
struct Ground {
    name: &'static str,
    givers: &'static [u32],
    middle: [f64; 2],
    radius: f64,
}

const GROUNDS: [Ground; 4] = [
    Ground {
        name: "The Thunderhead Plains",
        givers: &[190042, 190045],
        middle: [-2650.0, -2100.0],
        radius: 500.0,
    },
    Ground {
        name: "The Field of Giants",
        givers: &[190043],
        middle: [-3450.0, -2050.0],
        radius: 600.0,
    },
    Ground {
        name: "Blackthorn Ridge",
        givers: &[90106],
        middle: [-3880.0, -1730.0],
        radius: 350.0,
    },
    Ground {
        name: "The Razorfen Edge",
        givers: &[190044],
        middle: [-4560.0, -2050.0],
        radius: 450.0,
    },
];

fn toml_file(text: &str) -> toml::Value {
    toml::from_str(text).expect("content parses")
}

fn int(v: &toml::Value, k: &str) -> i64 {
    v.get(k).and_then(|x| x.as_integer()).unwrap_or(0)
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

struct Barrens {
    /// entry -> every spawn's (x, y)
    spawns: HashMap<u32, Vec<[f64; 2]>>,
    /// entry -> (name, rank, hostile, level_min)
    templates: HashMap<u32, (String, i64, bool, i64)>,
}

fn barrens() -> Barrens {
    let b = toml_file(include_str!("../content/barrens.toml"));
    let mut spawns: HashMap<u32, Vec<[f64; 2]>> = HashMap::new();
    for s in b["spawns"].as_array().unwrap() {
        let f = |i: usize| s[i].as_float().unwrap();
        spawns
            .entry(s[1].as_integer().unwrap() as u32)
            .or_default()
            .push([f(4), f(5)]);
    }
    let templates = b["template"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                int(t, "entry") as u32,
                (
                    t["name"].as_str().unwrap().to_string(),
                    int(t, "rank"),
                    t.get("hostile").and_then(|h| h.as_bool()).unwrap_or(true),
                    int(t, "level_min"),
                ),
            )
        })
        .collect();
    Barrens { spawns, templates }
}

fn quests() -> Vec<toml::Value> {
    toml_file(include_str!("../content/quests.toml"))["quest"]
        .as_array()
        .unwrap()
        .clone()
}

fn taurajo_quests() -> Vec<toml::Value> {
    let givers: HashSet<u32> = GROUNDS
        .iter()
        .flat_map(|g| g.givers.iter().copied())
        .collect();
    quests()
        .into_iter()
        .filter(|q| givers.contains(&(int(q, "giver") as u32)))
        .collect()
}

/// The creature types of the outer ring: most of their spawns lie south of its edge, at 20 or up.
fn outer_ring(b: &Barrens) -> Vec<u32> {
    let mut entries: Vec<u32> = b
        .spawns
        .iter()
        .filter(|(e, at)| {
            let south = at.iter().filter(|p| p[0] < OUTER_EDGE_X).count();
            south * 2 > at.len() && b.templates[*e].3 >= 20
        })
        .map(|(e, _)| *e)
        .collect();
    entries.sort();
    entries
}

#[test]
fn every_outer_ring_camp_type_is_a_hunting_quest_target_from_taurajo() {
    let b = barrens();
    let hunted: HashSet<u32> = taurajo_quests().iter().flat_map(targets).collect();
    let outer = outer_ring(&b);
    assert!(outer.len() > 15, "the outer ring: {outer:?}");
    for e in outer {
        let (name, _, hostile, _) = &b.templates[&e];
        if !hostile && !matches!(b.templates[&e].1, 2 | 4) {
            continue; // the kodo and zhevra herds graze; nobody pays to hunt them
        }
        assert!(
            hunted.contains(&e),
            "{name} ({e}) is no Taurajo quest's target"
        );
    }
}

#[test]
fn every_kept_rare_in_the_outer_ring_has_a_wanted_quest() {
    let b = barrens();
    let quests = taurajo_quests();
    let rares: Vec<u32> = outer_ring(&b)
        .into_iter()
        .filter(|e| matches!(b.templates[e].1, 2 | 4))
        .collect();
    assert!(!rares.is_empty());
    for e in rares {
        let name = &b.templates[&e].0;
        let title = format!("Wanted: {name}");
        assert!(
            quests
                .iter()
                .any(|q| q["title"].as_str() == Some(&title) && targets(q) == [e]),
            "no \"{title}\""
        );
    }
}

#[test]
fn every_taurajo_quests_targets_lie_in_its_givers_hunting_ground() {
    let b = barrens();
    let quests = taurajo_quests();
    let inside = |g: &Ground, p: [f64; 2]| {
        ((p[0] - g.middle[0]).powi(2) + (p[1] - g.middle[1]).powi(2)).sqrt() <= g.radius
    };
    for g in &GROUNDS {
        let mine: Vec<&toml::Value> = quests
            .iter()
            .filter(|q| g.givers.contains(&(int(q, "giver") as u32)))
            .collect();
        assert!(mine.len() >= 3, "{}: {} quests", g.name, mine.len());
        for q in mine {
            let id = int(q, "id");
            for e in targets(q) {
                for p in &b.spawns[&e] {
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
fn each_outer_ring_hunting_ground_has_one_find_quest() {
    let quests = taurajo_quests();
    for g in &GROUNDS {
        let finds = quests
            .iter()
            .filter(|q| g.givers.contains(&(int(q, "giver") as u32)))
            .filter(|q| q.get("explore").is_some() || q.get("ender").is_some())
            .count();
        assert_eq!(finds, 1, "{}", g.name);
    }
}

#[test]
fn taurajos_hunting_quests_carry_a_character_from_twenty_to_thirty() {
    let quests = taurajo_quests();
    for level in 20..30 {
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
            "at level {level} Taurajo offers {offered} quests"
        );
    }
    assert!(quests.iter().any(|q| int(q, "level") == 30));
}
