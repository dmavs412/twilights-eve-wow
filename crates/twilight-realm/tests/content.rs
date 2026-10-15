//! The realm's generated tables under `content/` still say what the reference dumps in
//! `data/twilights-eve/` say. The realm cannot read JSON (the shipped app keeps `serde_json` out),
//! so the tables are TOML copies; this is what keeps them honest.

use std::path::PathBuf;

use serde_json::Value;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn json(name: &str) -> Value {
    let path = repo().join("data/twilights-eve").join(name);
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

fn toml_table(name: &str) -> toml::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("content")
        .join(name);
    toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// One field of every row, ordered by level.
fn column(rows: &Value, field: &str) -> Vec<f64> {
    let mut rows: Vec<&Value> = rows.as_array().unwrap().iter().collect();
    rows.sort_by_key(|r| r["level"].as_u64().unwrap());
    rows.iter().map(|r| r[field].as_f64().unwrap()).collect()
}

fn floats(v: &toml::Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| {
            x.as_float()
                .or_else(|| x.as_integer().map(|i| i as f64))
                .unwrap()
        })
        .collect()
}

#[test]
fn the_level_tables_match_the_reference() {
    let levels = toml_table("levels.toml");
    assert_eq!(
        floats(&levels["xp_to_next"]),
        column(&json("xp_table.json")["levels"], "xp_to_next")
    );

    let stats = json("warrior_level_stats.json");
    let w = &levels["warrior"];
    assert_eq!(
        floats(&w["base_health"]),
        column(&stats["per_level"], "base_health")
    );
    assert_eq!(
        floats(&w["agi_per_crit"]),
        column(&stats["per_level"], "agi_per_crit_pct")
    );
    assert_eq!(
        floats(&w["agi_per_dodge"]),
        column(&stats["per_level"], "agi_per_dodge_pct")
    );
    for race in [
        "human",
        "orc",
        "dwarf",
        "night_elf",
        "undead",
        "tauren",
        "gnome",
        "troll",
    ] {
        let rows: Vec<Vec<f64>> = w["race_stats"][race]
            .as_array()
            .unwrap()
            .iter()
            .map(floats)
            .collect();
        let reference = &stats["race_base_stats"][race];
        let want: Vec<Vec<f64>> = (0..rows.len())
            .map(|i| {
                ["str", "agi", "sta", "int", "spi"]
                    .iter()
                    .map(|f| column(reference, f)[i])
                    .collect()
            })
            .collect();
        assert_eq!(rows, want, "{race}");
    }
}

#[test]
fn the_mob_class_tables_match_the_reference() {
    let mobs = toml_table("mob_class_stats.toml");
    let reference = json("mob_base_stats.json");
    for class in ["warrior", "paladin", "mage"] {
        let rows = &reference["by_unit_class"][class];
        for field in ["health", "melee_damage_avg", "armor"] {
            assert_eq!(
                floats(&mobs[class][field]),
                column(rows, field),
                "{class} {field}"
            );
        }
    }
}
