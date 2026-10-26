//! Looting: a dead mob's money and items, through the stock loot window, into the bags.

mod common;
mod gear_common;

use benilla_protocol::messages::{loot_error, LootItem};
use benilla_protocol::SessionEvent;
use common::*;
use gear_common::*;
use twilight_realm::rules::loot::{money, pool, Quality};

/// `LOOT_CORPSE`.
const LOOT_CORPSE: u8 = 1;
/// `BAG_PLAYER_INVENTORY` and the backpack's first slot.
const BACKPACK: u8 = 255;
const PACK_FIRST: u32 = 23;

fn loot_window(events: &[SessionEvent]) -> Option<(u64, u8, u32, Vec<LootItem>)> {
    events.iter().find_map(|e| match e {
        SessionEvent::LootResponse {
            guid,
            loot_type,
            gold,
            items,
        } => Some((*guid, *loot_type, *gold, items.clone())),
        _ => None,
    })
}

/// A level 5 warrior, the level 3 mob dead at its feet.
fn after_a_kill() -> (Realm, u64, Mob, Vec<SessionEvent>) {
    let (mut realm, me, events) = fighter_with(5, 0, &[]);
    let mob = the_mob(&events);
    let events = kill(&mut realm, &mob);
    (realm, me, mob, events)
}

#[test]
fn a_killed_mob_turns_lootable() {
    let (_, _, mob, events) = after_a_kill();
    assert!(descriptor(&events, mob.guid).unit_lootable());
}

#[test]
fn the_loot_window_shows_the_mobs_money_and_item() {
    let (mut realm, _, mob, _) = after_a_kill();
    realm.handle(RealmCommand::Loot { guid: mob.guid });
    let (guid, loot_type, gold, items) = loot_window(&drain(&mut realm)).expect("a loot window");
    assert_eq!((guid, loot_type), (mob.guid, LOOT_CORPSE));
    let (lo, hi) = money(3);
    assert!((lo..=hi).contains(&gold), "{gold} copper");
    assert_eq!(items.len(), 1, "one item a kill");
    let item = items[0];
    assert_eq!(item.slot, 0);
    assert!(item.count >= 1);
    // A level 3 mob's drop: from the pools around its level.
    let near = [Quality::Poor, Quality::Common, Quality::Uncommon]
        .iter()
        .any(|q| pool(3, *q).contains(&item.item_id));
    assert!(near, "item {} is not a level 3 drop", item.item_id);

    // The row's icon is the item's.
    realm.handle(RealmCommand::ItemQuery {
        entry: item.item_id,
    });
    let info = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::ItemTemplate { entry, info } if entry == item.item_id => info,
            _ => None,
        })
        .expect("the item's template");
    assert_eq!(info.display_info_id, item.display_info_id);
    assert!(!info.name.is_empty());
}

#[test]
fn a_corpse_is_in_reach_as_far_as_an_npc_is() {
    let (mut realm, _, mob, _) = after_a_kill();
    let step_back = |realm: &mut Realm, yards: f32| {
        realm.handle(RealmCommand::Move {
            motion: Default::default(),
            position: [mob.position[0] + yards, mob.position[1], mob.position[2]],
            orientation: std::f32::consts::PI,
        });
    };
    step_back(&mut realm, 8.0);
    realm.handle(RealmCommand::Loot { guid: mob.guid });
    assert!(loot_window(&drain(&mut realm)).is_some(), "8 yd off");
    realm.handle(RealmCommand::LootRelease { guid: mob.guid });
    step_back(&mut realm, 12.0);
    realm.handle(RealmCommand::Loot { guid: mob.guid });
    let far = drain(&mut realm);
    assert!(loot_window(&far).is_none(), "12 yd off");
    assert!(far.iter().any(|e| matches!(
        e,
        SessionEvent::LootError { error, .. } if *error == loot_error::TOO_FAR
    )));
}

#[test]
fn a_living_mob_cannot_be_looted() {
    let (mut realm, _, events) = fighter_with(5, 0, &[]);
    let mob = the_mob(&events);
    stand_by(&mut realm, &mob);
    realm.handle(RealmCommand::Loot { guid: mob.guid });
    let events = drain(&mut realm);
    assert!(loot_window(&events).is_none());
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::LootError { guid, .. } if *guid == mob.guid)));
}

