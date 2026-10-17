//! Writes `content/items.toml` from `data/twilights-eve/loot_items.json`: the realm cannot read
//! JSON (the shipped app keeps `serde_json` out), so it reads this TOML copy instead.
//! `tests/content.rs` checks the two still agree. Gear (weapons and armour) is left out: it lives
//! in `data/twilights-eve/gear.csv` (ADR 0007), which `tools/twilights-eve/build_gear.py` turns
//! into `content/gear.toml`.
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
    let mut items: Vec<&Value> = reference["items"]
        .as_object()
        .unwrap()
        .values()
        .filter(|i| !is_gear(i))
        .collect();
    items.sort_by_key(|i| i["entry"].as_i64().unwrap());

    let mut out = String::from(
        "# The world-drop items of levels 1-32 that are no Gear (junk, bags), one per line.\n\
         # Generated from data/twilights-eve/loot_items.json (the vmangos world DB; see\n\
         # docs/reference/items-and-vendors.md) by `cargo run -p twilight-realm --example gen_items`.\n\
         # tests/content.rs checks that the two agree.\n\
         #\n\
         # A field left out is at its default: -1 for `allowable_class` and `allowable_race`, empty\n\
         # for the lists, 0 for the rest. `stats` are `[stat id, value]` (3 agility, 4 strength,\n\
         # 5 intellect, 6 spirit, 7 stamina). `resistances` are holy, fire, nature, frost, shadow,\n\
         # arcane. `damage` is `[min, max, school]`. `equip_spells` are the tooltip's \"Equip:\"\n\
         # spells, `equip_spell_names` their `Spell.dbc` names (tests/content_spell_ids.rs);\n\
         # `attack_power` and `defense` are what the realm applies of them. `pool_level` is\n\
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
}

/// Weapons and armour: Gear, which `content/gear.toml` holds.
fn is_gear(item: &Value) -> bool {
    matches!(item["class"].as_i64(), Some(2 | 4))
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
    let (mut spells, mut spell_names) = (Vec::new(), Vec::new());
    let (mut attack_power, mut defense) = (0, 0);
    for e in item["effects"].as_array().unwrap() {
        assert_eq!(e["trigger"], "equip");
        spells.push(e["spell"].to_string());
        spell_names.push(toml_string(e["name"].as_str().unwrap()));
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
        parts.push(format!("equip_spell_names = [{}]", spell_names.join(", ")));
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
