//! Quivers and ammo pouches (#158), played through the realm's seam: a quiver holds only arrows,
//! and while one is in a bag slot the bow shoots 10% faster, back to its own speed once it is
//! taken off.

mod common;
mod gear_common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::{cast, stand_at, Mob};
use common::*;
use gear_common::{descriptor, save_shooting, BACKPACK_1};
use twilight_realm::{Realm, RealmCommand};

const RANGED: u8 = 17;
const CRACKED_SHORTBOW: u32 = 2773;
const BOW_SPEED_MS: u32 = 2300;
const AUTO_SHOT: u32 = 75;
/// Small Quiver: 8 slots, 10% ranged haste.
const SMALL_QUIVER: u32 = 5439;
const QUIVER_HASTE_PCT: f32 = 10.0;
/// The first bag slot, and the backpack's wire bag.
const BAG_1: u8 = 19;
const PLAYER_BAG: u8 = 255;
/// Worn Shortsword: no arrow.
const WORN_SHORTSWORD: u32 = 25;
/// `EQUIP_ERR_ITEM_DOESNT_GO_INTO_BAG`.
const EQUIP_ERR_WRONG_BAG_TYPE: u8 = 15;

/// A level 1 Ranger with the bow, 200 arrows set as ammo, a Small Quiver in the first bag slot and
/// a sword in the backpack, in the world beside a mob: the realm, its guid, the mob, the burst.
fn ranger() -> (Realm, u64, Mob, Vec<SessionEvent>) {
    let (guid, blob) = save_shooting(
        1,
        &[
            (RANGED, CRACKED_SHORTBOW, 1),
            (BAG_1, SMALL_QUIVER, 1),
            (BACKPACK_1 + 1, WORN_SHORTSWORD, 1),
        ],
    );
    let blob = blob.replace("calling = \"basic_warrior\"", "calling = \"ranger\"");
    let mut realm = Realm::with_seed(0xA7_0001);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    let mob = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                ..
            } => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .expect("a mob near the Crossroads");
    (realm, guid, mob, events)
}

fn wrong_bag(events: &[SessionEvent]) -> bool {
    events.iter().any(|e| {
        matches!(
            e,
            SessionEvent::InventoryFailure {
                reason: EQUIP_ERR_WRONG_BAG_TYPE,
                ..
            }
        )
    })
}

#[test]
fn a_quiver_takes_only_arrows() {
    let (mut realm, _, _, _) = ranger();
    realm.handle(RealmCommand::SwapItem {
        src_bag: PLAYER_BAG,
        src_slot: BACKPACK_1 + 1,
        dst_bag: BAG_1,
        dst_slot: 0,
    });
    assert!(wrong_bag(&drain(&mut realm)), "the sword is refused");
    realm.handle(RealmCommand::AutoStoreBagItem {
        src_bag: PLAYER_BAG,
        src_slot: BACKPACK_1 + 1,
        dst_bag: BAG_1,
    });
    assert!(wrong_bag(&drain(&mut realm)), "and auto-storing it too");
    realm.handle(RealmCommand::SwapItem {
        src_bag: PLAYER_BAG,
        src_slot: BACKPACK_1,
        dst_bag: BAG_1,
        dst_slot: 0,
    });
    let events = drain(&mut realm);
    assert!(!wrong_bag(&events), "the arrows go in");
    assert!(!events
        .iter()
        .any(|e| matches!(e, SessionEvent::InventoryFailure { .. })));
}

/// With the quiver on, Auto Shot comes every 2.3 s / 1.1; the sheet's ranged speed says so, and
/// taking the quiver off brings back the bow's 2.3 s.
#[test]
fn a_quiver_speeds_the_bow_until_taken_off() {
    let hasted = BOW_SPEED_MS as f32 / (1.0 + QUIVER_HASTE_PCT / 100.0);
    let (mut realm, me, mob, entry) = ranger();
    assert_eq!(
        descriptor(&entry, me).unit_ranged_attack_time(),
        Some(hasted.round() as u32)
    );
    stand_at(&mut realm, &mob, 34.0);
    realm.tick(1.0, &FlatGround);
    drain(&mut realm);
    realm.handle(cast(AUTO_SHOT, Some(mob.guid)));
    let mut shots = Vec::new();
    let mut now = 0.0f32;
    while shots.len() < 2 && now < 5.0 {
        realm.tick(0.01, &FlatGround);
        now += 0.01;
        for e in drain(&mut realm) {
            if matches!(e, SessionEvent::SpellGo { spell_id, .. } if spell_id == AUTO_SHOT) {
                shots.push(now);
            }
        }
    }
    assert_eq!(
        shots.len(),
        2,
        "two shots before the mob closes in: {shots:?}"
    );
    let gap = (shots[1] - shots[0]) * 1000.0;
    assert!(
        (gap - hasted).abs() < 15.0,
        "a shot every {hasted} ms: {gap}"
    );

    // Off into the backpack: the bow's own speed again.
    realm.handle(RealmCommand::CancelAutoRepeat);
    realm.handle(RealmCommand::SwapItem {
        src_bag: PLAYER_BAG,
        src_slot: BAG_1,
        dst_bag: PLAYER_BAG,
        dst_slot: BACKPACK_1 + 2,
    });
    let off = drain(&mut realm);
    let mut all = entry;
    all.extend(off);
    assert_eq!(
        descriptor(&all, me).unit_ranged_attack_time(),
        Some(BOW_SPEED_MS)
    );
}
