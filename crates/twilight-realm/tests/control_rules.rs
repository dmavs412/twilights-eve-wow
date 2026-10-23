//! Control rules on mobs (ADR 0020): diminishing returns on stuns and incapacitates, and a
//! boss's immunity to incapacitates. Played through the realm's seam with a test Calling whose
//! stun, incapacitate and short stun are free.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_controller"
name = "Test Controller"
tier = 0
class = 8
cap = 30
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "stun"
spell_id = 1771
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 100.0]
effects = [{ kind = "stun", aura_spell_id = 1000771, duration_ms = 8000 }]

[[calling.skill]]
key = "short_stun"
spell_id = 1772
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 100.0]
effects = [{ kind = "stun", aura_spell_id = 1000772, duration_ms = 1000 }]

[[calling.skill]]
key = "incapacitate"
spell_id = 1773
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 100.0]
effects = [{ kind = "incapacitate", duration_ms = 8000 }]
"#;

const STUN: u32 = 1000771;
const SHORT_STUN: u32 = 1000772;
const STUN_CAST: u32 = 1771;
const SHORT_STUN_CAST: u32 = 1772;
const INCAPACITATE: u32 = 1773;
/// The Vile Familiar, and where its pack spawns.
const FAMILIAR: u32 = 3101;
const FAMILIAR_AT: [f32; 3] = [-731.0, -2518.0, 0.0];
/// The Scorpid Worker of the camp south of the Crossroads, which the boss tests flag.
const SCORPID: u32 = 3124;
const SHADOW_BOLT: &str = "shadow_bolt_creature";

/// A level 20 test Controller `yards` from the nearest creature of `entry` around `spawn`, with
/// `spells` cast by every creature of that entry.
fn world(
    seed: u64,
    entry: u32,
    spawn: [f32; 3],
    spells: &[&str],
    yards: f32,
) -> (Realm, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Binder", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 20\n")
        .replace(
            "calling = \"basic_warrior\"",
            "calling = \"test_controller\"",
        );
    let mut realm = Realm::with_seed(seed)
        .with_callings(TEST_CALLINGS)
        .with_creature_spells(entry, spells);
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
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [mob.position[0] + yards, mob.position[1], mob.position[2]],
        orientation: std::f32::consts::PI,
    });
    (realm, mob, view)
}

/// Casts `spell_id` at `mob`, asserting it went; the events that came with it.
fn control(realm: &mut Realm, view: &mut View, spell_id: u32, mob: &Mob) -> Vec<SessionEvent> {
    realm.handle(cast(spell_id, Some(mob.guid)));
    let events = view.drain(realm);
    assert_eq!(refusal(&events, spell_id), None, "{spell_id} is cast");
    events
}

/// Casts `spell_id`, and checks that its `aura` lasts between `secs - 0.5` and `secs + 0.5`
/// seconds, which leaves the mob free again.
fn lasts(realm: &mut Realm, view: &mut View, spell_id: u32, aura: u32, mob: &Mob, secs: f32) {
    control(realm, view, spell_id, mob);
    assert!(view.aura(mob.guid, aura).is_some(), "{aura} lands");
    view.run(realm, secs - 0.5);
    assert!(view.aura(mob.guid, aura).is_some(), "{aura} lasts {secs} s");
    view.run(realm, 1.0);
    assert!(
        view.aura(mob.guid, aura).is_none(),
        "{aura} ends by {secs} s"
    );
}

/// Ticks until `mob` starts casting `spell_id`.
fn until_cast(realm: &mut Realm, view: &mut View, mob: u64, spell_id: u32) {
    for _ in 0..600 {
        realm.tick(0.1, &FlatGround);
        let started = view.drain(realm).iter().any(|e| {
            matches!(e, SessionEvent::SpellStart { caster, spell_id: s, .. }
                if *caster == mob && *s == spell_id)
        });
        if started {
            return;
        }
    }
    panic!("the creature never began spell {spell_id}");
}

fn cast_stopped(events: &[SessionEvent], mob: &Mob) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::SpellFailedOther { caster, .. } if *caster == mob.guid))
}

