//! The realm's generated tables under `content/` still say what the reference dumps in
//! `data/twilights-eve/` say. The realm cannot read JSON (the shipped app keeps `serde_json` out),
//! so the tables are TOML copies; this is what keeps them honest.

use std::path::PathBuf;

use benilla_protocol::messages::ItemInfo;
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
        floats(&levels["quest_xp_basis"]),
        column(&json("xp_table.json")["levels"], "xp_to_next")
    );

    for class in [
        "warrior", "paladin", "hunter", "rogue", "priest", "shaman", "mage", "warlock", "druid",
    ] {
        let stats = json(&format!("{class}_level_stats.json"));
        let w = &levels[class];
        for (field, reference) in [
            ("base_health", "base_health"),
            ("base_mana", "base_mana"),
            ("agi_per_crit", "agi_per_crit_pct"),
            ("agi_per_dodge", "agi_per_dodge_pct"),
        ] {
            assert_eq!(
                floats(&w[field]),
                column(&stats["per_level"], reference),
                "{class} {field}"
            );
        }
        let race_stats = w["race_stats"].as_table().unwrap();
        let reference_races = stats["race_base_stats"].as_object().unwrap();
        assert_eq!(
            race_stats.keys().collect::<Vec<_>>(),
            reference_races.keys().collect::<Vec<_>>(),
            "{class}: the same races"
        );
        assert_eq!(race_stats.len(), 8, "{class}: every race");
        for (race, rows) in race_stats {
            let rows: Vec<Vec<f64>> = rows.as_array().unwrap().iter().map(floats).collect();
            let reference = &reference_races[race];
            let want: Vec<Vec<f64>> = (0..rows.len())
                .map(|i| {
                    ["str", "agi", "sta", "int", "spi"]
                        .iter()
                        .map(|f| column(reference, f)[i])
                        .collect()
                })
                .collect();
            assert_eq!(rows, want, "{class} {race}");
        }
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
    let (mut spells, mut spell_names) = (Vec::new(), Vec::new());
    let (mut attack_power, mut defense) = (0, 0);
    for e in item["effects"].as_array().unwrap() {
        assert_eq!(e["trigger"], "equip", "only equip spells are kept");
        spells.push(toml::Value::Integer(e["spell"].as_i64().unwrap()));
        spell_names.push(toml::Value::from(e["name"].as_str().unwrap()));
        for m in e["mods"].as_array().unwrap() {
            match (m["stat"].as_str().unwrap(), m["skill"].as_i64()) {
                ("attack_power", _) => attack_power += m["value"].as_i64().unwrap(),
                ("skill", Some(95)) => defense += m["value"].as_i64().unwrap(),
                _ => {}
            }
        }
    }
    t.insert("equip_spells".into(), toml::Value::Array(spells));
    t.insert("equip_spell_names".into(), toml::Value::Array(spell_names));
    t.insert("attack_power".into(), attack_power.into());
    t.insert("defense".into(), defense.into());
    toml::Value::Table(t)
}

