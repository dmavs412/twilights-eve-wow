//! The Vanguard's defensive stats: parry and block chance from talents, and Shield of the
//! Penitent's block aura. Played through the realm's seam by a level 10 Vanguard with the talents
//! in its save, near the Crossroads, in the starting outfit's sword and shield (or without the
//! shield).

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::callings;
use twilight_realm::{Realm, RealmCommand};

const SEED: u64 = 0x5EED_0099;
const LEVEL: u8 = 10;
const OFF_HAND: u8 = 16;

fn vanguard() -> &'static callings::CallingDef {
    callings::calling("vanguard").expect("the Vanguard is a shipped Calling")
}

/// The `Talent.dbc` id of the Vanguard talent `key`.
fn talent(key: &str) -> u32 {
    vanguard()
        .talent_tabs
        .iter()
        .flat_map(|t| &t.talents)
        .find(|t| t.key == key)
        .unwrap_or_else(|| panic!("the Vanguard has {key}"))
        .talent_id
}

/// The spell id of the Vanguard skill `key`.
fn skill(key: &str) -> u32 {
    vanguard()
        .skills
        .iter()
        .find(|s| s.key == key)
        .unwrap_or_else(|| panic!("the Vanguard has {key} on the bar"))
        .spell_id
}

/// A level 10 Vanguard with `talents`, the shield worn when `shield`, in the world: the realm,
/// its guid, a mob near it and what the client has seen.
fn vanguard_in_world(talents: &[(u32, u8)], shield: bool) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Bulwark", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {LEVEL}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"vanguard\"");
    if !shield {
        blob = drop_slot(&blob, OFF_HAND);
    }
    if !talents.is_empty() {
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        blob = blob.replace(
            "money = 0\n",
            &format!("money = 0\ntalents = [{}]\n", rows.join(", ")),
        );
    }
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let entry = view.drain(&mut realm);
    let mob = entry
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
    (realm, guid, mob, view)
}

fn block(view: &View, me: u64) -> f32 {
    view.fields(me).player_block_percentage().unwrap()
}

fn parry(view: &View, me: u64) -> f32 {
    view.fields(me).player_parry_percentage().unwrap()
}

#[test]
fn shield_specialization_adds_block_only_with_a_shield() {
    let spec = [(talent("shield_specialization"), 5)];
    let (_, me, _, bare) = vanguard_in_world(&[], true);
    let (_, me2, _, trained) = vanguard_in_world(&spec, true);
    let (_, me3, _, unshielded) = vanguard_in_world(&spec, false);
    assert!(block(&bare, me) > 0.0, "a shield blocks");
    assert!(
        (block(&trained, me2) - block(&bare, me) - 5.0).abs() < 1e-3,
        "block {} -> {}",
        block(&bare, me),
        block(&trained, me2)
    );
    assert_eq!(block(&unshielded, me3), 0.0, "no shield, no block");
}

#[test]
fn reactive_strike_adds_parry() {
    let (_, me, _, bare) = vanguard_in_world(&[], true);
    let (_, me2, _, trained) = vanguard_in_world(&[(talent("reactive_strike"), 5)], true);
    assert!(
        (parry(&trained, me2) - parry(&bare, me) - 5.0).abs() < 1e-3,
        "parry {} -> {}",
        parry(&bare, me),
        parry(&trained, me2)
    );
}

#[test]
fn shield_of_the_penitent_adds_seventy_five_block_for_ten_seconds() {
    let penitent = skill("shield_of_penitent");
    let (mut realm, me, mob, mut view) = vanguard_in_world(&[], true);
    let base = block(&view, me);
    // Rage for the cast, from trading blows with the mob.
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let mut waited = 0.0;
    while view.rage(me) < 10 {
        view.run(&mut realm, 0.1);
        waited += 0.1;
        assert!(waited < 30.0, "never reached 10 rage");
    }
    realm.handle(cast(penitent, None));
    assert_eq!(refusal(&view.drain(&mut realm), penitent), None, "the cast");
    assert!((block(&view, me) - base - 75.0).abs() < 1e-3, "on");
    view.run(&mut realm, 9.0);
    assert!(
        (block(&view, me) - base - 75.0).abs() < 1e-3,
        "still on at 9 s"
    );
    view.run(&mut realm, 2.0);
    assert!((block(&view, me) - base).abs() < 1e-3, "off after 11 s");
}
