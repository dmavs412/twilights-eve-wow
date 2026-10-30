//! Snare Trap, the Ranger's utility skill: a ground area at the caster's feet that arms after
//! 2 s, lasts 60 s and catches the first enemy that is not a boss, with a 12 s incapacitate that
//! damage breaks. Played through the realm's seam with a test Calling whose trap has the shipped
//! trap's shape, so the shipped rows are only read, not cast.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::callings::{self, Effect};
use twilight_realm::{Realm, RealmCommand};

const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_trapper"
name = "Test Trapper"
tier = 0
class = 3
cap = 30
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "snare_trap"
spell_id = 1499
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "ground_area", radius_yd = 2.5, duration_ms = 60000, arm_ms = 2000, first_only = true, enemy_aura_spell_id = 3355, object_display = 3071 }]

[[calling.aura]]
key = "aura_snare_trap"
spell_id = 3355
duration_ms = 12000
effects = [{ kind = "incapacitate" }]
row = { break_on = { damage = true } }
"#;

const TRAP: u32 = 1499;
const TRAPPED: u32 = 3355;
/// `GameObjectDisplayInfo` of Freezing Trap's GameObject: the trap the player sees.
const TRAP_DISPLAY: u32 = 3071;
/// `GAMEOBJECT_FLAGS` and `GAMEOBJECT_TYPE_ID`; `GO_FLAG_NOT_SELECTABLE` and
/// `GAMEOBJECT_TYPE_GENERIC`.
const GAMEOBJECT_FLAGS: u16 = 9;
const GAMEOBJECT_TYPE_ID: u16 = 21;
const NOT_SELECTABLE: u32 = 0x10;
const GENERIC: u32 = 5;
/// The Vile Familiar, and where its pack spawns. Its entry is flagged a boss by one test, so the
/// other tests use the scorpid camp.
const FAMILIAR: u32 = 3101;
const FAMILIAR_AT: [f32; 3] = [-731.0, -2518.0, 0.0];

/// A level 20 test Trapper 40 yd from `spawn`, and the creatures within 15 yd of it, nearest
/// first.
fn world(spawn: [f32; 3]) -> (Realm, u64, Vec<Mob>, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Hunter", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 20\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_trapper\"");
    let mut realm = Realm::with_seed(0x5EED_0341).with_callings(TEST_CALLINGS);
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
    let mut mobs: Vec<Mob> = view
        .run(&mut realm, 1.0)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                ..
            } if flat(*position, spawn) < 15.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .collect();
    mobs.sort_by(|a, b| flat(a.position, spawn).total_cmp(&flat(b.position, spawn)));
    assert!(mobs.len() >= 2, "the pack is in view");
    (realm, guid, mobs, view)
}

/// Moves the player to `yards` east of the mob, facing it. No tick passes.
fn beside(realm: &mut Realm, mob: &Mob, yards: f32) {
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [mob.position[0] + yards, mob.position[1], mob.position[2]],
        orientation: std::f32::consts::PI,
    });
}

/// Lays the trap at the player's feet, asserting it went. Returns what the client saw.
fn lay(realm: &mut Realm, view: &mut View) -> Vec<SessionEvent> {
    realm.handle(cast(TRAP, None));
    let events = view.drain(realm);
    assert_eq!(refusal(&events, TRAP), None, "the trap is laid");
    events
}

/// The DynamicObjects `events` create: guid and centre.
fn patch_objects(events: &[SessionEvent]) -> Vec<(u64, [f32; 3])> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::DynamicObject,
                position,
                ..
            } => Some((*guid, *position)),
            _ => None,
        })
        .collect()
}

/// The GameObjects `events` create with the trap's display: guid and position. Each must be
/// scenery no one can click.
fn trap_shows(events: &[SessionEvent]) -> Vec<(u64, [f32; 3])> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::GameObject,
                display_id: Some(TRAP_DISPLAY),
                position,
                fields,
                ..
            } => {
                let raw: Vec<(u16, u32)> = fields.raw_fields().collect();
                assert!(raw.contains(&(GAMEOBJECT_TYPE_ID, GENERIC)), "a GENERIC");
                assert!(
                    raw.contains(&(GAMEOBJECT_FLAGS, NOT_SELECTABLE)),
                    "that no one can click"
                );
                Some((*guid, *position))
            }
            _ => None,
        })
        .collect()
}

