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

/// `ItemPrototype`'s stat ids, as `SMSG_ITEM_QUERY_SINGLE_RESPONSE` carries them.
fn stat_id(name: &str) -> i64 {
    match name {
        "agility" => 3,
        "strength" => 4,
        "intellect" => 5,
        "spirit" => 6,
        "stamina" => 7,
        other => panic!("an unknown item stat {other}"),
    }
}

/// `SpellSchools`.
fn school_id(name: &str) -> i64 {
    match name {
        "physical" => 0,
        "holy" => 1,
        "fire" => 2,
        "nature" => 3,
        "frost" => 4,
        "shadow" => 5,
        "arcane" => 6,
        other => panic!("an unknown school {other}"),
    }
}

/// One reference item as the realm's table should spell it.
fn expected_item(item: &Value) -> toml::Value {
    let int = |k: &str| toml::Value::Integer(item[k].as_i64().unwrap_or(0));
    let mut t = toml::Table::new();
    for k in [
        "entry",
        "quality",
        "item_level",
        "required_level",
        "class",
        "subclass",
        "inventory_type",
        "allowable_class",
        "allowable_race",
        "display_id",
        "armor",
        "block",
        "speed_ms",
        "durability",
        "sell_price",
        "buy_price",
        "buy_count",
        "bonding",
        "max_count",
        "stackable",
        "container_slots",
        "sheath",
        "material",
        "pool_level",
    ] {
        t.insert(k.into(), int(k));
    }
    t.insert("name".into(), item["name"].as_str().unwrap().into());
    let stats: Vec<toml::Value> = item["stats"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            toml::Value::Array(vec![
                stat_id(s["stat"].as_str().unwrap()).into(),
                s["value"].as_i64().unwrap().into(),
            ])
        })
        .collect();
    t.insert("stats".into(), toml::Value::Array(stats));
    let resistances: Vec<toml::Value> = ["holy", "fire", "nature", "frost", "shadow", "arcane"]
        .iter()
        .map(|s| item["resistances"][s].as_i64().unwrap_or(0).into())
        .collect();
    t.insert("resistances".into(), toml::Value::Array(resistances));
    let damage: Vec<toml::Value> = item["damage"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            toml::Value::Array(vec![
                d["min"].as_f64().unwrap().into(),
                d["max"].as_f64().unwrap().into(),
                school_id(d["school"].as_str().unwrap()).into(),
            ])
        })
        .collect();
    t.insert("damage".into(), toml::Value::Array(damage));
    let mut spells = Vec::new();
    let (mut attack_power, mut defense) = (0, 0);
    for e in item["effects"].as_array().unwrap() {
        assert_eq!(e["trigger"], "equip", "only equip spells are kept");
        spells.push(toml::Value::Integer(e["spell"].as_i64().unwrap()));
        for m in e["mods"].as_array().unwrap() {
            match (m["stat"].as_str().unwrap(), m["skill"].as_i64()) {
                ("attack_power", _) => attack_power += m["value"].as_i64().unwrap(),
                ("skill", Some(95)) => defense += m["value"].as_i64().unwrap(),
                _ => {}
            }
        }
    }
    t.insert("equip_spells".into(), toml::Value::Array(spells));
    t.insert("attack_power".into(), attack_power.into());
    t.insert("defense".into(), defense.into());
    toml::Value::Table(t)
}

#[test]
fn the_item_table_matches_the_reference() {
    let reference = json("loot_items.json");
    let table = toml_table("items.toml");
    let rows = table["item"].as_array().unwrap();
    let mut want: Vec<(i64, toml::Value)> = reference["items"]
        .as_object()
        .unwrap()
        .values()
        .map(|i| (i["entry"].as_i64().unwrap(), expected_item(i)))
        .collect();
    want.sort_by_key(|(e, _)| *e);
    assert_eq!(rows.len(), want.len());
    for (row, (entry, want)) in rows.iter().zip(&want) {
        assert_eq!(&with_defaults(row), want, "item {entry}");
    }
}

/// A row with the fields it leaves out filled in at their defaults.
fn with_defaults(row: &toml::Value) -> toml::Value {
    let mut t = row.as_table().unwrap().clone();
    let want = expected_item(&serde_json::json!({
        "entry": 0, "name": "", "stats": [], "damage": [], "effects": [], "resistances": {},
        "allowable_class": -1, "allowable_race": -1,
    }));
    for (k, v) in want.as_table().unwrap() {
        t.entry(k.clone()).or_insert_with(|| v.clone());
    }
    toml::Value::Table(t)
}

#[test]
fn the_reference_pools_follow_each_items_pool_level_and_quality() {
    // The realm builds its pools from each row's `pool_level` and quality, so the reference's
    // own pools must be exactly that grouping.
    let reference = json("loot_items.json");
    let items = &reference["items"];
    for (level, pools) in reference["pools_by_level"].as_object().unwrap() {
        for (quality, entries) in pools.as_object().unwrap() {
            for entry in entries.as_array().unwrap() {
                let item = &items[entry.as_i64().unwrap().to_string()];
                assert_eq!(item["pool_level"].to_string(), *level, "{entry}");
                assert_eq!(item["quality"].to_string(), *quality, "{entry}");
            }
        }
    }
}
