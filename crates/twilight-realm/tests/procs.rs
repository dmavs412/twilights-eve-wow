//! `proc`: a talent's "when X happens, do Y". Vanguard's Resolve heals on an experience-yielding
//! killing blow, played through the realm's seam; a cast proc's skill filter is the pure
//! trigger rule.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::*;
use common::*;
use twilight_realm::callings::ProcEvent;
use twilight_realm::rules::procs::{Happened, Trigger};
use twilight_realm::{Realm, RealmCommand};

/// The Recruit tree's Vanguard's Resolve: its borrowed `Talent.dbc` id.
const VANGUARDS_RESOLVE: u32 = 382;

/// How much health the killing blow's step gave back, and the maximum.
fn health_at_the_kill(level: u8, talents: &[(u32, u8)]) -> (u32, u32) {
    let (mut realm, me, mob, mut view) = warrior_at(level, talents);
    hurt(&mut realm, &mut view, &mob);
    let fight = fight_until_dead(&mut realm, &mut view, me, &mob);
    let max = view.fields(me).unit_max_health().unwrap();
    let n = fight.len();
    (fight[n - 1].saturating_sub(fight[n - 2]), max)
}

#[test]
fn an_experience_yielding_kill_heals_a_tenth_of_max_health() {
    let (gain, max) = health_at_the_kill(6, &[(VANGUARDS_RESOLVE, 1)]);
    assert!(gain >= max / 10, "healed {gain} of {max}");
    let (plain, _) = health_at_the_kill(6, &[]);
    assert!(plain < max / 10, "without the talent {plain} came back");
}

#[test]
fn a_kill_that_yields_no_experience_heals_nothing() {
    // A level 10 warrior finds the level 3 mob gray.
    let (gain, max) = health_at_the_kill(10, &[(VANGUARDS_RESOLVE, 1)]);
    assert!(gain < max / 10, "healed {gain} of {max}");
}

#[test]
fn a_cast_proc_filtered_to_one_skill_fires_only_for_that_skill() {
    let trigger = Trigger {
        on: ProcEvent::Cast,
        skill: Some("charge"),
        yields_xp: false,
    };
    let cast = |skill| Happened {
        event: ProcEvent::Cast,
        skill,
        yields_xp: false,
        target: None,
    };
    assert!(trigger.fires(&cast(Some("charge"))));
    assert!(!trigger.fires(&cast(Some("heroic_strike"))));
    assert!(!trigger.fires(&Happened {
        event: ProcEvent::Kill,
        skill: None,
        yields_xp: true,
        target: None,
    }));
    let unfiltered = Trigger {
        skill: None,
        ..trigger
    };
    assert!(unfiltered.fires(&cast(Some("heroic_strike"))));
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

/// A human warrior of `level` in the world at the Crossroads with `talents` (`(id, rank)`) in
/// its save, the mob near it, and what the client has seen.
fn warrior_at(level: u8, talents: &[(u32, u8)]) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"));
    if !talents.is_empty() {
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        blob = blob.replace(
            "money = 0\n",
            &format!("money = 0\ntalents = [{}]\n", rows.join(", ")),
        );
    }
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
