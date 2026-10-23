//! Kicks (ADR 0020): the `interrupt` effect stops a creature's cast and locks its school for the
//! Lockout. The effects after it in the row run only on a cast it stopped. Played through the
//! realm's seam with a test Mage Calling whose kick is an `interrupt` and a combo point.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

/// `kick` locks a school for 6 s and gives a combo point; `soft_kick` does the same but leaves a
/// boss alone.
const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_kicker"
name = "Test Kicker"
tier = 0
class = 8
cap = 30
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "kick"
spell_id = 1766
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 100.0]
effects = [{ kind = "interrupt", lockout_ms = 6000 }, { kind = "add_combo_points", amount = 1 }]

[[calling.skill]]
key = "soft_kick"
spell_id = 1767
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 100.0]
effects = [{ kind = "interrupt", lockout_ms = 6000, bosses = false }, { kind = "add_combo_points", amount = 1 }]
"#;

const KICK: u32 = 1766;
const SOFT_KICK: u32 = 1767;
/// The Vile Familiar, and where its pack spawns.
const FAMILIAR: u32 = 3101;
const FAMILIAR_AT: [f32; 3] = [-731.0, -2518.0, 0.0];
/// The Scorpid Worker of the camp south of the Crossroads, which the boss tests flag.
const SCORPID: u32 = 3124;
const SHADOW_BOLT: &str = "shadow_bolt_creature";
const FIRE_BOLT: &str = "fire_bolt_creature";
/// `PLAYER_FIELD_BYTES`, whose byte 1 is the combo points.
const PLAYER_FIELD_BYTES: u16 = 1222;

/// A level 20 test Kicker beside the nearest creature of `entry` around `spawn`, with `spells`
/// cast by every creature of that entry: the realm, the player's guid, the mob and the view. The
/// player stands `yards` from the mob.
fn world(
    seed: u64,
    entry: u32,
    spawn: [f32; 3],
    spells: &[&str],
    yards: f32,
) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Cutter", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 20\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_kicker\"");
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

