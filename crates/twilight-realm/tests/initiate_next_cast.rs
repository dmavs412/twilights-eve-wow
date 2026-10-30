//! Timed casts: a skill with a cast bar goes off when the bar fills, and a step interrupts it.
//! Played through the realm's seam with a test Mage Calling whose skill hits for 1. The shipped
//! Arcane Substrate and Ley-Line Attunement auras load as next-cast buffs.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::callings::{self, AuraEffect};
use twilight_realm::{Realm, RealmCommand};

const MANA: u8 = 0;

const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_mage"
name = "Test Mage"
tier = 0
class = 8
cap = 10
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "biting_cold"
spell_id = 116
power_cost = 20
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
meta = { cast_time_ms = 2500 }
effects = [{ kind = "school_damage", school = "frost", amount = { base = 1.0 } }]
"#;

const BITING_COLD: u32 = 116;

/// A level 10 test Mage 20 yd from a camp mob: the realm, their guid, the mob and the view.
fn world() -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Weaver", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_mage\"");
    let mut realm = Realm::with_seed(5).with_callings(TEST_CALLINGS);
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
        .find_map(|e| match e {
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
        .expect("a camp mob in view");
    stand_at(&mut realm, &mob, 20.0);
    view.drain(&mut realm);
    (realm, guid, mob, view)
}

/// Starts `spell_id` at `mob`: its cast bar's length, after asserting it was not refused.
fn start(realm: &mut Realm, view: &mut View, spell_id: u32, mob: &Mob) -> u32 {
    realm.handle(cast(spell_id, Some(mob.guid)));
    let events = view.drain(realm);
    assert_eq!(refusal(&events, spell_id), None, "spell {spell_id} starts");
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::SpellStart {
                spell_id: s,
                cast_time_ms,
                ..
            } if *s == spell_id => Some(*cast_time_ms),
            _ => None,
        })
        .expect("a spell start")
}

/// Whether `spell_id` dealt damage in `events`.
fn landed(events: &[SessionEvent], spell_id: u32) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id))
}

#[test]
fn a_cast_bar_goes_off_when_it_fills_and_moving_interrupts_it() {
    let (mut realm, me, mob, mut view) = world();
    assert_eq!(start(&mut realm, &mut view, BITING_COLD, &mob), 2500);
    assert!(!landed(&view.run(&mut realm, 2.4), BITING_COLD), "not yet");
    assert!(landed(&view.run(&mut realm, 0.2), BITING_COLD), "it lands");

    start(&mut realm, &mut view, BITING_COLD, &mob);
    let mana = view.fields(me).unit_power(MANA);
    view.run(&mut realm, 1.0);
    stand_at(&mut realm, &mob, 25.0);
    let after = view.run(&mut realm, 2.0);
    assert!(!landed(&after, BITING_COLD), "interrupted by the step");
    assert_eq!(view.fields(me).unit_power(MANA), mana, "nothing paid");
}

/// Each Arcane Substrate rider names its own skill and shares no group with the other, so each
/// goes with its own skill's next cast (#506).
#[test]
fn the_shipped_substrate_riders_name_their_skills_and_share_no_group() {
    let cold = callings::aura("aura_arcane_substrate_cold_5").expect("modelled");
    let ember = callings::aura("aura_arcane_substrate_ember_5").expect("modelled");
    assert_eq!(cold.row.exclusive_group, None);
    assert_eq!(ember.row.exclusive_group, None);
    assert!(matches!(
        &cold.effects[0],
        AuraEffect::NextCast(n) if n.skill == ["biting_cold"] && n.cast_time_ms == -500 && !n.family_bits.is_empty()
    ));
    assert!(matches!(
        &ember.effects[0],
        AuraEffect::NextCast(n) if n.skill == ["searing_ember"] && n.duration_ms == 5000
    ));
}

// Arcane Substrate and Thermal Feedback (#506), played by a shipped Initiate at 24 on the level 8
// or 9 mob south of the Crossroads.

use common::initiate::*;
use common::vanguard::STURDY;

