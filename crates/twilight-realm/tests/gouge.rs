//! Gouge, the first incapacitate: a stun that any damage after the cast ends. The target stays in
//! the fight (#165). Played through the realm's seam by a level 1 Basic Warrior at a level 3 mob.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

const CHARGE: u32 = 100;
const GOUGE: u32 = 1776;
/// `UNIT_FLAG_IN_COMBAT`.
const UNIT_FLAG_IN_COMBAT: u32 = 0x0008_0000;

/// A warrior with Gouge's 15 rage in hand, 2 yd from the mob it has just stopped hitting.
fn ready() -> (Realm, u64, Mob, View) {
    let (mut realm, me, mob, mut view) = warrior_at(1);
    rage_up(&mut realm, &mut view, me, &mob, CHARGE, 15);
    (realm, me, mob, view)
}

/// Gouges the mob; the events of the cast.
fn gouge(realm: &mut Realm, view: &mut View, mob: &Mob) -> Vec<SessionEvent> {
    realm.handle(cast(GOUGE, Some(mob.guid)));
    let events = view.drain(realm);
    assert_eq!(refusal(&events, GOUGE), None, "gouge is cast");
    events
}

fn started_on_player(events: &[SessionEvent], mob: &Mob) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::AttackStart { attacker, .. } if *attacker == mob.guid))
}

#[test]
fn gouges_own_damage_does_not_break_it_but_later_damage_does() {
    let (mut realm, _, mob, mut view) = ready();
    let events = gouge(&mut realm, &mut view, &mob);
    assert!(
        events.iter().any(
            |e| matches!(e, SessionEvent::SpellDamageLog(l) if l.spell_id == GOUGE && l.damage > 0)
        ),
        "gouge hurts"
    );
    assert!(view.aura(mob.guid, GOUGE).is_some(), "incapacitated");
    view.run(&mut realm, 2.0);
    assert!(view.aura(mob.guid, GOUGE).is_some(), "still, 2 s on");

    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    view.run(&mut realm, 3.0);
    assert!(view.aura(mob.guid, GOUGE).is_none(), "a swing woke it");
}

#[test]
fn the_target_stays_in_combat_while_incapacitated() {
    let (mut realm, me, mob, mut view) = ready();
    let events = gouge(&mut realm, &mut view, &mob);
    assert!(
        !stopped(&events, &mob),
        "the cast drops no one from the fight"
    );
    let during = view.run(&mut realm, 1.0);
    assert!(view.aura(mob.guid, GOUGE).is_some());
    assert!(!stopped(&during, &mob));
    assert_ne!(view.fields(mob.guid).unit_flags() & UNIT_FLAG_IN_COMBAT, 0);
}

#[test]
fn the_caster_is_out_of_combat_while_the_target_is_incapacitated() {
    let (mut realm, me, mob, mut view) = ready();
    gouge(&mut realm, &mut view, &mob);
    view.run(&mut realm, 1.0);
    assert!(view.aura(mob.guid, GOUGE).is_some());
    assert_eq!(view.fields(me).unit_flags() & UNIT_FLAG_IN_COMBAT, 0);
    // A swing wakes it and the fight is on again.
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    view.run(&mut realm, 0.5);
    assert_ne!(view.fields(me).unit_flags() & UNIT_FLAG_IN_COMBAT, 0);
}

#[test]
fn waking_with_the_caster_away_the_target_chases_instead_of_going_home() {
    let (mut realm, me, mob, mut view) = ready();
    gouge(&mut realm, &mut view, &mob);
    stand_at(&mut realm, &mob, 30.0);
    let during = view.run(&mut realm, 3.0);
    assert!(swung(&during, &mob).is_empty(), "held while it sleeps");
    let after = view.run(&mut realm, 10.0);
    assert!(!stopped(&after, &mob), "it never gives up the fight");
    assert!(swung(&after, &mob).contains(&me), "it runs the caster down");
}

#[test]
fn waking_with_the_caster_close_the_target_fights_again() {
    let (mut realm, me, mob, mut view) = ready();
    gouge(&mut realm, &mut view, &mob);
    let during = view.run(&mut realm, 3.0);
    assert!(swung(&during, &mob).is_empty(), "held while it sleeps");
    assert!(
        !started_on_player(&during, &mob),
        "still the caster's fight"
    );
    let after = view.run(&mut realm, 3.0);
    assert!(swung(&after, &mob).contains(&me), "it swings at the caster");
}

/// Whether `mob` stopped fighting in `events`: an evade starts with its `AttackStop`.
fn stopped(events: &[SessionEvent], mob: &Mob) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::AttackStop { attacker, .. } if *attacker == mob.guid))
}

/// The victims of `mob`'s melee swings in `events`.
fn swung(events: &[SessionEvent], mob: &Mob) -> Vec<u64> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::AttackerState(s) if s.attacker == mob.guid => Some(s.victim),
            _ => None,
        })
        .collect()
}

/// A human warrior of `level` in the world at the Crossroads, the mob near it, and what the
/// client has seen so far.
fn warrior_at(level: u8) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"));
    let mut realm = Realm::with_seed(0x5EED_0008);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let mob = view
        .drain(&mut realm)
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: benilla_protocol::EntityKind::Unit,
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

/// Charges `mob` for its rage, then swings at it until there is `rage`, and stops.
fn rage_up(realm: &mut Realm, view: &mut View, me: u64, mob: &Mob, charge: u32, rage: u32) {
    stand_at(realm, mob, 15.0);
    realm.handle(cast(charge, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(realm), charge), None, "the charge");
    stand_at(realm, mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    for _ in 0..600 {
        if view.rage(me) >= rage {
            realm.handle(RealmCommand::AttackStop);
            view.drain(realm);
            return;
        }
        view.run(realm, 0.1);
        assert!(view.health(me) > 0, "died waiting for {rage} rage");
        assert!(view.health(mob.guid) > 0, "killed it before {rage} rage");
    }
    panic!("a minute of swings built no {rage} rage");
}
