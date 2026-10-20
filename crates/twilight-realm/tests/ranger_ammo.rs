//! The Ranger's ammo (#157), played through the realm's seam: the arrows stay in the bags and
//! `PLAYER_AMMO_ID` names them; no arrows, no shot; each shot spends one; bullets do not fit a
//! bow; the arrows' DPS rides on every shot.

mod common;
mod gear_common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::{cast, refusal, stand_at, Mob};
use common::*;
use gear_common::{descriptor, save_shooting, save_with, Carried, BACKPACK_1, ROUGH_ARROW};
use twilight_realm::{Realm, RealmCommand};

const RANGED: u8 = 17;
const CRACKED_SHORTBOW: u32 = 2773;
const BOW_SPEED_SECS: f32 = 2.3;
const LIGHT_SHOT: u32 = 2516;
static AUTO_SHOT: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("ranger", "auto_shot"));
static SNAPSHOT: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("ranger", "snapshot"));
const SPELL_FAILED_NO_AMMO: u8 = 0x43;
/// `EQUIP_ERR_ONLY_AMMO_CAN_GO_HERE`.
const EQUIP_ERR_AMMO_ONLY: u8 = 7;

/// A level 1 Ranger from `blob`, in the world beside a mob near the Crossroads: the realm, its
/// guid, the mob and the entry burst.
fn enter(guid: u64, blob: &str) -> (Realm, u64, Mob, Vec<SessionEvent>) {
    let blob = blob.replace("calling = \"basic_warrior\"", "calling = \"ranger\"");
    let mut realm = Realm::with_seed(0xA6_0001);
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

/// A Ranger with the bow and `items`, and no ammo set.
fn unloaded(items: &[Carried]) -> (Realm, u64, Mob, Vec<SessionEvent>) {
    let mut all = vec![(RANGED, CRACKED_SHORTBOW, 1)];
    all.extend_from_slice(items);
    let (guid, blob) = save_with(1, 0, &all);
    enter(guid, &blob)
}

/// The arrows the save holds, by its `count`.
fn arrows_left(realm: &Realm, guid: u64) -> u32 {
    let blob = realm.export_character(guid).unwrap();
    let at = blob
        .find(&format!("entry = {ROUGH_ARROW}"))
        .expect("arrows in the save");
    let count = &blob[at..];
    let count = &count[count.find("count = ").unwrap() + 8..];
    count[..count.find('\n').unwrap()].parse().unwrap()
}

#[test]
fn without_arrows_snapshot_is_refused_and_auto_shot_stops() {
    let (mut realm, _, mob, _) = unloaded(&[]);
    stand_at(&mut realm, &mob, 20.0);
    drain(&mut realm);
    realm.handle(cast(*SNAPSHOT, Some(mob.guid)));
    assert_eq!(
        refusal(&drain(&mut realm), *SNAPSHOT),
        Some(SPELL_FAILED_NO_AMMO)
    );

    // One arrow: the first shot spends it, and the next one cannot go.
    let (guid, blob) = save_shooting(1, &[(RANGED, CRACKED_SHORTBOW, 1)]);
    let blob = blob.replace("count = 200", "count = 1");
    let (mut realm, me, mob, _) = enter(guid, &blob);
    stand_at(&mut realm, &mob, 20.0);
    realm.tick(1.0, &FlatGround);
    drain(&mut realm);
    realm.handle(cast(*AUTO_SHOT, Some(mob.guid)));
    let mut events = Vec::new();
    for _ in 0..60 {
        realm.tick(0.1, &FlatGround);
        events.extend(drain(&mut realm));
    }
    let shots = events
        .iter()
        .filter(|e| matches!(e, SessionEvent::SpellGo { spell_id, .. } if *spell_id == *AUTO_SHOT))
        .count();
    assert_eq!(shots, 1, "one arrow, one shot");
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::CancelAutoRepeat)));
    assert_eq!(
        descriptor(&events, me).player_ammo_id(),
        Some(0),
        "the ammo clears"
    );
}

#[test]
fn each_shot_spends_one_arrow() {
    let (guid, blob) = save_shooting(1, &[(RANGED, CRACKED_SHORTBOW, 1)]);
    let (mut realm, me, mob, entry) = enter(guid, &blob);
    assert_eq!(descriptor(&entry, me).player_ammo_id(), Some(ROUGH_ARROW));
    stand_at(&mut realm, &mob, 20.0);
    realm.tick(1.0, &FlatGround);
    drain(&mut realm);
    realm.handle(cast(*SNAPSHOT, Some(mob.guid)));
    assert_eq!(refusal(&drain(&mut realm), *SNAPSHOT), None);
    assert_eq!(arrows_left(&realm, me), 199);
    realm.handle(cast(*AUTO_SHOT, Some(mob.guid)));
    realm.tick(0.05, &FlatGround);
    drain(&mut realm);
    assert_eq!(arrows_left(&realm, me), 198);
}

#[test]
fn bullets_do_not_fit_a_bow() {
    let (mut realm, _, mob, _) = unloaded(&[(BACKPACK_1, LIGHT_SHOT, 200)]);
    realm.handle(RealmCommand::SetAmmo { entry: LIGHT_SHOT });
    let events = drain(&mut realm);
    assert!(events.iter().any(|e| matches!(
        e,
        SessionEvent::InventoryFailure {
            reason: EQUIP_ERR_AMMO_ONLY,
            ..
        }
    )));
    stand_at(&mut realm, &mob, 20.0);
    drain(&mut realm);
    realm.handle(cast(*SNAPSHOT, Some(mob.guid)));
    assert_eq!(
        refusal(&drain(&mut realm), *SNAPSHOT),
        Some(SPELL_FAILED_NO_AMMO)
    );
}

/// Setting arrows by hand (`CMSG_SET_AMMO`) names them on the sheet, and their 1.5 DPS times the
/// bow's 2.3 s rides on the ranged damage.
#[test]
fn the_arrows_dps_rides_on_the_shot() {
    let (mut realm, me, _, entry) = unloaded(&[(BACKPACK_1, ROUGH_ARROW, 200)]);
    let bare = descriptor(&entry, me);
    realm.handle(RealmCommand::SetAmmo { entry: ROUGH_ARROW });
    let mut events = entry;
    events.extend(drain(&mut realm));
    let loaded = descriptor(&events, me);
    assert_eq!(loaded.player_ammo_id(), Some(ROUGH_ARROW));
    let more = 1.5 * BOW_SPEED_SECS;
    for (with, without) in [
        (
            loaded.unit_min_ranged_damage(),
            bare.unit_min_ranged_damage(),
        ),
        (
            loaded.unit_max_ranged_damage(),
            bare.unit_max_ranged_damage(),
        ),
    ] {
        let (with, without) = (with.unwrap(), without.unwrap());
        assert!(
            (with - without - more).abs() < 0.01,
            "{without} then {with}"
        );
    }
    let saved = realm.export_character(me).unwrap();
    assert!(saved.contains(&format!("ammo = {ROUGH_ARROW}")), "saved");
}
