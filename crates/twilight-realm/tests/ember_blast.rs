//! Ember Blast (ADR 0020): a `self_area` skill with school damage, the `knockback` effect and an
//! `interrupt` that leaves no Lockout. Every enemy in reach takes the damage and slides straight
//! away from the caster, losing the cast it had under way; a boss takes the damage only. Played
//! through the realm's seam with a test Calling whose blast is free.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::callings::{Effect, Target};
use twilight_realm::{Host, Realm, RealmCommand};

/// A free `self_area` blast of radius 6 with the three effects of the shipped row.
const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_blaster"
name = "Test Blaster"
tier = 0
class = 8
cap = 30
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "blast"
spell_id = 11113
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self_area"
radius_yd = 6.0
max_targets = 100
effects = [{ kind = "school_damage", school = "fire", amount = { base = 8.0, per_level = 1.0 } }, { kind = "knockback", distance_yd = 10.0 }, { kind = "interrupt", lockout_ms = 0, bosses = false }]
"#;

const BLAST: u32 = 11113;
/// The Vile Familiar, and where its pack spawns.
const FAMILIAR: u32 = 3101;
const FAMILIAR_AT: [f32; 3] = [-731.0, -2518.0, 0.0];
/// The Scorpid Worker of the camp south of the Crossroads, which the boss test flags.
const SCORPID: u32 = 3124;
const SHADOW_BOLT: &str = "shadow_bolt_creature";

/// Flat ground with a wall across it: nothing is seen through the plane `x = wall`.
struct Walled {
    wall: f32,
}

impl Host for Walled {
    fn ground_height(&self, _map: u32, _x: f32, _y: f32, _z_hint: f32) -> Option<f32> {
        Some(0.0)
    }
    fn line_of_sight(&self, _map: u32, from: [f32; 3], to: [f32; 3]) -> bool {
        !(from[0] > self.wall && to[0] <= self.wall)
    }
    fn area_at(&self, _map: u32, _x: f32, _y: f32) -> Option<u32> {
        None
    }
}

/// A level 20 test Blaster `yards` east of the nearest creature of `entry` around `spawn`, which
/// casts `spells`: the realm, the player's guid, the mob and the view.
fn world(
    seed: u64,
    entry: u32,
    spawn: [f32; 3],
    spells: &[&str],
    yards: f32,
) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Cinder", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 20\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_blaster\"");
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
    (realm, guid, mob, view)
}