fn destroyed(events: &[SessionEvent], object: u64) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::ObjectDestroyed(g) if *g == object))
}

/// Ticks in 0.1 s steps until `mob` holds the trap's aura, for at most `secs`; the events seen.
fn until_trapped(realm: &mut Realm, view: &mut View, mob: &Mob, secs: f32) -> Vec<SessionEvent> {
    let mut events = Vec::new();
    for _ in 0..(secs / 0.1).round() as u32 {
        events.extend(view.run(realm, 0.1));
        if view.aura(mob.guid, TRAPPED).is_some() {
            break;
        }
    }
    events
}

#[test]
fn laying_the_trap_shows_its_object_at_the_casters_feet() {
    let (mut realm, _, mobs, mut view) = world(CAMP);
    let feet = [
        mobs[0].position[0] + 8.0,
        mobs[0].position[1],
        mobs[0].position[2],
    ];
    beside(&mut realm, &mobs[0], 8.0);
    let laid = lay(&mut realm, &mut view);
    let shows = trap_shows(&laid);
    let [(_, at)] = shows[..] else {
        panic!("one trap GameObject, has {shows:?}");
    };
    assert!(flat(at, feet) < 0.01, "at the caster's feet, is at {at:?}");
}

#[test]
fn a_mob_inside_catches_nothing_while_the_trap_arms_and_the_aura_once_it_has() {
    let (mut realm, _, mobs, mut view) = world(CAMP);
    let mob = &mobs[0];
    beside(&mut realm, mob, 1.0);
    let laid = lay(&mut realm, &mut view);
    let [(object, _)] = patch_objects(&laid)[..] else {
        panic!("one trap object");
    };
    let [(shown, _)] = trap_shows(&laid)[..] else {
        panic!("one trap GameObject");
    };
    let early = view.run(&mut realm, 1.5);
    assert!(
        view.aura(mob.guid, TRAPPED).is_none(),
        "not caught at 1.5 s"
    );
    assert!(!destroyed(&early, object), "the trap is still down");
    assert!(!destroyed(&early, shown), "and still shows");
    let late = until_trapped(&mut realm, &mut view, mob, 1.5);
    assert!(view.aura(mob.guid, TRAPPED).is_some(), "caught once armed");
    assert!(destroyed(&late, object), "and the trap is spent");
    assert!(destroyed(&late, shown), "and no longer shows");
    let aura_for = |secs: f32, realm: &mut Realm, view: &mut View| {
        view.run(realm, secs);
        view.aura(mob.guid, TRAPPED).is_some()
    };
    assert!(
        aura_for(10.5, &mut realm, &mut view),
        "still held at 10.5 s"
    );
    assert!(!aura_for(2.0, &mut realm, &mut view), "free by 12.5 s");
}

#[test]
fn a_second_mob_walking_in_afterwards_gets_nothing() {
    let (mut realm, _, mobs, mut view) = world(CAMP);
    beside(&mut realm, &mobs[0], 1.0);
    lay(&mut realm, &mut view);
    until_trapped(&mut realm, &mut view, &mobs[0], 4.0);
    assert!(
        view.aura(mobs[0].guid, TRAPPED).is_some(),
        "the first is caught"
    );
    let other = mobs
        .iter()
        .skip(1)
        .find(|m| flat(m.position, mobs[0].position) > 6.0)
        .expect("a camp mob well away from the first");
    beside(&mut realm, other, 1.0);
    view.run(&mut realm, 4.0);
    assert!(
        view.aura(other.guid, TRAPPED).is_none(),
        "the trap is spent"
    );
}

#[test]
fn the_caster_standing_in_it_is_never_caught() {
    let (mut realm, me, _, mut view) = world(CAMP);
    let laid = lay(&mut realm, &mut view);
    let [(object, _)] = patch_objects(&laid)[..] else {
        panic!("one trap object");
    };
    let events = view.run(&mut realm, 5.0);
    assert!(view.aura(me, TRAPPED).is_none(), "no aura on the caster");
    assert!(!destroyed(&events, object), "and the trap is still armed");
}

