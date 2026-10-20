//! What a Calling may equip, and where (vmangos `Player::CanUseItem`, `Player::FindEquipSlot`).
//! Gear follows the Calling, never the level, so a level reset never takes gear away: every
//! Calling of the Warrior line wields every Warrior weapon, and wears the armour its data lists
//! (`content/callings.toml`). Wands, librams, idols and totems never. Items keep their required
//! level only for the loot bands. Only the ten Gear Slots exist (ADR 0007): Weapon, Offhand,
//! Ranged, Ammo, two Rings, two Trinkets, Chest and Amulet; an item for any other body part is
//! refused.

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
    /// `EQUIP_ERR_ITEM_DOESNT_GO_TO_SLOT`: worn on a body part outside the ten Gear Slots.
    NoGearSlot,
}

impl Refusal {
    /// The wire `InventoryResult`.
    pub fn code(self) -> u8 {
        match self {
            Self::Proficiency => 8,
            Self::NeverUsable => 10,
            Self::NotEquippable => 20,
            Self::NoGearSlot => 3,
        }
    }
}

/// `ITEM_CLASS_WEAPON` and `ITEM_CLASS_ARMOR`.
pub(crate) const CLASS_WEAPON: u32 = 2;
pub(crate) const CLASS_ARMOR: u32 = 4;

/// The required level the client is shown for every item: none, so no tooltip reads "Requires
/// Level" and no move is refused for one. A template keeps its own level for the loot bands.
pub(crate) const SHOWN_REQUIRED_LEVEL: u32 = 0;

/// `CLASSMASK_WARRIOR`.
const WARRIOR_MASK: i32 = 1;

/// Whether a weapon subclass is one no Warrior ever wields: wands, and the unused spears,
/// obsolete and exotic slots.
fn never_warrior_weapon(subclass: u32) -> bool {
    matches!(subclass, 9 | 11 | 12 | 17 | 19)
}

/// Whether `calling` knows a weapon or armour subclass: its `weapons` and `armour` lists.
pub(crate) fn knows(calling: &CallingDef, class: u32, subclass: u32) -> bool {
    match (class, subclass) {
        (CLASS_WEAPON, sub) => calling.weapons.iter().any(|w| w.subclass() == Some(sub)),
        // Rings, necks, trinkets, shirts and the like.
        (CLASS_ARMOR, 0) => true,
        (CLASS_ARMOR, sub) => calling.armour.iter().any(|a| a.subclasses().contains(&sub)),
        _ => true,
    }
}

/// Who an item is for and whether it is equipment at all, ahead of proficiency.
fn usable(entry: u32, race: u8) -> Result<&'static items::Template, Refusal> {
    let t = items::template(entry).ok_or(Refusal::NotEquippable)?;
    if !is_gear_slot(t.inventory_type) {
        return Err(slotless(t.inventory_type));
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
        (CLASS_WEAPON, sub) if never_warrior_weapon(sub) => None,
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

/// The `InventoryType`s of the body parts outside the ten Gear Slots: head, shoulders, shirt,
/// waist, legs, feet, wrists, hands, back and tabard. A Chest's Wardrobe dresses them.
const WORN_ELSEWHERE: [u32; 10] = [1, 3, 4, 6, 7, 8, 9, 10, 16, 19];

/// Whether an `InventoryType` goes in one of the ten Gear Slots: a hand (main hand, off hand,
/// one-hand, two-hand, shield, held in off hand), ranged (bow, gun, crossbow, wand, thrown,
/// relic), ammo, a finger, a trinket, the chest (and robe) or the neck.
pub fn is_gear_slot(inventory_type: u32) -> bool {
    matches!(
        inventory_type,
        2 | 5 | 11 | 12 | 13 | 14 | 15 | 17 | 20 | 21 | 22 | 23 | 24 | 25 | 26 | 28
    )
}

/// Why an item with no Gear Slot is refused: a body part outside the ten, or not wearable at all.
pub(crate) fn slotless(inventory_type: u32) -> Refusal {
    if WORN_ELSEWHERE.contains(&inventory_type) {
        Refusal::NoGearSlot
    } else {
        Refusal::NotEquippable
    }
}

/// The equipment slots an `InventoryType` goes in, the first free one first: only the ten Gear
/// Slots' and the bag slots. One-handers go in the main hand only: a Basic Warrior has no Dual
/// Wield, so an off-hand-only weapon has no slot either. A bag goes in a bag slot; ammo is not
/// equipment here (1.12 keeps it in the bags).
pub fn equip_slots(inventory_type: u32) -> &'static [u8] {
    match inventory_type {
        2 => &[1],
        5 | 20 => &[4],
        11 => &[10, 11],
        12 => &[12, 13],
        13 | 17 | 21 => &[SLOT_MAIN_HAND],
        14 | 23 => &[SLOT_OFF_HAND],
        15 | 25 | 26 | 28 => &[17],
        18 => &[19, 20, 21, 22],
        _ => &[],
    }
}
