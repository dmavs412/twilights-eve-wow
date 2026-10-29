//! The Vanguard's block chance: the shield's, and Shield of the Penitent's block aura with the
//! burn it deals on each block. Played through the realm's seam by a level 10 Vanguard near the
//! Crossroads, in the starting outfit's sword and shield (or without the shield).

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

/// The spell id of the Vanguard skill `key`.
fn skill(key: &str) -> u32 {
    vanguard()
        .skills
        .iter()
        .find(|s| s.key == key)
        .unwrap_or_else(|| panic!("the Vanguard has {key} on the bar"))
        .spell_id
}

/// A level 10 Vanguard, the shield worn when `shield`, in the world: the realm, its guid, a mob
/// near it and what the client has seen.
fn vanguard_in_world(shield: bool) -> (Realm, u64, Mob, View) {
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

#[test]
fn a_shield_blocks_and_no_shield_does_not() {
    let (_, me, _, shielded) = vanguard_in_world(true);
    let (_, me2, _, unshielded) = vanguard_in_world(false);
    assert!(block(&shielded, me) > 0.0, "a shield blocks");
    assert_eq!(block(&unshielded, me2), 0.0, "no shield, no block");
}

#[test]
fn shield_of_the_penitent_adds_fifty_block_for_ten_seconds() {
    let penitent = skill("shield_of_penitent");
    let (mut realm, me, mob, mut view) = vanguard_in_world(true);
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
    assert!((block(&view, me) - base - 50.0).abs() < 1e-3, "on");
    view.run(&mut realm, 9.0);
    assert!(
        (block(&view, me) - base - 50.0).abs() < 1e-3,
        "still on at 9 s"
    );
    view.run(&mut realm, 2.0);
    assert!((block(&view, me) - base).abs() < 1e-3, "off after 11 s");
}

/// Whether `e` is `mob`'s swing at `me` that the player blocked.
fn blocked(e: &SessionEvent, mob: u64, me: u64) -> bool {
    matches!(e, SessionEvent::AttackerState(s)
        if s.attacker == mob && s.victim == me && (s.blocked > 0 || s.victim_state == 5))
}

/// The damage `me` dealt under `spell_id` in `events`, one entry a hit.
fn dealt(events: &[SessionEvent], me: u64, spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.attacker == me && l.spell_id == spell_id => {
                Some(l.damage)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn a_block_under_shield_of_the_penitent_burns_the_attacker() {
    let penitent = skill("shield_of_penitent");
    let (mut realm, me, mob, mut view) = vanguard_in_world(true);
    // Rage for the cast from trading blows, then the player stops swinging.
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let mut waited = 0;
    while view.rage(me) < 10 {
        view.run(&mut realm, 0.1);
        waited += 1;
        assert!(waited < 300, "never reached 10 rage");
    }
    realm.handle(RealmCommand::AttackStop);
    realm.handle(cast(penitent, None));
    let mut seen = view.drain(&mut realm);
    assert_eq!(refusal(&seen, penitent), None, "the cast");
    for _ in 0..95 {
        seen.extend(view.run(&mut realm, 0.1));
    }
    let blocks = seen.iter().filter(|e| blocked(e, mob.guid, me)).count();
    let burns = dealt(&seen, me, penitent);
    assert!(blocks > 0, "the seed blocks under the aura");
    assert_eq!(burns.len(), blocks, "one burn a block");
    // 12 + 1.2 a level, Holy: no armour, no roll.
    // The Power Level of a Tier 1 level 10 is 16.
    let want = 31;
    assert!(burns.iter().all(|&d| d == want), "{burns:?}, want {want}");
}
