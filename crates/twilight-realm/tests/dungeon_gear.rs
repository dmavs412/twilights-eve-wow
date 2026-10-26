//! The three Dungeon pools and their bosses' rolls (#455): Jasperlode (entries 905000 to 905019),
//! Azora (905020 to 905039) and Hogger's camp (905040 to 905059 and the four Capstone Chests), and
//! the nine epics 905900 to 905908. The phased harness of `northshire_abbey.rs` cannot kill a
//! boss in a dungeon cheaply, so these assert the `rolls` data: chances, lists, no stand-in.

use std::collections::HashMap;

use toml::Value;

const JASPERLODE: &str = include_str!("../content/jasperlode.toml");
const AZORA: &str = include_str!("../content/azora.toml");
const FOREST_EDGE: &str = include_str!("../content/forest_edge.toml");
const MOBS: &str = include_str!("../content/mobs.toml");
const GEAR: &str = include_str!("../content/gear.toml");

/// Entry to (quality, item level) of every sheet row.
fn rows() -> HashMap<i64, (i64, i64)> {
    let gear: Value = toml::from_str(GEAR).expect("parses");
    gear["item"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            (
                i["entry"].as_integer().unwrap(),
                (
                    i.get("quality").and_then(Value::as_integer).unwrap_or(0),
                    i.get("item_level").and_then(Value::as_integer).unwrap_or(0),
                ),
            )
        })
        .collect()
}

fn parse(text: &str) -> Value {
    toml::from_str(text).expect("parses")
}

fn creatures(file: &Value) -> Vec<&Value> {
    file["creature"].as_array().unwrap().iter().collect()
}

fn entry_of(c: &Value) -> i64 {
    c["entry"].as_integer().unwrap()
}

