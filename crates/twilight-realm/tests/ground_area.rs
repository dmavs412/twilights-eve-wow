//! A ground area: Miasma at a point gives every enemy inside its damage-over-time aura and every
//! friendly player inside its healing aura, and takes them off on leaving. Trap springs (#681): a
//! `first_only` area with a spring radius catches every enemy near it, and `detonate_traps` sets
//! off the caster's armed traps, among the Test Grounds' dummies. Played through the realm's seam
//! with a test Priest Calling.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::acolyte::{dummy, Yard, YardGround, DUMMY_SPOTS};
use common::fight::*;
use common::*;
use twilight_realm::callings::{self, Effect};
use twilight_realm::{Realm, RealmCommand};

const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_priest"
name = "Test Priest"
tier = 0
class = 5
cap = 10
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "miasma"
spell_id = 2120
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "dest"
range_yd = [0.0, 30.0]
effects = [{ kind = "ground_area", radius_yd = 8.0, duration_ms = 10000, enemy_aura_spell_id = 2120, ally_aura_spell_id = 2121 }]

[[calling.skill]]
key = "mend"
spell_id = 2061
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "heal", amount = { base = 40.0 } }]

[[calling.skill]]
key = "bleed"
spell_id = 8092
power_cost = 50
cooldown_ms = 0
gcd_ms = 0
target = "self"
meta = { cost_kind = "health" }
effects = []

[[calling.skill]]
key = "snare_trap"
spell_id = 1499
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "dest"
range_yd = [0.0, 30.0]
effects = [{ kind = "ground_area", radius_yd = 2.5, duration_ms = 60000, arm_ms = 1000, first_only = true, spring_radius_yd = 7.0, enemy_aura_spell_id = 3355, object_display = 3071 }]

[[calling.skill]]
key = "frost_trap"
spell_id = 13809
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "dest"
range_yd = [0.0, 30.0]
effects = [{ kind = "ground_area", radius_yd = 1.0, duration_ms = 60000, arm_ms = 1000, first_only = true, spring_radius_yd = 4.0, enemy_aura_spell_id = 13810 }]

[[calling.skill]]
key = "slow_trap"
spell_id = 13813
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "dest"
range_yd = [0.0, 30.0]
effects = [{ kind = "ground_area", radius_yd = 1.0, duration_ms = 60000, arm_ms = 60000, first_only = true, spring_radius_yd = 5.0, enemy_aura_spell_id = 13812 }]

[[calling.skill]]
key = "far_trap"
spell_id = 13795
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "dest"
range_yd = [0.0, 30.0]
effects = [{ kind = "ground_area", radius_yd = 1.0, duration_ms = 60000, arm_ms = 1000, first_only = true, spring_radius_yd = 4.0, enemy_aura_spell_id = 13797 }]

[[calling.skill]]
key = "detonate"
spell_id = 3044
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "dest"
range_yd = [0.0, 30.0]
effects = [{ kind = "detonate_traps", radius_yd = 10.0 }]

[[calling.aura]]
key = "aura_snare_trap"
spell_id = 3355
duration_ms = 3000
effects = [{ kind = "periodic_damage", tick_ms = 1000, tick = { base = 1.0 }, school = "fire" }]

[[calling.aura]]
key = "aura_frost_trap"
spell_id = 13810
duration_ms = 3000
effects = [{ kind = "periodic_damage", tick_ms = 1000, tick = { base = 1.0 }, school = "frost" }]

[[calling.aura]]
key = "aura_slow_trap"
spell_id = 13812
duration_ms = 3000
effects = [{ kind = "periodic_damage", tick_ms = 1000, tick = { base = 1.0 }, school = "frost" }]

[[calling.aura]]
key = "aura_far_trap"
spell_id = 13797
duration_ms = 3000
effects = [{ kind = "periodic_damage", tick_ms = 1000, tick = { base = 1.0 }, school = "fire" }]

[[calling.aura]]
key = "miasma_enemy"
spell_id = 2120
duration_ms = 10000
effects = [{ kind = "periodic_damage", tick_ms = 1000, tick = { base = 1.0 }, school = "shadow" }]

[[calling.aura]]
key = "miasma_ally"
spell_id = 2121
duration_ms = 10000
effects = [{ kind = "mod_healing_taken_pct", pct = 15 }]
"#;

