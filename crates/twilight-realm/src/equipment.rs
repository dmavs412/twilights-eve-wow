//! Items in the world: their objects and slot fields on the wire, and the moves the stock bag and
//! character frames ask for (equip, unequip, swap, destroy), checked against the Calling's
//! proficiency (vmangos `Player::SwapItem`, `Player::CanEquipItem`).

use benilla_protocol::messages::{ItemPushResult, ObjectType};
use benilla_protocol::{CharEnumItem, ObjectFields, SessionEvent};

use crate::callings;
use crate::characters::Char;
use crate::fields::*;
use crate::inventory::{bag_size, Inventory, Item, Pos, PACK, PLAYER_BAG};
use crate::items;
use crate::rules::gear::{self, can_equip, equip_slots, Refusal};
use crate::rules::stats::CharacterStats;

// Item update fields (`UpdateFields_1_12_1.h`).
const ITEM_FIELD_OWNER: u16 = 6;
const ITEM_FIELD_CONTAINED: u16 = 8;
pub(crate) const ITEM_FIELD_STACK_COUNT: u16 = 14;
pub(crate) const ITEM_FIELD_DURABILITY: u16 = 46;
const ITEM_FIELD_MAXDURABILITY: u16 = 47;
/// `TYPEMASK_OBJECT | TYPEMASK_ITEM`.
const TYPE_ITEM: u32 = 0x01 | 0x02;
/// `TYPEMASK_OBJECT | TYPEMASK_ITEM | TYPEMASK_CONTAINER`.
const TYPE_CONTAINER: u32 = TYPE_ITEM | 0x04;
/// `CONTAINER_FIELD_NUM_SLOTS`, then `CONTAINER_FIELD_SLOT_1`, two dwords a slot.
const CONTAINER_FIELD_NUM_SLOTS: u16 = 48;
const CONTAINER_FIELD_SLOT_1: u16 = 50;

/// `PLAYER_FIELD_INV_SLOT_HEAD`: two dwords a slot, the backpack straight after the equipment
/// and bag slots.
const PLAYER_FIELD_INV_SLOT_HEAD: u16 = 486;
/// `PLAYER_VISIBLE_ITEM_1_0`: the entry worn in each equipment slot, twelve dwords apart.
const PLAYER_VISIBLE_ITEM_1_0: u16 = 260;
const PLAYER_FIELD_COINAGE: u16 = 1176;

// `InventoryResult`s the realm sends besides [`Refusal`]'s.
const EQUIP_ERR_ITEM_DOESNT_GO_TO_SLOT: u8 = 3;
/// `EQUIP_ERR_NONEMPTY_BAG_OVER_OTHER_BAG`: a bag into its own slots.
const EQUIP_ERR_BAG_IN_BAG: u8 = 5;
/// `EQUIP_ERR_CAN_ONLY_DO_WITH_EMPTY_BAGS`.
const EQUIP_ERR_ONLY_EMPTY_BAGS: u8 = 31;
const EQUIP_ERR_CANT_EQUIP_WITH_TWOHANDED: u8 = 13;
const EQUIP_ERR_ITEM_NOT_FOUND: u8 = 23;
pub(crate) const EQUIP_ERR_INVENTORY_FULL: u8 = 50;

/// The object of `item`, at `at`: owned by the player, contained by the player or by the bag it
/// is in. A bag is a container carrying its size and, once equipped, what it holds.
pub(crate) fn item_create(c: &Char, at: Pos, item: &Item) -> SessionEvent {
    let max = items::template(item.entry).map_or(0, |t| t.durability);
    let size = bag_size(item.entry);
    let container = size > 0;
    let mut pairs = vec![
        (
            OBJECT_FIELD_TYPE,
            if container { TYPE_CONTAINER } else { TYPE_ITEM },
        ),
        (OBJECT_FIELD_ENTRY, item.entry),
        (OBJECT_FIELD_SCALE_X, float(1.0)),
        (ITEM_FIELD_STACK_COUNT, item.count),
        (ITEM_FIELD_DURABILITY, item.durability),
        (ITEM_FIELD_MAXDURABILITY, max),
    ];
    pairs.extend(guid_pairs(OBJECT_FIELD_GUID, item.guid));
    pairs.extend(guid_pairs(ITEM_FIELD_OWNER, c.guid));
    pairs.extend(guid_pairs(ITEM_FIELD_CONTAINED, container_of(c, at)));
    if container {
        pairs.push((CONTAINER_FIELD_NUM_SLOTS, u32::from(size)));
        if at.is_bag_slot() {
            pairs.extend(container_pairs(c, at.slot));
        }
    }
    let kind = if container {
        ObjectType::Container
    } else {
        ObjectType::Item
    };
    SessionEvent::ItemCreate {
        guid: item.guid,
        container,
        fields: ObjectFields::from_pairs(&pairs).into_created(kind),
    }
}

