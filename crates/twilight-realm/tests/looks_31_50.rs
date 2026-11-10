//! Looks that follow item level past 30 (#660, spec #650): the arc's Quest Line rows, Tharil'zun's
//! blues, the Thresher's and the courier's rewards, the four 30-50 dungeon sets and every plate
//! Chest of the Loot Bands 31-50 wear vanilla looks of their own. A row's model is its
//! `icon_base`, or its `display_id` when it has no icon; its icon is its `icon` path, or its stock
//! `display_id` when it has none.

use std::collections::HashMap;

use toml::Value;

const GEAR: &str = include_str!("../content/gear.toml");

/// `inventory_type` of a Chest and of a robe.
const CHEST: i64 = 5;
const ROBE: i64 = 20;

/// The Chests of #660: the Quest Lines' four bands, Tharil'zun's blues and the four dungeon sets
/// (cloth, leather, mail, plate each), and the Loot Bands' plate Chests.
const CHESTS: [i64; 40] = [
    903200, 903201, 903202, 903203, 903220, 903221, 903222, 903223, 903240, 903241, 903242, 903243,
    903260, 903261, 903262, 903263, 903306, 903307, 903308, 903309, 905140, 905141, 905142, 905143,
    905160, 905161, 905162, 905163, 905180, 905181, 905182, 905183, 905200, 905201, 905202, 905203,
    902103, 902123, 902143, 902163,
];

/// The first of each run of seven weapon, shield, bow and held rows: sword, shield, dagger, bow,
/// staff, mace and held item.
const MODEL_RUNS: [i64; 8] = [
    903204, 903224, 903244, 903264, 905144, 905164, 905184, 905204,
];

/// The Thresher's sword, dagger and staff, and the courier's shield, bow and held item.
const MODEL_ROWS: [i64; 6] = [903300, 903301, 903302, 903313, 903314, 903315];

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

fn is_chest(i: &Value) -> bool {
    matches!(int(i, "inventory_type"), CHEST | ROBE)
}

/// The model a row wears.
fn model(i: &Value) -> i64 {
    match int(i, "icon_base") {
        0 => int(i, "display_id"),
        base => base,
    }
}

/// What a row shows in a bag: its own icon path, or its stock display's icon.
fn icon(i: &Value) -> String {
    match i.get("icon").and_then(Value::as_str) {
        Some(path) if !path.is_empty() => path.to_lowercase(),
        _ => format!("display {}", int(i, "display_id")),
    }
}

fn wardrobe(i: &Value) -> Option<Vec<i64>> {
    i.get("wardrobe")
        .and_then(Value::as_array)
        .map(|cells| cells.iter().filter_map(Value::as_integer).collect())
}

#[test]
fn each_chest_wears_its_own_wardrobe_and_model() {
    let items = items();
    for entry in CHESTS {
        let i = item(&items, entry);
        assert!(is_chest(i), "{entry} is no Chest");
        assert!(int(i, "icon_base") > 0, "{entry} has no icon_base");
        let mine = wardrobe(i).unwrap_or_else(|| panic!("{entry} has no wardrobe"));
        assert!(
            mine.iter().any(|&c| c != 0),
            "{entry} wears an empty wardrobe"
        );
        for other in items
            .iter()
            .filter(|o| is_chest(o) && int(o, "entry") != entry)
        {
            let o = int(other, "entry");
            assert_ne!(
                wardrobe(other).as_ref(),
                Some(&mine),
                "{entry} and {o} wear one wardrobe"
            );
            assert_ne!(model(other), model(i), "{entry} and {o} wear one model");
        }
    }
}

#[test]
fn each_weapon_shield_bow_and_held_row_wears_its_own_model() {
    let items = items();
    let rows = MODEL_RUNS
        .iter()
        .flat_map(|&first| first..first + 7)
        .chain(MODEL_ROWS);
    for entry in rows {
        let i = item(&items, entry);
        let base = int(i, "icon_base");
        assert!(base > 0, "{entry} has no icon_base");
        for other in items.iter().filter(|o| int(o, "entry") != entry) {
            assert_ne!(
                model(other),
                base,
                "{entry} and {} wear one model",
                int(other, "entry")
            );
        }
    }
}

#[test]
fn every_quest_dungeon_and_world_line_shows_one_icon() {
    let items = items();
    let mut line_icon: HashMap<&str, String> = HashMap::new();
    for i in &items {
        let Some(l) = i.get("line").and_then(Value::as_str) else {
            continue;
        };
        if ["quest-", "dungeon-", "world-"]
            .iter()
            .any(|p| l.starts_with(p))
        {
            let shown = icon(i);
            let first = line_icon.entry(l).or_insert_with(|| shown.clone());
            assert_eq!(*first, shown, "{l} shows two icons ({})", int(i, "entry"));
        }
    }
    assert!(!line_icon.is_empty(), "no quest, dungeon or world Line");
}