const MIASMA: u32 = 2120;
const MIASMA_ALLY: u32 = 2121;
const MEND: u32 = 2061;
const BLEED: u32 = 8092;
/// The trap skills and the aura each puts on what it catches.
const SNARE: u32 = 1499;
const SNARED: u32 = 3355;
const FROST: u32 = 13809;
const FROSTED: u32 = 13810;
const SLOW: u32 = 13813;
const SLOWED: u32 = 13812;
const FAR: u32 = 13795;
const DETONATE: u32 = 3044;
/// The Test Grounds' level 30 elite dummy, flagged a boss by one test; the trap tests leave it
/// outside every spring radius but that test's.
const ELITE_DUMMY: u32 = 97002;

/// Where the test Priest stands: east of the scorpid camp, out of its notice.
const STAND: [f32; 3] = [CAMP[0] + 40.0, CAMP[1], 0.0];

/// A level 10 test Priest east of the camp: the realm, their guid, the camp's mobs and the view.
fn world() -> (Realm, u64, Vec<Mob>, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Vellune", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_priest\"");
    let mut realm = Realm::with_seed(4).with_callings(TEST_CALLINGS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    view.run(&mut realm, 1.0);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: STAND,
        orientation: std::f32::consts::PI,
    });
    let mobs: Vec<Mob> = view
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
        .collect();
    assert!(mobs.len() >= 2, "the camp is in view");
    (realm, guid, mobs, view)
}

/// Casts Miasma at `dest`, asserting it went.
fn miasma(realm: &mut Realm, view: &mut View, dest: [f32; 3]) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::CastSpellAtDest {
        spell_id: MIASMA,
        dest,
    });
    let events = view.drain(realm);
    assert_eq!(refusal(&events, MIASMA), None, "Miasma is cast");
    events
}

/// Miasma's ticks on `mob` in `events`.
fn ticks_on(events: &[SessionEvent], mob: u64) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, SessionEvent::PeriodicAuraLog(l) if l.spell_id == MIASMA && l.target == mob))
        .count()
}

/// The DynamicObjects `events` create: guid, spell id and centre.
fn patch_objects(events: &[SessionEvent]) -> Vec<(u64, u32, [f32; 3])> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::DynamicObject,
                position,
                fields,
                ..
            } => Some((*guid, fields.dynamicobject_spell_id()?, *position)),
            _ => None,
        })
        .collect()
}

/// Each Miasma shows as a DynamicObject at its point; a second one elsewhere destroys the first.
#[test]
fn a_second_miasma_takes_the_first_ones_object_away() {
    let (mut realm, _, _, mut view) = world();
    let here = [STAND[0] - 10.0, STAND[1], 0.0];
    let there = [STAND[0] - 10.0, STAND[1] + 12.0, 0.0];
    let first = patch_objects(&miasma(&mut realm, &mut view, here));
    let [(old, MIASMA, at)] = first[..] else {
        panic!("one Miasma object, has {first:?}");
    };
    assert_eq!(at, here);
    let events = miasma(&mut realm, &mut view, there);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SessionEvent::ObjectDestroyed(g) if *g == old)),
        "the first object goes"
    );
    let second = patch_objects(&events);
    let [(new, MIASMA, at)] = second[..] else {
        panic!("one new Miasma object, has {second:?}");
    };
    assert_ne!(new, old, "a re-cast is a new object");
    assert_eq!(at, there);
}

/// The circle the client draws is the circle the realm uses: 8 yd from the point.
#[test]
fn miasma_reaches_a_mob_at_7_5_yards_and_not_one_at_8_5() {
    for (yards, inside) in [(7.5, true), (8.5, false)] {
        let (mut realm, _, mobs, mut view) = world();
        let mob = &mobs[0];
        stand_at(&mut realm, mob, 20.0);
        view.drain(&mut realm);
        // Toward the caster, who stands toward the Crossroads.
        let (dx, dy) = (
            CROSSROADS_XY[0] - mob.position[0],
            CROSSROADS_XY[1] - mob.position[1],
        );
        let len = (dx * dx + dy * dy).sqrt();
        let dest = [
            mob.position[0] + dx / len * yards,
            mob.position[1] + dy / len * yards,
            mob.position[2],
        ];
        miasma(&mut realm, &mut view, dest);
        assert_eq!(
            view.aura(mob.guid, MIASMA).is_some(),
            inside,
            "a mob {yards} yd from the point"
        );
    }
}

