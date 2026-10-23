//! Sap, the Cutthroat's utility skill: from Stealth, on a mob that is not in combat, a 20 s
//! incapacitate that damage breaks. The Cutthroat stays stealthed, and the mob and its camp stay
//! idle. Played through the realm's seam by a level 20 Cutthroat at the Vile Familiars.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

static STEALTH: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "stealth"));
static SAP: std::sync::LazyLock<u32> = std::sync::LazyLock::new(|| spell("cutthroat", "sap"));
/// `SPELL_FAILED_ONLY_STEALTHED` and `SPELL_FAILED_TARGET_AFFECTING_COMBAT`.
const ONLY_STEALTHED: u8 = 0x57;
const TARGET_AFFECTING_COMBAT: u8 = 0x66;
/// The Scorpid Worker of the camp south of the Crossroads, which the boss test flags for the rest
/// of the process, so no other test uses it.
const SCORPID: u32 = 3124;
/// The Vile Familiar, and where its pack spawns.
const FAMILIAR: u32 = 3101;
const FAMILIAR_AT: [f32; 3] = [-731.0, -2518.0, 0.0];

/// A test Calling whose one skill has Sap's target gate and none of its other rules.
const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_sapper"
name = "Test Sapper"
tier = 0
class = 8
cap = 30
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "idle_only"
spell_id = 1774
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 100.0]
requires_target_out_of_combat = true
effects = [{ kind = "incapacitate", duration_ms = 8000 }]
"#;
const IDLE_ONLY: u32 = 1774;

/// A level 20 Cutthroat 40 yd from the creature of `entry` nearest `spawn`.
fn world(entry: u32, spawn: [f32; 3]) -> (Realm, u64, Mob, View) {
    world_of("cutthroat", entry, spawn)
}

/// A level 20 character of `calling`, 40 yd from the creature of `entry` nearest `spawn`.
fn world_of(calling: &str, entry: u32, spawn: [f32; 3]) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Garrote", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 20\n")
        .replace(
            "calling = \"basic_warrior\"",
            &format!("calling = \"{calling}\""),
        );
    let mut realm = Realm::with_seed(0x5EED_0339).with_callings(TEST_CALLINGS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    view.run(&mut realm, 1.0);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [spawn[0] + 40.0, spawn[1], 0.0],
        orientation: std::f32::consts::PI,
    });
    let mob = view
        .run(&mut realm, 1.0)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                fields,
                ..
            } if fields.object_entry() == Some(entry) && flat(*position, spawn) < 15.0 => {
                Some(Mob {
                    guid: *guid,
                    position: *position,
                })
            }
            _ => None,
        })
        .min_by(|a, b| flat(a.position, spawn).total_cmp(&flat(b.position, spawn)))
        .expect("the creature is in view");
    (realm, guid, mob, view)
}

/// Moves the player to `yards` east of the mob, facing it. No tick passes.
fn beside(realm: &mut Realm, mob: &Mob, yards: f32) {
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [mob.position[0] + yards, mob.position[1], mob.position[2]],
        orientation: std::f32::consts::PI,
    });
}

fn go_stealthed(realm: &mut Realm, view: &mut View, me: u64) {
    realm.handle(cast(*STEALTH, None));
    let events = view.drain(realm);
    assert_eq!(refusal(&events, *STEALTH), None, "stealth is cast");
    assert!(view.aura(me, *STEALTH).is_some(), "the aura is up");
}

/// The mobs that started a fight in `events`.
fn fighters(events: &[SessionEvent]) -> Vec<u64> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::AttackStart { attacker, .. } => Some(*attacker),
            _ => None,
        })
        .collect()
}

#[test]
fn sap_out_of_stealth_is_refused() {
    let (mut realm, _, mob, mut view) = world(FAMILIAR, FAMILIAR_AT);
    beside(&mut realm, &mob, 2.0);
    realm.handle(cast(*SAP, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, *SAP), Some(ONLY_STEALTHED));
    assert!(view.aura(mob.guid, *SAP).is_none());
}

#[test]
fn the_target_gate_refuses_a_mob_that_is_fighting() {
    // A stealthed player cannot hold a fight open long enough to cast, since the mob's first blow
    // breaks stealth, so the gate is tried on a test Calling's skill that has only the target rule.
    let (mut realm, _, mob, mut view) = world_of("test_sapper", FAMILIAR, FAMILIAR_AT);
    beside(&mut realm, &mob, 2.0);
    realm.handle(cast(IDLE_ONLY, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, IDLE_ONLY), None, "an idle mob takes it");
    assert!(view.aura(mob.guid, IDLE_ONLY).is_some(), "and is put down");

    let (mut realm, _, mob, mut view) = world_of("test_sapper", FAMILIAR, FAMILIAR_AT);
    let pulled = pull(&mut realm, &mut view, &mob);
    assert!(fighters(&pulled).contains(&mob.guid), "the mob is fighting");
    realm.handle(cast(IDLE_ONLY, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, IDLE_ONLY), Some(TARGET_AFFECTING_COMBAT));
    assert!(view.aura(mob.guid, IDLE_ONLY).is_none());
}

#[test]
fn sap_puts_an_idle_mob_down_for_20_s_and_leaves_the_fight_unstarted() {
    let (mut realm, me, mob, mut view) = world(FAMILIAR, FAMILIAR_AT);
    go_stealthed(&mut realm, &mut view, me);
    beside(&mut realm, &mob, 2.0);
    realm.handle(cast(*SAP, Some(mob.guid)));
    let mut events = view.drain(&mut realm);
    assert_eq!(refusal(&events, *SAP), None, "sap is cast");
    assert!(view.aura(mob.guid, *SAP).is_some(), "sapped");
    events.extend(view.run(&mut realm, 19.0));
    assert!(view.aura(mob.guid, *SAP).is_some(), "still sapped at 19 s");
    assert!(
        view.aura(me, *STEALTH).is_some(),
        "the Cutthroat is still stealthed"
    );
    assert_eq!(fighters(&events), Vec::<u64>::new(), "nobody engaged");
    let health = view.health(mob.guid);
    events.extend(view.run(&mut realm, 2.0));
    assert!(
        view.aura(mob.guid, *SAP).is_none(),
        "the aura ran out by 21 s"
    );
    assert_eq!(view.health(mob.guid), health);
}

#[test]
fn a_swing_at_a_sapped_mob_ends_the_aura_and_starts_the_fight() {
    let (mut realm, me, mob, mut view) = world(FAMILIAR, FAMILIAR_AT);
    go_stealthed(&mut realm, &mut view, me);
    beside(&mut realm, &mob, 2.0);
    realm.handle(cast(*SAP, Some(mob.guid)));
    view.drain(&mut realm);
    assert!(view.aura(mob.guid, *SAP).is_some(), "sapped");
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let events = view.run(&mut realm, 3.0);
    assert!(view.aura(mob.guid, *SAP).is_none(), "the swing woke it");
    assert!(fighters(&events).contains(&mob.guid), "the fight is on");
}

#[test]
fn sap_does_not_go_on_a_boss() {
    let _ = Realm::with_seed(0).with_bosses(&[SCORPID]);
    let (mut realm, me, mob, mut view) = world(SCORPID, CAMP);
    go_stealthed(&mut realm, &mut view, me);
    beside(&mut realm, &mob, 2.0);
    realm.handle(cast(*SAP, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert!(view.aura(mob.guid, *SAP).is_none(), "no aura on a boss");
    assert!(!fighters(&events).contains(&mob.guid), "and no fight");
}
