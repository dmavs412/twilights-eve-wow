//! The Hearthstone: every character carries one, and using it casts the stock hearth spell, which
//! brings the player back to the Crossroads and then rests for five minutes.

mod common;
mod gear_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::{descriptor, stand_by, the_mob};
use twilight_realm::CROSSROADS;

const HEARTHSTONE: u32 = 6948;
/// `Spell.dbc` 8690, "Hearthstone": a 10 s cast.
const HEARTH_SPELL: u32 = 8690;
const CAST_MS: u32 = 10_000;
const COOLDOWN_MS: u32 = 300_000;

// `SpellCastResult`.
const SPELL_FAILED_INTERRUPTED: u8 = 0x23;
const SPELL_FAILED_NOT_READY: u8 = 0x3c;
const SPELL_FAILED_SPELL_IN_PROGRESS: u8 = 0x61;

/// The backpack's first slot, where a new character's Hearthstone sits.
const FIRST_PACK_SLOT: u8 = 23;
/// The player's own slots, as `CMSG_USE_ITEM` names them.
const PLAYER_BAG: u8 = 255;

/// Far out past The Barrens' camps: nothing streams in to bother the cast.
const FAR_AWAY: [f32; 3] = [CROSSROADS[0] + 3000.0, CROSSROADS[1], 0.0];

fn use_hearthstone() -> RealmCommand {
    RealmCommand::UseItem {
        bag: PLAYER_BAG,
        slot: FIRST_PACK_SLOT,
    }
}

/// A new warrior, walked out far from the Crossroads; its guid and its Hearthstone's guid.
fn far_from_home(realm: &mut Realm) -> (u64, u64) {
    let (me, events) = enter_new(realm, "Thrall", ORC);
    let stone = descriptor(&events, me)
        .player_pack_slot(0)
        .expect("something in the first backpack slot");
    realm.handle(RealmCommand::Move {
        position: FAR_AWAY,
        orientation: 0.0,
    });
    realm.tick(0.1, &FlatGround);
    drain(realm);
    (me, stone)
}

/// Ticks `secs` in 0.1 s steps, collecting the events.
fn run(realm: &mut Realm, secs: f32) -> Vec<SessionEvent> {
    let mut events = Vec::new();
    for _ in 0..(secs * 10.0).round() as u32 {
        realm.tick(0.1, &FlatGround);
        events.extend(drain(realm));
    }
    events
}

fn teleport(events: &[SessionEvent]) -> Option<([f32; 3], f32)> {
    events.iter().find_map(|e| match e {
        SessionEvent::Teleport {
            position,
            orientation,
            ..
        } => Some((*position, *orientation)),
        _ => None,
    })
}

fn spell_start(events: &[SessionEvent]) -> Option<(u64, u32, u32)> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellStart {
            caster,
            spell_id,
            cast_time_ms,
            ..
        } => Some((*caster, *spell_id, *cast_time_ms)),
        _ => None,
    })
}

/// The failure reason of the first failed cast result.
fn cast_failure(events: &[SessionEvent]) -> Option<(u32, Option<u8>)> {
    events.iter().find_map(|e| match e {
        SessionEvent::CastResult {
            spell_id,
            success: false,
            reason,
            ..
        } => Some((*spell_id, *reason)),
        _ => None,
    })
}

fn at_crossroads(position: [f32; 3]) -> bool {
    let [x, y, z, _] = CROSSROADS;
    (position[0] - x).abs() < 0.01 && (position[1] - y).abs() < 0.01 && (position[2] - z).abs() < 0.01
}

#[test]
fn a_new_character_carries_a_hearthstone_in_its_backpack() {
    let mut realm = realm();
    let (me, events) = enter_new(&mut realm, "Thrall", ORC);
    let stone = descriptor(&events, me)
        .player_pack_slot(0)
        .expect("an item in the first backpack slot");
    assert_eq!(descriptor(&events, stone).object_entry(), Some(HEARTHSTONE));
}

#[test]
fn the_hearthstone_is_used_for_the_hearth_spell_on_a_five_minute_cooldown() {
    let mut realm = realm();
    realm.handle(RealmCommand::ItemQuery { entry: HEARTHSTONE });
    let info = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::ItemTemplate { entry, info } if entry == HEARTHSTONE => info,
            _ => None,
        })
        .expect("the Hearthstone's template");
    assert_eq!(info.name, "Hearthstone");
    let use_spell = info.use_spell.expect("an on-use spell");
    assert_eq!(use_spell.spell_id, HEARTH_SPELL);
    assert_eq!(use_spell.cooldown_ms, COOLDOWN_MS as i32);
    // Not the spell's own hour-long category cooldown.
    assert_eq!(use_spell.category_cooldown_ms, 0);
    // The ON_USE block the client's `CMSG_USE_ITEM` names.
    assert_eq!(info.use_spell_index(), Some(0));
}