#[test]
fn miasma_ticks_on_a_mob_inside_and_not_on_one_outside() {
    let (mut realm, _, mobs, mut view) = world();
    let inside = &mobs[0];
    let outside = mobs
        .iter()
        .find(|m| flat(m.position, inside.position) > 8.0)
        .expect("a camp mob more than 8 yd from the first");
    stand_at(&mut realm, inside, 25.0);
    view.drain(&mut realm);
    miasma(&mut realm, &mut view, inside.position);
    assert!(view.aura(inside.guid, MIASMA).is_some(), "inside: the aura");
    assert!(view.aura(outside.guid, MIASMA).is_none(), "outside: none");
    let first = view.run(&mut realm, 1.05);
    assert_eq!(ticks_on(&first, inside.guid), 1, "a tick a second");
    assert_eq!(ticks_on(&first, outside.guid), 0);
    // Struck, the mob runs at the caster, out of the circle, and the aura comes off.
    view.run(&mut realm, 5.0);
    assert!(view.aura(inside.guid, MIASMA).is_none(), "off once it left");
}

#[test]
fn miasma_raises_the_healing_its_caster_receives_by_15_percent_inside() {
    let (mut realm, me, _, mut view) = world();
    let mend = |realm: &mut Realm, view: &mut View| {
        realm.handle(cast(BLEED, None));
        view.drain(realm);
        realm.handle(cast(MEND, None));
        view.drain(realm)
            .iter()
            .find_map(|e| match e {
                SessionEvent::SpellHealLog(l) if l.spell_id == MEND => Some(l.amount),
                _ => None,
            })
            .expect("a heal")
    };
    assert_eq!(mend(&mut realm, &mut view), 40);
    miasma(&mut realm, &mut view, STAND);
    assert!(
        view.aura(me, MIASMA_ALLY).is_some(),
        "the caster stands in it"
    );
    assert_eq!(mend(&mut realm, &mut view), 46, "15% more");
}

#[test]
fn the_shipped_miasma_is_a_ground_area_with_both_auras() {
    let acolyte = callings::calling("acolyte").unwrap();
    let miasma = acolyte.skills.iter().find(|s| s.key == "miasma").unwrap();
    let (enemy, ally) = (
        spell("acolyte", "aura_miasma"),
        spell("acolyte", "aura_miasma_ally"),
    );
    assert!(matches!(
        miasma.effects[..],
        [Effect::GroundArea {
            enemy_aura_spell_id: Some(e),
            ally_aura_spell_id: Some(a),
            ..
        }] if (e, a) == (enemy, ally)
    ));
    assert!(
        callings::aura("aura_miasma").is_some(),
        "the enemy aura loads"
    );
    assert!(
        callings::aura("aura_miasma_ally").is_some(),
        "the ally aura loads"
    );
}

/// The level 10 test Priest in the Test Grounds 15 yd west of the dummies, facing them.
fn yard() -> Yard {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Vellune", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = benilla_protocol::START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_priest\"");
    let mut realm = Realm::with_seed(4).with_callings(TEST_CALLINGS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &YardGround);
    let mut view = View::default();
    view.drain(&mut realm);
    let mut y = Yard { realm, view, me };
    for (xy, facing) in [([-9720.0, 255.0], None), ([-9655.0, 255.0], Some(0.0))] {
        y.realm.handle(RealmCommand::DevTeleport {
            x: xy[0],
            y: xy[1],
            facing,
        });
        let landed = [y.view.drain(&mut y.realm), y.run(1.0)].concat();
        // The client acknowledges the teleport, or the realm ignores its steps.
        if let Some(counter) = landed.iter().find_map(|e| match e {
            SessionEvent::Teleport { counter, .. } => Some(*counter),
            _ => None,
        }) {
            y.realm.handle(RealmCommand::TeleportAck { counter });
        }
    }
    y
}

/// Casts `spell_id` at the Test Grounds' point `xy`, asserting it went: what the client saw.
fn cast_at(y: &mut Yard, spell_id: u32, xy: [f32; 2]) -> Vec<SessionEvent> {
    y.realm.handle(RealmCommand::CastSpellAtDest {
        spell_id,
        dest: [xy[0], xy[1], DUMMY_SPOTS[0][2]],
    });
    let events = y.view.drain(&mut y.realm);
    assert_eq!(refusal(&events, spell_id), None, "spell {spell_id} is cast");
    events
}

