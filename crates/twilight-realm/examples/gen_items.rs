//! Writes `content/items.toml` from `data/twilights-eve/loot_items.json`: the realm cannot read
//! JSON (the shipped app keeps `serde_json` out), so it reads this TOML copy instead.
//! `tests/content.rs` checks the two still agree.
//!
//! ```text
//! cargo run -p twilight-realm --example gen_items
//! ```

use std::path::PathBuf;

use serde_json::Value;

/// Integer fields, written only when they differ from their default (`-1` for the two
/// `allowable_*` masks, 0 for the rest).
const INTS: [&str; 24] = [
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
    "entry",
];

fn main() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = crate_dir.join("../../data/twilights-eve/loot_items.json");
    let reference: Value =
        serde_json::from_str(&std::fs::read_to_string(&src).expect("loot_items.json")).unwrap();
    let mut items: Vec<&Value> = reference["items"].as_object().unwrap().values().collect();
    items.sort_by_key(|i| i["entry"].as_i64().unwrap());

    let mut out = String::from(
        "# The items the realm knows: every world-drop item of levels 1-32, one per line.\n\
         # Generated from data/twilights-eve/loot_items.json (the vmangos world DB; see\n\
         # docs/reference/items-and-vendors.md) by `cargo run -p twilight-realm --example gen_items`.\n\
         # tests/content.rs checks that the two agree.\n\
         #\n\
         # A field left out is at its default: -1 for `allowable_class` and `allowable_race`, empty\n\
         # for the lists, 0 for the rest. `stats` are `[stat id, value]` (3 agility, 4 strength,\n\
         # 5 intellect, 6 spirit, 7 stamina). `resistances` are holy, fire, nature, frost, shadow,\n\
         # arcane. `damage` is `[min, max, school]`. `equip_spells` are the tooltip's \"Equip:\"\n\
         # spells; `attack_power` and `defense` are what the realm applies of them. `pool_level` is\n\
         # the loot level the item drops at.\n\n\
         item = [\n",
    );
    for item in &items {
        out.push_str(&row(item));
    }
    out.push_str("]\n");
    let dst = crate_dir.join("content/items.toml");
    std::fs::write(&dst, out).unwrap();
    println!("wrote {}", dst.display());

    // The Wailing Caverns boss pool's own drops, which no world pool holds.
    let known: Vec<i64> = items.iter().map(|i| i["entry"].as_i64().unwrap()).collect();
    let blues = &reference["dungeon_blues"];
    let pool: Vec<i64> = blues["proposed_boss_pool"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e.as_i64().unwrap())
        .filter(|e| !known.contains(e))
        .collect();
    let mut dungeon: Vec<&Value> = blues["wailing_caverns"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| pool.contains(&i["entry"].as_i64().unwrap()))
        .collect();
    dungeon.sort_by_key(|i| i["entry"].as_i64().unwrap());
    let mut out = String::from(
        "# The Wailing Caverns boss's own blues: the entries of `proposed_boss_pool` that no world\n\
         # pool holds, from `dungeon_blues.wailing_caverns` in data/twilights-eve/loot_items.json.\n\
         # Generated with items.toml by `cargo run -p twilight-realm --example gen_items`; the same\n\
         # fields and defaults. tests/content.rs checks that the two agree.\n\n\
         item = [\n",
    );
    for item in dungeon {
        out.push_str(&row(item));
    }
    out.push_str("]\n");
    let dst = crate_dir.join("content/dungeon_items.toml");
    std::fs::write(&dst, out).unwrap();
    println!("wrote {}", dst.display());
}

/// One item's line in the table.
fn row(item: &Value) -> String {
    let mut parts = vec![
        format!("entry = {}", item["entry"]),
        format!("name = {}", toml_string(item["name"].as_str().unwrap())),
    ];
    for k in INTS.iter().filter(|k| **k != "entry") {
        let v = item[*k].as_i64().unwrap_or(0);
        let default = if k.starts_with("allowable_") { -1 } else { 0 };
        if v != default {
            parts.push(format!("{k} = {v}"));
        }
    }
    let stats: Vec<String> = item["stats"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| format!("[{}, {}]", stat_id(s["stat"].as_str().unwrap()), s["value"]))
        .collect();
    if !stats.is_empty() {
        parts.push(format!("stats = [{}]", stats.join(", ")));
    }
    let resistances: Vec<i64> = ["holy", "fire", "nature", "frost", "shadow", "arcane"]
        .iter()
        .map(|s| item["resistances"][s].as_i64().unwrap_or(0))
        .collect();
    if resistances.iter().any(|r| *r != 0) {
        let r: Vec<String> = resistances.iter().map(i64::to_string).collect();
        parts.push(format!("resistances = [{}]", r.join(", ")));
    }
    let damage: Vec<String> = item["damage"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            format!(
                "[{:?}, {:?}, {}]",
                d["min"].as_f64().unwrap(),
                d["max"].as_f64().unwrap(),
                school_id(d["school"].as_str().unwrap())
            )
        })
        .collect();
    if !damage.is_empty() {
        parts.push(format!("damage = [{}]", damage.join(", ")));
    }
    let (mut spells, mut attack_power, mut defense) = (Vec::new(), 0, 0);
    for e in item["effects"].as_array().unwrap() {
        assert_eq!(e["trigger"], "equip");
        spells.push(e["spell"].to_string());
        for m in e["mods"].as_array().unwrap() {
            match (m["stat"].as_str().unwrap(), m["skill"].as_i64()) {
                ("attack_power", _) => attack_power += m["value"].as_i64().unwrap(),
                ("skill", Some(95)) => defense += m["value"].as_i64().unwrap(),
                _ => {}
            }
        }
    }
    if !spells.is_empty() {
        parts.push(format!("equip_spells = [{}]", spells.join(", ")));
    }
    if attack_power != 0 {
        parts.push(format!("attack_power = {attack_power}"));
    }
    if defense != 0 {
        parts.push(format!("defense = {defense}"));
    }
    format!("  {{ {} }},\n", parts.join(", "))
}

fn toml_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

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
