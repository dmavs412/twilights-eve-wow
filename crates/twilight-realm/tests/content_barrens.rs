//! `content/barrens.toml` is what `examples/import_barrens.rs` makes of the research dumps, and
//! every creature it spawns has a template.

#[path = "../examples/import_barrens.rs"]
#[allow(dead_code)]
mod import_barrens;

mod common;

use benilla_protocol::SessionEvent;
use common::*;

#[test]
fn barrens_toml_is_the_import_of_the_dumps() {
    let (barrens, low) = import_barrens::dumps();
    let want = import_barrens::convert(&barrens, &low);
    let have = std::fs::read_to_string(import_barrens::output_path())
        .unwrap()
        .replace("\r\n", "\n");
    assert!(
        have == want,
        "content/barrens.toml is stale: run `cargo run -p twilight-realm --example import_barrens`"
    );
}

#[test]
fn every_spawned_creature_answers_to_its_name() {
    let text = std::fs::read_to_string(import_barrens::output_path()).unwrap();
    let file: toml::Value = toml::from_str(&text).unwrap();
    let mut entries: Vec<i64> = file["spawns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r[1].as_integer().unwrap())
        .collect();
    entries.sort_unstable();
    entries.dedup();
    assert!(entries.len() > 140, "{} creatures", entries.len());

    let mut realm = realm();
    for entry in entries {
        let entry = entry as u32;
        realm.handle(RealmCommand::CreatureQuery { entry });
        let named = drain(&mut realm).into_iter().any(|e| {
            matches!(e, SessionEvent::CreatureName { entry: e, name: Some(_), .. } if e == entry)
        });
        assert!(named, "entry {entry} has no template");
    }
}