#[test]
fn using_it_casts_for_ten_seconds_then_returns_the_player_to_the_crossroads() {
    let mut realm = realm();
    let (me, stone) = far_from_home(&mut realm);

    realm.handle(use_hearthstone());
    let events = drain(&mut realm);
    assert_eq!(spell_start(&events), Some((me, HEARTH_SPELL, CAST_MS)));

    let events = run(&mut realm, 9.8);
    assert_eq!(teleport(&events), None, "still casting");

    let events = run(&mut realm, 0.3);
    assert!(events.iter().any(|e| matches!(
        e,
        SessionEvent::CastResult { spell_id: HEARTH_SPELL, success: true, .. }
    )));
    assert!(events.iter().any(|e| matches!(
        e,
        SessionEvent::SpellGo { caster, spell_id: HEARTH_SPELL, item_caster: Some(item), .. }
            if *caster == me && *item == stone
    )));
    let (position, orientation) = teleport(&events).expect("a teleport");
    assert!(at_crossroads(position), "{position:?}");
    assert_eq!(orientation, CROSSROADS[3]);

    // The realm has the player there too: logged out, the roster says so.
    realm.handle(RealmCommand::Logout);
    let c = roster(&mut realm).into_iter().find(|c| c.guid == me).unwrap();
    assert!(at_crossroads([c.position.x, c.position.y, c.position.z]));
}

#[test]
fn a_move_sent_before_the_client_saw_the_teleport_does_not_undo_it() {
    let mut realm = realm();
    let (me, _) = far_from_home(&mut realm);
    realm.handle(use_hearthstone());
    let events = run(&mut realm, 10.1);
    let counter = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::Teleport { counter, .. } => Some(*counter),
            _ => None,
        })
        .expect("a teleport");

    // The client's movement from before the snap, then its ack.
    realm.handle(RealmCommand::Move {
        position: FAR_AWAY,
        orientation: 0.0,
    });
    realm.handle(RealmCommand::TeleportAck { counter });
    realm.handle(RealmCommand::Logout);
    let c = roster(&mut realm).into_iter().find(|c| c.guid == me).unwrap();
    assert!(at_crossroads([c.position.x, c.position.y, c.position.z]));
}

#[test]
fn moving_interrupts_the_cast() {
    let mut realm = realm();
    let (me, _) = far_from_home(&mut realm);
    realm.handle(use_hearthstone());
    run(&mut realm, 3.0);

    realm.handle(RealmCommand::Move {
        position: [FAR_AWAY[0] + 1.0, FAR_AWAY[1], FAR_AWAY[2]],
        orientation: 0.0,
    });
    let events = run(&mut realm, 0.1);
    assert!(events.iter().any(|e| matches!(
        e,
        SessionEvent::SpellFailedOther { caster, spell_id: HEARTH_SPELL } if *caster == me
    )));
    assert_eq!(
        cast_failure(&events),
        Some((HEARTH_SPELL, Some(SPELL_FAILED_INTERRUPTED)))
    );

    assert_eq!(teleport(&run(&mut realm, 12.0)), None);
}

#[test]
fn turning_on_the_spot_does_not_interrupt_the_cast() {
    let mut realm = realm();
    let (_, _) = far_from_home(&mut realm);
    realm.handle(use_hearthstone());
    run(&mut realm, 3.0);
    realm.handle(RealmCommand::Move {
        position: FAR_AWAY,
        orientation: 2.0,
    });
    assert!(teleport(&run(&mut realm, 7.1)).is_some());
}

#[test]
fn an_interrupted_cast_puts_nothing_on_cooldown() {
    let mut realm = realm();
    far_from_home(&mut realm);
    realm.handle(use_hearthstone());
    run(&mut realm, 1.0);
    realm.handle(RealmCommand::Move {
        position: [FAR_AWAY[0] + 1.0, FAR_AWAY[1], FAR_AWAY[2]],
        orientation: 0.0,
    });
    run(&mut realm, 0.1);

    realm.handle(use_hearthstone());
    assert!(spell_start(&drain(&mut realm)).is_some());
}

