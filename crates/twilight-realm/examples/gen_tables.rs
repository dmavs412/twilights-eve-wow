//! Writes `content/levels.toml` and `content/mob_class_stats.toml`, the per-level tables, from
//! `data/twilights-eve/xp_table.json`, the nine `<class>_level_stats.json` and
//! `mob_base_stats.json`: the realm cannot read JSON (the shipped app keeps `serde_json` out), so
//! it reads these TOML copies instead. `tests/content.rs` checks that they still agree.
//!
//! ```text
//! cargo run -p twilight-realm --example gen_tables
//! ```

use std::path::{Path, PathBuf};

use serde_json::Value;

/// The races in `player_levelstats`' order, as `content/levels.toml` names them.
const RACES: [&str; 8] = [
    "human",
    "orc",
    "dwarf",
    "night_elf",
    "undead",
    "tauren",
    "gnome",
    "troll",
];

/// The nine player Classes, in `ChrClasses.dbc` order; each has a `<class>_level_stats.json`.
const CLASSES: [&str; 9] = [
    "warrior", "paladin", "hunter", "rogue", "priest", "shaman", "mage", "warlock", "druid",
];

/// `creature_classlevelstats`' unit classes.
const UNIT_CLASSES: [&str; 3] = ["warrior", "paladin", "mage"];

fn main() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data = crate_dir.join("../../data/twilights-eve");
    let content = crate_dir.join("content");
    write(&content.join("levels.toml"), &levels(&data));
    write(
        &content.join("mob_class_stats.toml"),
        &mob_class_stats(&data),
    );
}

fn json(data: &Path, name: &str) -> Value {
    let text = std::fs::read_to_string(data.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn write(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    println!("wrote {}", path.display());
}

/// A number as TOML writes it: an integer bare, a float with its shortest round-trip digits.
fn number(v: &Value) -> String {
    match v.as_i64() {
        Some(i) => i.to_string(),
        None => format!("{:?}", v.as_f64().expect("a number")),
    }
}

/// `field` of every row, as a TOML array.
fn column(rows: &Value, field: &str) -> String {
    let cells: Vec<String> = rows
        .as_array()
        .expect("rows")
        .iter()
        .map(|row| number(&row[field]))
        .collect();
    format!("[{}]", cells.join(", "))
}

fn levels(data: &Path) -> String {
    let xp = json(data, "xp_table.json");
    let mut out = String::from(
        "# The per-level tables, generated from data/twilights-eve/xp_table.json and the nine\n\
         # <class>_level_stats.json (the vmangos world DB; see docs/reference/vanilla-combat-rules.md)\n\
         # by `cargo run -p twilight-realm --example gen_tables`. tests/content.rs checks that the\n\
         # two agree. Every array is indexed by level - 1.\n\
         \n\
         # player_xp_for_level: XP from a level to the next.\n",
    );
    out += &format!("xp_to_next = {}\n", column(&xp["levels"], "xp_to_next"));
    for class in CLASSES {
        let stats = json(data, &format!("{class}_level_stats.json"));
        let per_level = &stats["per_level"];
        out += &format!("\n[{class}]\n# player_classlevelstats\n");
        out += &format!("base_health = {}\n", column(per_level, "base_health"));
        out += &format!("base_mana = {}\n", column(per_level, "base_mana"));
        out += "# player_crit_per_agility: agility per 1% melee crit\n";
        out += &format!("agi_per_crit = {}\n", column(per_level, "agi_per_crit_pct"));
        out += "# player_dodge_per_agility: agility per 1% dodge\n";
        out += &format!(
            "agi_per_dodge = {}\n",
            column(per_level, "agi_per_dodge_pct")
        );
        out += &format!(
            "\n# player_levelstats: naked [str, agi, sta, int, spi], by race (a race vanilla never\n\
             # paired with the Class is synthesised; see the JSON's _synthesised_note)\n\
             [{class}.race_stats]\n"
        );
        for race in RACES {
            let Some(rows) = stats["race_base_stats"][race].as_array() else {
                continue;
            };
            let rows: Vec<String> = rows
                .iter()
                .map(|row| {
                    let cells: Vec<String> = ["str", "agi", "sta", "int", "spi"]
                        .iter()
                        .map(|f| number(&row[*f]))
                        .collect();
                    format!("[{}]", cells.join(", "))
                })
                .collect();
            out += &format!("{race} = [{}]\n", rows.join(", "));
        }
    }
    out
}

fn mob_class_stats(data: &Path) -> String {
    let reference = json(data, "mob_base_stats.json");
    let mut out = String::from(
        "# creature_classlevelstats by unit class, generated from data/twilights-eve/mob_base_stats.json\n\
         # (the vmangos world DB) by `cargo run -p twilight-realm --example gen_tables`.\n\
         # tests/content.rs checks that the two agree. Indexed by level - 1.\n",
    );
    for class in UNIT_CLASSES {
        let rows = &reference["by_unit_class"][class];
        out += &format!("\n[{class}]\n");
        for field in ["health", "melee_damage_avg", "armor"] {
            out += &format!("{field} = {}\n", column(rows, field));
        }
    }
    out
}
