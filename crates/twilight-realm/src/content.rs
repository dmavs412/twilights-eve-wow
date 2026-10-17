//! Static content: where characters start, and what each race looks like and fights for.

/// Kalimdor, the continent The Barrens is on.
pub const KALIMDOR: u32 = 1;
/// `AreaTable.dbc`'s The Barrens.
pub const THE_BARRENS: u32 = 17;

/// The Hub: the Crossroads, in front of the inn, facing the road north. `[x, y, z, facing]`.
pub const CROSSROADS: [f32; 4] = [-441.8, -2648.6, 95.9, 1.6];

/// The races in `ChrRaces.dbc` order (ids 1..=8), as `content/starting_outfits.toml` names them.
const RACE_KEYS: [&str; 8] = [
    "human",
    "orc",
    "dwarf",
    "night_elf",
    "undead",
    "tauren",
    "gnome",
    "troll",
];
pub(crate) const HEARTHSTONE: u32 = 6948;
/// `INVENTORY_SLOT_ITEM_START`: the backpack's first slot.
const FIRST_PACK_SLOT: u8 = 23;

/// One item of an outfit; the display id and inventory type are there for the content test.
#[derive(serde::Deserialize)]
struct OutfitItem {
    item: u32,
}

/// A race's Warrior starting outfit (`content/starting_outfits.toml`, from `CharStartOutfit.dbc`).
fn starting_outfit(race: u8) -> impl Iterator<Item = u32> {
    static OUTFITS: std::sync::OnceLock<std::collections::HashMap<String, Vec<OutfitItem>>> =
        std::sync::OnceLock::new();
    let outfits = OUTFITS.get_or_init(|| {
        toml::from_str(include_str!("../content/starting_outfits.toml"))
            .expect("content/starting_outfits.toml parses")
    });
    usize::from(race)
        .checked_sub(1)
        .and_then(|i| RACE_KEYS.get(i))
        .and_then(|key| outfits.get(*key))
        .into_iter()
        .flatten()
        .map(|it| it.item)
}

/// A new character's belongings: its race's Warrior outfit, each item worn in its slot, and a
/// Hearthstone in the backpack's first slot.
pub(crate) fn starting_inventory(race: u8) -> crate::inventory::Inventory {
    let mut inventory = crate::inventory::Inventory::default();
    for entry in starting_outfit(race) {
        let slot = crate::items::template(entry)
            .and_then(|t| crate::rules::gear::equip_slots(t.inventory_type).first())
            .copied();
        if let Some(slot) = slot.filter(|s| inventory.get(*s).is_none()) {
            let item = inventory.new_item(entry, 1);
            inventory.put(slot, item);
        }
    }
    let stone = inventory.new_item(HEARTHSTONE, 1);
    inventory.put(FIRST_PACK_SLOT, stone);
    inventory
}

/// A node in the advancement tree (`GLOSSARY.md`): a Calling's key in `content/callings.toml`
/// (`basic_warrior`, Tier 0, where every character starts; `vanguard`, its Advancement;
/// `zealot`, Vanguard's) or one a test added through `Realm::with_callings`. Saved by that key,
/// so the keys are the format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Calling(&'static str);

impl Calling {
    /// Tier 0, where every character starts.
    pub(crate) const BASIC_WARRIOR: Calling = Calling("basic_warrior");

    /// Its key in `content/callings.toml`, the same as its save name.
    pub(crate) fn key(self) -> &'static str {
        self.0
    }

    /// The Calling with this key. A save from before the rename may say "warrior" for the
    /// Vanguard.
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        let key = if key == "warrior" { "vanguard" } else { key };
        crate::callings::calling(key).map(|def| Calling(def.key.as_str()))
    }
}

impl serde::Serialize for Calling {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.0)
    }
}

impl<'de> serde::Deserialize<'de> for Calling {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let key = String::deserialize(d)?;
        Self::from_key(&key)
            .ok_or_else(|| serde::de::Error::custom(format!("an unknown Calling {key:?}")))
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
