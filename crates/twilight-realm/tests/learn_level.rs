//! `learn_level` (#335): a Calling's skill is known from its level. The spell book, the cast check,
//! the level-up and the login all follow it. Played on a test Calling (`Realm::with_callings`)
//! with one skill from level 1 and one from level 3.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::*;
use common::*;
use twilight_realm::{DevMultipliers, Realm, RealmCommand};

const SEED: u64 = 0x5EED_0335;
/// `SPELL_FAILED_NOT_KNOWN`.
const NOT_KNOWN: u8 = 0x38;

const FIRST: u32 = 100;
const SECOND: u32 = 101;

const THE_CROSSROADS: u32 = 380;
const CAMP_TAURAJO: u32 = 378;
const AT_CROSSROADS: [f32; 3] = [-441.8, -2648.6, 0.0];
const AT_TAURAJO: [f32; 3] = [-2350.0, -1900.0, 0.0];

const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_learner"
name = "Test Learner"
tier = 0
class = 1
cap = 30
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth", "leather", "mail", "plate", "shield"]

[[calling.skill]]
key = "first"
spell_id = 100
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = []

[[calling.skill]]
key = "second"
spell_id = 101
learn_level = 3
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = []
"#;

/// A save of the test Calling at `level`.
fn save(level: u8) -> (u64, String) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Learner", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"test_learner\"");
    (guid, blob)
}

/// The test Calling at `level`, entered with XP at 10 times: a discovery pays more than the XP to
/// the next level, and less than two. The entry burst comes with it.
fn entered(level: u8) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save(level);
    let dev = DevMultipliers {
        xp: 10.0,
        ..DevMultipliers::default()
    };
    let mut realm = Realm::with_seed(SEED)
        .with_dev(dev)
        .with_callings(TEST_CALLINGS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

/// Moves the player to `to` and ticks once on a map with two areas: the events.
fn step(realm: &mut Realm, to: [f32; 3]) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: to,
        orientation: 0.0,
    });
    realm.tick(
        0.0,
        &Areas(vec![
            ([AT_CROSSROADS[0], AT_CROSSROADS[1]], 150.0, THE_CROSSROADS),
            ([AT_TAURAJO[0], AT_TAURAJO[1]], 150.0, CAMP_TAURAJO),
        ]),
    );
    drain(realm)
}

fn spellbook(events: &[SessionEvent]) -> Vec<u32> {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::SpellBook { spell_ids, .. } => Some(spell_ids.clone()),
            _ => None,
        })
        .expect("the entry sends the spell book")
}

fn learned(events: &[SessionEvent], spell_id: u32) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::SpellLearned { spell_id: s } if *s == spell_id))
}

/// The refusal of a self cast of `spell_id`, if any.
fn cast_refusal(realm: &mut Realm, spell_id: u32) -> Option<u8> {
    realm.handle(cast(spell_id, None));
    realm.tick(0.0, &FlatGround);
    refusal(&drain(realm), spell_id)
}

#[test]
fn at_level_1_only_the_first_skill_is_in_the_spell_book_and_the_second_is_refused() {
    let (mut realm, _, entry) = entered(1);
    let book = spellbook(&entry);
    assert!(book.contains(&FIRST), "{book:?}");
    assert!(!book.contains(&SECOND), "{book:?}");
    assert_eq!(cast_refusal(&mut realm, SECOND), Some(NOT_KNOWN));
    assert_eq!(cast_refusal(&mut realm, FIRST), None);
}

#[test]
fn reaching_level_3_teaches_the_second_skill_and_not_level_2() {
    let (mut realm, _, _) = entered(1);
    let to_level_2 = step(&mut realm, AT_CROSSROADS);
    assert!(
        to_level_2
            .iter()
            .any(|e| matches!(e, SessionEvent::LevelUp(l) if l.level == 2)),
        "no level 2"
    );
    assert!(!learned(&to_level_2, SECOND), "not yet at level 2");
    assert_eq!(cast_refusal(&mut realm, SECOND), Some(NOT_KNOWN));

    let to_level_3 = step(&mut realm, AT_TAURAJO);
    assert!(
        to_level_3
            .iter()
            .any(|e| matches!(e, SessionEvent::LevelUp(l) if l.level == 3)),
        "no level 3"
    );
    assert!(learned(&to_level_3, SECOND), "learned at level 3");
    assert!(!learned(&to_level_3, FIRST));
    assert_eq!(cast_refusal(&mut realm, SECOND), None);
}

#[test]
fn a_character_saved_at_level_3_has_both_skills_at_login() {
    let (_, _, entry) = entered(3);
    let book = spellbook(&entry);
    assert!(book.contains(&FIRST) && book.contains(&SECOND), "{book:?}");
}