/// What contains the item at `at`: the bag it is in, else the player.
fn container_of(c: &Char, at: Pos) -> u64 {
    match at.own_slot() {
        Some(_) => c.guid,
        None => c.inventory.get(at.bag).map_or(c.guid, |b| b.guid),
    }
}

/// The container slots of the bag in bag slot `bag`, as it holds them: every guid, 0 for empty.
fn container_pairs(c: &Char, bag: u8) -> Vec<(u16, u32)> {
    let held = c.inventory.bag_contents(bag);
    let size = c.inventory.get(bag).map_or(0, |b| bag_size(b.entry));
    (0..size)
        .flat_map(|i| {
            let guid = held
                .get(usize::from(i))
                .and_then(Option::as_ref)
                .map_or(0, |h| h.guid);
            guid_pairs(CONTAINER_FIELD_SLOT_1 + 2 * u16::from(i), guid)
        })
        .collect()
}

/// The fields that say what place `at` holds, `(object, field, value)`: a player slot's guid
/// (and for a worn slot the entry others see), or the guid in the bag's container slot.
fn slot_pairs(c: &Char, at: Pos, item: Option<&Item>) -> Vec<(u64, u16, u32)> {
    let guid = item.map_or(0, |i| i.guid);
    let (object, field) = match at.own_slot() {
        Some(slot) => (c.guid, PLAYER_FIELD_INV_SLOT_HEAD + 2 * u16::from(slot)),
        None => (
            c.inventory.get(at.bag).map_or(0, |b| b.guid),
            CONTAINER_FIELD_SLOT_1 + 2 * u16::from(at.slot),
        ),
    };
    let mut pairs: Vec<(u64, u16, u32)> = guid_pairs(field, guid)
        .into_iter()
        .map(|(f, v)| (object, f, v))
        .collect();
    if at.is_worn() {
        pairs.push((
            c.guid,
            PLAYER_VISIBLE_ITEM_1_0 + 12 * u16::from(at.slot),
            item.map_or(0, |i| i.entry),
        ));
    }
    pairs
}

/// The character's stats in what it wears.
pub(crate) fn stats(c: &Char) -> CharacterStats {
    let bonus = crate::callings::of(c.calling).tier_bonus.stats();
    // Every shipped Calling sits on the Warrior Class until the Calling carries its own.
    CharacterStats::geared_at_tier(
        crate::rules::power::class::WARRIOR,
        c.race,
        c.level,
        bonus,
        &c.inventory.worn_entries(),
    )
}

/// The Calling's proficiencies the tooltips and the bag check against (`SMSG_SET_PROFICIENCY`):
/// at login, and again on Advancement.
pub(crate) fn proficiency(c: &Char, events: &mut Vec<SessionEvent>) {
    let calling = callings::of(c.calling);
    for class in [gear::CLASS_WEAPON, gear::CLASS_ARMOR] {
        let mask = (0..32u32)
            .filter(|sub| gear::knows(calling, class, *sub))
            .fold(0, |m, sub| m | 1 << sub);
        events.push(SessionEvent::Proficiency {
            item_class: class,
            subclass_mask: mask,
        });
    }
}