/// The DynamicObject of `spell_id` that `events` create.
fn trap_object(events: &[SessionEvent], spell_id: u32) -> u64 {
    let made: Vec<u64> = patch_objects(events)
        .into_iter()
        .filter(|&(_, id, _)| id == spell_id)
        .map(|(guid, _, _)| guid)
        .collect();
    let [object] = made[..] else {
        panic!("one object of {spell_id}, has {made:?}");
    };
    object
}

fn destroyed(events: &[SessionEvent], object: u64) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::ObjectDestroyed(g) if *g == object))
}

/// Which of the four dummies wear `spell_id`.
fn wearing(y: &Yard, spell_id: u32) -> [bool; 4] {
    [0, 1, 2, 3].map(|i| y.view.aura(dummy(i), spell_id).is_some())
}

/// The Snare Trap at dummy 1 springs once armed on it, the first inside: dummies 0 and 2, 6 yd
/// off, are within its 7 yd spring radius and take the aura too; dummy 3, 12 yd off, does not.
#[test]
fn a_trap_with_a_spring_radius_catches_every_enemy_within_it_and_is_spent() {
    let mut y = yard();
    let [_, at, ..] = DUMMY_SPOTS;
    let laid = cast_at(&mut y, SNARE, [at[0], at[1]]);
    let object = trap_object(&laid, SNARE);
    let early = y.run(0.5);
    assert_eq!(wearing(&y, SNARED), [false; 4], "nothing while it arms");
    assert!(!destroyed(&early, object), "and it stands");
    let late = y.run(1.0);
    assert_eq!(wearing(&y, SNARED), [true, true, true, false]);
    assert!(destroyed(&late, object), "the trap is spent");
}

/// The Snare Trap at dummy 2 catches it and dummy 1, but not the boss beside them.
#[test]
fn a_boss_within_the_spring_radius_takes_nothing() {
    let _ = Realm::with_seed(0).with_bosses(&[ELITE_DUMMY]);
    let mut y = yard();
    let [_, _, at, _] = DUMMY_SPOTS;
    cast_at(&mut y, SNARE, [at[0], at[1]]);
    y.run(1.5);
    assert_eq!(wearing(&y, SNARED), [false, true, true, false]);
}

#[test]
fn two_different_traps_stand_at_once_and_a_recast_replaces_only_its_own() {
    let mut y = yard();
    let snare = trap_object(&cast_at(&mut y, SNARE, [-9660.0, 250.0]), SNARE);
    let frost = trap_object(&cast_at(&mut y, FROST, [-9660.0, 245.0]), FROST);
    y.run(1.5);
    let events = cast_at(&mut y, SNARE, [-9662.0, 240.0]);
    assert!(destroyed(&events, snare), "the first Snare Trap goes");
    assert!(!destroyed(&events, frost), "the Frost Trap stays");
    assert_ne!(trap_object(&events, SNARE), snare, "a new Snare Trap comes");
}

/// Three traps west of the dummies, then a detonation 5 yd from the armed Frost Trap, whose 4 yd
/// spring radius holds dummy 1 though its 1 yd circle holds no one. The Slow Trap, 6.4 yd from the
/// point with dummy 0 in its spring radius, is still arming; the other one is 15 yd off.
#[test]
fn detonating_springs_the_armed_traps_near_the_point_and_leaves_the_rest() {
    let mut y = yard();
    let frost = trap_object(&cast_at(&mut y, FROST, [-9643.0, 258.0]), FROST);
    let slow = trap_object(&cast_at(&mut y, SLOW, [-9644.0, 263.0]), SLOW);
    let far = trap_object(&cast_at(&mut y, FAR, [-9663.0, 258.0]), FAR);
    let waited = y.run(1.5);
    assert_eq!(
        wearing(&y, FROSTED),
        [false; 4],
        "no one steps into the Frost Trap"
    );
    assert!(!destroyed(&waited, frost), "so it stands, armed");
    let events = [cast_at(&mut y, DETONATE, [-9648.0, 258.0]), y.run(0.2)].concat();
    assert_eq!(
        wearing(&y, FROSTED),
        [false, true, false, false],
        "the Frost Trap springs"
    );
    assert!(destroyed(&events, frost), "and is spent");
    assert_eq!(
        wearing(&y, SLOWED),
        [false; 4],
        "the arming trap does not spring"
    );
    assert!(!destroyed(&events, slow), "and stands");
    assert!(!destroyed(&events, far), "the far trap stands");
}
