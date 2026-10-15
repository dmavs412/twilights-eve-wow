//! Items in the world: their objects and slot fields on the wire, and the moves the stock bag and
//! character frames ask for (equip, unequip, swap, destroy), checked against the Warrior's
//! proficiency (vmangos `Player::SwapItem`, `Player::CanEquipItem`).

use benilla_protocol::messages::{ItemPushResult, ObjectType};
use benilla_protocol::{CharEnumItem, ObjectFields, SessionEvent};

use crate::characters::Char;
use crate::fields::*;
use crate::inventory::{Inventory, Item, EQUIPMENT_END, PACK, SLOTS};
use crate::items;
use crate::rules::gear::{self, can_equip, equip_slots, Refusal};
use crate::rules::stats::WarriorStats;

// Item update fields (`UpdateFields_1_12_1.h`).
const ITEM_FIELD_OWNER: u16 = 6;
const ITEM_FIELD_CONTAINED: u16 = 8;
pub(crate) const ITEM_FIELD_STACK_COUNT: u16 = 14;
pub(crate) const ITEM_FIELD_DURABILITY: u16 = 46;
const ITEM_FIELD_MAXDURABILITY: u16 = 47;
/// `TYPEMASK_OBJECT | TYPEMASK_ITEM`.
const TYPE_ITEM: u32 = 0x01 | 0x02;

/// `PLAYER_FIELD_INV_SLOT_HEAD`: two dwords a slot, the backpack straight after the equipment
/// and bag slots.
const PLAYER_FIELD_INV_SLOT_HEAD: u16 = 486;
/// `PLAYER_VISIBLE_ITEM_1_0`: the entry worn in each equipment slot, twelve dwords apart.
const PLAYER_VISIBLE_ITEM_1_0: u16 = 260;
const PLAYER_FIELD_COINAGE: u16 = 1176;

/// `BAG_PLAYER_INVENTORY`: the wire's name for the player's own slots.
const PLAYER_BAG: u8 = 255;

// `InventoryResult`s the realm sends besides [`Refusal`]'s.
const EQUIP_ERR_ITEM_DOESNT_GO_TO_SLOT: u8 = 3;
const EQUIP_ERR_CANT_EQUIP_WITH_TWOHANDED: u8 = 13;
const EQUIP_ERR_ITEM_NOT_FOUND: u8 = 23;
pub(crate) const EQUIP_ERR_INVENTORY_FULL: u8 = 50;

/// An item's object, owned by and contained in the player.
pub(crate) fn item_create(owner: u64, item: &Item) -> SessionEvent {
    let max = items::template(item.entry).map_or(0, |t| t.durability);
    let mut pairs = vec![
        (OBJECT_FIELD_TYPE, TYPE_ITEM),
        (OBJECT_FIELD_ENTRY, item.entry),
        (OBJECT_FIELD_SCALE_X, float(1.0)),
        (ITEM_FIELD_STACK_COUNT, item.count),
        (ITEM_FIELD_DURABILITY, item.durability),
        (ITEM_FIELD_MAXDURABILITY, max),
    ];
    pairs.extend(guid_pairs(OBJECT_FIELD_GUID, item.guid));
    pairs.extend(guid_pairs(ITEM_FIELD_OWNER, owner));
    pairs.extend(guid_pairs(ITEM_FIELD_CONTAINED, owner));
    SessionEvent::ItemCreate {
        guid: item.guid,
        container: false,
        fields: ObjectFields::from_pairs(&pairs).into_created(ObjectType::Item),
    }
}

/// A slot's player fields: the guid it holds (0 for none), and for a worn slot the entry others
/// see.
fn slot_pairs(slot: u8, item: Option<&Item>) -> Vec<(u16, u32)> {
    let mut pairs = guid_pairs(
        PLAYER_FIELD_INV_SLOT_HEAD + 2 * u16::from(slot),
        item.map_or(0, |i| i.guid),
    )
    .to_vec();
    if slot < EQUIPMENT_END {
        pairs.push((
            PLAYER_VISIBLE_ITEM_1_0 + 12 * u16::from(slot),
            item.map_or(0, |i| i.entry),
        ));
    }
    pairs
}

