//! `content/elwynn.toml` is what `examples/import_elwynn.rs` makes of the research dump.

#[path = "../examples/import_elwynn.rs"]
#[allow(dead_code)]
mod import_elwynn;

#[test]
fn elwynn_toml_is_the_import_of_the_dump() {
    let want = import_elwynn::convert(&import_elwynn::dump());
    let have = std::fs::read_to_string(import_elwynn::output_path())
        .unwrap()
        .replace("\r\n", "\n");
    assert!(
        have == want,
        "content/elwynn.toml is stale: run `cargo run -p twilight-realm --example import_elwynn`"
    );
}

/// The nine Elwynn elites (#454) each roll their band's Elite pool at 20% and the band's World
/// greens at 50%; the pool's rows are Rare at the band's set level.
#[test]
fn the_nine_elwynn_elites_roll_their_elite_pool_then_their_bands_greens() {
    let mobs: toml::Value = toml::from_str(include_str!("../content/mobs.toml")).expect("parses");
    let gear: toml::Value = toml::from_str(include_str!("../content/gear.toml")).expect("parses");
    let rows: std::collections::HashMap<i64, (i64, i64)> = gear["item"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            (
                i["entry"].as_integer().unwrap(),
                (
                    i.get("quality").and_then(|q| q.as_integer()).unwrap_or(0),
                    i.get("item_level")
                        .and_then(|l| l.as_integer())
                        .unwrap_or(0),
                ),
            )
        })
        .collect();
    // (template, band's set level)
    let elites = [
        (330, 12),
        (96012, 12),
        (79, 15),
        (96014, 15),
        (96025, 15),
        (96040, 23),
        (96043, 23),
        (97013, 28),
        (97014, 28),
    ];
    for (entry, set_level) in elites {
        let template = mobs["template"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["entry"].as_integer() == Some(entry))
            .unwrap_or_else(|| panic!("template {entry}"));
        let rolls = template["rolls"].as_array().expect("rolls");
        assert_eq!(rolls.len(), 2, "template {entry}");
        assert_eq!(
            rolls[0]["chance"].as_integer(),
            Some(20),
            "template {entry}"
        );
        assert_eq!(
            rolls[1]["chance"].as_integer(),
            Some(50),
            "template {entry}"
        );
        for (n, roll) in rolls.iter().enumerate() {
            for item in roll["one_of"].as_array().unwrap() {
                let id = item.as_integer().unwrap();
                let (quality, level) = rows
                    .get(&id)
                    .unwrap_or_else(|| panic!("template {entry} rolls {id}, not a sheet row"));
                if n == 0 {
                    assert_eq!((*quality, *level), (3, set_level), "elite row {id}");
                }
            }
        }
        assert_eq!(rolls[0]["one_of"].as_array().unwrap().len(), 6);
        assert_eq!(rolls[1]["one_of"].as_array().unwrap().len(), 20);
    }
}
