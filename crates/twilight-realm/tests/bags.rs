//! Bags: a bag goes in one of the four bag slots, where it holds items of its own. Loot, buys and
//! hand-ins fill the backpack, then the bags in order; a bag that holds items stays where it is.

mod common;
mod gear_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::*;

const SMALL_RED_POUCH: u32 = 805; // 6 slots
const BLUE_LEATHER_BAG: u32 = 856; // 8 slots
const DULL_BLADE: u32 = 1384;

/// `BAG_PLAYER_INVENTORY`.
const BACKPACK: u8 = 255;
const BAG_1: u8 = 19;
const BAG_2: u8 = 20;
const PACK_1: u8 = 23;
const PACK_2: u8 = 24;

/// `OBJECT_FIELD_TYPE` and `TYPEMASK_OBJECT | TYPEMASK_ITEM | TYPEMASK_CONTAINER`.
const OBJECT_FIELD_TYPE: u16 = 2;
const TYPE_CONTAINER: u32 = 0x07;
/// `ITEM_FIELD_CONTAINED`.
const ITEM_FIELD_CONTAINED: u16 = 8;
/// `EQUIP_ERR_CAN_ONLY_DO_WITH_EMPTY_BAGS`.
const ONLY_EMPTY_BAGS: u8 = 31;

/// A warrior carrying items, with every event so far, so the descriptors can be merged.
struct Script {
    realm: Realm,
    me: u64,
    events: Vec<SessionEvent>,
}

impl Script {
    fn new(items: &[Carried], bagged: &[Bagged]) -> Self {
        let (realm, me, events) = warrior_with_bagged(5, 0, items, bagged);
        Self { realm, me, events }
    }

    fn send(&mut self, command: RealmCommand) -> Vec<SessionEvent> {
        self.realm.handle(command);
        let new = drain(&mut self.realm);
        self.events.extend(new.clone());
        new
    }

    /// The guid in player slot `slot`, 0 for none, as the client holds it.
    fn slot(&self, slot: u8) -> u64 {
        let f = descriptor(&self.events, self.me);
        if slot < 23 {
            f.player_inv_slot(slot).unwrap_or(0)
        } else {
            f.player_pack_slot(slot - 23).unwrap_or(0)
        }
    }

    /// The entry of the item in `slot` of the bag in bag slot `bag`, as the client holds it.
    fn in_bag(&self, bag: u8, slot: u8) -> Option<u32> {
        let bag = self.slot(bag);
        let guid = descriptor(&self.events, bag)
            .container_slot(slot)
            .filter(|g| *g != 0)?;
        descriptor(&self.events, guid).object_entry()
    }
}

fn field(fields: &benilla_protocol::ObjectFields, index: u16) -> Option<u32> {
    fields
        .raw_fields()
        .find_map(|(i, v)| (i == index).then_some(v))
}

fn refusal(events: &[SessionEvent]) -> Option<u8> {
    events.iter().find_map(|e| match e {
        SessionEvent::InventoryFailure { reason, .. } => Some(*reason),
        _ => None,
    })
}

#[test]
fn a_bag_is_a_container_with_its_slot_count() {
    let s = Script::new(&[(PACK_1, SMALL_RED_POUCH, 1)], &[]);
    let (container, fields) = s
        .events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ItemCreate {
                container, fields, ..
            } if fields.object_entry() == Some(SMALL_RED_POUCH) => Some((*container, fields)),
            _ => None,
        })
        .expect("the pouch's object");
    assert!(container, "created as a container");
    assert_eq!(field(fields, OBJECT_FIELD_TYPE), Some(TYPE_CONTAINER));
    assert_eq!(fields.container_num_slots(), Some(6));
}

#[test]
fn a_bag_from_the_backpack_equips_into_the_first_bag_slot() {
    let mut s = Script::new(&[(PACK_1, SMALL_RED_POUCH, 1)], &[]);
    let pouch = s.slot(PACK_1);
    let events = s.send(RealmCommand::AutoEquipItem {
        bag: BACKPACK,
        slot: PACK_1,
    });
    assert_eq!(refusal(&events), None);
    assert_eq!(s.slot(BAG_1), pouch);
    assert_eq!(s.slot(PACK_1), 0);
    // A second bag takes the next slot.
    let mut s = Script::new(
        &[(BAG_1, SMALL_RED_POUCH, 1), (PACK_2, BLUE_LEATHER_BAG, 1)],
        &[],
    );
    let bag = s.slot(PACK_2);
    s.send(RealmCommand::AutoEquipItem {
        bag: BACKPACK,
        slot: PACK_2,
    });
    assert_eq!(s.slot(BAG_2), bag);
}

#[test]
fn only_a_bag_goes_in_a_bag_slot() {
    let mut s = Script::new(&[(PACK_1, DULL_BLADE, 1)], &[]);
    let events = s.send(RealmCommand::SwapInvItem {
        src: PACK_1,
        dst: BAG_1,
    });
    assert!(refusal(&events).is_some());
    assert_eq!(s.slot(BAG_1), 0);
}

