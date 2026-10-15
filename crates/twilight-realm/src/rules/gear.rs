//! What a Warrior may equip, and where (vmangos `Player::CanUseItem`, `Player::FindEquipSlot`).
//! Proficiency follows `data/twilights-eve/warrior_proficiency.json`: the slice has no Weapon
//! Master, so every Warrior weapon is known from level 1 but polearms (level 20); plate comes at
//! 40, beyond the slice; wands, librams, idols and totems never.

use crate::items;

/// Why an item cannot be equipped, each a stock `InventoryResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// `EQUIP_ERR_CANT_EQUIP_LEVEL_I`: the character is under the item's required level.
    Level(u8),
    /// `EQUIP_ERR_NO_REQUIRED_PROFICIENCY`: a weapon or armour kind the Warrior does not know.
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
            Self::Level(_) => 1,
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

/// The level a Warrior learns a weapon or armour subclass at; `None` for never.
pub(crate) fn proficiency_level(class: u32, subclass: u32) -> Option<u8> {
    match (class, subclass) {
        // Polearms.
        (CLASS_WEAPON, 6) => Some(20),
        // Wands, and the unused spears, obsolete and exotic slots.
        (CLASS_WEAPON, 9 | 11 | 12 | 17 | 19) => None,
        (CLASS_WEAPON, _) => Some(1),
        // Plate.
        (CLASS_ARMOR, 4) => Some(40),
        // Librams, idols, totems.
        (CLASS_ARMOR, 7..=9) => None,
        (CLASS_ARMOR, _) => Some(1),
        _ => Some(1),
    }
}

/// Whether a Warrior of `race` at `level` may equip item `entry`, in vmangos's order: who it is
/// for, then proficiency, then level.
pub fn can_equip(entry: u32, race: u8, level: u8) -> Result<(), Refusal> {
    let t = items::template(entry).ok_or(Refusal::NotEquippable)?;
    if equip_slots(t.inventory_type).is_empty() {
        return Err(Refusal::NotEquippable);
    }
    let race_mask = 1i32 << (race.max(1) - 1);
    if t.allowable_class & WARRIOR_MASK == 0 || t.allowable_race & race_mask == 0 {
        return Err(Refusal::NeverUsable);
    }
    if !proficiency_level(t.class, t.subclass).is_some_and(|l| level >= l) {
        return Err(Refusal::Proficiency);
    }
    if level < t.required_level {
        return Err(Refusal::Level(t.required_level));
    }
    Ok(())
}

// `EquipmentSlots`.
pub(crate) const SLOT_MAIN_HAND: u8 = 15;
pub(crate) const SLOT_OFF_HAND: u8 = 16;
/// `INVTYPE_2HWEAPON`.
pub(crate) const INVTYPE_2HWEAPON: u32 = 17;

/// The equipment slots an `InventoryType` goes in, the first free one first. One-handers go in
/// the main hand only: a Basic Warrior has no Dual Wield. Bags and ammo are not equipment here.
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
        _ => &[],
    }
}