/// The character's stats in what it wears.
pub(crate) fn stats(c: &Char) -> WarriorStats {
    let bonus = crate::callings::of(c.calling).tier_bonus.stats();
    WarriorStats::geared_at_tier(c.race, c.level, bonus, &c.inventory.worn_entries())
}

/// At login, ahead of the player's own create: the proficiencies the tooltips and the bag check
/// against (`SMSG_SET_PROFICIENCY`), and every carried item's object.
pub(crate) fn login(c: &Char, events: &mut Vec<SessionEvent>) {
    for class in [gear::CLASS_WEAPON, gear::CLASS_ARMOR] {
        let mask = (0..32u32)
            .filter(|sub| gear::proficiency_level(class, *sub).is_some_and(|l| c.level >= l))
            .fold(0, |m, sub| m | 1 << sub);
        events.push(SessionEvent::Proficiency {
            item_class: class,
            subclass_mask: mask,
        });
    }
    for (_, item) in c.inventory.iter() {
        events.push(item_create(c.guid, item));
    }
}

// Player stat fields the gear fills (`UpdateFields_1_12_1.h`).
const UNIT_FIELD_ATTACK_POWER_MODS: u16 = 166;
const PLAYER_FIELD_POSSTAT0: u16 = 1177;

/// `PLAYER_SKILL_INFO` slots after Unarmed and Defense: the Warrior's weapon skill lines.
const WEAPON_SKILL_SLOT: u16 = 2;
/// `SkillLine.dbc` ids of the weapon skills, and the weapon subclass each covers.
const WEAPON_SKILLS: [(u32, u32); 14] = [
    (44, 0),   // Axes
    (172, 1),  // Two-Handed Axes
    (45, 2),   // Bows
    (46, 3),   // Guns
    (54, 4),   // Maces
    (160, 5),  // Two-Handed Maces
    (229, 6),  // Polearms
    (43, 7),   // Swords
    (55, 8),   // Two-Handed Swords
    (136, 10), // Staves
    (473, 13), // Fist Weapons
    (173, 15), // Daggers
    (176, 16), // Thrown
    (226, 18), // Crossbows
];

/// The sheet's gear fields: each stat's green part, gear's flat attack power, and the weapon
/// skill lines the character knows, at the level's maximum.
pub(crate) fn sheet_pairs(s: &WarriorStats) -> Vec<(u16, u32)> {
    let mut pairs: Vec<(u16, u32)> = (0..5)
        .map(|i| (PLAYER_FIELD_POSSTAT0 + i as u16, s.bonus[i]))
        .collect();
    pairs.push((
        UNIT_FIELD_ATTACK_POWER_MODS,
        s.attack_power_bonus.clamp(0, i32::from(i16::MAX)) as u32,
    ));
    let skill = u32::from(s.weapon_skill);
    let known = WEAPON_SKILLS.iter().filter(|(_, sub)| {
        gear::proficiency_level(gear::CLASS_WEAPON, *sub).is_some_and(|l| s.level >= l)
    });
    for (i, (id, _)) in known.enumerate() {
        let base = PLAYER_SKILL_INFO_1_1 + 3 * (WEAPON_SKILL_SLOT + i as u16);
        pairs.extend([(base, *id), (base + 1, skill | skill << 16), (base + 2, 0)]);
    }
    pairs
}

/// The player's inventory fields at login: every slot and the money.
pub(crate) fn inventory_pairs(inventory: &Inventory) -> Vec<(u16, u32)> {
    let mut pairs = vec![(PLAYER_FIELD_COINAGE, inventory.money)];
    for (slot, item) in inventory.iter() {
        pairs.extend(slot_pairs(slot, Some(item)));
    }
    pairs
}

/// The money field.
pub(crate) fn money_pair(inventory: &Inventory) -> (u16, u32) {
    (PLAYER_FIELD_COINAGE, inventory.money)
}

/// The character-select doll: each worn item's look.
pub(crate) fn roster_gear(inventory: &Inventory) -> [CharEnumItem; 19] {
    std::array::from_fn(|slot| {
        inventory
            .get(slot as u8)
            .and_then(|i| items::template(i.entry))
            .map_or(CharEnumItem::default(), |t| CharEnumItem {
                display_id: t.display_id,
                inventory_type: t.inventory_type as u8,
            })
    })
}

