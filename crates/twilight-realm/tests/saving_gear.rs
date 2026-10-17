//! Bags, equipment and money are saved, and come back with the character.

mod common;
mod gear_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::*;
use twilight_realm::SaveError;

const FINE_SCIMITAR: u32 = 4560;
const MELTED_CANDLE: u32 = 755; // grey trash, stacks to 5
const GNOLL_WAR_HARNESS: u32 = 1211;

/// Enters the world with `blob` in a fresh realm; the entry burst.
fn reload(guid: u64, blob: &str) -> Vec<SessionEvent> {
    let mut realm = Realm::new();
    realm.import_character(blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    drain(&mut realm)
}

/// `(slot, entry, count, durability)` of every carried item, as the client sees them.
fn carried(events: &[SessionEvent], me: u64) -> Vec<(u8, u32, u32, u32)> {
    let f = descriptor(events, me);
    let creates = item_creates(events);
    let mut out = Vec::new();
    for slot in 0..39u8 {
        let guid = if slot < 23 {
            f.player_inv_slot(slot)
        } else {
            f.player_pack_slot(slot - 23)
        };
        let Some(guid) = guid.filter(|g| *g != 0) else {
            continue;
        };
        let (_, item) = creates
            .iter()
            .find(|(g, _)| *g == guid)
            .expect("its object");
        out.push((
            slot,
            item.object_entry().unwrap(),
            item.item_stack_count().unwrap(),
            item.item_durability().unwrap_or(0),
        ));
    }
    out
}

#[test]
fn bags_equipment_and_money_survive_a_reload() {
    let (mut realm, me, _) = warrior_with(
        10,
        1234,
        &[
            (23, FINE_SCIMITAR, 1),
            (24, MELTED_CANDLE, 3),
            (25, GNOLL_WAR_HARNESS, 1),
        ],
    );
    realm.handle(RealmCommand::AutoEquipItem { bag: 255, slot: 23 });
    realm.handle(RealmCommand::AutoEquipItem { bag: 255, slot: 25 });
    realm.handle(RealmCommand::Logout);
    drain(&mut realm);
    let blob = realm.export_character(me).unwrap();

    let events = reload(me, &blob);
    assert_eq!(descriptor(&events, me).player_money(), Some(1234));
    // Fine Scimitar has 30 durability, the harness 65.
    assert_eq!(
        carried(&events, me),
        [
            (4, GNOLL_WAR_HARNESS, 1, 65),
            (15, FINE_SCIMITAR, 1, 30),
            (24, MELTED_CANDLE, 3, 0),
        ]
    );
    // And worn, it still counts: the sword's swing.
    assert_eq!(descriptor(&events, me).unit_base_attack_time(0), Some(1900));
}

#[test]
fn worn_down_durability_is_kept() {
    let (guid, blob) = save_with(1, 0, &[(15, FINE_SCIMITAR, 1)]);
    let blob = blob.replace("count = 1\n", "count = 1\ndurability = 7\n");
    let mut realm = Realm::new();
    realm.import_character(&blob).unwrap();
    assert!(realm
        .export_character(guid)
        .unwrap()
        .contains("durability = 7"));
    assert_eq!(
        carried(&reload(guid, &blob), guid),
        [(15, FINE_SCIMITAR, 1, 7)]
    );
}

#[test]
fn loot_taken_is_there_after_a_reload() {
    let (mut realm, me, events) = warrior_with(5, 0, &[]);
    let mob = the_mob(&events);
    kill(&mut realm, &mob);
    realm.handle(RealmCommand::Loot { guid: mob.guid });
    let (gold, entry) = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::LootResponse { gold, items, .. } => Some((gold, items[0].item_id)),
            _ => None,
        })
        .unwrap();
    realm.handle(RealmCommand::LootMoney);
    realm.handle(RealmCommand::AutostoreLootItem { slot: 0 });
    let blob = realm
        .drain_storage()
        .into_iter()
        .rev()
        .find_map(|op| match op {
            twilight_realm::Storage::Write { blob, .. } => Some(blob),
            _ => None,
        })
        .expect("looting saved");

    let events = reload(me, &blob);
    assert_eq!(descriptor(&events, me).player_money(), Some(gold));
    let back = carried(&events, me);
    assert_eq!(back.len(), 1);
    assert_eq!((back[0].0, back[0].1), (23, entry));
}

#[test]
fn a_save_from_before_gear_loads_with_empty_bags() {
    let (guid, blob) = save_with(3, 0, &[]);
    let old = blob.replace("money = 0\n", "");
    assert!(!old.contains("money") && !old.contains("items"));
    let events = reload(guid, &old);
    assert_eq!(descriptor(&events, guid).player_money(), Some(0));
    assert!(carried(&events, guid).is_empty());
}

#[test]
fn a_save_naming_an_unknown_item_is_unreadable() {
    let (_, blob) = save_with(1, 0, &[(23, 999_999, 1)]);
    assert_eq!(
        Realm::new().import_character(&blob),
        Err(SaveError::Unreadable)
    );
}

#[test]
fn a_save_from_before_the_gear_slots_loads_without_the_retired_gear() {
    // Recruit's Pants, which the ten Gear Slots retired (ADR 0007), and a sword that stays.
    let (guid, blob) = save_with(1, 0, &[(6, 39, 1), (23, FINE_SCIMITAR, 1)]);
    let events = reload(guid, &blob);
    let entries: Vec<u32> = carried(&events, guid).iter().map(|c| c.1).collect();
    assert_eq!(entries, [FINE_SCIMITAR]);
}

#[test]
fn a_save_naming_a_slot_twice_is_unreadable() {
    let (_, blob) = save_with(1, 0, &[(23, FINE_SCIMITAR, 1), (23, FINE_SCIMITAR, 1)]);
    assert_eq!(
        Realm::new().import_character(&blob),
        Err(SaveError::Unreadable)
    );
}
