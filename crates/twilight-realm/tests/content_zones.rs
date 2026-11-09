//! `content/duskwood.toml`, `redridge.toml`, `stormwind.toml` and `burning_steppes.toml` are what
//! `examples/import_zone.rs` makes of their research dumps.

#[path = "../examples/import_zone.rs"]
#[allow(dead_code)]
mod import_zone;

use std::collections::HashSet;

/// The Burning Steppes spawns the realm loads after `zones.toml`'s removals (#634), as
/// `docs/reference/zone-spawns.md` gives it.
const STEPPES_LOADED: usize = 459;

#[test]
fn the_realm_loads_the_burning_steppes_spawns_the_doc_counts() {
    let zone = import_zone::ZONES
        .iter()
        .find(|z| z.toml == "burning_steppes.toml")
        .expect("the Burning Steppes is imported");
    let text = std::fs::read_to_string(import_zone::output_path(zone)).expect("the file reads");
    let file: toml::Value = toml::from_str(&text).expect("the file parses");
    let rows: HashSet<String> = file["spawns"]
        .as_array()
        .expect("spawns")
        .iter()
        .map(|r| format!("creature {} (entry {})", r[0], r[1]))
        .collect();
    let loaded = twilight_realm::placed()
        .into_iter()
        .filter(|p| p.map == 0 && rows.contains(&p.what))
        .count();
    assert_eq!(loaded, STEPPES_LOADED);
    let doc = include_str!("../../../docs/reference/zone-spawns.md");
    assert!(
        doc.contains(&format!("{STEPPES_LOADED} Burning Steppes spawns")),
        "zone-spawns.md gives another count"
    );
}

#[test]
fn each_zone_toml_is_the_import_of_its_dump() {
    for zone in import_zone::ZONES {
        let want = import_zone::convert(zone, &import_zone::dump(zone));
        let have = std::fs::read_to_string(import_zone::output_path(zone))
            .unwrap_or_default()
            .replace("\r\n", "\n");
        assert!(
            have == want,
            "content/{} ({}) is stale: run `cargo run -p twilight-realm --example import_zone`",
            zone.toml,
            zone.name
        );
    }
}

/// The Black Dragon Whelp (441) stood 35 times on Lake Everstill's shore, telling a level 35
/// player "dragons" ten levels before Stonewatch Keep shows them; `zones.toml` removes it from
/// the open world (#595 decision 8). A Phased Dungeon's own creature of that entry is allowed.
#[test]
fn no_black_dragon_whelp_spawns_in_the_open_world() {
    const WHELP: &str = "(entry 441)";
    let phased: Vec<String> = twilight_realm::audit_rows()
        .into_iter()
        .filter(|r| r.phased)
        .map(|r| r.what)
        .collect();
    let found: Vec<String> = twilight_realm::placed()
        .into_iter()
        .filter(|p| p.map == 0 && p.what.ends_with(WHELP) && !phased.contains(&p.what))
        .map(|p| p.what)
        .collect();
    assert!(found.is_empty(), "{}", found.join("\n"));
}