/// Ticks `secs` in 0.1 s steps, each event with the time it came at.
fn timed(realm: &mut Realm, view: &mut View, secs: f32) -> Vec<(f32, SessionEvent)> {
    let mut events: Vec<_> = view.drain(realm).into_iter().map(|e| (0.0, e)).collect();
    for n in 1..=(secs * 10.0).round() as u32 {
        realm.tick(0.1, &FlatGround);
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

/// Casts `spell_id` at `mob`, asserting it went, and the events that came with it.
fn kick(realm: &mut Realm, view: &mut View, spell_id: u32, mob: u64) -> Vec<(f32, SessionEvent)> {
    realm.handle(cast(spell_id, Some(mob)));
    let events = view.drain(realm);
    assert_eq!(refusal(&events, spell_id), None, "kick {spell_id} is cast");
    events.into_iter().map(|e| (0.0, e)).collect()
}

/// Whether `mob` starts casting `spell_id` between `from` and `to` seconds in `events`.
fn starts(events: &[(f32, SessionEvent)], mob: u64, spell_id: u32, from: f32, to: f32) -> bool {
    events.iter().any(|(t, e)| {
        *t >= from
            && *t < to
            && matches!(e, SessionEvent::SpellStart { caster, spell_id: s, .. }
                if *caster == mob && *s == spell_id)
    })
}

/// The combo points the client sees on `me`.
fn combo_points(view: &View, me: u64) -> u32 {
    view.fields(me)
        .raw_fields()
        .find(|(i, _)| *i == PLAYER_FIELD_BYTES)
        .map_or(0, |(_, v)| (v >> 8) & 0xff)
}

#[test]
fn a_kick_stops_shadow_bolt_and_locks_shadow_out_for_six_seconds() {
    let shadow = creature_spell(SHADOW_BOLT);
    let (mut realm, me, mob, mut view) =
        world(1, FAMILIAR, FAMILIAR_AT, &[SHADOW_BOLT, FIRE_BOLT], 2.0);
    until_cast(&mut realm, &mut view, mob.guid, shadow);
    let mut events = kick(&mut realm, &mut view, KICK, mob.guid);
    events.extend(timed(&mut realm, &mut view, 10.0));
    assert!(
        events.iter().any(|(_, e)| matches!(e,
            SessionEvent::SpellFailedOther { caster, spell_id }
            if *caster == mob.guid && *spell_id == shadow)),
        "the client sees Shadow Bolt stop"
    );
    assert!(
        !events.iter().any(|(t, e)| *t < 6.0
            && matches!(e, SessionEvent::SpellGo { caster, spell_id, .. }
            if *caster == mob.guid && *spell_id == shadow)),
        "the stopped bolt did not land"
    );
    assert!(
        !starts(&events, mob.guid, shadow, 0.0, 5.9),
        "no Shadow Bolt inside the Lockout"
    );
    assert!(
        starts(&events, mob.guid, shadow, 6.0, 10.0),
        "Shadow Bolt starts again once the Lockout ends"
    );
    assert_eq!(combo_points(&view, me), 1, "the stopped cast gave a point");
}

#[test]
fn during_the_lockout_the_mob_can_still_start_fire_bolt() {
    let shadow = creature_spell(SHADOW_BOLT);
    let fire = creature_spell(FIRE_BOLT);
    let (mut realm, _, mob, mut view) =
        world(2, FAMILIAR, FAMILIAR_AT, &[SHADOW_BOLT, FIRE_BOLT], 2.0);
    until_cast(&mut realm, &mut view, mob.guid, shadow);
    let mut events = kick(&mut realm, &mut view, KICK, mob.guid);
    events.extend(timed(&mut realm, &mut view, 5.0));
    assert!(
        starts(&events, mob.guid, fire, 0.0, 5.9),
        "Fire Bolt is another school"
    );
}

#[test]
fn a_kick_on_a_mob_that_is_not_casting_gives_no_point_and_no_lockout() {
    let shadow = creature_spell(SHADOW_BOLT);
    let (mut realm, me, mob, mut view) = world(3, FAMILIAR, FAMILIAR_AT, &[SHADOW_BOLT], 20.0);
    let mut events = kick(&mut realm, &mut view, KICK, mob.guid);
    events.extend(timed(&mut realm, &mut view, 3.0));
    assert_eq!(combo_points(&view, me), 0, "nothing was stopped");
    assert!(
        starts(&events, mob.guid, shadow, 0.0, 3.0),
        "no Lockout: the mob casts Shadow Bolt at once"
    );
}

#[test]
fn each_shipped_kick_loads_at_level_ten_with_interrupt_first() {
    for (calling, key) in [
        ("vanguard", "rebuke"),
        ("cutthroat", "kick"),
        ("initiate", "counterspell"),
        ("ranger", "silencing_shot"),
        ("acolyte", "hush"),
    ] {
        let skill = twilight_realm::callings::skill(spell(calling, key))
            .unwrap_or_else(|| panic!("{calling}/{key} loads"));
        assert_eq!(skill.learn_level, 10, "{calling}/{key}");
        assert!(
            matches!(
                skill.effects.first(),
                Some(twilight_realm::callings::Effect::Interrupt { bosses: true, .. })
            ),
            "{calling}/{key} opens with interrupt"
        );
    }
}

/// The Scorpid Worker flagged as a boss, for the rest of the process.
fn flag_boss() {
    let _ = Realm::with_seed(0).with_bosses(&[SCORPID]);
}

#[test]
fn a_kick_stops_a_boss_cast() {
    flag_boss();
    let shadow = creature_spell(SHADOW_BOLT);
    let (mut realm, me, mob, mut view) = world(4, SCORPID, CAMP, &[SHADOW_BOLT], 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    until_cast(&mut realm, &mut view, mob.guid, shadow);
    let mut events = kick(&mut realm, &mut view, KICK, mob.guid);
    events.extend(timed(&mut realm, &mut view, 1.0));
    assert!(
        events.iter().any(|(_, e)| matches!(e,
            SessionEvent::SpellFailedOther { caster, spell_id }
            if *caster == mob.guid && *spell_id == shadow)),
        "a kick stops a boss's cast"
    );
    assert_eq!(combo_points(&view, me), 1);
}

#[test]
fn a_kick_with_bosses_off_does_nothing_to_a_boss() {
    flag_boss();
    let shadow = creature_spell(SHADOW_BOLT);
    let (mut realm, me, mob, mut view) = world(5, SCORPID, CAMP, &[SHADOW_BOLT], 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    until_cast(&mut realm, &mut view, mob.guid, shadow);
    let mut events = kick(&mut realm, &mut view, SOFT_KICK, mob.guid);
    events.extend(timed(&mut realm, &mut view, 3.0));
    assert!(
        !events.iter().any(|(_, e)| matches!(e,
            SessionEvent::SpellFailedOther { caster, .. } if *caster == mob.guid)),
        "the cast is not stopped"
    );
    assert!(
        events.iter().any(|(_, e)| matches!(e,
            SessionEvent::SpellGo { caster, spell_id, .. }
            if *caster == mob.guid && *spell_id == shadow)),
        "the bolt goes off"
    );
    assert_eq!(combo_points(&view, me), 0, "and gives no point");
}