#[test]
fn loot_that_finds_the_backpack_full_goes_in_the_bag() {
    let mut items: Vec<Carried> = (23..39).map(|slot| (slot, DULL_BLADE, 1)).collect();
    items.push((BAG_1, SMALL_RED_POUCH, 1));
    let mut s = Script::new(&items, &[]);
    let mob = the_mob(&s.events);
    let fight = kill(&mut s.realm, &mob);
    s.events.extend(fight);
    s.send(RealmCommand::Loot { guid: mob.guid });
    let events = s.send(RealmCommand::AutostoreLootItem { slot: 0 });

    assert_eq!(refusal(&events), None);
    let push = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ItemPushResult(p) => Some(*p),
            _ => None,
        })
        .expect("the you-receive-loot line");
    assert_eq!((push.bag_slot, push.item_slot), (BAG_1, 0));
    assert_eq!(s.in_bag(BAG_1, 0), Some(push.item_entry));
    let (looted, fields) = item_creates(&events)
        .into_iter()
        .next()
        .expect("the loot's object");
    assert_eq!(
        field(&fields, ITEM_FIELD_CONTAINED),
        Some(s.slot(BAG_1) as u32),
        "contained by the bag"
    );
    assert_ne!(looted, 0);
}

#[test]
fn items_move_into_and_out_of_a_bag() {
    let mut s = Script::new(&[(BAG_1, SMALL_RED_POUCH, 1), (PACK_1, DULL_BLADE, 1)], &[]);
    let events = s.send(RealmCommand::SwapItem {
        src_bag: BACKPACK,
        src_slot: PACK_1,
        dst_bag: BAG_1,
        dst_slot: 3,
    });
    assert_eq!(refusal(&events), None);
    assert_eq!(s.in_bag(BAG_1, 3), Some(DULL_BLADE));
    assert_eq!(s.slot(PACK_1), 0);

    // Equipped straight out of the bag.
    s.send(RealmCommand::AutoEquipItem {
        bag: BAG_1,
        slot: 3,
    });
    assert_eq!(s.in_bag(BAG_1, 3), None);
    assert_ne!(s.slot(15), 0, "in the main hand");
}

#[test]
fn a_bag_that_holds_items_stays_put_and_an_empty_one_moves() {
    let mut s = Script::new(&[(BAG_1, SMALL_RED_POUCH, 1)], &[(BAG_1, 0, DULL_BLADE, 1)]);
    let pouch = s.slot(BAG_1);
    assert_eq!(s.in_bag(BAG_1, 0), Some(DULL_BLADE));
    let events = s.send(RealmCommand::SwapInvItem {
        src: BAG_1,
        dst: PACK_1,
    });
    assert_eq!(refusal(&events), Some(ONLY_EMPTY_BAGS));
    assert_eq!(s.slot(BAG_1), pouch);
    let events = s.send(RealmCommand::DestroyItem {
        bag: BACKPACK,
        slot: BAG_1,
        count: 0,
    });
    assert_eq!(refusal(&events), Some(ONLY_EMPTY_BAGS));
    assert_eq!(s.slot(BAG_1), pouch);

    // Emptied, it comes off.
    s.send(RealmCommand::SwapItem {
        src_bag: BAG_1,
        src_slot: 0,
        dst_bag: BACKPACK,
        dst_slot: PACK_2,
    });
    let events = s.send(RealmCommand::SwapInvItem {
        src: BAG_1,
        dst: PACK_1,
    });
    assert_eq!(refusal(&events), None);
    assert_eq!(s.slot(PACK_1), pouch);
    assert_eq!(s.slot(BAG_1), 0);
}

#[test]
fn items_in_bags_survive_a_save_and_reload() {
    let mut s = Script::new(&[(BAG_1, SMALL_RED_POUCH, 1), (PACK_1, DULL_BLADE, 1)], &[]);
    s.send(RealmCommand::SwapItem {
        src_bag: BACKPACK,
        src_slot: PACK_1,
        dst_bag: BAG_1,
        dst_slot: 5,
    });
    let blob = s.realm.export_character(s.me).unwrap();

    let (realm, me, events) = enter_saved(s.me, &blob);
    let again = Script { realm, me, events };
    assert_eq!(
        descriptor(&again.events, again.slot(BAG_1)).object_entry(),
        Some(SMALL_RED_POUCH)
    );
    assert_eq!(again.in_bag(BAG_1, 5), Some(DULL_BLADE));
}

#[test]
fn a_save_from_before_bags_still_loads() {
    // Every item a player slot, no `bag` key: the save as it was before bags.
    let (guid, blob) = save_with(5, 0, &[(PACK_1, DULL_BLADE, 1)]);
    assert!(!blob.contains("bag ="), "{blob}");
    let (_, me, events) = enter_saved(guid, &blob);
    let s = Script {
        realm: realm(),
        me,
        events,
    };
    assert_ne!(s.slot(PACK_1), 0);
}