#[test]
fn taking_the_money_puts_it_in_the_purse() {
    let (mut realm, me, mob, _) = after_a_kill();
    realm.handle(RealmCommand::Loot { guid: mob.guid });
    let (_, _, gold, _) = loot_window(&drain(&mut realm)).unwrap();

    realm.handle(RealmCommand::LootMoney);
    let events = drain(&mut realm);
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::LootClearMoney)));
    assert_eq!(descriptor(&events, me).player_money(), Some(gold));

    // Taken once only.
    realm.handle(RealmCommand::LootMoney);
    assert!(descriptor(&drain(&mut realm), me).player_money().is_none());
}

#[test]
fn taking_the_item_puts_it_in_the_backpack() {
    let (mut realm, me, mob, _) = after_a_kill();
    realm.handle(RealmCommand::Loot { guid: mob.guid });
    let (_, _, _, items) = loot_window(&drain(&mut realm)).unwrap();
    let row = items[0];

    realm.handle(RealmCommand::AutostoreLootItem { slot: 0 });
    let events = drain(&mut realm);

    let creates = item_creates(&events);
    assert_eq!(creates.len(), 1);
    let (item_guid, fields) = &creates[0];
    assert_eq!(item_guid >> 48, 0x4000, "an item guid");
    assert_eq!(fields.object_entry(), Some(row.item_id));
    assert_eq!(fields.item_stack_count(), Some(row.count));
    // The backpack's first slot holds it.
    assert_eq!(
        descriptor(&events, me).player_pack_slot(0),
        Some(*item_guid)
    );
    let push = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ItemPushResult(p) => Some(*p),
            _ => None,
        })
        .expect("the you-receive-loot line");
    assert_eq!(
        (
            push.player_guid,
            push.bag_slot,
            push.item_slot,
            push.item_entry,
            push.count
        ),
        (me, BACKPACK, PACK_FIRST, row.item_id, row.count)
    );
    assert!(!push.from_npc && push.show_in_chat);
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::LootRemoved { slot: 0 })));

    // Gone from the corpse.
    realm.handle(RealmCommand::AutostoreLootItem { slot: 0 });
    assert!(item_creates(&drain(&mut realm)).is_empty());
}

#[test]
fn an_emptied_corpse_is_no_longer_lootable() {
    let (mut realm, _, mob, _) = after_a_kill();
    realm.handle(RealmCommand::Loot { guid: mob.guid });
    drain(&mut realm);
    realm.handle(RealmCommand::LootMoney);
    realm.handle(RealmCommand::AutostoreLootItem { slot: 0 });
    realm.handle(RealmCommand::LootRelease { guid: mob.guid });
    let events = drain(&mut realm);
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::LootReleaseResponse { guid } if *guid == mob.guid)));
    assert!(!descriptor(&events, mob.guid).unit_lootable());
}

#[test]
fn a_corpse_released_with_loot_left_stays_lootable() {
    let (mut realm, _, mob, _) = after_a_kill();
    realm.handle(RealmCommand::Loot { guid: mob.guid });
    drain(&mut realm);
    realm.handle(RealmCommand::LootRelease { guid: mob.guid });
    drain(&mut realm);

    realm.handle(RealmCommand::Loot { guid: mob.guid });
    let (_, _, gold, items) = loot_window(&drain(&mut realm)).expect("the loot is still there");
    assert!(gold > 0);
    assert_eq!(items.len(), 1);
}

#[test]
fn looting_asks_for_a_save() {
    let (mut realm, me, mob, _) = after_a_kill();
    realm.drain_storage();
    realm.handle(RealmCommand::Loot { guid: mob.guid });
    realm.handle(RealmCommand::LootMoney);
    realm.handle(RealmCommand::AutostoreLootItem { slot: 0 });
    realm.handle(RealmCommand::LootRelease { guid: mob.guid });
    assert!(realm
        .drain_storage()
        .iter()
        .any(|s| matches!(s, twilight_realm::Storage::Write { guid, .. } if *guid == me)));
}

#[test]
fn a_full_backpack_refuses_the_item() {
    // Sixteen backpack slots, all taken.
    let full: Vec<Carried> = (23..39).map(|slot| (slot, 1384, 1)).collect();
    let (mut realm, _, events) = fighter_with(5, 0, &full);
    let mob = the_mob(&events);
    kill(&mut realm, &mob);
    realm.handle(RealmCommand::Loot { guid: mob.guid });
    drain(&mut realm);
    realm.handle(RealmCommand::AutostoreLootItem { slot: 0 });
    let events = drain(&mut realm);
    assert!(item_creates(&events).is_empty());
    assert!(events.iter().any(|e| matches!(
        e,
        // `EQUIP_ERR_INVENTORY_FULL`.
        SessionEvent::InventoryFailure { reason: 50, .. }
    )));
}
