//! Static content: where characters start, and what each race looks like and fights for.

/// Kalimdor, the continent The Barrens is on.
pub const KALIMDOR: u32 = 1;
/// `AreaTable.dbc`'s The Barrens.
pub const THE_BARRENS: u32 = 17;

/// The Hub: the Crossroads, in front of the inn, facing the road north. `[x, y, z, facing]`.
pub const CROSSROADS: [f32; 4] = [-441.8, -2648.6, 95.9, 1.6];

/// What every new character carries, `(player slot, item entry)`: vanilla's Warrior starting
/// weapon in the main hand, and a Hearthstone in the backpack's first slot.
const STARTING_ITEMS: [(u8, u32); 2] = [(15, WORN_SHORTSWORD), (23, HEARTHSTONE)];
const WORN_SHORTSWORD: u32 = 25;
const HEARTHSTONE: u32 = 6948;

/// A new character's belongings.
pub(crate) fn starting_inventory() -> crate::inventory::Inventory {
    let mut inventory = crate::inventory::Inventory::default();
    for (slot, entry) in STARTING_ITEMS {
        let item = inventory.new_item(entry, 1);
        inventory.put(slot, item);
    }
    inventory
}

/// A node in the advancement tree (`GLOSSARY.md`). Saved by name, so the names are the format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Calling {
    /// Tier 0, where every character starts.
    BasicWarrior,
    /// Tier 1, the Basic Warrior's Advancement.
    Warrior,
}

impl Calling {
    /// Its key in `content/callings.toml`, the same as its save name.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Calling::BasicWarrior => "basic_warrior",
            Calling::Warrior => "warrior",
        }
    }
}

/// `ChrRaces.dbc` ids 1..=8: Human … Troll.
pub(crate) fn valid_race(race: u8) -> bool {
    (1..=8).contains(&race)
}

/// A race's `(male, female)` native `CreatureDisplayInfo` ids (`ChrRaces.dbc`).
pub(crate) fn display_id(race: u8, gender: u8) -> u32 {
    let (male, female) = match race {
        1 => (49, 50),
        2 => (51, 52),
        3 => (53, 54),
        4 => (55, 56),
        5 => (57, 58),
        6 => (59, 60),
        7 => (1563, 1564),
        _ => (1478, 1479),
    };
    if gender == 0 {
        male
    } else {
        female
    }
}

/// A race's player `FactionTemplate.dbc` id (`ChrRaces.dbc`).
pub(crate) fn faction_template(race: u8) -> u32 {
    match race {
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        5 => 5,
        6 => 6,
        7 => 115,
        _ => 116,
    }
}
