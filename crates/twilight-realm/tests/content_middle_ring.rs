//! The middle ring (levels 10-20, the north and central Barrens) against the Crossroads' quests:
//! every Camp type there is a Hunting Quest target, every kept rare has a "Wanted" quest, every
//! quest's targets lie in its giver's Hunting Ground, each Hunting Ground has one find quest, and
//! each quest's level is its targets'.

use std::collections::{HashMap, HashSet};

/// North of this (WoW x, yards) is the middle ring; south of it, the outer ring.
const OUTER_EDGE_X: f64 = -2250.0;
/// The inner ring: within this many yards of the Crossroads.
const INNER_RADIUS: f64 = 600.0;
const CROSSROADS: [f64; 2] = [-440.0, -2650.0];
/// A Camp's spawns sit within this many yards of their middle. The plains' roamers, which spread
/// over the whole Barrens, are no Camp, and no quest hunts them.
const CAMP_SPREAD: f64 = 800.0;

/// A Hunting Ground: its name, the givers whose quests it holds, its middle and its radius.
struct Ground {
    name: &'static str,
    givers: &'static [u32],
    middle: [f64; 2],
    radius: f64,
}

const GROUNDS: [Ground; 8] = [
    Ground {
        name: "The Southfury Banks",
        givers: &[90103],
        middle: [150.0, -3550.0],
        radius: 1000.0,
    },
    Ground {
        name: "The Kolkar Lands",
        givers: &[90104, 190020],
        middle: [-1280.0, -2600.0],
        radius: 650.0,
    },
    Ground {
        name: "Lushwater Oasis",
        givers: &[90107],
        middle: [-900.0, -2100.0],
        radius: 300.0,
    },
    Ground {
        name: "The Sludge Fen",
        givers: &[90108],
        middle: [1150.0, -3250.0],
        radius: 700.0,
    },
    Ground {
        name: "The Merchant Coast",
        givers: &[90109],
        middle: [-1500.0, -3900.0],
        radius: 720.0,
    },
    Ground {
        name: "The Dry Hills",
        givers: &[90110],
        middle: [400.0, -2400.0],
        radius: 670.0,
    },
    Ground {
        name: "The Witchwing Hills",
        givers: &[90111],
        middle: [400.0, -1450.0],
        radius: 550.0,
    },
    Ground {
        name: "The Southern Barrens",
        givers: &[90105],
        middle: [-1900.0, -2150.0],
        radius: 830.0,
    },
];

fn toml_file(text: &str) -> toml::Value {
    toml::from_str(text).expect("content parses")
}