/// Stores a loot row's `count` of `entry` in the backpack: the new objects, the slot fields, the
/// stack counts and the "You receive loot" line. `Err` with the `InventoryResult` when it does
/// not fit.
pub(crate) fn receive(
    c: &mut Char,
    entry: u32,
    count: u32,
    events: &mut Vec<SessionEvent>,
) -> Result<(), u8> {
    receive_from(c, entry, count, false, events)
}

/// [`receive`], saying whether an NPC handed it over (a purchase) rather than loot.
pub(crate) fn receive_from(
    c: &mut Char,
    entry: u32,
    count: u32,
    from_npc: bool,
    events: &mut Vec<SessionEvent>,
) -> Result<(), u8> {
    let changed = c
        .inventory
        .store(entry, count)
        .map_err(|_| EQUIP_ERR_INVENTORY_FULL)?;
    let mut pairs = Vec::new();
    for &(slot, new) in &changed {
        let Some(item) = c.inventory.get(slot) else {
            continue;
        };
        if new {
            events.push(item_create(c.guid, item));
            pairs.extend(slot_pairs(slot, Some(item)));
        } else {
            events.push(values(item.guid, &[(ITEM_FIELD_STACK_COUNT, item.count)]));
        }
    }
    if !pairs.is_empty() {
        events.push(values(c.guid, &pairs));
    }
    if let Some(&(slot, _)) = changed.first() {
        events.push(SessionEvent::ItemPushResult(ItemPushResult {
            player_guid: c.guid,
            from_npc,
            created: false,
            show_in_chat: true,
            bag_slot: PLAYER_BAG,
            item_slot: u32::from(slot),
            item_entry: entry,
            suffix_factor: 0,
            random_property_id: 0,
            count,
        }));
    }
    Ok(())
}

/// What an inventory move did: whether the worn gear changed, so the sheet must be redone.
#[derive(Default)]
pub(crate) struct Moved {
    pub(crate) gear_changed: bool,
}

/// A refused move: `SMSG_INVENTORY_CHANGE_FAILURE`.
fn refuse(reason: u8, required_level: Option<u8>, item: u64, events: &mut Vec<SessionEvent>) {
    events.push(SessionEvent::InventoryFailure {
        reason,
        required_level: required_level.map(u32::from),
        item_guid: item,
        bag_slot: 0,
    });
}

fn refusal(r: Refusal, item: u64, events: &mut Vec<SessionEvent>) {
    let level = match r {
        Refusal::Level(l) => Some(l),
        _ => None,
    };
    refuse(r.code(), level, item, events);
}

/// Whether a wire `(bag, slot)` names one of the player's own slots; equipped bags are not
/// modelled, so their inner slots name nothing.
fn own_slot(bag: u8, slot: u8) -> Option<u8> {
    (bag == PLAYER_BAG && usize::from(slot) < SLOTS).then_some(slot)
}

/// `CMSG_AUTOEQUIP_ITEM`: wear the item at `(bag, slot)` in its first free slot (or its first
/// slot, swapping out what is there).
pub(crate) fn auto_equip(c: &mut Char, bag: u8, slot: u8, events: &mut Vec<SessionEvent>) -> Moved {
    let Some(src) = own_slot(bag, slot) else {
        refuse(EQUIP_ERR_ITEM_NOT_FOUND, None, 0, events);
        return Moved::default();
    };
    let Some(item) = c.inventory.get(src) else {
        refuse(EQUIP_ERR_ITEM_NOT_FOUND, None, 0, events);
        return Moved::default();
    };
    let Some(t) = items::template(item.entry) else {
        refusal(Refusal::NotEquippable, item.guid, events);
        return Moved::default();
    };
    let slots = equip_slots(t.inventory_type);
    let Some(&first) = slots.first() else {
        refusal(Refusal::NotEquippable, item.guid, events);
        return Moved::default();
    };
    let dst = slots
        .iter()
        .copied()
        .find(|s| c.inventory.get(*s).is_none())
        .unwrap_or(first);
    swap(c, src, dst, events)
}