#[test]
fn the_item_table_matches_the_reference() {
    let reference = json("loot_items.json");
    let table = toml_table("items.toml");
    let rows = table["item"].as_array().unwrap();
    // Weapons and armour are Gear, in gear.toml instead (ADR 0007).
    let mut want: Vec<(i64, toml::Value)> = reference["items"]
        .as_object()
        .unwrap()
        .values()
        .filter(|i| !matches!(i["class"].as_i64(), Some(2 | 4)))
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

/// The fields the gear sheet does not carry (ADR 0007): vendor prices and bundles, uniqueness, the
/// class and race masks and the stack size, which no copper vendor or Calling reads for Gear.
const NOT_IN_THE_SHEET: [&str; 7] = [
    "buy_price",
    "buy_count",
    "max_count",
    "allowable_class",
    "allowable_race",
    "stackable",
    "wardrobe",
];

/// A Gear row with the fields the sheet leaves out dropped, and its stats in stat-id order.
fn sheet_fields(row: &toml::Value) -> toml::Value {
    let mut t = with_defaults(row).as_table().unwrap().clone();
    for k in NOT_IN_THE_SHEET {
        t.remove(k);
    }
    let mut stats = t["stats"].as_array().unwrap().clone();
    stats.sort_by_key(|s| s[0].as_integer().unwrap());
    t.insert("stats".into(), toml::Value::Array(stats));
    toml::Value::Table(t)
}

/// Every piece of Gear seeded from the reference (world drops and the Wailing Caverns boss's own
/// blues) still answers with the reference's numbers: the sheet is the source now, but the seed
/// changed nothing.
#[test]
fn the_seeded_gear_keeps_the_references_numbers() {
    let reference = json("loot_items.json");
    let dungeon = reference["dungeon_blues"]["wailing_caverns"]
        .as_array()
        .unwrap();
    let gear = toml_table("gear.toml");
    let mut seeded = 0;
    for row in gear["item"].as_array().unwrap() {
        let entry = row["entry"].as_integer().unwrap();
        let mut item = reference["items"][entry.to_string()].clone();
        if item.is_null() {
            // The boss's own blues drop from her pool, not a world pool.
            match dungeon.iter().find(|i| i["entry"].as_i64() == Some(entry)) {
                Some(i) => {
                    item = i.clone();
                    item["pool_level"] = 0.into();
                }
                None => continue,
            }
        }
        // The vanilla rows of pool level 12 and under left the drop file (spec #272:
        // Northshire's loot is hand-authored); their numbers stay.
        if item["pool_level"].as_i64().is_some_and(|l| l <= 12) {
            item["pool_level"] = 0.into();
        }
        assert_eq!(
            sheet_fields(row),
            sheet_fields(&expected_item(&item)),
            "gear {entry}"
        );
        seeded += 1;
    }
    assert!(seeded > 400, "only {seeded} seeded rows");
}

/// Running `tools/twilights-eve/build_gear.py` on the committed sheet writes the committed
/// `content/gear.toml`, byte for byte. Skips where no Python 3 runs.
#[test]
fn the_gear_table_is_the_gear_sheet_built() {
    let script = repo().join("tools/twilights-eve/build_gear.py");
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();
    let candidates = [
        std::env::var("PYTHON").unwrap_or_default(),
        format!("{home}/.local/bin/python3.15.exe"),
        format!("{home}/.local/bin/python3.15"),
        "python3".to_owned(),
    ];
    let built = candidates
        .iter()
        .filter(|p| !p.is_empty())
        .filter_map(|p| {
            std::process::Command::new(p)
                .arg(&script)
                .args(["--out", "-"])
                .output()
                .ok()
        })
        .find(|o| o.status.success());
    let Some(built) = built else {
        eprintln!("skipped: no Python 3 to run build_gear.py");
        return;
    };
    let committed =
        std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("content/gear.toml")).unwrap();
    assert!(
        built.stdout == committed,
        "content/gear.toml is stale: run tools/twilights-eve/build_gear.py"
    );
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

/// Items never bind: an item that is Bind on Equip in the data (Dragonmaw Shortsword, Riverpaw
/// Leather Vest) is queried with bonding 0, so no tooltip bind line and no bind popup.
#[test]
fn item_queries_always_report_bonding_zero() {
    let mut realm = twilight_realm::Realm::new();
    for entry in [753, 821] {
        let info = queried(&mut realm, entry).unwrap_or_else(|| panic!("{entry} has no template"));
        assert_eq!(info.bonding, 0, "{entry} reports bonding 0");
    }
}

/// The item template the realm answers `entry`'s query with.
fn queried(realm: &mut twilight_realm::Realm, entry: u32) -> Option<ItemInfo> {
    realm.handle(twilight_realm::RealmCommand::ItemQuery { entry });
    realm.drain_events().into_iter().find_map(|e| match e {
        benilla_protocol::SessionEvent::ItemTemplate { info, .. } => info.map(|i| *i),
        _ => None,
    })
}

/// Every Calling's outfit (`content/starting_outfits.toml`) is exactly one Chest and the weapons,
/// ranged weapon and ammo that go with it, each an item the realm answers queries with and the
/// Calling may wear.
#[test]
fn every_callings_starting_outfit_is_one_chest_and_its_weapons() {
    let outfits = toml_table("starting_outfits.toml");
    let outfits = outfits.as_table().unwrap();
    for calling in ["basic_warrior", "vanguard", "zealot"] {
        assert!(outfits.contains_key(calling), "{calling} has an outfit");
    }
    let mut realm = twilight_realm::Realm::new();
    for (calling, outfit) in outfits {
        let def = twilight_realm::callings::calling(calling).expect("a Calling");
        let mut chests = 0;
        for entry in outfit["items"].as_array().unwrap() {
            let entry = entry.as_integer().unwrap() as u32;
            let info = queried(&mut realm, entry)
                .unwrap_or_else(|| panic!("{calling}: item {entry} has no template"));
            match info.inventory_type {
                // Chest and robe.
                5 | 20 => chests += 1,
                // Main hand, one-hand, two-hand, shield, held, bow, thrown, gun, ammo.
                21 | 13 | 17 | 14 | 23 | 15 | 25 | 26 | 24 => {}
                other => panic!("{calling}: {} has inventory type {other}", info.name),
            }
            assert_eq!(
                twilight_realm::rules::gear::can_equip(entry, 1, def),
                Ok(()),
                "{calling} wears {}",
                info.name
            );
        }
        assert_eq!(chests, 1, "{calling}: one starter Chest");
    }
}

/// Whether the realm's template for `entry` is equipment for a body part outside the ten Gear
/// Slots.
fn outside_the_gear_slots(realm: &mut twilight_realm::Realm, entry: u32) -> bool {
    let info = queried(realm, entry).unwrap_or_else(|| panic!("{entry} has no template"));
    // 0 is no equipment at all, 18 a bag.
    !matches!(info.inventory_type, 0 | 18)
        && !twilight_realm::rules::gear::is_gear_slot(info.inventory_type)
}

/// No loot pool, vendor, Token vendor band or Dungeon boss hands out an item for a body part the
/// ten Gear Slots leave out (ADR 0007). Quests reward XP and Tokens, no items.
#[test]
fn nothing_hands_out_gear_outside_the_ten_slots() {
    use twilight_realm::rules::loot::{pool, Quality};
    let mut entries: Vec<(String, u32)> = Vec::new();
    for level in 1..=60 {
        for q in [
            Quality::Poor,
            Quality::Common,
            Quality::Uncommon,
            Quality::Rare,
        ] {
            let from = format!("the level {level} {q:?} pool");
            entries.extend(pool(level, q).iter().map(|e| (from.clone(), *e)));
        }
        let from = format!("the level {level} Token band");
        entries.extend(
            twilight_realm::rules::tokens::stock(level)
                .into_iter()
                .map(|(e, _)| (from.clone(), e)),
        );
    }
    for npc in toml_table("hub.toml")["npc"].as_array().unwrap() {
        let stock = npc.get("stock").and_then(|s| s.as_array());
        for e in stock.into_iter().flatten() {
            entries.push(("a Hub vendor".into(), e.as_integer().unwrap() as u32));
        }
    }
    for e in toml_table("wailing_caverns.toml")["boss_loot"]
        .as_array()
        .unwrap()
    {
        entries.push(("the boss".into(), e.as_integer().unwrap() as u32));
    }
    entries.sort_unstable_by_key(|(_, e)| *e);
    entries.dedup_by_key(|(_, e)| *e);
    assert!(entries.len() > 300, "only {} entries", entries.len());
    let mut realm = twilight_realm::Realm::new();
    for (from, entry) in entries {
        assert!(
            !outside_the_gear_slots(&mut realm, entry),
            "{from}: {entry}"
        );
    }
}

/// The templates `mobs.toml` flags `boss`: the significant creatures whose fights raise the Boss
/// Bar (#280), and none of the ten the spec leaves out.
#[test]
fn the_flagged_bosses_in_mobs_toml_are_exactly_the_spec_s() {
    let mobs = toml_table("mobs.toml");
    let templates = mobs["template"].as_array().unwrap();
    let mut flagged: Vec<&str> = templates
        .iter()
        .filter(|t| t.get("boss").and_then(|b| b.as_bool()) == Some(true))
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    flagged.sort_unstable();
    flagged.dedup();
    let mut want = vec![
        "Hogger",
        "Abbot Ashworth",
        "Garrick Padfoot",
        "Ghost Howl",
        "The Rake",
        "Bloodtalon Scythemaw",
        "Morgan the Collector",
        "Narg the Taskmaster",
        "Gllrg the Tidecaller",
        "Gresh the Drowned",
        "Speaker Thane",
        "The Kept",
        "Shadow of Morganth",
        "The Eye Below",
        "Ritualist Serane",
        "Speaker Vael",
        "Acolyte Marren",
        "Geomancer Vurx",
        "Princess",
    ];
    want.sort_unstable();
    assert_eq!(flagged, want);
    for name in [
        "Twilight Disciple",
        "Twilight Acolyte",
        "Lesser Voidwalker",
        "Voidwalker",
        "Shadowhide Gnoll",
        "Twilight Delver",
        "Elite Training Dummy",
    ] {
        assert!(!flagged.contains(&name), "{name} must not be a boss");
    }
}

#[test]
fn rumours_toml_has_its_rank_factors() {
    let table = toml_table("rumours.toml");
    let factors = table["rank_factor"].as_table().unwrap();
    for key in ["normal", "elite", "boss"] {
        assert!(factors[key].as_float().unwrap() > 0.0, "{key}");
    }
}
