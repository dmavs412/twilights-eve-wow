//! Static content: where characters start, and what each race looks like and fights for.

/// Kalimdor, the continent The Barrens is on.
pub const KALIMDOR: u32 = 1;
/// Eastern Kingdoms, the continent Elwynn Forest is on.
pub const EASTERN_KINGDOMS: u32 = 0;

/// Whether `map` is an open-world continent, whose spawns stream in around the player, rather
/// than a Dungeon's instance.
pub(crate) fn open_world(map: u32) -> bool {
    matches!(map, KALIMDOR | EASTERN_KINGDOMS)
}
/// `AreaTable.dbc`'s The Barrens.
pub const THE_BARRENS: u32 = 17;

/// `AreaTable.dbc`'s Elwynn Forest, and its Northshire Valley.
pub const ELWYNN_FOREST: u32 = 12;
pub const NORTHSHIRE_VALLEY: u32 = 9;

/// Northshire Abbey's steps, where a character who chose Elwynn Forest first appears: the classic
/// human start (vmangos `playercreateinfo`). `[x, y, z, facing]`.
pub const NORTHSHIRE: [f32; 4] = [-8949.95, -132.493, 83.5312, 0.0];

/// The Hub: the Crossroads inn's hall, where characters start, facing the bar. `[x, y, z, facing]`.
pub const CROSSROADS: [f32; 4] = [-417.0, -2646.0, 96.0, 0.0];

pub(crate) const HEARTHSTONE: u32 = 6948;
/// `INVENTORY_SLOT_ITEM_START`: the backpack's first slot.
const FIRST_PACK_SLOT: u8 = 23;

/// A Calling's starting outfit (`content/starting_outfits.toml`): one starter Chest and its
/// weapons, ranged weapon and ammo, all Gear.
#[derive(serde::Deserialize)]
pub(crate) struct Outfit {
    pub(crate) items: Vec<u32>,
}

/// Every Calling's starting outfit, by Calling key.
pub(crate) fn starting_outfits() -> &'static std::collections::HashMap<String, Outfit> {
    static OUTFITS: std::sync::OnceLock<std::collections::HashMap<String, Outfit>> =
        std::sync::OnceLock::new();
    OUTFITS.get_or_init(|| {
        toml::from_str(include_str!("../content/starting_outfits.toml"))
            .expect("content/starting_outfits.toml parses")
    })
}

/// A new character's belongings: its Calling's outfit, each item worn in its slot (a stack of
/// ammo or thrown weapons whole), and a Hearthstone in the backpack's first slot.
pub(crate) fn starting_inventory(calling: Calling) -> crate::inventory::Inventory {
    let mut inventory = crate::inventory::Inventory::default();
    let outfit = starting_outfits().get(calling.key());
    for &entry in outfit.into_iter().flat_map(|o| &o.items) {
        let Some(t) = crate::items::template(entry) else {
            continue;
        };
        let slot = crate::rules::gear::equip_slots(t.inventory_type)
            .first()
            .copied();
        if let Some(slot) = slot.filter(|s| inventory.get(*s).is_none()) {
            let item = inventory.new_item(entry, t.stack_size());
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

/// The `FactionTemplate.dbc` id every player reports, whatever its race: Orc (Horde). Twilight's
/// Eve is co-op, so players of both factions are one side to each other, and the client reads a
/// player's reaction to another from the two templates. The mob spawns are hostile to either
/// side, so mobs react the same to every race.
pub(crate) const PLAYER_FACTION_TEMPLATE: u32 = 2;