/// At login, ahead of the player's own create: the proficiencies, and every carried item's
/// object.
pub(crate) fn login(c: &Char, events: &mut Vec<SessionEvent>) {
    proficiency(c, events);
    for (at, item) in c.inventory.iter() {
        events.push(item_create(c, at, item));
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

/// `PLAYER_SKILL_INFO` slots after the weapon lines: the armour lines, one slot per armour kind.
const ARMOUR_SKILL_SLOT: u16 = WEAPON_SKILL_SLOT + WEAPON_SKILLS.len() as u16;
const ARMOUR_KINDS: usize = 5;

/// The sheet's gear fields: each stat's green part, gear's flat attack power, the weapon skill
/// lines the character knows at the level's maximum, and the armour lines its Calling allows at
/// 1 of 1, as a 1.12 armour skill reads. The armour slots past the Calling's are cleared.
pub(crate) fn sheet_pairs(c: &Char, s: &CharacterStats) -> Vec<(u16, u32)> {
    let mut pairs: Vec<(u16, u32)> = (0..5)
        .map(|i| (PLAYER_FIELD_POSSTAT0 + i as u16, s.bonus[i]))
        .collect();
    pairs.push((
        UNIT_FIELD_ATTACK_POWER_MODS,
        s.attack_power_bonus.clamp(0, i32::from(i16::MAX)) as u32,
    ));
    let skill = u32::from(s.weapon_skill);
    // Every Calling of the Warrior line wields every Warrior weapon.
    for (i, (id, _)) in WEAPON_SKILLS.iter().enumerate() {
        let base = PLAYER_SKILL_INFO_1_1 + 3 * (WEAPON_SKILL_SLOT + i as u16);
        pairs.extend([(base, *id), (base + 1, skill | skill << 16), (base + 2, 0)]);
    }
    let armour = &callings::of(c.calling).armour;
    for i in 0..ARMOUR_KINDS {
        let base = PLAYER_SKILL_INFO_1_1 + 3 * (ARMOUR_SKILL_SLOT + i as u16);
        let (id, value) = armour
            .get(i)
            .map_or((0, 0), |a| (a.skill_line(), 1 | 1 << 16));
        pairs.extend([(base, id), (base + 1, value), (base + 2, 0)]);
    }
    pairs
}

/// The player's inventory fields at login: every player slot and the money. What a bag holds is
/// on the bag's own object.
pub(crate) fn inventory_pairs(c: &Char) -> Vec<(u16, u32)> {
    let mut pairs = vec![(PLAYER_FIELD_COINAGE, c.inventory.money)];
    for (at, item) in c.inventory.iter() {
        if at.own_slot().is_some() {
            let own = slot_pairs(c, at, Some(item));
            pairs.extend(own.into_iter().map(|(_, f, v)| (f, v)));
        }
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
    let mut fresh = Vec::new();
    for &(at, new) in &changed {
        let Some(item) = c.inventory.get(at) else {
            continue;
        };
        if new {
            events.push(item_create(c, at, item));
            fresh.push(at);
        } else {
            events.push(values(item.guid, &[(ITEM_FIELD_STACK_COUNT, item.count)]));
        }
    }
    if !fresh.is_empty() {
        send_slots(c, &fresh, events);
    }
    if let Some(&(at, _)) = changed.first() {
        events.push(SessionEvent::ItemPushResult(ItemPushResult {
            player_guid: c.guid,
            from_npc,
            created: false,
            show_in_chat: true,
            bag_slot: at.bag,
            item_slot: u32::from(at.slot),
            item_entry: entry,
            suffix_factor: 0,
            random_property_id: 0,
            count,
        }));
    }
    Ok(())
}

/// What an inventory move did: whether the worn gear changed, so the sheet must be redone, and
/// whether a bag went on or came off, which the save must keep.
#[derive(Default)]
pub(crate) struct Moved {
    pub(crate) gear_changed: bool,
    pub(crate) bags_changed: bool,
}

impl Moved {
    fn of(changed: &[Pos]) -> Self {
        Self {
            gear_changed: changed.iter().any(|p| p.is_worn()),
            bags_changed: changed.iter().any(|p| p.is_bag_slot()),
        }
    }
}

/// `SMSG_INVENTORY_CHANGE_FAILURE` for `item` (0 for none). It never names a level: no item
/// requires one ([`gear::SHOWN_REQUIRED_LEVEL`]).
pub(crate) fn inventory_failure(reason: u8, item: u64) -> SessionEvent {
    SessionEvent::InventoryFailure {
        reason,
        required_level: None,
        item_guid: item,
        bag_slot: 0,
    }
}

/// A refused move.
fn refuse(reason: u8, item: u64, events: &mut Vec<SessionEvent>) {
    events.push(inventory_failure(reason, item));
}

fn refusal(r: Refusal, item: u64, events: &mut Vec<SessionEvent>) {
    refuse(r.code(), item, events);
}

/// The wire `(bag, slot)` as a place, if it names one: a player slot, or a slot of an equipped
/// bag.
fn place(c: &Char, bag: u8, slot: u8) -> Option<Pos> {
    let at = Pos { bag, slot };
    c.inventory.exists(at).then_some(at)
}

/// `CMSG_AUTOEQUIP_ITEM`: wear the item at `(bag, slot)` in its first free slot (or its first
/// slot, swapping out what is there). A bag goes in the first free bag slot.
pub(crate) fn auto_equip(c: &mut Char, bag: u8, slot: u8, events: &mut Vec<SessionEvent>) -> Moved {
    let Some(src) = place(c, bag, slot) else {
        refuse(EQUIP_ERR_ITEM_NOT_FOUND, 0, events);
        return Moved::default();
    };
    let Some(item) = c.inventory.get(src) else {
        refuse(EQUIP_ERR_ITEM_NOT_FOUND, 0, events);
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

/// `CMSG_AUTOSTORE_BAG_ITEM`: put the item at `(bag, slot)` in the first free slot of `dst_bag`,
/// the backpack or an equipped bag.
pub(crate) fn auto_store(
    c: &mut Char,
    bag: u8,
    slot: u8,
    dst_bag: u8,
    events: &mut Vec<SessionEvent>,
) -> Moved {
    let item = place(c, bag, slot).and_then(|at| c.inventory.get(at).map(|i| (at, i.guid)));
    let Some((src, guid)) = item else {
        refuse(EQUIP_ERR_ITEM_NOT_FOUND, 0, events);
        return Moved::default();
    };
    let free = if dst_bag == PLAYER_BAG {
        PACK.map(Pos::own).find(|at| c.inventory.get(*at).is_none())
    } else {
        c.inventory.free_slot_in_bag(dst_bag)
    };
    let Some(dst) = free else {
        refuse(EQUIP_ERR_INVENTORY_FULL, guid, events);
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
    match (place(c, src.0, src.1), place(c, dst.0, dst.1)) {
        (Some(s), Some(d)) => swap(c, s, d, events),
        _ => {
            refuse(EQUIP_ERR_ITEM_DOESNT_GO_TO_SLOT, 0, events);
            Moved::default()
        }
    }
}

/// `CMSG_SWAP_INV_ITEM` (two player slots), and every other move: the item at `src` to `dst`,
/// swapping with what is there, or merging onto a stack of the same item.
pub(crate) fn swap(
    c: &mut Char,
    src: impl Into<Pos>,
    dst: impl Into<Pos>,
    events: &mut Vec<SessionEvent>,
) -> Moved {
    let (src, dst) = (src.into(), dst.into());
    let Some(moving) = c.inventory.get(src).cloned() else {
        refuse(EQUIP_ERR_ITEM_NOT_FOUND, 0, events);
        return Moved::default();
    };
    if src == dst || !c.inventory.exists(dst) {
        return Moved::default();
    }
    let other = c.inventory.get(dst).cloned();

    // A bag that holds items stays where it is, and no bag goes inside itself.
    for (from, to, item) in [(src, dst, Some(&moving)), (dst, src, other.as_ref())] {
        let Some(item) = item else {
            continue;
        };
        if !from.is_bag_slot() {
            continue;
        }
        if c.inventory.bag_holds_items(from.slot) {
            refuse(EQUIP_ERR_ONLY_EMPTY_BAGS, item.guid, events);
            return Moved::default();
        }
        if to.bag == from.slot {
            refuse(EQUIP_ERR_BAG_IN_BAG, item.guid, events);
            return Moved::default();
        }
    }

    // A stack dropped on a stack of the same item tops it up.
    if let Some(o) = &other {
        let stack = items::template(o.entry).map_or(1, |t| t.stack_size());
        if o.entry == moving.entry && stack > 1 && src.is_storage() && dst.is_storage() {
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
            refuse(code, item.guid, events);
            return Moved::default();
        }
    }
    // A two-hander takes both hands: the off hand goes to the bags.
    if dst == Pos::own(gear::SLOT_MAIN_HAND) && is_two_hander(moving.entry) {
        if let Some(off) = c.inventory.get(gear::SLOT_OFF_HAND) {
            let room = if src.is_storage() && other.is_none() {
                // The two-hander's own slot frees up as it leaves.
                Some(src)
            } else {
                c.inventory.free_slot()
            };
            let Some(room) = room else {
                refuse(EQUIP_ERR_INVENTORY_FULL, off.guid, events);
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
            changed.extend([Pos::own(gear::SLOT_OFF_HAND), room]);
        }
    }
    // A bag that went on opens its slots, empty.
    for at in changed.iter().filter(|at| at.is_bag_slot()) {
        if let Some(bag) = c.inventory.get(*at) {
            events.push(values(bag.guid, &container_pairs(c, at.slot)));
        }
    }
    send_slots(c, &changed, events);
    Moved::of(&changed)
}

/// `CMSG_DESTROYITEM`: `count` of the item at `(bag, slot)`, 0 for all of it. A bag that holds
/// items stays.
pub(crate) fn destroy(
    c: &mut Char,
    bag: u8,
    slot: u8,
    count: u8,
    events: &mut Vec<SessionEvent>,
) -> Moved {
    let Some(at) = place(c, bag, slot) else {
        return Moved::default();
    };
    let Some(item) = c.inventory.get(at) else {
        refuse(EQUIP_ERR_ITEM_NOT_FOUND, 0, events);
        return Moved::default();
    };
    if at.is_bag_slot() && c.inventory.bag_holds_items(at.slot) {
        refuse(EQUIP_ERR_ONLY_EMPTY_BAGS, item.guid, events);
        return Moved::default();
    }
    let Some(item) = c.inventory.get_mut(at) else {
        return Moved::default();
    };
    let count = u32::from(count);
    if count > 0 && count < item.count {
        item.count -= count;
        events.push(values(item.guid, &[(ITEM_FIELD_STACK_COUNT, item.count)]));
        return Moved::default();
    }
    let guid = item.guid;
    c.inventory.take(at);
    send_slots(c, &[at], events);
    events.push(SessionEvent::ObjectDestroyed(guid));
    Moved::of(&[at])
}

/// Destroys `count` of `entry` from wherever the character carries it, last stacks first: a
/// hand-in's collected items, a Token purchase's Tokens. The caller has checked they are there.
pub(crate) fn take(c: &mut Char, entry: u32, mut count: u32, events: &mut Vec<SessionEvent>) {
    let stacks: Vec<(Pos, u32)> = c
        .inventory
        .iter()
        .filter(|(_, i)| i.entry == entry)
        .map(|(at, i)| (at, i.count))
        .collect();
    for (at, have) in stacks.into_iter().rev() {
        if count == 0 {
            break;
        }
        let take = have.min(count);
        // 0 destroys the whole stack.
        let n = if take == have { 0 } else { take as u8 };
        destroy(c, at.bag, at.slot, n, events);
        count -= take;
    }
    debug_assert_eq!(count, 0, "the items to take are all there");
}

/// Whether the item at `at` may leave it: not a bag that holds items. Sells and the like ask.
pub(crate) fn may_leave(c: &Char, at: Pos) -> bool {
    !(at.is_bag_slot() && c.inventory.bag_holds_items(at.slot))
}

fn is_two_hander(entry: u32) -> bool {
    items::template(entry).is_some_and(|t| t.inventory_type == gear::INVTYPE_2HWEAPON)
}

/// Whether `item` may sit at `to`: anything in the backpack or a bag; in a bag slot, only a bag;
/// in a worn slot, only what goes there and what the character's Calling may wear. The
/// `InventoryResult` of a refusal.
fn check_place(c: &Char, item: &Item, to: Pos) -> Result<(), u8> {
    if to.is_storage() {
        return Ok(());
    }
    if to.is_bag_slot() {
        return if bag_size(item.entry) > 0 {
            Ok(())
        } else {
            Err(EQUIP_ERR_ITEM_DOESNT_GO_TO_SLOT)
        };
    }
    can_equip(item.entry, c.race, callings::of(c.calling)).map_err(Refusal::code)?;
    let t = items::template(item.entry).ok_or(EQUIP_ERR_ITEM_DOESNT_GO_TO_SLOT)?;
    if !equip_slots(t.inventory_type).contains(&to.slot) {
        return Err(EQUIP_ERR_ITEM_DOESNT_GO_TO_SLOT);
    }
    // Nothing goes in the off hand beside a two-hander.
    if to == Pos::own(gear::SLOT_OFF_HAND)
        && c.inventory
            .get(gear::SLOT_MAIN_HAND)
            .is_some_and(|m| m.guid != item.guid && is_two_hander(m.entry))
    {
        return Err(EQUIP_ERR_CANT_EQUIP_WITH_TWOHANDED);
    }
    Ok(())
}

/// Moves as much of the stack at `src` onto the stack at `dst` as fits.
fn merge(c: &mut Char, src: Pos, dst: Pos, stack: u32, events: &mut Vec<SessionEvent>) -> Moved {
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

/// The fields of the places in `changed`: the player's in one update, each bag's in its own.
pub(crate) fn send_slots(c: &Char, changed: &[Pos], events: &mut Vec<SessionEvent>) {
    let pairs: Vec<(u64, u16, u32)> = changed
        .iter()
        .flat_map(|at| slot_pairs(c, *at, c.inventory.get(*at)))
        .collect();
    let mut objects: Vec<u64> = Vec::new();
    for (object, ..) in &pairs {
        if !objects.contains(object) {
            objects.push(*object);
        }
    }
    for object in objects {
        let fields: Vec<(u16, u32)> = pairs
            .iter()
            .filter(|(o, ..)| *o == object)
            .map(|(_, f, v)| (*f, *v))
            .collect();
        events.push(values(object, &fields));
    }
}
