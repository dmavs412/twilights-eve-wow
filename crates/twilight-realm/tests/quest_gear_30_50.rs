//! The Quest Lines of the 30-50 arc (#600, spec #595): 20 green `quest-*` Lines and 16 blue
//! `quest-elite-*` Lines that every later quest reward of the arc extends. Band 30-35 holds the
//! greens 903200 to 903219 and The Thresher's blues 903300 to 903302; band 35-40 (#605) the greens
//! 903220 to 903239 and Sven's Revenge's blue rings 903303 to 903305; band 40-45 (#610) the greens
//! 903240 to 903259 and Tharil'zun's Chests 903306 to 903309; band 45-50 (#615) the greens 903260
//! to 903279 and the named elites' blues 903310 to 903318. Each Line shows one icon that
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

/// Sven's Revenge's blues (98706): entry and Line.
const SVENS_REVENGE: [(i64, &str); 3] = [
    (903303, "quest-elite-ring-might"),
    (903304, "quest-elite-ring-swift"),
    (903305, "quest-elite-ring-mind"),
];

/// Tharil'zun's blue Chests (98304): entry and Line.
const THARILZUN: [(i64, &str); 4] = [
    (903306, "quest-elite-chest-cloth"),
    (903307, "quest-elite-chest-leather"),
    (903308, "quest-elite-chest-mail"),
    (903309, "quest-elite-chest-plate"),
];

/// The band 45-50 named elites' blues: The Grey Captain (98902), The Last Courier (98404) and
/// Mor'Ladim (98903). Entry and Line.
const BAND_45_50_ELITES: [(i64, &str); 9] = [
    (903310, "quest-elite-amulet-might"),
    (903311, "quest-elite-amulet-swift"),
    (903312, "quest-elite-amulet-mind"),
    (903313, "quest-elite-shield"),
    (903314, "quest-elite-bow"),
    (903315, "quest-elite-held"),
    (903316, "quest-elite-ring-might"),
    (903317, "quest-elite-ring-swift"),
    (903318, "quest-elite-ring-mind"),
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
fn band_35_40_greens_repeat_each_quest_line_with_its_icon() {
    let items = items();
    let mut seen = HashSet::new();
    for (n, want) in GREEN_LINES.iter().enumerate() {
        let i = item(&items, 903220 + n as i64);
        assert_row(i, 2, 38, 36, want);
        assert!(
            seen.insert(line(i).unwrap()),
            "{want} has two rows in band 35-40"
        );
        let first = item(&items, 903200 + n as i64);
        assert_eq!(
            icon(i),
            icon(first),
            "{want} changes its icon in band 35-40"
        );
    }
}

#[test]
fn band_40_45_greens_repeat_each_quest_line_with_its_icon() {
    let items = items();
    let mut seen = HashSet::new();
    for (n, want) in GREEN_LINES.iter().enumerate() {
        let i = item(&items, 903240 + n as i64);
        assert_row(i, 2, 43, 40, want);
        assert!(
            seen.insert(line(i).unwrap()),
            "{want} has two rows in band 40-45"
        );
        let first = item(&items, 903200 + n as i64);
        assert_eq!(
            icon(i),
            icon(first),
            "{want} changes its icon in band 40-45"
        );
    }
}

#[test]
fn band_45_50_greens_repeat_each_quest_line_with_its_icon() {
    let items = items();
    let mut seen = HashSet::new();
    for (n, want) in GREEN_LINES.iter().enumerate() {
        let i = item(&items, 903260 + n as i64);
        assert_row(i, 2, 48, 45, want);
        assert!(
            seen.insert(line(i).unwrap()),
            "{want} has two rows in band 45-50"
        );
        let first = item(&items, 903200 + n as i64);
        assert_eq!(
            icon(i),
            icon(first),
            "{want} changes its icon in band 45-50"
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
fn svens_revenge_pays_three_quest_elite_rings() {
    let items = items();
    for (entry, want) in SVENS_REVENGE {
        assert_row(item(&items, entry), 3, 38, 36, want);
    }
}

#[test]
fn tharilzun_pays_one_quest_elite_chest_per_armour_type() {
    let items = items();
    let mut types = HashSet::new();
    for (entry, want) in THARILZUN {
        let i = item(&items, entry);
        assert_row(i, 3, 43, 40, want);
        assert_eq!(int(i, "inventory_type"), 5, "{entry} is a Chest");
        types.insert(int(i, "subclass"));
    }
    assert_eq!(types.len(), 4, "Tharil'zun's Chests repeat an armour type");
}

#[test]
fn band_45_50_elites_pay_nine_quest_elite_blues() {
    let items = items();
    for (entry, want) in BAND_45_50_ELITES {
        assert_row(item(&items, entry), 3, 48, 45, want);
    }
    for (later, (earlier, _)) in BAND_45_50_ELITES[6..].iter().zip(SVENS_REVENGE) {
        assert_eq!(
            icon(item(&items, later.0)),
            icon(item(&items, earlier)),
            "{} changes its Line's icon",
            later.0
        );
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
    let blues = THRESHER
        .iter()
        .chain(SVENS_REVENGE.iter())
        .chain(THARILZUN.iter())
        .chain(BAND_45_50_ELITES.iter());
    for want in GREEN_LINES.iter().chain(blues.map(|(_, l)| l)) {
        assert!(line_icon.contains_key(want), "{want} has no row");
    }
    assert_eq!(line_icon.len(), 36, "the arc has 36 Quest Lines");
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