/// `CMSG_AUTOSTORE_BAG_ITEM`: put the item at `(bag, slot)` in the first free slot of `dst_bag`.
pub(crate) fn auto_store(
    c: &mut Char,
    bag: u8,
    slot: u8,
    dst_bag: u8,
    events: &mut Vec<SessionEvent>,
) -> Moved {
    let item = own_slot(bag, slot).and_then(|s| c.inventory.get(s).map(|i| (s, i.guid)));
    let Some((src, guid)) = item else {
        refuse(EQUIP_ERR_ITEM_NOT_FOUND, None, 0, events);
        return Moved::default();
    };
    // Only the backpack can take it: there are no equipped bags.
    let free = (dst_bag == PLAYER_BAG)
        .then(|| c.inventory.free_pack_slot())
        .flatten();
    let Some(dst) = free else {
        refuse(EQUIP_ERR_INVENTORY_FULL, None, guid, events);
        return Moved::default();
    };
    swap(c, src, dst, events)
}

/// `CMSG_SWAP_ITEM`, between two wire positions.
pub(crate) fn swap_positions(
    c: &mut Char,
    src: (u8, u8),
    dst: (u8, u8),
    events: &mut Vec<SessionEvent>,
) -> Moved {
    match (own_slot(src.0, src.1), own_slot(dst.0, dst.1)) {
        (Some(s), Some(d)) => swap(c, s, d, events),
        _ => {
            refuse(EQUIP_ERR_ITEM_DOESNT_GO_TO_SLOT, None, 0, events);
            Moved::default()
        }
    }
}

/// `CMSG_SWAP_INV_ITEM`: move the item in player slot `src` to `dst`, swapping with what is
/// there, or merging onto a stack of the same item.
pub(crate) fn swap(c: &mut Char, src: u8, dst: u8, events: &mut Vec<SessionEvent>) -> Moved {
    let Some(moving) = c.inventory.get(src).cloned() else {
        refuse(EQUIP_ERR_ITEM_NOT_FOUND, None, 0, events);
        return Moved::default();
    };
    if src == dst || usize::from(dst) >= SLOTS {
        return Moved::default();
    }
    let other = c.inventory.get(dst).cloned();

    // A stack dropped on a stack of the same item tops it up.
    if let Some(o) = &other {
        let stack = items::template(o.entry).map_or(1, |t| t.stack_size());
        if o.entry == moving.entry && stack > 1 && !is_worn(src) && !is_worn(dst) {
            return merge(c, src, dst, stack, events);
        }
    }

    // Each item must be allowed where it lands.
    let mut freed_off_hand = None;
    for (item, to) in [(Some(&moving), dst), (other.as_ref(), src)] {
        let Some(item) = item else {
            continue;
        };
        if let Err(code) = check_place(c, item, to) {
            refuse(code.0, code.1, item.guid, events);
            return Moved::default();
        }
    }
    // A two-hander takes both hands: the off hand goes to the backpack.
    if dst == gear::SLOT_MAIN_HAND && is_two_hander(moving.entry) {
        if let Some(off) = c.inventory.get(gear::SLOT_OFF_HAND) {
            let room = if src >= PACK.start && other.is_none() {
                // The two-hander's own slot frees up as it leaves.
                Some(src)
            } else {
                c.inventory.free_pack_slot()
            };
            let Some(room) = room else {
                refuse(EQUIP_ERR_INVENTORY_FULL, None, off.guid, events);
                return Moved::default();
            };
            freed_off_hand = Some(room);
        }
    }

    let a = c.inventory.take(src);
    let b = c.inventory.take(dst);
    if let Some(a) = a {
        c.inventory.put(dst, a);
    }
    if let Some(b) = b {
        c.inventory.put(src, b);
    }
    let mut changed = vec![src, dst];
    if let Some(room) = freed_off_hand {
        if let Some(off) = c.inventory.take(gear::SLOT_OFF_HAND) {
            c.inventory.put(room, off);
            changed.extend([gear::SLOT_OFF_HAND, room]);
        }
    }
    send_slots(c, &changed, events);
    Moved {
        gear_changed: changed.iter().any(|s| is_worn(*s)),
    }
}