#[test]
fn taking_damage_interrupts_the_cast() {
    let mut realm = realm();
    let (me, events) = enter_new(&mut realm, "Thrall", ORC);
    let full = descriptor(&events, me).unit_max_health().unwrap();
    let mob = the_mob(&events);
    stand_by(&mut realm, &mob);
    // Pull it, then stop swinging and hearth with it on us.
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    realm.handle(RealmCommand::AttackStop);
    drain(&mut realm);
    realm.handle(use_hearthstone());

    let mut events = drain(&mut realm);
    for _ in 0..100 {
        realm.tick(0.1, &FlatGround);
        events.extend(drain(&mut realm));
    }
    let hit = events.iter().position(|e| {
        matches!(e, SessionEvent::ObjectValues { guid, fields }
            if *guid == me && fields.unit_health().is_some_and(|h| h < full))
    });
    let failed = events.iter().position(|e| {
        matches!(
            e,
            SessionEvent::CastResult {
                success: false,
                reason: Some(SPELL_FAILED_INTERRUPTED),
                ..
            }
        )
    });
    let (hit, failed) = (hit.expect("the mob landed a hit"), failed.expect("an interrupt"));
    assert!(hit < failed, "the hit came first");
    assert_eq!(teleport(&events), None);
}

#[test]
fn a_second_cast_while_casting_is_refused() {
    let mut realm = realm();
    far_from_home(&mut realm);
    realm.handle(use_hearthstone());
    drain(&mut realm);
    realm.handle(use_hearthstone());
    let events = drain(&mut realm);
    assert_eq!(spell_start(&events), None);
    assert_eq!(
        cast_failure(&events),
        Some((HEARTH_SPELL, Some(SPELL_FAILED_SPELL_IN_PROGRESS)))
    );
}

#[test]
fn after_hearthing_the_stone_rests_for_five_minutes() {
    let mut realm = realm();
    far_from_home(&mut realm);
    realm.handle(use_hearthstone());
    run(&mut realm, 10.1);

    realm.handle(use_hearthstone());
    let events = drain(&mut realm);
    assert_eq!(spell_start(&events), None);
    assert_eq!(
        cast_failure(&events),
        Some((HEARTH_SPELL, Some(SPELL_FAILED_NOT_READY)))
    );

    run(&mut realm, 289.0);
    realm.handle(use_hearthstone());
    assert_eq!(spell_start(&drain(&mut realm)), None, "not yet");

    run(&mut realm, 1.0);
    realm.handle(use_hearthstone());
    assert!(spell_start(&drain(&mut realm)).is_some(), "ready again");
}

#[test]
fn the_cooldown_is_kept_across_a_save_and_reload() {
    let mut realm = realm();
    let (me, _) = far_from_home(&mut realm);
    realm.handle(use_hearthstone());
    run(&mut realm, 10.0 + 0.1 + 60.0);
    realm.handle(RealmCommand::Logout);
    drain(&mut realm);
    let blob = realm.export_character(me).unwrap();

    let mut back = Realm::new();
    back.import_character(&blob).unwrap();
    back.handle(RealmCommand::EnterWorld { guid: me });
    let events = drain(&mut back);
    let cooldowns = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::SpellBook { cooldowns, .. } => Some(cooldowns.clone()),
            _ => None,
        })
        .expect("the spell book, with its cooldowns");
    let hearth = cooldowns
        .iter()
        .find(|c| u32::from(c.spell_id) == HEARTH_SPELL)
        .expect("the Hearthstone's cooldown");
    assert_eq!(u32::from(hearth.item_id), HEARTHSTONE);
    // Five minutes less the minute (and a tenth) spent waiting.
    assert!(
        (239_000..=240_000).contains(&hearth.spell_cd_ms),
        "{}",
        hearth.spell_cd_ms
    );

    back.handle(use_hearthstone());
    assert_eq!(
        cast_failure(&drain(&mut back)),
        Some((HEARTH_SPELL, Some(SPELL_FAILED_NOT_READY)))
    );
}

#[test]
fn a_rested_stone_sends_no_cooldown_at_login() {
    let mut realm = realm();
    let (_, events) = enter_new(&mut realm, "Thrall", ORC);
    let cooldowns = events.iter().find_map(|e| match e {
        SessionEvent::SpellBook { cooldowns, .. } => Some(cooldowns.clone()),
        _ => None,
    });
    assert_eq!(cooldowns, Some(Vec::new()));
}
