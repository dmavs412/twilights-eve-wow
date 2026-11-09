//! The Loot Bands past 30 (spec #650): one row on each of the 20 `world-*` Lines and the six
//! `elite-*` Lines per band. Band 31-35 (#654) holds the greens 902100 to 902119 and the Elite
//! blues 904024 to 904029; band 36-40 the greens 902120 to 902139 and the blues 904030 to 904035.
//! Band 41-45 (#655) holds 902140 to 902159 and 904036 to 904041; band 46-50 902160 to 902179
//! and 904042 to 904047. Each green scales the 27-30 row of its Line (902080 + i), each blue the band's Elite row
//! (904018 + i). A row's icon here is its `icon` path, or its stock `display_id` when it has none;
//! `icon_check.py` compares the DBC's icon names behind those.

use std::collections::{HashMap, HashSet};

use toml::Value;

const GEAR: &str = include_str!("../content/gear.toml");

/// `bonding` of an Equip item.
const EQUIP: i64 = 2;

/// `inventory_type` of a Chest and of a robe.
const CHEST: i64 = 5;
const ROBE: i64 = 20;

/// Each band: first green, first Elite blue, item level, required level, and the Chest armour
/// for cloth, leather, mail and plate.
const BANDS: [(i64, i64, i64, i64, [i64; 4]); 4] = [
    (902100, 904024, 33, 31, [492, 861, 1475, 1844]),
    (902120, 904030, 38, 36, [570, 998, 1710, 2138]),
    (902140, 904036, 43, 41, [661, 1156, 1983, 2478]),
    (902160, 904042, 48, 46, [766, 1341, 2298, 2873]),
];

/// The 27-30 band's first green and first Elite blue: the sources of every row here.
const GREEN_SOURCE: i64 = 902080;
const ELITE_SOURCE: i64 = 904018;

fn items() -> Vec<Value> {
    let gear: Value = toml::from_str(GEAR).expect("parses");
    gear["item"].as_array().unwrap().clone()
}

fn item(items: &[Value], entry: i64) -> &Value {
    items
        .iter()
        .find(|i| i["entry"].as_integer() == Some(entry))
        .unwrap_or_else(|| panic!("{entry} is no gear row"))
}

fn int(i: &Value, key: &str) -> i64 {
    i.get(key).and_then(Value::as_integer).unwrap_or(0)
}

fn line(i: &Value) -> Option<&str> {
    i.get("line").and_then(Value::as_str)
}

/// What a row shows in a bag: its own icon path, or its stock display's icon.
fn icon(i: &Value) -> String {
    match i.get("icon").and_then(Value::as_str) {
        Some(path) if !path.is_empty() => path.to_lowercase(),
        _ => format!("display {}", int(i, "display_id")),
    }
}

fn is_chest(i: &Value) -> bool {
    matches!(int(i, "inventory_type"), CHEST | ROBE)
}

fn wardrobe(i: &Value) -> Option<Vec<i64>> {
    i.get("wardrobe")
        .and_then(Value::as_array)
        .map(|cells| cells.iter().filter_map(Value::as_integer).collect())
}

#[track_caller]
fn assert_row(i: &Value, quality: i64, item_level: i64, required_level: i64, want_line: &str) {
    let entry = int(i, "entry");
    assert_eq!(int(i, "quality"), quality, "{entry} quality");
    assert_eq!(int(i, "item_level"), item_level, "{entry} item level");
    assert_eq!(
        int(i, "required_level"),
        required_level,
        "{entry} required level"
    );
    assert_eq!(int(i, "bonding"), EQUIP, "{entry} binds on equip");
    assert_eq!(line(i), Some(want_line), "{entry} line");
}

#[test]
fn each_band_has_one_green_on_every_world_line() {
    let items = items();
    for (first, _, item_level, required, _) in BANDS {
        let mut seen = HashSet::new();
        for n in 0..20 {
            let source = item(&items, GREEN_SOURCE + n);
            let want = line(source).expect("the source has a line");
            assert!(want.starts_with("world-"), "{want} is no world Line");
            let i = item(&items, first + n);
            assert_row(i, 2, item_level, required, want);
            assert!(seen.insert(want), "{want} has two rows in one band");
        }
    }
}

#[test]
fn each_band_has_one_blue_on_every_elite_line() {
    let items = items();
    for (_, first, item_level, required, _) in BANDS {
        let mut seen = HashSet::new();
        for n in 0..6 {
            let source = item(&items, ELITE_SOURCE + n);
            let want = line(source).expect("the source has a line");
            assert!(want.starts_with("elite-"), "{want} is no elite Line");
            let i = item(&items, first + n);
            assert_row(i, 3, item_level, required, want);
            assert!(seen.insert(want), "{want} has two rows in one band");
        }
    }
}

#[test]
fn each_chest_has_its_bands_armour() {
    let items = items();
    for (first, _, item_level, _, armour) in BANDS {
        for (n, want) in armour.into_iter().enumerate() {
            let i = item(&items, first + n as i64);
            assert!(is_chest(i), "{} is no Chest", int(i, "entry"));
            assert_eq!(
                int(i, "armor"),
                want,
                "{} armour at item level {item_level}",
                int(i, "entry")
            );
        }
    }
}

#[test]
fn every_line_has_one_row_per_band_past_30() {
    let items = items();
    let mut rows: HashMap<&str, usize> = HashMap::new();
    for i in &items {
        let entry = int(i, "entry");
        let past_30 = (902100..=902179).contains(&entry) || (904024..=904047).contains(&entry);
        if let Some(l) = line(i).filter(|_| past_30) {
            *rows.entry(l).or_default() += 1;
        }
    }
    assert_eq!(rows.len(), 26, "20 world Lines and six elite Lines");
    for (l, n) in rows {
        assert_eq!(n, BANDS.len(), "{l} rows across the bands past 30");
    }
}

#[test]
fn every_world_and_elite_line_shows_one_icon() {
    let items = items();
    let mut line_icon: HashMap<&str, String> = HashMap::new();
    for i in &items {
        if let Some(l) = line(i).filter(|l| l.starts_with("world-") || l.starts_with("elite-")) {
            let shown = icon(i);
            let first = line_icon.entry(l).or_insert_with(|| shown.clone());
            assert_eq!(*first, shown, "{l} shows two icons");
        }
    }
    assert_eq!(line_icon.len(), 26, "20 world Lines and six elite Lines");
}

#[test]
fn the_new_cloth_leather_and_mail_chests_wear_their_own_wardrobes() {
    let items = items();
    for (first, _, _, _, _) in BANDS {
        // Cloth, leather and mail: the plate Chest keeps the 27-30 look for now.
        for n in 0..3 {
            let i = item(&items, first + n);
            let entry = int(i, "entry");
            let mine = wardrobe(i).unwrap_or_else(|| panic!("{entry} has no wardrobe"));
            assert!(
                mine.iter().any(|&c| c != 0),
                "{entry} wears an empty wardrobe"
            );
            for other in items
                .iter()
                .filter(|o| is_chest(o) && int(o, "entry") != entry)
            {
                assert_ne!(
                    wardrobe(other).as_ref(),
                    Some(&mine),
                    "{entry} and {} wear one wardrobe",
                    int(other, "entry")
                );
            }
        }
    }
}
