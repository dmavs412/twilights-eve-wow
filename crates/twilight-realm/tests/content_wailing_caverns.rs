//! `content/wailing_caverns.toml` still says what the research's hand-picked run says
//! (`data/twilights-eve/wailing_caverns_run.json`, `docs/reference/wailing-caverns.md`).

use std::path::PathBuf;

use serde_json::Value;

fn run() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/twilights-eve/wailing_caverns_run.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn dungeon() -> toml::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("content/wailing_caverns.toml");
    toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn floats(v: &toml::Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| {
            x.as_float()
                .unwrap_or_else(|| x.as_integer().unwrap() as f64)
        })
        .collect()
}

fn int(v: &toml::Value) -> i64 {
    v.as_integer().unwrap()
}

#[test]
fn the_map_and_its_triggers_match_the_reference() {
    let (run, d) = (run(), dungeon());
    assert_eq!(int(&d["map"]), run["map_id"].as_i64().unwrap());
    assert_eq!(int(&d["zone"]), run["zone_id"].as_i64().unwrap());
    for side in ["entrance", "exit"] {
        let (want, got) = (&run[side], &d[side]);
        let t = &want["area_trigger"];
        assert_eq!(int(&got["trigger"]), t["id"].as_i64().unwrap(), "{side}");
        assert_eq!(int(&got["map"]), t["map"].as_i64().unwrap(), "{side}");
        let at: Vec<f64> = ["x", "y", "z"]
            .iter()
            .map(|k| t[k].as_f64().unwrap())
            .collect();
        assert_eq!(floats(&got["position"]), at, "{side}");
        assert_eq!(got["radius"].as_float(), t["radius"].as_f64(), "{side}");
        let to = &want["teleport_to"];
        assert_eq!(
            int(&got["to"]["map"]),
            to["map"].as_i64().unwrap(),
            "{side}"
        );
        let at: Vec<f64> = ["x", "y", "z"]
            .iter()
            .map(|k| to[k].as_f64().unwrap())
            .collect();
        assert_eq!(floats(&got["to"]["position"]), at, "{side}");
        assert_eq!(
            got["to"]["orientation"].as_float(),
            to["o"].as_f64(),
            "{side}"
        );
    }
    // A death inside comes back where the entrance puts the player.
    let respawn = &run["death"]["respawn_inside_at"];
    assert_eq!(
        floats(&d["entrance"]["to"]["position"]),
        ["x", "y", "z"]
            .iter()
            .map(|k| respawn[k].as_f64().unwrap())
            .collect::<Vec<_>>()
    );
}

#[test]
fn the_spawns_are_the_runs_ten_mobs_and_its_boss() {
    let (run, d) = (run(), dungeon());
    let spawns = d["spawn"].as_array().unwrap();
    let mobs = run["mobs"].as_array().unwrap();
    assert_eq!(spawns.len(), mobs.len());
    assert_eq!(mobs.len(), 11, "ten trash mobs and the boss");
    for (got, want) in spawns.iter().zip(mobs) {
        let guid = want["guid"].as_i64().unwrap();
        assert_eq!(int(&got["guid"]), guid);
        assert_eq!(
            int(&got["entry"]),
            want["entry"].as_i64().unwrap(),
            "{guid}"
        );
        assert_eq!(int(&got["level_min"]), want["level_min"].as_i64().unwrap());
        assert_eq!(int(&got["level_max"]), want["level_max"].as_i64().unwrap());
        let at: Vec<f64> = ["x", "y", "z"]
            .iter()
            .map(|k| want[k].as_f64().unwrap())
            .collect();
        assert_eq!(floats(&got["position"]), at, "{guid}");
        assert_eq!(got["orientation"].as_float(), want["o"].as_f64(), "{guid}");
        let camp = want["group_leader"].as_i64().unwrap_or(guid);
        assert_eq!(int(&got["camp"]), camp, "{guid}");
        // Its creature, at the reference's display, faction, rank and speeds.
        let template = d["template"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| int(&t["entry"]) == int(&got["entry"]))
            .expect("a template for every spawn");
        assert_eq!(template["name"].as_str(), want["name"].as_str());
        assert_eq!(
            int(&template["display_id"]),
            want["display_ids"][0].as_i64().unwrap()
        );
        assert_eq!(
            int(&template["faction_template"]),
            want["faction"].as_i64().unwrap()
        );
        assert_eq!(int(&template["rank"]), want["rank"].as_i64().unwrap());
        assert_eq!(
            template["detection_range"].as_float(),
            want["detection_range"].as_f64()
        );
    }
    assert_eq!(int(&d["boss"]), run["boss"]["guid"].as_i64().unwrap());
}

