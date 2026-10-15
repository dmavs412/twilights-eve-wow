//! What a Calling may equip, and where (vmangos `Player::CanUseItem`, `Player::FindEquipSlot`).
//! Gear follows the Calling, never the level, so a level reset never takes gear away: every
//! Calling of the Warrior line wields every Warrior weapon, and wears the armour its data lists
//! (`content/callings.toml`). Wands, librams, idols and totems never. Items keep their required
//! level only for the loot bands.

use crate::callings::CallingDef;
use crate::items;

/// Why an item cannot be equipped, each a stock `InventoryResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// `EQUIP_ERR_NO_REQUIRED_PROFICIENCY`: a weapon or armour kind the Calling does not know.
    Proficiency,
    /// `EQUIP_ERR_YOU_CAN_NEVER_USE_THAT_ITEM`: made for other classes or races.
    NeverUsable,
    /// `EQUIP_ERR_ITEM_CANT_BE_EQUIPPED`: not wearable at all.
    NotEquippable,
}

impl Refusal {
    /// The wire `InventoryResult`.
    pub fn code(self) -> u8 {
        match self {
            Self::Proficiency => 8,
            Self::NeverUsable => 10,
            Self::NotEquippable => 20,
        }
    }
}

/// `ITEM_CLASS_WEAPON` and `ITEM_CLASS_ARMOR`.
pub(crate) const CLASS_WEAPON: u32 = 2;
pub(crate) const CLASS_ARMOR: u32 = 4;

/// `CLASSMASK_WARRIOR`.
const WARRIOR_MASK: i32 = 1;

/// Whether `calling` knows a weapon or armour subclass.
pub(crate) fn knows(calling: &CallingDef, class: u32, subclass: u32) -> bool {
    match (class, subclass) {
        // Wands, and the unused spears, obsolete and exotic slots.
        (CLASS_WEAPON, 9 | 11 | 12 | 17 | 19) => false,
        (CLASS_WEAPON, _) => true,
        // Rings, necks, trinkets, shirts and the like.
        (CLASS_ARMOR, 0) => true,
        (CLASS_ARMOR, sub) => calling.armour.iter().any(|a| a.subclasses().contains(&sub)),
        _ => true,
    }
}

/// Who an item is for and whether it is equipment at all, ahead of proficiency.
fn usable(entry: u32, race: u8) -> Result<&'static items::Template, Refusal> {
    let t = items::template(entry).ok_or(Refusal::NotEquippable)?;
    if equip_slots(t.inventory_type).is_empty() {
        return Err(Refusal::NotEquippable);
    }
    let race_mask = 1i32 << (race.max(1) - 1);
    if t.allowable_class & WARRIOR_MASK == 0 || t.allowable_race & race_mask == 0 {
        return Err(Refusal::NeverUsable);
    }
    Ok(t)
}

/// Whether a character of `race` in `calling` may equip item `entry`, in vmangos's order: who it
/// is for, then proficiency. Never the level.
pub fn can_equip(entry: u32, race: u8, calling: &CallingDef) -> Result<(), Refusal> {
    let t = usable(entry, race)?;
    if !knows(calling, t.class, t.subclass) {
        return Err(Refusal::Proficiency);
    }
    Ok(())
}

/// The level a vanilla Warrior learns a weapon or armour subclass at; `None` for never
/// (`data/twilights-eve/warrior_proficiency.json`).
fn vanilla_level(class: u32, subclass: u32) -> Option<u8> {
    match (class, subclass) {
        // Polearms.
        (CLASS_WEAPON, 6) => Some(20),
        (CLASS_WEAPON, 9 | 11 | 12 | 17 | 19) => None,
        (CLASS_WEAPON, _) => Some(1),
        // Plate.
        (CLASS_ARMOR, 4) => Some(40),
        (CLASS_ARMOR, 7..=9) => None,
        _ => Some(1),
    }
}

/// Whether a vanilla Warrior of `race` could wear item `entry` once it reaches the item's level:
/// the Dungeon boss's blue pool, which follows the run's level band and not the Calling.
pub(crate) fn vanilla_wearable(entry: u32, race: u8, level: u8) -> bool {
    usable(entry, race).is_ok_and(|t| {
        vanilla_level(t.class, t.subclass).is_some_and(|l| t.required_level.max(level) >= l)
    })
}

// `EquipmentSlots`.
pub(crate) const SLOT_MAIN_HAND: u8 = 15;
pub(crate) const SLOT_OFF_HAND: u8 = 16;
/// `INVTYPE_2HWEAPON`.
pub(crate) const INVTYPE_2HWEAPON: u32 = 17;

/// The equipment slots an `InventoryType` goes in, the first free one first. One-handers go in
/// the main hand only: a Basic Warrior has no Dual Wield. A bag goes in a bag slot; ammo is not
/// equipment here.
pub fn equip_slots(inventory_type: u32) -> &'static [u8] {
    match inventory_type {
        1 => &[0],
        2 => &[1],
        3 => &[2],
        4 => &[3],
        5 | 20 => &[4],
        6 => &[5],
        7 => &[6],
        8 => &[7],
        9 => &[8],
        10 => &[9],
        11 => &[10, 11],
        12 => &[12, 13],
        16 => &[14],
        13 | 17 | 21 => &[SLOT_MAIN_HAND],
        14 | 23 => &[SLOT_OFF_HAND],
        15 | 25 | 26 => &[17],
        19 => &[18],
        18 => &[19, 20, 21, 22],
        _ => &[],
    }
}
