//! The Quest Lines of the 30-50 arc (#600, spec #595): 20 green `quest-*` Lines and 16 blue
//! `quest-elite-*` Lines that every later quest reward of the arc extends. Band 30-35 holds the
//! greens 903200 to 903219 and The Thresher's blues 903300 to 903302. Each Line shows one icon that
//! no other Line or unlined item shows. A row's icon here is its `icon` path, or its stock
//! `display_id` when it has none; `icon_check.py` compares the DBC's icon names behind those.

use std::collections::{HashMap, HashSet};

use toml::Value;

const GEAR: &str = include_str!("../content/gear.toml");

/// `bonding` of a Pickup item.
const PICKUP: i64 = 1;

const GREEN_LINES: [&str; 20] = [
    "quest-chest-cloth",
    "quest-chest-leather",
    "quest-chest-mail",
    "quest-chest-plate",
    "quest-sword",
    "quest-shield",
    "quest-dagger",
    "quest-bow",
    "quest-staff",
    "quest-mace",
    "quest-held",
    "quest-amulet-might",
    "quest-amulet-swift",
    "quest-amulet-mind",
    "quest-ring-might-a",
    "quest-ring-might-b",
    "quest-ring-swift-a",
    "quest-ring-swift-b",
    "quest-ring-mind-a",
    "quest-ring-mind-b",
];

/// The Thresher's blues (98107): entry and Line.
const THRESHER: [(i64, &str); 3] = [
    (903300, "quest-elite-sword"),
    (903301, "quest-elite-dagger"),
    (903302, "quest-elite-staff"),
];

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

fn is_quest_line(line: &str) -> bool {
    line.starts_with("quest-")
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
    assert_eq!(int(i, "bonding"), PICKUP, "{entry} binds on pickup");
    assert_eq!(line(i), Some(want_line), "{entry} line");
}

#[test]
fn band_30_35_greens_are_one_row_of_each_quest_line() {
    let items = items();
    let mut seen = HashSet::new();
    for (n, want) in GREEN_LINES.iter().enumerate() {
        let i = item(&items, 903200 + n as i64);
        assert_row(i, 2, 33, 30, want);
        assert!(
            seen.insert(line(i).unwrap()),
            "{want} has two rows in band 30-35"
        );
    }
}

#[test]
fn the_thresher_pays_three_quest_elite_blues() {
    let items = items();
    for (entry, want) in THRESHER {
        assert_row(item(&items, entry), 3, 33, 30, want);
    }
}

#[test]
fn every_quest_line_shows_one_icon_no_other_item_shows() {
    let items = items();
    let mut line_icon: HashMap<&str, String> = HashMap::new();
    for i in &items {
        if let Some(l) = line(i).filter(|l| is_quest_line(l)) {
            let shown = icon(i);
            let first = line_icon.entry(l).or_insert_with(|| shown.clone());
            assert_eq!(*first, shown, "{l} shows two icons");
        }
    }
    for want in GREEN_LINES.iter().chain(THRESHER.iter().map(|(_, l)| l)) {
        assert!(line_icon.contains_key(want), "{want} has no row");
    }
    let owner: HashMap<&String, &str> = line_icon.iter().map(|(l, s)| (s, *l)).collect();
    assert_eq!(
        owner.len(),
        line_icon.len(),
        "two Quest Lines share an icon"
    );
    for i in &items {
        if line(i).is_some_and(is_quest_line) {
            continue;
        }
        if let Some(l) = owner.get(&icon(i)) {
            panic!("{} shows {l}'s icon", int(i, "entry"));
        }
    }
}
