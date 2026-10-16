//! `content/barrens.toml` is what `examples/import_barrens.rs` makes of the research dumps, and
//! every creature it spawns has a template.

#[path = "../examples/import_barrens.rs"]
#[allow(dead_code)]
mod import_barrens;

mod common;

use benilla_protocol::SessionEvent;
use common::*;

#[test]
fn barrens_toml_is_the_import_of_the_dumps() {
    let (barrens, low) = import_barrens::dumps();
    let want = import_barrens::convert(&barrens, &low);
    let have = std::fs::read_to_string(import_barrens::output_path())
        .unwrap()
        .replace("\r\n", "\n");
    assert!(
        have == want,
        "content/barrens.toml is stale: run `cargo run -p twilight-realm --example import_barrens`"
    );
}

#[test]
fn every_spawned_creature_answers_to_its_name() {
    let text = std::fs::read_to_string(import_barrens::output_path()).unwrap();
    let file: toml::Value = toml::from_str(&text).unwrap();
    let mut entries: Vec<i64> = file["spawns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r[1].as_integer().unwrap())
        .collect();
    entries.sort_unstable();
    entries.dedup();
    assert!(entries.len() > 130, "{} creatures", entries.len());

    let mut realm = realm();
    for entry in entries {
        let entry = entry as u32;
        realm.handle(RealmCommand::CreatureQuery { entry });
        let named = drain(&mut realm).into_iter().any(|e| {
            matches!(e, SessionEvent::CreatureName { entry: e, name: Some(_), .. } if e == entry)
        });
        assert!(named, "entry {entry} has no template");
    }
}

// ── The rings ─────────────────────────────────────────────────────────────────────────────────

/// A spawn on Kalimdor out of `barrens.toml` or `mobs.toml`, with its template's name and rank.
struct Spawned {
    entry: u64,
    at: [f64; 2],
    levels: [u64; 2],
    name: String,
    rank: i64,
}

fn toml_file(name: &str) -> toml::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("content")
        .join(name);
    toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Every spawn in The Barrens: the generated rows and the hand-written ones.
fn barrens_spawns() -> Vec<Spawned> {
    let barrens = toml_file("barrens.toml");
    let hand = toml_file("mobs.toml");
    let template = |entry: u64| {
        barrens["template"]
            .as_array()
            .unwrap()
            .iter()
            .chain(hand["template"].as_array().unwrap())
            .find(|t| t["entry"].as_integer() == Some(entry as i64))
            .unwrap_or_else(|| panic!("entry {entry}'s template"))
            .clone()
    };
    let int = |v: &toml::Value| v.as_integer().unwrap() as u64;
    let float = |v: &toml::Value| v.as_float().unwrap();
    let spawned = |entry: u64, at: [f64; 2], levels: [u64; 2]| {
        let t = template(entry);
        Spawned {
            entry,
            at,
            levels,
            name: t["name"].as_str().unwrap().to_owned(),
            rank: t.get("rank").map_or(0, |r| r.as_integer().unwrap()),
        }
    };
    let mut all: Vec<Spawned> = barrens["spawns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            spawned(
                int(&r[1]),
                [float(&r[4]), float(&r[5])],
                [int(&r[2]), int(&r[3])],
            )
        })
        .collect();
    all.extend(hand["spawn"].as_array().unwrap().iter().map(|s| {
        let p = &s["position"];
        spawned(
            int(&s["entry"]),
            [float(&p[0]), float(&p[1])],
            [int(&s["level"]); 2],
        )
    }));
    all
}

fn crossroads() -> [f64; 2] {
    import_barrens::crossroads(&import_barrens::dumps().1)
}

#[test]
fn no_spawn_in_the_inner_ring_is_above_level_10() {
    let town = crossroads();
    let inner: Vec<_> = barrens_spawns()
        .into_iter()
        .filter(|s| (s.at[0] - town[0]).hypot(s.at[1] - town[1]) <= import_barrens::INNER_RING_YD)
        .collect();
    assert!(
        inner.len() > 20,
        "the low camps stand there: {}",
        inner.len()
    );
    for s in inner {
        assert!(
            s.levels[1] <= import_barrens::INNER_RING_TOP,
            "a level {:?} {} at {:?}",
            s.levels,
            s.name,
            s.at
        );
    }
}

#[test]
fn no_spawn_is_above_level_30_and_the_razorfen_quilboar_are_the_capstone() {
    let all = barrens_spawns();
    for s in &all {
        assert!(
            s.levels[1] <= import_barrens::LEVEL_CAP,
            "a level {:?} {}",
            s.levels,
            s.name
        );
    }
    let quilboar: Vec<_> = all
        .iter()
        .filter(|s| import_barrens::RAZORFEN_QUILBOAR.contains(&s.entry))
        .collect();
    assert!(quilboar.len() > 50, "{} Razorfen quilboar", quilboar.len());
    for s in quilboar {
        assert_eq!(s.levels, [27, 30], "{}", s.name);
    }
    let barrens = toml_file("barrens.toml");
    for t in barrens["template"].as_array().unwrap() {
        assert!(t["level_max"].as_integer().unwrap() <= 30, "{}", t["name"]);
    }
}

#[test]
fn no_spawn_is_far_off_its_neighbours_level() {
    let all = barrens_spawns();
    let placed: Vec<_> = all
        .iter()
        .map(|s| import_barrens::Placed {
            at: s.at,
            levels: s.levels,
            normal: s.rank == 0,
        })
        .collect();
    let strays: Vec<_> = import_barrens::strays(&placed)
        .into_iter()
        .map(|i| (&all[i].name, all[i].levels, all[i].at))
        .collect();
    assert!(strays.is_empty(), "{strays:?}");
}

#[test]
fn no_cultist_ambassador_trigore_or_boahn_spawns() {
    for s in barrens_spawns() {
        assert!(
            !s.name.starts_with("Death's Head")
                && !s.name.starts_with("Ambassador")
                && s.name != "Trigore the Lasher"
                && s.name != "Boahn",
            "{} spawns",
            s.name
        );
    }
}

#[test]
fn every_quest_target_spawns() {
    let mut spawned: Vec<u64> = barrens_spawns().into_iter().map(|s| s.entry).collect();
    let dungeon = toml_file("wailing_caverns.toml");
    spawned.extend(
        dungeon["spawn"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["entry"].as_integer().unwrap() as u64),
    );
    let quests = toml_file("quests.toml");
    for q in quests["quest"].as_array().unwrap() {
        let kills = q.get("kill").and_then(|k| k.as_array());
        let collects = q.get("collect").and_then(|c| c.as_array());
        let targets = kills
            .into_iter()
            .flatten()
            .map(|k| &k[0])
            .chain(
                collects
                    .into_iter()
                    .flatten()
                    .flat_map(|c| c["from"].as_array().unwrap()),
            )
            .map(|e| e.as_integer().unwrap() as u64);
        for entry in targets {
            assert!(
                spawned.contains(&entry),
                "quest {}: creature {entry} does not spawn",
                q["id"]
            );
        }
    }
}