/// Ticks `secs` in 0.1 s steps on `host`, each event with the time it came at.
fn timed(
    realm: &mut Realm,
    view: &mut View,
    host: &dyn Host,
    secs: f32,
) -> Vec<(f32, SessionEvent)> {
    let mut events: Vec<_> = view.drain(realm).into_iter().map(|e| (0.0, e)).collect();
    for n in 1..=(secs * 10.0).round() as u32 {
        realm.tick(0.1, host);
        events.extend(view.drain(realm).into_iter().map(|e| (n as f32 * 0.1, e)));
    }
    events
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

/// Casts the blast, asserting it went, and the events that came with it.
fn blast(realm: &mut Realm, view: &mut View) -> Vec<(f32, SessionEvent)> {
    realm.handle(cast(BLAST, None));
    let events = view.drain(realm);
    assert_eq!(refusal(&events, BLAST), None, "the blast is cast");
    events.into_iter().map(|e| (0.0, e)).collect()
}

/// Where `mob`'s slide starts and ends, if the client saw one.
fn slide(events: &[(f32, SessionEvent)], mob: u64) -> Option<([f32; 3], [f32; 3])> {
    events.iter().find_map(|(_, e)| match e {
        SessionEvent::MonsterMove {
            guid, start, path, ..
        } if *guid == mob => Some((*start, *path.last().expect("a path"))),
        _ => None,
    })
}

fn starts(events: &[(f32, SessionEvent)], mob: u64, spell_id: u32, from: f32, to: f32) -> bool {
    events.iter().any(|(t, e)| {
        *t >= from
            && *t < to
            && matches!(e, SessionEvent::SpellStart { caster, spell_id: s, .. }
                if *caster == mob && *s == spell_id)
    })
}

#[test]
fn a_mob_three_yards_off_takes_fire_damage_and_slides_ten_yards_away() {
    let (mut realm, me, mob, mut view) = world(1, FAMILIAR, FAMILIAR_AT, &[SHADOW_BOLT], 3.0);
    let before = view.health(mob.guid);
    let mut events = blast(&mut realm, &mut view);
    assert!(view.health(mob.guid) < before, "the mob takes damage");
    events.extend(timed(&mut realm, &mut view, &FlatGround, 0.4));
    let (start, end) = slide(&events, mob.guid).expect("a MonsterMove goes out for the mob");
    let caster = [mob.position[0] + 3.0, mob.position[1], mob.position[2]];
    let gained = flat(end, caster) - flat(start, caster);
    assert!(
        (gained - 10.0).abs() < 0.5,
        "the mob ends about 10 yd further from the caster, not {gained}"
    );
    assert!(
        events.iter().all(
            |(_, e)| !matches!(e, SessionEvent::SpellStart { caster, .. } if *caster == mob.guid)
                && !matches!(e, SessionEvent::AttackerState(s) if s.attacker == mob.guid)
        ),
        "it neither casts nor swings while it slides"
    );
    let _ = me;
}

#[test]
fn a_mob_nine_yards_off_is_untouched() {
    let (mut realm, _, mob, mut view) = world(2, FAMILIAR, FAMILIAR_AT, &[SHADOW_BOLT], 9.0);
    let before = view.health(mob.guid);
    let mut events = blast(&mut realm, &mut view);
    events.extend(timed(&mut realm, &mut view, &FlatGround, 0.4));
    assert_eq!(view.health(mob.guid), before, "no damage");
    assert!(slide(&events, mob.guid).is_none(), "no slide");
}

#[test]
fn a_blast_stops_a_cast_and_leaves_no_lockout() {
    let shadow = creature_spell(SHADOW_BOLT);
    let (mut realm, _, mob, mut view) = world(3, FAMILIAR, FAMILIAR_AT, &[SHADOW_BOLT], 3.0);
    until_cast(&mut realm, &mut view, mob.guid, shadow);
    let mut events = blast(&mut realm, &mut view);
    events.extend(timed(&mut realm, &mut view, &FlatGround, 4.0));
    assert!(
        events.iter().any(|(_, e)| matches!(e,
            SessionEvent::SpellFailedOther { caster, spell_id }
            if *caster == mob.guid && *spell_id == shadow)),
        "the client sees Shadow Bolt stop"
    );
    assert!(
        starts(&events, mob.guid, shadow, 0.0, 4.0),
        "Shadow Bolt starts again once the slide ends"
    );
}

#[test]
fn a_boss_takes_the_damage_and_keeps_its_place_and_its_cast() {
    let _ = Realm::with_seed(0).with_bosses(&[SCORPID]);
    let shadow = creature_spell(SHADOW_BOLT);
    let (mut realm, _, mob, mut view) = world(4, SCORPID, CAMP, &[SHADOW_BOLT], 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    until_cast(&mut realm, &mut view, mob.guid, shadow);
    realm.handle(RealmCommand::AttackStop);
    view.drain(&mut realm);
    let before = view.health(mob.guid);
    let mut events = blast(&mut realm, &mut view);
    assert!(view.health(mob.guid) < before, "the boss takes damage");
    events.extend(timed(&mut realm, &mut view, &FlatGround, 3.0));
    assert!(slide(&events, mob.guid).is_none(), "the boss is not moved");
    assert!(
        !events.iter().any(|(_, e)| matches!(e,
            SessionEvent::SpellFailedOther { caster, .. } if *caster == mob.guid)),
        "its cast is not stopped"
    );
    assert!(
        events.iter().any(|(_, e)| matches!(e,
            SessionEvent::SpellGo { caster, spell_id, .. }
            if *caster == mob.guid && *spell_id == shadow)),
        "the bolt goes off"
    );
}

#[test]
fn a_wall_behind_the_mob_stops_the_slide_short() {
    let (mut realm, _, mob, mut view) = world(5, FAMILIAR, FAMILIAR_AT, &[SHADOW_BOLT], 3.0);
    let wall = Walled {
        wall: mob.position[0] - 4.0,
    };
    let mut events = blast(&mut realm, &mut view);
    events.extend(timed(&mut realm, &mut view, &wall, 0.4));
    let (start, end) = slide(&events, mob.guid).expect("it slides some");
    assert!(end[0] > wall.wall, "it stops before the wall");
    assert!(
        flat(start, end) < 6.0,
        "it slid {} yd, short of 10",
        flat(start, end)
    );
}

#[test]
fn the_shipped_ember_blast_loads_at_level_twenty_with_the_interrupt_last() {
    let skill = twilight_realm::callings::skill(spell("initiate", "ember_blast"))
        .expect("initiate/ember_blast loads");
    assert_eq!(skill.learn_level, 20);
    assert_eq!(skill.target, Target::SelfArea);
    assert!(matches!(skill.effects[0], Effect::SchoolDamage { .. }));
    assert!(matches!(
        skill.effects[1],
        Effect::Knockback { distance_yd } if distance_yd == 10.0
    ));
    assert!(matches!(
        skill.effects[2],
        Effect::Interrupt {
            lockout_ms: 0,
            bosses: false
        }
    ));
}
