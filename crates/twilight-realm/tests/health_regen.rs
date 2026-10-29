//! Health regeneration: the Spirit-based tick out of combat. Played through the realm's seam by
//! human warriors.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::*;
use common::*;
use twilight_realm::rules::regen;
use twilight_realm::{Realm, RealmCommand};

#[test]
fn the_tick_is_spirit_based_and_a_regen_percent_raises_it() {
    assert_eq!(regen::tick_heal(0, 0), 1, "never less than a point");
    assert!(regen::tick_heal(100, 0) > regen::tick_heal(50, 0));
    let plain = regen::tick_heal(200, 0);
    assert_eq!(regen::tick_heal(200, 100), 2 * plain, "+100% doubles it");
}

#[test]
fn health_regenerates_out_of_combat_and_not_in_it() {
    // Level 6: creatures hit half again as hard now, and a level 1 warrior dies to this one.
    let (mut realm, me, mob, mut view) = warrior_at(6);
    hurt(&mut realm, &mut view, &mob);
    // In combat, nothing healed: health only fell.
    let fight = fight_until_dead(&mut realm, &mut view, me, &mob);
    assert!(
        fight.windows(2).all(|w| w[1] <= w[0]),
        "no health came back in the fight: {fight:?}"
    );
    view.run(&mut realm, 0.5);
    let max = view.fields(me).unit_max_health().unwrap();
    let after = view.health(me);
    assert!(after < max, "the fight left the warrior hurt");
    view.run(&mut realm, 10.0);
    assert!(view.health(me) > after, "10 s out of combat healed");
}

/// Stands by the mob and lets it hit the warrior for a few seconds.
fn hurt(realm: &mut Realm, view: &mut View, mob: &Mob) {
    stand_at(realm, mob, 2.0);
    view.run(realm, 25.0);
}

/// Fights the mob to its death; the warrior's health at each step.
fn fight_until_dead(realm: &mut Realm, view: &mut View, me: u64, mob: &Mob) -> Vec<u32> {
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let mut seen = vec![view.health(me)];
    for _ in 0..1200 {
        view.run(realm, 0.1);
        seen.push(view.health(me));
        assert!(view.health(me) > 0, "died");
        if view.health(mob.guid) == 0 {
            return seen;
        }
    }
    panic!("the mob outlived two minutes");
}

/// A human warrior of `level` in the world at the Crossroads, the mob near it, and what the
/// client has seen.
fn warrior_at(level: u8) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"));
    let mut realm = Realm::with_seed(0x5EED_0056);
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
