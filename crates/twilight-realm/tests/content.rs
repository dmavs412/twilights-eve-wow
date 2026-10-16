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
fn the_dungeon_bosss_own_blues_match_the_reference() {
    // Every boss-pool entry that no world pool holds, as the reference spells it.
    let reference = json("loot_items.json");
    let blues = &reference["dungeon_blues"];
    let pool = blues["proposed_boss_pool"].as_array().unwrap();
    let table = toml_table("dungeon_items.toml");
    let rows = table["item"].as_array().unwrap();
    let mut want: Vec<(i64, toml::Value)> = blues["wailing_caverns"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| {
            let entry = &i["entry"];
            pool.contains(entry) && reference["items"][entry.to_string()].is_null()
        })
        .map(|i| (i["entry"].as_i64().unwrap(), expected_item(i)))
        .collect();
    want.sort_by_key(|(e, _)| *e);
    assert_eq!(rows.len(), want.len());
    assert_eq!(rows.len(), 7, "the instance's own boss drops");
    for (row, (entry, want)) in rows.iter().zip(&want) {
        assert_eq!(&with_defaults(row), want, "item {entry}");
    }
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

#[test]
fn the_goods_vendors_food_and_drink_match_the_reference() {
    let reference = json("goods_vendor.json");
    let rows = &toml_table("goods.toml")["item"];
    let stock: Vec<i64> = reference["proposed_crossroads_stock"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect();
    let entries: Vec<i64> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["entry"].as_integer().unwrap())
        .collect();
    assert_eq!(entries, stock, "the Crossroads' stock, in order");
    let hub = toml_table("hub.toml");
    let sold_by = |role: &str| -> Vec<i64> {
        hub["npc"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["role"].as_str() == Some(role))
            .and_then(|n| n.get("stock"))
            .and_then(|s| s.as_array())
            .map(|s| s.iter().map(|v| v.as_integer().unwrap()).collect())
            .unwrap_or_default()
    };
    assert_eq!(sold_by("innkeeper"), stock, "the innkeeper sells it");
    assert!(
        !sold_by("goods").iter().any(|e| stock.contains(e)),
        "and the goods vendor does not"
    );

    for row in rows.as_array().unwrap() {
        let entry = row["entry"].as_integer().unwrap();
        let want = reference["food_and_drink"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["entry"].as_i64() == Some(entry))
            .unwrap();
        let int = |k: &str| row.get(k).and_then(|v| v.as_integer()).unwrap_or(0);
        for (ours, theirs) in [
            ("required_level", "required_level"),
            ("item_level", "item_level"),
            ("buy_price", "buy_price"),
            ("buy_count", "buy_count"),
            ("sell_price", "sell_price"),
            ("stackable", "stackable"),
            ("display_id", "display_id"),
            ("use_spell", "spell"),
        ] {
            assert_eq!(int(ours), want[theirs].as_i64().unwrap(), "{entry} {ours}");
        }
        assert_eq!(row["name"].as_str(), want["name"].as_str());
        assert_eq!(
            int("restores_health"),
            want["restores"]["health"].as_i64().unwrap_or(0),
            "{entry}"
        );
        assert_eq!(
            int("restores_mana"),
            want["restores"]["mana"].as_i64().unwrap_or(0),
            "{entry}"
        );
        assert_eq!(
            int("duration_ms"),
            (want["duration_s"].as_f64().unwrap() * 1000.0) as i64
        );
    }
}

#[test]
fn the_repair_tables_match_the_reference() {
    let repair = &json("goods_vendor.json")["repair"];
    let table = toml_table("durability.toml");
    let costs = table["costs"].as_array().unwrap();
    let reference = repair["durability_costs"].as_object().unwrap();
    assert_eq!(costs.len(), reference.len());
    for (level, row) in reference {
        let i = level.parse::<usize>().unwrap() - 1;
        let want: Vec<f64> = row
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        assert_eq!(floats(&costs[i]), want, "item level {level}");
    }
    let by_quality = &repair["quality_mod_by_item_quality"];
    let want: Vec<f64> = ["poor", "common", "uncommon", "rare", "epic"]
        .iter()
        .map(|q| by_quality[q].as_f64().unwrap())
        .collect();
    assert_eq!(floats(&table["quality"]), want);
}

/// Every race's Warrior outfit (`content/starting_outfits.toml`, from `CharStartOutfit.dbc`) has
/// an item template the realm answers queries with, worn in an equipment slot, and the template's
/// display id and inventory type are the ones the client's outfit row names.
#[test]
fn every_starting_outfit_item_has_a_template_that_agrees_with_the_client() {
    let outfits = toml_table("starting_outfits.toml");
    let races = outfits.as_table().unwrap();
    assert_eq!(races.len(), 8, "one outfit per race");
    let mut realm = twilight_realm::Realm::new();
    for (race, items) in races {
        let items = items.as_array().unwrap();
        assert!(
            items.len() >= 3,
            "{race}: shirt, legs and a weapon at least"
        );
        for row in items {
            let int = |k: &str| row[k].as_integer().unwrap() as u32;
            let entry = int("item");
            realm.handle(twilight_realm::RealmCommand::ItemQuery { entry });
            let info = realm
                .drain_events()
                .into_iter()
                .find_map(|e| match e {
                    benilla_protocol::SessionEvent::ItemTemplate { info, .. } => info,
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{race}: item {entry} has no template"));
            assert_eq!(
                (info.display_info_id, info.inventory_type),
                (int("display_id"), int("inventory_type")),
                "{race}: {} looks as the client dresses it",
                info.name
            );
            assert!(
                !twilight_realm::rules::gear::equip_slots(info.inventory_type).is_empty(),
                "{race}: {} (inventory type {}) is worn",
                info.name,
                info.inventory_type
            );
        }
    }
}
