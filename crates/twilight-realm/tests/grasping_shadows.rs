//! Grasping Shadows, the Acolyte's utility skill: a ground area 8 yd across that slows every
//! enemy inside by 60%, and a boss by 30%. Played through the realm's seam with a test Calling
//! whose skills have the shipped rows' shape, so the shipped rows are only read, not cast.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::callings::{self, AuraEffect, Effect};
use twilight_realm::{Realm, RealmCommand};

const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_acolyte"
name = "Test Acolyte"
tier = 0
class = 9
cap = 30
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "grasping_shadows"
spell_id = 3354
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "dest"
range_yd = [0.0, 30.0]
effects = [{ kind = "ground_area", radius_yd = 8.0, duration_ms = 10000, enemy_aura_spell_id = 3355 }]

[[calling.skill]]
key = "miasma"
spell_id = 2120
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "dest"
range_yd = [0.0, 30.0]
effects = [{ kind = "ground_area", radius_yd = 8.0, duration_ms = 10000, enemy_aura_spell_id = 2121 }]

[[calling.skill]]
key = "poke"
spell_id = 686
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 40.0]
effects = [{ kind = "school_damage", school = "shadow", amount = { base = 1.0 } }]

[[calling.aura]]
key = "aura_grasping_shadows"
spell_id = 3355
duration_ms = 10000
effects = [{ kind = "mod_run_speed", pct = -60 }]

[[calling.aura]]
key = "aura_miasma"
spell_id = 2121
duration_ms = 10000
effects = [{ kind = "periodic_damage", tick_ms = 1000, tick = { base = 1.0 }, school = "shadow" }]
"#;

const SHADOWS: u32 = 3354;
const SHADOWS_AURA: u32 = 3355;
const MIASMA: u32 = 2120;
const MIASMA_AURA: u32 = 2121;
const POKE: u32 = 686;
/// The Scorpid Worker of the camp, which the speed test flags as a boss.
const SCORPID: u32 = 3124;

/// A level 20 test Acolyte 40 yd east of the camp, and the camp's nearest creature.
fn world() -> (Realm, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Vellune", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 20\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_acolyte\"");
    let mut realm = Realm::with_seed(0x5EED_0342).with_callings(TEST_CALLINGS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    view.run(&mut realm, 1.0);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [CAMP[0] + 40.0, CAMP[1], 0.0],
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
                ..
            } if flat(*position, CAMP) < 15.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .min_by(|a, b| flat(a.position, CAMP).total_cmp(&flat(b.position, CAMP)))
        .expect("the camp is in view");
    (realm, mob, view)
}

/// The Scorpid Worker flagged as a boss, for the rest of the process.
fn flag_boss() {
    let _ = Realm::with_seed(0).with_bosses(&[SCORPID]);
}

/// Casts `spell_id` at `dest`, asserting it went; the events that came with it.
fn put_down(
    realm: &mut Realm,
    view: &mut View,
    spell_id: u32,
    dest: [f32; 3],
) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::CastSpellAtDest { spell_id, dest });
    let events = view.drain(realm);
    assert_eq!(refusal(&events, spell_id), None, "{spell_id} is cast");
    events
}

/// The yards a second of `mob`'s last chase step in `events`.
fn chase_speed(events: &[SessionEvent], mob: u64) -> Option<f32> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::MonsterMove {
            guid,
            start,
            path,
            duration_ms,
            ..
        } if *guid == mob && *duration_ms > 0 => {
            let mut from = *start;
            let mut yards = 0.0;
            for p in path {
                yards += flat(from, *p);
                from = *p;
            }
            Some(yards / (*duration_ms as f32 / 1000.0))
        }
        _ => None,
    })
}

/// Where `mob` last said it was heading in `events`.
fn last_position(events: &[SessionEvent], mob: &Mob) -> [f32; 3] {
    events
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::MonsterMove { guid, path, .. } if *guid == mob.guid => path.last(),
            _ => None,
        })
        .copied()
        .expect("the mob moved")
}