/// `CMSG_DESTROYITEM`: `count` of the item at `(bag, slot)`, 0 for all of it.
pub(crate) fn destroy(
    c: &mut Char,
    bag: u8,
    slot: u8,
    count: u8,
    events: &mut Vec<SessionEvent>,
) -> Moved {
    let Some(slot) = own_slot(bag, slot) else {
        return Moved::default();
    };
    let Some(item) = c.inventory.get_mut(slot) else {
        refuse(EQUIP_ERR_ITEM_NOT_FOUND, None, 0, events);
        return Moved::default();
    };
    let count = u32::from(count);
    if count > 0 && count < item.count {
        item.count -= count;
        events.push(values(item.guid, &[(ITEM_FIELD_STACK_COUNT, item.count)]));
        return Moved::default();
    }
    let guid = item.guid;
    c.inventory.take(slot);
    send_slots(c, &[slot], events);
    events.push(SessionEvent::ObjectDestroyed(guid));
    Moved {
        gear_changed: is_worn(slot),
    }
}

fn is_worn(slot: u8) -> bool {
    slot < EQUIPMENT_END
}

fn is_two_hander(entry: u32) -> bool {
    items::template(entry).is_some_and(|t| t.inventory_type == gear::INVTYPE_2HWEAPON)
}

/// Whether `item` may sit in player slot `to`: anything in the backpack; in a worn slot, only
/// what goes there and what the character may wear. The `InventoryResult` and required level of
/// a refusal.
fn check_place(c: &Char, item: &Item, to: u8) -> Result<(), (u8, Option<u8>)> {
    if PACK.contains(&to) {
        return Ok(());
    }
    if !is_worn(to) {
        // The bag slots: equipped bags are not modelled.
        return Err((EQUIP_ERR_ITEM_DOESNT_GO_TO_SLOT, None));
    }
    can_equip(item.entry, c.race, c.level).map_err(|r| {
        let level = match r {
            Refusal::Level(l) => Some(l),
            _ => None,
        };
        (r.code(), level)
    })?;
    let t = items::template(item.entry).ok_or((EQUIP_ERR_ITEM_DOESNT_GO_TO_SLOT, None))?;
    if !equip_slots(t.inventory_type).contains(&to) {
        return Err((EQUIP_ERR_ITEM_DOESNT_GO_TO_SLOT, None));
    }
    // Nothing goes in the off hand beside a two-hander.
    if to == gear::SLOT_OFF_HAND
        && c.inventory
            .get(gear::SLOT_MAIN_HAND)
            .is_some_and(|m| m.guid != item.guid && is_two_hander(m.entry))
    {
        return Err((EQUIP_ERR_CANT_EQUIP_WITH_TWOHANDED, None));
    }
    Ok(())
}

/// Moves as much of the stack at `src` onto the stack at `dst` as fits.
fn merge(c: &mut Char, src: u8, dst: u8, stack: u32, events: &mut Vec<SessionEvent>) -> Moved {
    let room = c
        .inventory
        .get(dst)
        .map_or(0, |d| stack.saturating_sub(d.count));
    let moving = c.inventory.get(src).map_or(0, |s| s.count);
    let add = room.min(moving);
    if add == 0 {
        return Moved::default();
    }
    if let Some(d) = c.inventory.get_mut(dst) {
        d.count += add;
        events.push(values(d.guid, &[(ITEM_FIELD_STACK_COUNT, d.count)]));
    }
    if add == moving {
        if let Some(gone) = c.inventory.take(src) {
            send_slots(c, &[src], events);
            events.push(SessionEvent::ObjectDestroyed(gone.guid));
        }
    } else if let Some(s) = c.inventory.get_mut(src) {
        s.count -= add;
        events.push(values(s.guid, &[(ITEM_FIELD_STACK_COUNT, s.count)]));
    }
    Moved::default()
}

/// The player fields of the slots in `changed`.
pub(crate) fn send_slots(c: &Char, changed: &[u8], events: &mut Vec<SessionEvent>) {
    let pairs: Vec<(u16, u32)> = changed
        .iter()
        .flat_map(|s| slot_pairs(*s, c.inventory.get(*s)))
        .collect();
    events.push(values(c.guid, &pairs));
}
