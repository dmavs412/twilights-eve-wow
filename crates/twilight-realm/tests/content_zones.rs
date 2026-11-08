//! `content/duskwood.toml`, `redridge.toml` and `stormwind.toml` are what
//! `examples/import_zone.rs` makes of their research dumps.

#[path = "../examples/import_zone.rs"]
#[allow(dead_code)]
mod import_zone;

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