/// How fast the camp's nearest mob chases from inside Grasping Shadows, and how fast it chases
/// outside. Also checks that it is back to full speed once it has walked out of the circle.
fn slowed_against_free() -> (f32, f32) {
    let (mut realm, mob, mut view) = world();
    stand_at(&mut realm, &mob, 25.0);
    realm.handle(cast(POKE, Some(mob.guid)));
    let free = chase_speed(&view.run(&mut realm, 0.5), mob.guid).expect("the mob gives chase");

    let (mut realm, mob, mut view) = world();
    stand_at(&mut realm, &mob, 25.0);
    view.drain(&mut realm);
    put_down(&mut realm, &mut view, SHADOWS, mob.position);
    assert!(
        view.aura(mob.guid, SHADOWS_AURA).is_some(),
        "the mob inside holds the aura"
    );
    realm.handle(cast(POKE, Some(mob.guid)));
    let mut seen = view.run(&mut realm, 0.5);
    let slowed = chase_speed(&seen, mob.guid).expect("a snared chase");

    // It walks slowly out of the circle; the aura comes off with it, a long way short of 10 s.
    for _ in 0..80 {
        seen.extend(view.run(&mut realm, 0.1));
        if view.aura(mob.guid, SHADOWS_AURA).is_none() {
            break;
        }
    }
    assert!(
        view.aura(mob.guid, SHADOWS_AURA).is_none(),
        "the aura comes off outside"
    );
    let outside = Mob {
        guid: mob.guid,
        position: last_position(&seen, &mob),
    };
    stand_at(&mut realm, &outside, 20.0);
    let after = chase_speed(&view.run(&mut realm, 1.0), mob.guid).expect("the chase goes on");
    assert!(
        (after / free - 1.0).abs() < 0.05,
        "back to full outside: {after} against {free}"
    );
    (slowed, free)
}

/// One test, because flagging the camp's only creature a boss lasts for the rest of the process:
/// the ordinary mob goes first.
#[test]
fn a_mob_inside_runs_at_40_percent_a_boss_at_70_and_both_are_back_to_full_outside() {
    let (slowed, free) = slowed_against_free();
    let ratio = slowed / free;
    assert!((0.35..0.45).contains(&ratio), "{slowed} against {free}");
    flag_boss();
    let (slowed, free) = slowed_against_free();
    let ratio = slowed / free;
    assert!(
        (0.65..0.75).contains(&ratio),
        "boss: {slowed} against {free}"
    );
}

#[test]
fn miasma_cast_after_grasping_shadows_leaves_both_areas_down() {
    let (mut realm, mob, mut view) = world();
    stand_at(&mut realm, &mob, 25.0);
    view.drain(&mut realm);
    let first = put_down(&mut realm, &mut view, SHADOWS, mob.position);
    let object = first
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::DynamicObject,
                ..
            } => Some(*guid),
            _ => None,
        })
        .expect("Grasping Shadows shows as an object");
    let second = put_down(&mut realm, &mut view, MIASMA, mob.position);
    assert!(
        !second
            .iter()
            .any(|e| matches!(e, SessionEvent::ObjectDestroyed(g) if *g == object)),
        "Miasma does not take Grasping Shadows' object away"
    );
    assert!(view.aura(mob.guid, SHADOWS_AURA).is_some(), "the snare");
    assert!(view.aura(mob.guid, MIASMA_AURA).is_some(), "and the mist");
    view.run(&mut realm, 2.0);
    assert!(
        view.aura(mob.guid, SHADOWS_AURA).is_some(),
        "both are still down"
    );
}

#[test]
fn the_shipped_grasping_shadows_is_a_ground_snare_beside_miasma() {
    let acolyte = callings::calling("acolyte").unwrap();
    let skill = acolyte
        .skills
        .iter()
        .find(|s| s.key == "grasping_shadows")
        .unwrap();
    let aura = spell("acolyte", "aura_grasping_shadows");
    assert!(matches!(
        skill.effects[..],
        [Effect::GroundArea {
            duration_ms: 10000,
            enemy_aura_spell_id: Some(e),
            ally_aura_spell_id: None,
            ..
        }] if e == aura
    ));
    let row = callings::aura("aura_grasping_shadows").expect("the aura loads");
    assert_eq!(row.duration_ms, 10000);
    assert!(matches!(
        row.effects[..],
        [AuraEffect::ModRunSpeed { pct: -60, .. }]
    ));
}
