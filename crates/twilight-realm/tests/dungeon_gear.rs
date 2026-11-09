//! The Dungeon pools and their bosses' rolls (#455): Jasperlode (entries 905000 to 905019), Azora
//! (905020 to 905039) and Hogger's camp (905040 to 905059 and the four Capstone Chests), and the
//! nine epics 905900 to 905908; then the Stockade's two sets (#579): the riot (905060 to 905079)
//! and the quiet wing (905080 to 905099); then the Deeprun Tram's two (#592): the Conductor's blues
//! (905100 to 905119) and the Platform greens (905120 to 905139); then Mistmantle Manor's set
//! (#642, 905140 to 905159); then the Tower of Ilgalar's (#644, 905160 to 905179). The phased
//! harness of `northshire_abbey.rs` cannot kill a boss in a dungeon cheaply, so these assert the
//! `rolls` data: chances, lists, no stand-in.

use std::collections::HashMap;

use toml::Value;

const JASPERLODE: &str = include_str!("../content/jasperlode.toml");
const AZORA: &str = include_str!("../content/azora.toml");
const FOREST_EDGE: &str = include_str!("../content/forest_edge.toml");
const STOCKADE_RIOT: &str = include_str!("../content/stockade_riot.toml");
const STOCKADE_QUIET_WING: &str = include_str!("../content/stockade_quiet_wing.toml");
const DEEPRUN_TRAM: &str = include_str!("../content/deeprun_tram.toml");
const MISTMANTLE_MANOR: &str = include_str!("../content/mistmantle_manor.toml");
const ILGALAR: &str = include_str!("../content/ilgalar.toml");
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
const RIOT20: (i64, i64) = (905060, 905079);
const QUIET20: (i64, i64) = (905080, 905099);
const CONDUCTOR20: (i64, i64) = (905100, 905119);
const PLATFORM20: (i64, i64) = (905120, 905139);
const MISTMANTLE20: (i64, i64) = (905140, 905159);
const ILGALAR20: (i64, i64) = (905160, 905179);

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
    let pools: [(&str, &str, Vec<i64>, i64); 5] = [
        ("jasperlode", JASPERLODE, range(J20.0, J20.1), 18),
        ("azora", AZORA, range(A20.0, A20.1), 22),
        ("forest_edge", FOREST_EDGE, range(905040, 905059), 30),
        (
            "stockade_riot",
            STOCKADE_RIOT,
            range(RIOT20.0, RIOT20.1),
            34,
        ),
        (
            "stockade_quiet_wing",
            STOCKADE_QUIET_WING,
            range(QUIET20.0, QUIET20.1),
            40,
        ),
    ];
    for (name, _, ids, level) in &pools {
        for id in ids {
            assert_eq!(rows.get(id), Some(&(3, *level)), "{name} row {id}");
        }
    }
    for id in range(905900, 905908) {
        assert_eq!(rows.get(&id).map(|r| r.0), Some(4), "epic {id}");
    }
    for text in [
        JASPERLODE,
        AZORA,
        FOREST_EDGE,
        STOCKADE_RIOT,
        STOCKADE_QUIET_WING,
    ] {
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
        ("stockade_riot", STOCKADE_RIOT),
        ("stockade_quiet_wing", STOCKADE_QUIET_WING),
    ] {
        assert!(!text.contains("900105"), "{name} still names 900105");
    }
}

/// The one boss row of `entry` in a dungeon file.
fn boss<'a>(file: &'a Value, entry: i64) -> &'a Value {
    let found: Vec<_> = creatures(file)
        .into_iter()
        .filter(|c| entry_of(c) == entry)
        .collect();
    assert_eq!(found.len(), 1, "boss {entry}");
    found[0]
}

#[test]
fn the_stockade_bosses_roll_their_wings_twenty_and_no_epic() {
    let riot = parse(STOCKADE_RIOT);
    let quiet = parse(STOCKADE_QUIET_WING);
    // Varl Cutter and Sister Quill half the time, Captain Rook and the Hush always.
    for (file, entry, chance, set) in [
        (&riot, 97202, 50, RIOT20),
        (&riot, 97204, 100, RIOT20),
        (&quiet, 97209, 50, QUIET20),
        (&quiet, 97210, 100, QUIET20),
    ] {
        let b = boss(file, entry);
        assert_eq!(
            rolls(b),
            vec![(chance, range(set.0, set.1))],
            "boss {entry}"
        );
        // Their corpses hold only their quests' items (#580), the 980000s, and no gear.
        assert!(
            drop_of(b).iter().all(|i| (980_000..981_000).contains(i)),
            "boss {entry}: {:?}",
            drop_of(b)
        );
    }
    let mobs = parse(MOBS);
    for entry in [97202, 97204, 97209, 97210] {
        assert!(rolls(template(&mobs, entry)).is_empty(), "template {entry}");
    }
}

#[test]
fn the_stockade_sets_are_twenty_blues_at_their_wings_item_level() {
    let rows = rows();
    for (set, level) in [(RIOT20, 34), (QUIET20, 40)] {
        for id in range(set.0, set.1) {
            assert_eq!(rows.get(&id), Some(&(3, level)), "row {id}");
        }
    }
}