/// After Arcane Substrate's Ley-Spark the next Biting Cold's bar is 1.5 s, not 2, and spends only
/// the Cold rider: a Searing Ember after it still burns 17 s, not 12. After plain Ley-Spark the
/// bar is 2 s and the burn 12 s.
#[test]
fn after_arcane_substrates_ley_spark_biting_cold_casts_in_1_5_s_and_the_next_ember_burns_17_s() {
    let substrate = choice("initiate", "ley_spark", "arcane_substrate");
    let (cold, ember, burn) = (
        initiate("biting_cold"),
        initiate("searing_ember"),
        initiate("aura_searing_ember"),
    );
    for (spark, rows, bar, burns) in [
        (substrate, vec![("ley_spark", substrate)], 1500, 17.0),
        (initiate("ley_spark"), vec![], 2000, 12.0),
    ] {
        let mut s = initiate_scene(5, &rows, STURDY);
        let mob = s.mob.guid;
        cast_through(&mut s.realm, &mut s.view, spark, mob);
        let events = cast_through(&mut s.realm, &mut s.view, cold, mob);
        assert_eq!(cast_bar(&events, cold), Some(bar), "{rows:?}");
        cast_through(&mut s.realm, &mut s.view, ember, mob);
        assert!(s.view.aura(mob, burn).is_some(), "burning");
        let secs = lasts(&mut s.realm, &mut s.view, mob, burn, 20.0);
        assert!((secs - burns).abs() <= 0.25, "{rows:?}: burned {secs} s");
    }
}

/// Arcane Substrate's Cold rider shortens a Skill Choice of Biting Cold as well (Frostbite): a
/// choice answers to its skill's next-cast buffs.
#[test]
fn arcane_substrates_rider_shortens_a_skill_choice_of_biting_cold() {
    let substrate = choice("initiate", "ley_spark", "arcane_substrate");
    let frostbite = choice("initiate", "biting_cold", "frostbite");
    let rows = [("ley_spark", substrate), ("biting_cold", frostbite)];
    let mut s = initiate_scene(5, &rows, STURDY);
    let mob = s.mob.guid;
    cast_through(&mut s.realm, &mut s.view, substrate, mob);
    let events = cast_through(&mut s.realm, &mut s.view, frostbite, mob);
    assert_eq!(cast_bar(&events, frostbite), Some(1500));
}

/// Across seeds some Thermal Feedback tick grants the Initiate Thermal Feedback's buff, and with
/// it their next Biting Cold is instant; plain Searing Ember's ticks never grant it.
#[test]
fn thermal_feedbacks_tick_grants_its_buff_and_plain_searing_ember_never_does() {
    let feedback = choice("initiate", "searing_ember", "thermal_feedback");
    let buff = initiate("aura_thermal_feedback");
    let cold = initiate("biting_cold");
    let mut granted = 0;
    for seed in 0..12 {
        let mut s = initiate_scene(seed, &[("searing_ember", feedback)], STURDY);
        let (me, mob) = (s.me, s.mob.guid);
        cast_through(&mut s.realm, &mut s.view, feedback, mob);
        let mut events = Vec::new();
        for _ in 0..130 {
            events.extend(s.view.run(&mut s.realm, 0.1));
            if s.view.aura(me, buff).is_some() {
                break;
            }
        }
        if s.view.aura(me, buff).is_none() {
            continue;
        }
        granted += 1;
        let ticked = events.iter().any(
            |e| matches!(e, SessionEvent::PeriodicAuraLog(l) if l.target == mob && l.caster == me),
        );
        assert!(ticked, "seed {seed}: granted on a tick");
        let events = cast_through(&mut s.realm, &mut s.view, cold, mob);
        assert_eq!(cast_bar(&events, cold), Some(0), "seed {seed}: instant");
    }
    assert!(granted > 0, "no tick granted the buff");

    let ember = initiate("searing_ember");
    for seed in 0..12 {
        let mut s = initiate_scene(seed, &[], STURDY);
        cast_through(&mut s.realm, &mut s.view, ember, s.mob.guid);
        s.view.run(&mut s.realm, 13.0);
        assert!(s.view.aura(s.me, buff).is_none(), "seed {seed}");
    }
}