/// The Scorpid Worker flagged as a boss, for the rest of the process.
fn flag_boss() {
    let _ = Realm::with_seed(0).with_bosses(&[SCORPID]);
}

#[test]
fn four_incapacitates_last_eight_four_two_and_nothing() {
    let (mut realm, mob, mut view) = world(1, FAMILIAR, FAMILIAR_AT, &[], 20.0);
    lasts(&mut realm, &mut view, INCAPACITATE, INCAPACITATE, &mob, 8.0);
    lasts(&mut realm, &mut view, INCAPACITATE, INCAPACITATE, &mob, 4.0);
    lasts(&mut realm, &mut view, INCAPACITATE, INCAPACITATE, &mob, 2.0);
    control(&mut realm, &mut view, INCAPACITATE, &mob);
    assert!(
        view.aura(mob.guid, INCAPACITATE).is_none(),
        "the fourth puts no aura on"
    );
}

#[test]
fn fifteen_seconds_after_the_last_an_incapacitate_lasts_in_full() {
    let (mut realm, mob, mut view) = world(2, FAMILIAR, FAMILIAR_AT, &[], 20.0);
    for _ in 0..3 {
        control(&mut realm, &mut view, INCAPACITATE, &mob);
    }
    view.run(&mut realm, 15.5);
    lasts(&mut realm, &mut view, INCAPACITATE, INCAPACITATE, &mob, 8.0);
}

#[test]
fn the_stun_clock_is_separate_from_the_incapacitate_clock() {
    let (mut realm, mob, mut view) = world(3, FAMILIAR, FAMILIAR_AT, &[], 20.0);
    control(&mut realm, &mut view, INCAPACITATE, &mob);
    control(&mut realm, &mut view, INCAPACITATE, &mob);
    lasts(&mut realm, &mut view, STUN_CAST, STUN, &mob, 8.0);
}

#[test]
fn a_fourth_stun_on_a_casting_mob_leaves_the_cast_going() {
    let shadow = creature_spell(SHADOW_BOLT);
    let (mut realm, mob, mut view) = world(4, FAMILIAR, FAMILIAR_AT, &[SHADOW_BOLT], 20.0);
    for _ in 0..3 {
        control(&mut realm, &mut view, SHORT_STUN_CAST, &mob);
        view.run(&mut realm, 1.2);
    }
    until_cast(&mut realm, &mut view, mob.guid, shadow);
    let events = control(&mut realm, &mut view, SHORT_STUN_CAST, &mob);
    assert!(
        view.aura(mob.guid, SHORT_STUN).is_none(),
        "the fourth stun lands nothing"
    );
    let mut later = view.run(&mut realm, 3.0);
    later.extend(events);
    assert!(!cast_stopped(&later, &mob), "the cast goes on");
}

#[test]
fn an_incapacitate_on_a_boss_lands_nothing_and_its_cast_goes_on() {
    flag_boss();
    let shadow = creature_spell(SHADOW_BOLT);
    let (mut realm, mob, mut view) = world(5, SCORPID, CAMP, &[SHADOW_BOLT], 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    until_cast(&mut realm, &mut view, mob.guid, shadow);
    let mut events = control(&mut realm, &mut view, INCAPACITATE, &mob);
    assert!(view.aura(mob.guid, INCAPACITATE).is_none(), "no aura");
    events.extend(view.run(&mut realm, 3.0));
    assert!(!cast_stopped(&events, &mob), "the cast goes on");
    assert!(
        events.iter().any(|e| matches!(e,
            SessionEvent::SpellGo { caster, spell_id, .. }
            if *caster == mob.guid && *spell_id == shadow)),
        "the bolt goes off"
    );
}

#[test]
fn a_stun_lands_on_a_boss() {
    flag_boss();
    let (mut realm, mob, mut view) = world(6, SCORPID, CAMP, &[], 20.0);
    lasts(&mut realm, &mut view, STUN_CAST, STUN, &mob, 8.0);
}