#[test]
fn no_stockade_file_names_the_abbots_placeholder_blues() {
    for (name, text) in [
        ("stockade_riot", STOCKADE_RIOT),
        ("stockade_quiet_wing", STOCKADE_QUIET_WING),
    ] {
        for c in creatures(&parse(text)) {
            for (_, list) in rolls(c) {
                for id in list {
                    assert!(
                        !(A20.0..=A20.1).contains(&id),
                        "{name} still rolls the placeholder {id}"
                    );
                }
            }
        }
    }
}

/// The integers of `list`.
fn integers(list: &Value) -> Vec<i64> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|i| i.as_integer().unwrap())
        .collect()
}

#[test]
fn the_deeprun_sets_are_twenty_blues_and_twenty_greens_at_their_item_levels() {
    let rows = rows();
    for (set, quality, level) in [(CONDUCTOR20, 3, 42), (PLATFORM20, 2, 40)] {
        for id in range(set.0, set.1) {
            assert_eq!(rows.get(&id), Some(&(quality, level)), "row {id}");
        }
    }
}

#[test]
fn the_conductor_rolls_the_twenty_blues_and_the_chests_hold_their_sets() {
    let tram = &parse(DEEPRUN_TRAM)["tram"];
    let rolls = tram["conductor_rolls"].as_array().unwrap();
    assert_eq!(rolls.len(), 1, "one roll");
    assert_eq!(rolls[0]["chance"].as_integer(), Some(100));
    assert_eq!(
        integers(&rolls[0]["one_of"]),
        range(CONDUCTOR20.0, CONDUCTOR20.1)
    );
    assert_eq!(
        integers(&tram["blues"]),
        range(CONDUCTOR20.0, CONDUCTOR20.1)
    );
    assert_eq!(integers(&tram["greens"]), range(PLATFORM20.0, PLATFORM20.1));
    // His template rolls nothing of its own.
    assert!(rolls_of_template(97218).is_empty());
}

fn rolls_of_template(entry: i64) -> Vec<(i64, Vec<i64>)> {
    rolls(template(&parse(MOBS), entry))
}

/// The sheet row of `entry` in `gear.toml`.
fn item(gear: &Value, entry: i64) -> Option<&Value> {
    gear["item"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["entry"].as_integer() == Some(entry))
}

fn int(v: &Value, key: &str) -> i64 {
    v.get(key).and_then(Value::as_integer).unwrap_or(0)
}

#[test]
fn the_mistmantle_set_is_twenty_blues_at_36_with_one_chest_of_each_armour() {
    let gear = parse(GEAR);
    let mut chests = Vec::new();
    for id in range(MISTMANTLE20.0, MISTMANTLE20.1) {
        let row = item(&gear, id).unwrap_or_else(|| panic!("row {id} missing"));
        assert_eq!(
            (
                int(row, "quality"),
                int(row, "item_level"),
                int(row, "required_level")
            ),
            (3, 36, 33),
            "row {id}"
        );
        if int(row, "inventory_type") == 5 {
            chests.push(int(row, "subclass"));
        }
    }
    // Cloth, leather, mail and plate.
    chests.sort();
    assert_eq!(chests, vec![1, 2, 3, 4]);
}

#[test]
fn the_mistmantle_bosses_roll_the_twenty_and_no_stand_in() {
    let manor = parse(MISTMANTLE_MANOR);
    // Steward Aldric and Lady Corrine half the time, Stalvan always.
    for (guid, chance) in [(974010, 50), (974013, 50), (974014, 100)] {
        let b = creatures(&manor)
            .into_iter()
            .find(|c| c["guid"].as_integer() == Some(guid))
            .unwrap_or_else(|| panic!("boss {guid}"));
        assert_eq!(
            rolls(b),
            vec![(chance, range(MISTMANTLE20.0, MISTMANTLE20.1))],
            "boss {guid}"
        );
    }
    for id in range(QUIET20.0, QUIET20.1) {
        assert!(
            !MISTMANTLE_MANOR.contains(&id.to_string()),
            "the manor still names the stand-in {id}"
        );
    }
}

#[test]
fn the_ilgalar_set_is_twenty_blues_at_38_with_one_chest_of_each_armour() {
    let gear = parse(GEAR);
    let mut chests = Vec::new();
    for id in range(ILGALAR20.0, ILGALAR20.1) {
        let row = item(&gear, id).unwrap_or_else(|| panic!("row {id} missing"));
        assert_eq!(
            (
                int(row, "quality"),
                int(row, "item_level"),
                int(row, "required_level")
            ),
            (3, 38, 35),
            "row {id}"
        );
        if int(row, "inventory_type") == 5 {
            chests.push(int(row, "subclass"));
        }
    }
    // Cloth, leather, mail and plate.
    chests.sort();
    assert_eq!(chests, vec![1, 2, 3, 4]);
}

#[test]
fn the_ilgalar_bosses_roll_the_twenty_and_no_stand_in() {
    let tower = parse(ILGALAR);
    // The Blackrock Overseer half the time, Morganth always.
    for (guid, chance) in [(974111, 50), (974114, 100)] {
        let b = creatures(&tower)
            .into_iter()
            .find(|c| c["guid"].as_integer() == Some(guid))
            .unwrap_or_else(|| panic!("boss {guid}"));
        assert_eq!(
            rolls(b),
            vec![(chance, range(ILGALAR20.0, ILGALAR20.1))],
            "boss {guid}"
        );
    }
    for id in range(QUIET20.0, QUIET20.1) {
        assert!(
            !ILGALAR.contains(&id.to_string()),
            "the tower still names the stand-in {id}"
        );
    }
}
