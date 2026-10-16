//! `content/areas.toml`, the areas a character can discover, against the client's
//! `AreaTable.dbc` it is generated from (`examples/gen_areas.rs`). Skipped without the game data.

use std::path::PathBuf;

#[derive(serde::Deserialize)]
struct File {
    area: Vec<Area>,
}

#[derive(serde::Deserialize, Debug, PartialEq)]
struct Area {
    id: u32,
    zone: u32,
    bit: u32,
    name: String,
}

fn areas() -> Vec<Area> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("content/areas.toml");
    let text = std::fs::read_to_string(&path).expect("content/areas.toml");
    toml::from_str::<File>(&text).expect("it parses").area
}

#[test]
fn every_barrens_subzone_the_spec_names_is_discoverable() {
    let areas = areas();
    for name in [
        "The Crossroads",
        "Thorn Hill",
        "Lushwater Oasis",
        "Camp Taurajo",
    ] {
        let a = areas.iter().find(|a| a.name == name).expect(name);
        assert_eq!(a.zone, 17, "{name} lies in The Barrens");
    }
}

#[test]
fn the_areas_match_the_clients_area_table() {
    let Some(data) = benilla_formats::wow_data() else {
        eprintln!("skipped: no WoW data (set WOW_DATA)");
        return;
    };
    let mut chain = benilla_formats::open_chain(&data).expect("the MPQs open");
    let table = benilla_formats::load_area_table_catalog(&mut chain).expect("AreaTable.dbc");
    let mut expected: Vec<Area> = table
        .entries()
        .filter(|(_, r)| r.map_id == 1)
        .map(|(id, r)| Area {
            id,
            zone: r.zone_id,
            bit: r.explore_flag,
            name: r.name.clone(),
        })
        .collect();
    expected.sort_by_key(|a| a.id);
    assert_eq!(
        areas(),
        expected,
        "rerun `cargo run -p twilight-realm --example gen_areas`"
    );
}