fn int(v: &toml::Value, k: &str) -> i64 {
    v.get(k).and_then(|x| x.as_integer()).unwrap_or(0)
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
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

struct Template {
    name: String,
    rank: i64,
    hostile: bool,
    level_min: i64,
    level_max: i64,
}

struct Barrens {
    /// entry -> every spawn's (x, y)
    spawns: HashMap<u32, Vec<[f64; 2]>>,
    templates: HashMap<u32, Template>,
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
                Template {
                    name: t["name"].as_str().unwrap().to_string(),
                    rank: int(t, "rank"),
                    hostile: t.get("hostile").and_then(|h| h.as_bool()).unwrap_or(true),
                    level_min: int(t, "level_min"),
                    level_max: int(t, "level_max"),
                },
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

fn hub_positions() -> HashMap<u32, [f64; 2]> {
    toml_file(include_str!("../content/hub.toml"))["npc"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| {
            let p = n["position"].as_array().unwrap();
            (
                int(n, "entry") as u32,
                [p[0].as_float().unwrap(), p[1].as_float().unwrap()],
            )
        })
        .collect()
}

fn middle_quests() -> Vec<toml::Value> {
    let givers: HashSet<u32> = GROUNDS
        .iter()
        .flat_map(|g| g.givers.iter().copied())
        .collect();
    quests()
        .into_iter()
        .filter(|q| givers.contains(&(int(q, "giver") as u32)))
        .collect()
}

/// The middle ring's Camp types: most of their spawns lie between the inner ring and the outer
/// ring's edge, they start at 10 to 19, and their spawns sit together.
fn middle_ring_camps(b: &Barrens) -> Vec<u32> {
    let mut entries: Vec<u32> = b
        .spawns
        .iter()
        .filter(|(e, _)| b.templates.contains_key(e))
        .filter(|(e, at)| {
            let t = &b.templates[*e];
            let inside = at
                .iter()
                .filter(|p| p[0] >= OUTER_EDGE_X && dist(**p, CROSSROADS) > INNER_RADIUS)
                .count();
            let n = at.len() as f64;
            let middle = [
                at.iter().map(|p| p[0]).sum::<f64>() / n,
                at.iter().map(|p| p[1]).sum::<f64>() / n,
            ];
            let together = at.iter().all(|p| dist(*p, middle) <= CAMP_SPREAD);
            inside * 2 > at.len() && (10..20).contains(&t.level_min) && together
        })
        .map(|(e, _)| *e)
        .collect();
    entries.sort();
    entries
}

fn is_rare(t: &Template) -> bool {
    matches!(t.rank, 2 | 4)
}

#[test]
fn every_middle_ring_camp_type_is_a_hunting_quest_target() {
    let b = barrens();
    let hunted: HashSet<u32> = middle_quests().iter().flat_map(targets).collect();
    let camps = middle_ring_camps(&b);
    assert!(camps.len() > 50, "the middle ring: {camps:?}");
    for e in camps {
        let t = &b.templates[&e];
        if !t.hostile && !is_rare(t) {
            continue; // the herds graze; nobody pays to hunt them
        }
        assert!(hunted.contains(&e), "{} ({e}) is no quest's target", t.name);
    }
}

#[test]
fn every_kept_rare_in_the_middle_ring_has_a_wanted_quest() {
    let b = barrens();
    let quests = middle_quests();
    let rares: Vec<u32> = middle_ring_camps(&b)
        .into_iter()
        .filter(|e| is_rare(&b.templates[e]))
        .collect();
    assert!(rares.len() >= 10, "the rares: {rares:?}");
    for e in rares {
        let title = format!("Wanted: {}", b.templates[&e].name);
        assert!(
            quests
                .iter()
                .any(|q| q["title"].as_str() == Some(&title) && targets(q) == [e]),
            "no \"{title}\""
        );
    }
}

#[test]
fn every_middle_quests_targets_lie_in_its_givers_hunting_ground() {
    let b = barrens();
    let quests = middle_quests();
    let hub = hub_positions();
    for g in &GROUNDS {
        let mine: Vec<&toml::Value> = quests
            .iter()
            .filter(|q| g.givers.contains(&(int(q, "giver") as u32)))
            .collect();
        assert!(mine.len() >= 4, "{}: {} quests", g.name, mine.len());
        for q in mine {
            let id = int(q, "id");
            for e in targets(q) {
                for p in &b.spawns[&e] {
                    assert!(
                        dist(g.middle, *p) <= g.radius,
                        "quest {id}: {e} spawns at {p:?}, outside {}",
                        g.name
                    );
                }
            }
            if let Some(x) = q.get("explore") {
                let at = x["position"].as_array().unwrap();
                let p = [at[0].as_float().unwrap(), at[1].as_float().unwrap()];
                assert!(
                    dist(g.middle, p) <= g.radius,
                    "quest {id}: its spot lies outside {}",
                    g.name
                );
            }
            if let Some(ender) = q.get("ender") {
                let p = hub[&(ender.as_integer().unwrap() as u32)];
                assert!(
                    dist(g.middle, p) <= g.radius,
                    "quest {id}: its ender stands outside {}",
                    g.name
                );
            }
        }
    }
}

#[test]
fn each_middle_ring_hunting_ground_has_one_find_quest() {
    let quests = middle_quests();
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
fn each_middle_quests_level_is_its_targets() {
    let b = barrens();
    for q in middle_quests() {
        let t = targets(&q);
        if t.is_empty() {
            continue;
        }
        let low = t.iter().map(|e| b.templates[e].level_min).min().unwrap();
        let high = t.iter().map(|e| b.templates[e].level_max).max().unwrap();
        let level = int(&q, "level");
        assert!(
            (low..=high).contains(&level),
            "quest {} is level {level}, its targets {low}-{high}",
            int(&q, "id")
        );
    }
}

#[test]
fn the_middle_rings_quests_carry_a_character_from_ten_to_twenty() {
    let quests = middle_quests();
    for level in 10..20 {
        let offered = quests
            .iter()
            .filter(|q| {
                let l = int(q, "level");
                let min = q
                    .get("min_level")
                    .map_or((l - 4).max(1), |m| m.as_integer().unwrap());
                min <= level && l >= level
            })
            .count();
        assert!(
            offered >= 3,
            "at level {level} the middle ring offers {offered} quests"
        );
    }
}
