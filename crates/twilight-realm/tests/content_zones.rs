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