#[test]
fn the_route_graph_is_the_references() {
    let (run, d) = (run(), dungeon());
    let route = &run["route"];
    let nodes = d["route"]["nodes"].as_array().unwrap();
    let want = route["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), want.len());
    for (got, want) in nodes.iter().zip(want) {
        assert_eq!(got["id"].as_str(), want["id"].as_str());
        let at: Vec<f64> = ["x", "y", "z"]
            .iter()
            .map(|k| want[k].as_f64().unwrap())
            .collect();
        assert_eq!(floats(&got["at"]), at, "{}", want["id"]);
    }
    let edges = d["route"]["edges"].as_array().unwrap();
    let want = route["edges"].as_array().unwrap();
    assert_eq!(edges.len(), want.len());
    for (got, want) in edges.iter().zip(want) {
        assert_eq!(got[0].as_str(), want["a"].as_str());
        assert_eq!(got[1].as_str(), want["b"].as_str());
    }
}

/// The reference pool's Leggings of the Fang, Cobrahn's Grasp, Glowing Lizardscale Cloak,
/// Thorbia's Gauntlets, Silver-linked Footguards, Stormbringer Belt, Feet of the Lynx and Sentry
/// Cloak: legs, waist, back, hands and feet.
const NOT_GEAR_SLOTS: [i64; 8] = [10410, 6460, 6449, 12994, 12982, 12978, 1121, 2059];

#[test]
fn the_boss_loot_is_the_references_boss_pool() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/twilights-eve/loot_items.json");
    let reference: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let want: Vec<i64> = reference["dungeon_blues"]["proposed_boss_pool"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e.as_i64().unwrap())
        // Less the blues for a body part outside the ten Gear Slots (ADR 0007).
        .filter(|e| !NOT_GEAR_SLOTS.contains(e))
        .collect();
    let got: Vec<i64> = dungeon()["boss_loot"]
        .as_array()
        .unwrap()
        .iter()
        .map(int)
        .collect();
    assert_eq!(got, want);
}

fn content(file: &str) -> toml::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("content")
        .join(file);
    toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// `AreaTable.dbc` 718, which files a quest under the Wailing Caverns in the log.
const WAILING_CAVERNS_ZONE: i64 = 718;
/// The skull's crown, the ridge above the Caverns' mouth in the Lushwater Oasis (pinned in the
/// client), and how far from it a Disciple may stand.
const SKULL: [f64; 2] = [-780.0, -2034.0];
const ON_THE_SKULL_YD: f64 = 15.0;

#[test]
fn the_caverns_quests_are_repeatable_from_sixteen_given_on_the_skull_for_the_runs_creatures() {
    let (quests, hub, d) = (content("quests.toml"), content("hub.toml"), dungeon());
    let in_the_run: Vec<i64> = d["spawn"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| int(&s["entry"]))
        .collect();
    let caverns: Vec<&toml::Value> = quests["quest"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|q| q.get("zone_or_sort").map(int) == Some(WAILING_CAVERNS_ZONE))
        .collect();
    assert_eq!(caverns.len(), 3, "kill, collect and the boss");
    let mut boss = false;
    for q in caverns {
        let id = int(&q["id"]);
        assert_eq!(q["kind"].as_str(), Some("repeatable"), "{id}");
        assert_eq!(int(&q["tokens"]), 3, "{id}");
        assert_eq!(int(&q["min_level"]), 16, "{id}");
        // Its giver stands on the skull's crown.
        let giver = hub["npc"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| int(&n["entry"]) == int(&q["giver"]))
            .unwrap_or_else(|| panic!("{id}'s giver"));
        let at = floats(&giver["position"]);
        let yd = (at[0] - SKULL[0]).hypot(at[1] - SKULL[1]);
        assert!(
            yd < ON_THE_SKULL_YD,
            "{id}'s giver stands {yd} yd from the crown"
        );
        // And everything it asks for is met in the run.
        let kills = q.get("kill").and_then(|k| k.as_array()).cloned();
        let collects = q.get("collect").and_then(|c| c.as_array()).cloned();
        let mut targets: Vec<i64> = Vec::new();
        for k in kills.iter().flatten() {
            let (entry, count) = (int(&k[0]), int(&k[1]));
            let standing = in_the_run.iter().filter(|&&e| e == entry).count() as i64;
            assert!(
                count <= standing,
                "{id}: {count} of {entry}, {standing} in a run"
            );
            boss |= entry == 3671;
            targets.push(entry);
        }
        for c in collects.iter().flatten() {
            targets.extend(c["from"].as_array().unwrap().iter().map(int));
        }
        assert!(!targets.is_empty(), "{id}");
        for entry in targets {
            assert!(
                in_the_run.contains(&entry),
                "{id}: {entry} is not in the run"
            );
        }
    }
    assert!(boss, "one asks for Lady Anacondra");
}