/// A creature's rolls as (chance, list) pairs.
fn rolls(c: &Value) -> Vec<(i64, Vec<i64>)> {
    c.get("rolls")
        .map(|r| {
            r.as_array()
                .unwrap()
                .iter()
                .map(|roll| {
                    (
                        roll["chance"].as_integer().unwrap(),
                        roll["one_of"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|i| i.as_integer().unwrap())
                            .collect(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn drop_of(c: &Value) -> Vec<i64> {
    c.get("drop")
        .map(|d| {
            d.as_array()
                .unwrap()
                .iter()
                .map(|i| i.as_integer().unwrap())
                .collect()
        })
        .unwrap_or_default()
}

fn template(mobs: &Value, entry: i64) -> &Value {
    mobs["template"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["entry"].as_integer() == Some(entry))
        .unwrap_or_else(|| panic!("template {entry}"))
}

fn range(from: i64, to: i64) -> Vec<i64> {
    (from..=to).collect()
}

const J20: (i64, i64) = (905000, 905019);
const A20: (i64, i64) = (905020, 905039);

fn hogger_pool() -> Vec<i64> {
    let mut v = range(905040, 905059);
    v.extend(903108..=903111);
    v
}

#[test]
fn the_eye_below_rolls_an_epic_or_a_jasperlode_blue_and_never_the_stand_in() {
    let file = parse(JASPERLODE);
    let eye: Vec<_> = creatures(&file)
        .into_iter()
        .filter(|c| entry_of(c) == 96022)
        .collect();
    assert_eq!(eye.len(), 1);
    assert_eq!(
        rolls(eye[0]),
        vec![
            (5, vec![905900, 905901, 905902]),
            (100, range(J20.0, J20.1))
        ]
    );
    assert!(!drop_of(eye[0]).contains(&900105));
}

#[test]
fn twilight_delvers_roll_a_jasperlode_blue_at_one_percent_and_tentacles_nothing() {
    let file = parse(JASPERLODE);
    let delvers: Vec<_> = creatures(&file)
        .into_iter()
        .filter(|c| entry_of(c) == 96020)
        .collect();
    assert_eq!(delvers.len(), 3);
    for c in delvers {
        assert_eq!(rolls(c), vec![(1, range(J20.0, J20.1))]);
    }
    for c in creatures(&file)
        .into_iter()
        .filter(|c| entry_of(c) == 96021)
    {
        assert!(rolls(c).is_empty() && drop_of(c).is_empty());
    }
}

#[test]
fn azora_stop_bosses_roll_half_the_time_and_morganth_rolls_an_epic_or_a_blue() {
    let file = parse(AZORA);
    for boss in [96034, 96035, 96036] {
        let found: Vec<_> = creatures(&file)
            .into_iter()
            .filter(|c| entry_of(c) == boss)
            .collect();
        assert_eq!(found.len(), 1, "boss {boss}");
        assert_eq!(rolls(found[0]), vec![(50, range(A20.0, A20.1))]);
        assert!(drop_of(found[0]).is_empty(), "boss {boss}");
    }
    let morganth = creatures(&file)
        .into_iter()
        .find(|c| entry_of(c) == 96037)
        .expect("the Shadow of Morganth");
    assert_eq!(drop_of(morganth), vec![980021, 980021, 980021]);
    assert_eq!(
        rolls(morganth),
        vec![
            (5, vec![905903, 905904, 905905]),
            (100, range(A20.0, A20.1))
        ]
    );
}

#[test]
fn azoras_rank_one_creatures_roll_an_azora_blue_at_one_percent() {
    let file = parse(AZORA);
    let mut seen = 0;
    for c in creatures(&file) {
        if [96031, 96032, 96033].contains(&entry_of(c)) {
            assert_eq!(rolls(c), vec![(1, range(A20.0, A20.1))]);
            seen += 1;
        }
    }
    assert_eq!(seen, 9);
    // The Shadowhide Gnolls (96038) only come in through the Shadow's `every`, so the template
    // carries the roll.
    let mobs = parse(MOBS);
    assert_eq!(
        rolls(template(&mobs, 96038)),
        vec![(1, range(A20.0, A20.1))]
    );
}

#[test]
fn hogger_rolls_an_epic_or_one_of_twenty_four_and_his_template_rolls_nothing() {
    let file = parse(FOREST_EDGE);
    let hogger: Vec<_> = creatures(&file)
        .into_iter()
        .filter(|c| entry_of(c) == 96010)
        .collect();
    assert_eq!(hogger.len(), 1);
    assert_eq!(
        rolls(hogger[0]),
        vec![(5, vec![905906, 905907, 905908]), (100, hogger_pool())]
    );
    let mobs = parse(MOBS);
    assert!(rolls(template(&mobs, 96010)).is_empty());
}

#[test]
fn every_dungeon_roll_names_a_sheet_row_of_the_right_quality_and_level() {
    let rows = rows();
    let pools: [(&str, &str, Vec<i64>, i64); 3] = [
        ("jasperlode", JASPERLODE, range(J20.0, J20.1), 18),
        ("azora", AZORA, range(A20.0, A20.1), 22),
        ("forest_edge", FOREST_EDGE, range(905040, 905059), 30),
    ];
    for (name, _, ids, level) in &pools {
        for id in ids {
            assert_eq!(rows.get(id), Some(&(3, *level)), "{name} row {id}");
        }
    }
    for id in range(905900, 905908) {
        assert_eq!(rows.get(&id).map(|r| r.0), Some(4), "epic {id}");
    }
    for text in [JASPERLODE, AZORA, FOREST_EDGE] {
        for c in creatures(&parse(text)) {
            for (_, list) in rolls(c) {
                for id in list {
                    assert!(rows.contains_key(&id), "roll names {id}, not a sheet row");
                }
            }
        }
    }
}

#[test]
fn no_dungeon_file_names_the_abbots_stand_in() {
    for (name, text) in [
        ("jasperlode", JASPERLODE),
        ("azora", AZORA),
        ("forest_edge", FOREST_EDGE),
    ] {
        assert!(!text.contains("900105"), "{name} still names 900105");
    }
}