#[test]
fn a_boss_walks_through_and_the_trap_stays_armed() {
    let _ = Realm::with_seed(0).with_bosses(&[FAMILIAR]);
    let (mut realm, _, mobs, mut view) = world(FAMILIAR_AT);
    let boss = &mobs[0];
    beside(&mut realm, boss, 1.0);
    let laid = lay(&mut realm, &mut view);
    let [(object, _)] = patch_objects(&laid)[..] else {
        panic!("one trap object");
    };
    let events = view.run(&mut realm, 6.0);
    assert!(
        view.aura(boss.guid, TRAPPED).is_none(),
        "a boss is not caught"
    );
    assert!(!destroyed(&events, object), "the trap stays armed");
}

#[test]
fn laying_a_second_trap_removes_the_first() {
    let (mut realm, _, _, mut view) = world(CAMP);
    let laid = lay(&mut realm, &mut view);
    let first = patch_objects(&laid);
    let [(old, _)] = first[..] else {
        panic!("one trap object, has {first:?}");
    };
    let [(old_shown, _)] = trap_shows(&laid)[..] else {
        panic!("one trap GameObject");
    };
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [CAMP[0] + 60.0, CAMP[1], 0.0],
        orientation: std::f32::consts::PI,
    });
    let events = lay(&mut realm, &mut view);
    assert!(destroyed(&events, old), "the first trap goes");
    assert!(destroyed(&events, old_shown), "and its GameObject");
    let second = patch_objects(&events);
    assert_eq!(second.len(), 1, "and one new trap comes");
    assert_ne!(second[0].0, old);
    let shown = trap_shows(&events);
    assert_eq!(shown.len(), 1, "showing one new GameObject");
    assert_ne!(shown[0].0, old_shown);
}

#[test]
fn a_trap_that_runs_out_takes_its_object_with_it() {
    let (mut realm, _, _, mut view) = world(CAMP);
    // Well clear of the camp, so nothing walks into it.
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [CAMP[0] + 60.0, CAMP[1], 0.0],
        orientation: std::f32::consts::PI,
    });
    let laid = lay(&mut realm, &mut view);
    let [(shown, _)] = trap_shows(&laid)[..] else {
        panic!("one trap GameObject");
    };
    let events = view.run(&mut realm, 61.0);
    assert!(destroyed(&events, shown), "gone after 60 s");
}

#[test]
fn a_swing_at_the_trapped_mob_ends_the_aura() {
    let (mut realm, _, mobs, mut view) = world(CAMP);
    let mob = &mobs[0];
    beside(&mut realm, mob, 1.0);
    lay(&mut realm, &mut view);
    until_trapped(&mut realm, &mut view, mob, 4.0);
    assert!(view.aura(mob.guid, TRAPPED).is_some(), "trapped");
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    view.run(&mut realm, 3.0);
    assert!(view.aura(mob.guid, TRAPPED).is_none(), "the swing woke it");
}

#[test]
fn the_shipped_snare_trap_is_an_armed_first_entry_area_with_an_incapacitate_aura() {
    let ranger = callings::calling("ranger").unwrap();
    let trap = ranger
        .skills
        .iter()
        .find(|s| s.key == "snare_trap")
        .unwrap();
    let aura = spell("ranger", "aura_snare_trap");
    assert!(matches!(
        trap.effects[..],
        [Effect::GroundArea {
            duration_ms: 60000,
            arm_ms: 2000,
            first_only: true,
            enemy_aura_spell_id: Some(e),
            ally_aura_spell_id: None,
            object_display: 3071,
            ..
        }] if e == aura
    ));
    let row = callings::aura("aura_snare_trap").expect("the aura loads");
    assert_eq!(row.duration_ms, 12000);
    assert!(matches!(
        row.effects[..],
        [callings::AuraEffect::Incapacitate]
    ));
    assert!(row.row.break_on.contains_key("damage"));
}
