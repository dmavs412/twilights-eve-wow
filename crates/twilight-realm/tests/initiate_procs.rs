//! The Initiate's procs: Thermal Feedback on a burn's tick, Ley-Line Tap's refund (or shield at
//! full mana), Frigid Conservation's refund on a target already slowed, and Arcane Momentum's
//! boost after Blink. Played through the realm's seam with a test Mage Calling.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
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
key = "ley_spark"
spell_id = 2912
power_cost = 20
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "school_damage", school = "arcane", amount = { base = 4.0 } }]

[[calling.skill]]
key = "biting_cold"
spell_id = 116
power_cost = 20
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
meta = { cast_time_ms = 2500 }
effects = [
    { kind = "school_damage", school = "frost", amount = { base = 0.0 } },
    { kind = "mod_run_speed", pct = -50, duration_ms = 5000 },
]

[[calling.skill]]
key = "searing_ember"
spell_id = 133
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "periodic_damage", tick_ms = 100, duration_ms = 60000, tick = { base = 0.0 } }]

[[calling.skill]]
key = "blink"
spell_id = 1953
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "blink", distance_yd = 1.0 }]

[[calling.skill]]
key = "drain"
spell_id = 8921
power_cost = 50
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = []

[[calling.aura]]
key = "feedback"
spell_id = 11069
duration_ms = 15000
charges = 1
effects = [{ kind = "next_cast", skill = "biting_cold", instant = true, cost_pct = -100 }]

[[calling.aura]]
key = "momentum"
spell_id = 15060
duration_ms = 5000
effects = [{ kind = "skill_amount_pct", skill = "ley_spark", pct = 45 }]

[[calling.talent_tab]]
id = 906
name = "Test"
background = "MageArcane"

[[calling.talent_tab.talent]]
key = "thermal_feedback"
talent_id = 26
tier = 1
column = 1
rank_spells = [11069, 12338, 12339, 12340, 12341]
spell_name = "Thermal Feedback"
effects = [{ kind = "proc", on = "periodic_tick", skill = "searing_ember", chances = [2, 4, 6, 8, 10], per_rank = [0, 0, 0, 0, 0], auras = [["feedback"], ["feedback"], ["feedback"], ["feedback"], ["feedback"]] }]

[[calling.talent_tab.talent]]
key = "ley_line_tap"
talent_id = 75
tier = 1
column = 2
rank_spells = [11213, 12574, 12575, 12576, 12577]
spell_name = "Ley-Line Tap"
effects = [{ kind = "proc", on = "cast", skill = "ley_spark", chances = [10, 20, 30, 40, 50], per_rank = [0, 0, 0, 0, 0], refund_cost_pct = [100, 100, 100, 100, 100], absorb_cost_pct = 200, absorb_duration_ms = 10000, at_full_power = "absorb" }]

[[calling.talent_tab.talent]]
key = "frigid_conservation"
talent_id = 741
tier = 1
column = 3
rank_spells = [16757, 16758]
spell_name = "Frigid Conservation"
effects = [{ kind = "proc", on = "cast", skill = "biting_cold", requires_target_aura = "slowed", per_rank = [0, 0], refund_cost_pct = [25, 50] }]

[[calling.talent_tab.talent]]
key = "arcane_momentum"
talent_id = 421
tier = 1
column = 4
rank_spells = [15058, 15059, 15060]
spell_name = "Arcane Momentum"
effects = [{ kind = "proc", on = "cast", skill = "blink", per_rank = [0, 0, 0], auras = [["momentum"], ["momentum"], ["momentum"]] }]
"#;

const LEY_SPARK: u32 = 2912;
const BITING_COLD: u32 = 116;
const SEARING_EMBER: u32 = 133;
const BLINK: u32 = 1953;
const DRAIN: u32 = 8921;
const FEEDBACK: u32 = 11069;
/// Ley-Line Tap's rank 5 spell: its refunds and its shield are logged under it.
const TAP_5: u32 = 12577;
const FRIGID_2: u32 = 16758;

const THERMAL_FEEDBACK: u32 = 26;
const LEY_LINE_TAP: u32 = 75;
const FRIGID_CONSERVATION: u32 = 741;
const ARCANE_MOMENTUM: u32 = 421;

/// A level 10 test Mage with `talents` at `seed`, 20 yd from a camp mob: the realm, their guid,
/// the mob and the view.
fn world(seed: u64, talents: &[(u32, u8)]) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Weaver", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_mage\"");
    if !talents.is_empty() {
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        blob = blob.replace(
            "money = 0\n",
            &format!("money = 0\ntalents = [{}]\n", rows.join(", ")),
        );
    }
    let mut realm = Realm::with_seed(seed).with_callings(TEST_CALLINGS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    view.run(&mut realm, 1.0);
    realm.handle(RealmCommand::Move {
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

/// Casts `spell_id` at `target`: the events, after asserting it was not refused.
fn go(realm: &mut Realm, view: &mut View, spell_id: u32, target: Option<u64>) -> Vec<SessionEvent> {
    realm.handle(cast(spell_id, target));
    let events = view.drain(realm);
    assert_eq!(refusal(&events, spell_id), None, "spell {spell_id} is cast");
    events
}

/// The power `spell_id` gave back in `events`.
fn refunded(events: &[SessionEvent], spell_id: u32) -> u32 {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellEnergizeLog(l) if l.spell_id == spell_id => Some(l.amount),
            _ => None,
        })
        .sum()
}

fn mana(view: &View, me: u64) -> u32 {
    view.fields(me).unit_power(MANA).unwrap()
}

#[test]
fn thermal_feedback_fires_on_a_tenth_of_ticks_and_makes_biting_cold_instant_and_free() {
    let (mut ticks, mut procs) = (0, 0);
    for seed in 0..10 {
        let (mut realm, me, mob, mut view) = world(seed, &[(THERMAL_FEEDBACK, 5)]);
        go(&mut realm, &mut view, SEARING_EMBER, Some(mob.guid));
        for _ in 0..200 {
            let events = view.run(&mut realm, 0.1);
            ticks += events
                .iter()
                .filter(|e| matches!(e, SessionEvent::PeriodicAuraLog(l) if l.spell_id == SEARING_EMBER))
                .count();
            if view.aura(me, FEEDBACK).is_none() {
                continue;
            }
            procs += 1;
            let before = mana(&view, me);
            realm.handle(cast(BITING_COLD, Some(mob.guid)));
            let cold = view.drain(&mut realm);
            assert!(
                cold.iter().any(
                    |e| matches!(e, SessionEvent::SpellDamageLog(l) if l.spell_id == BITING_COLD)
                ),
                "instant: it lands at once"
            );
            assert_eq!(mana(&view, me), before, "free");
            assert!(view.aura(me, FEEDBACK).is_none(), "used up");
        }
    }
    let rate = procs as f64 / ticks as f64;
    assert!(ticks > 1500, "{ticks} ticks");
    assert!(
        (0.07..0.13).contains(&rate),
        "{procs} procs in {ticks} ticks"
    );
}

#[test]
fn ley_line_tap_refunds_about_half_of_ley_sparks_below_full_mana() {
    let (mut casts, mut refunds) = (0, 0);
    for seed in 0..40 {
        let (mut realm, me, mob, mut view) = world(seed, &[(LEY_LINE_TAP, 5)]);
        go(&mut realm, &mut view, DRAIN, None);
        for _ in 0..5 {
            let before = mana(&view, me);
            let events = go(&mut realm, &mut view, LEY_SPARK, Some(mob.guid));
            casts += 1;
            match refunded(&events, TAP_5) {
                0 => assert_eq!(mana(&view, me), before - 20),
                back => {
                    assert_eq!(back, 20, "the whole cost");
                    assert_eq!(mana(&view, me), before);
                    refunds += 1;
                }
            }
        }
    }
    let rate = refunds as f64 / casts as f64;
    assert!(
        (0.4..0.6).contains(&rate),
        "{refunds} refunds in {casts} casts"
    );
}

#[test]
fn ley_line_tap_at_full_mana_gives_a_shield_that_soaks_twice_the_cost() {
    // A seed where the first Ley-Spark, at full mana, procs.
    let (mut realm, me, mob, mut view) = (0..40)
        .map(|seed| {
            let (mut realm, me, mob, mut view) = world(seed, &[(LEY_LINE_TAP, 5)]);
            go(&mut realm, &mut view, LEY_SPARK, Some(mob.guid));
            (realm, me, mob, view)
        })
        .find(|(_, me, _, view)| view.aura(*me, TAP_5).is_some())
        .expect("a seed where the shield goes up");
    let _ = &mob;
    // No refund: the shield instead.
    let full = view.fields(me).unit_max_power(MANA).unwrap();
    assert_eq!(mana(&view, me), full - 20);
    // The camp mob's blows go into the shield while it lasts (the realm's unit test checks its
    // 40-point cap).
    let health = view.health(me);
    let mut taken = 0;
    while view.aura(me, TAP_5).is_some() {
        for e in view.run(&mut realm, 0.1) {
            if let SessionEvent::AttackerState(s) = e {
                if s.victim == me {
                    taken += s.damage;
                }
            }
        }
    }
    assert!(taken > 0, "the mob landed blows");
    assert_eq!(
        health - view.health(me),
        taken.saturating_sub(40),
        "the shield soaked up to 40 of {taken}"
    );
}

#[test]
fn frigid_conservation_refunds_half_only_on_a_target_already_slowed() {
    let (mut realm, me, mob, mut view) = world(3, &[(FRIGID_CONSERVATION, 2)]);
    go(&mut realm, &mut view, DRAIN, None);
    let mut cold = |realm: &mut Realm, view: &mut View| {
        go(realm, view, BITING_COLD, Some(mob.guid));
        view.run(realm, 2.6)
    };
    let first = cold(&mut realm, &mut view);
    assert_eq!(refunded(&first, FRIGID_2), 0, "not yet slowed");
    let before = mana(&view, me);
    let second = cold(&mut realm, &mut view);
    assert_eq!(refunded(&second, FRIGID_2), 10, "half of 20");
    assert_eq!(mana(&view, me), before - 10);
}

#[test]
fn arcane_momentum_adds_45_percent_to_ley_spark_within_5_seconds_of_blink() {
    let hit = |realm: &mut Realm, view: &mut View, mob: &Mob| {
        (0..10)
            .find_map(|_| {
                go(realm, view, LEY_SPARK, Some(mob.guid))
                    .iter()
                    .find_map(|e| match e {
                        SessionEvent::SpellDamageLog(l)
                            if l.spell_id == LEY_SPARK && l.hit_info == 0 =>
                        {
                            Some(l.damage)
                        }
                        _ => None,
                    })
            })
            .expect("a Ley-Spark that does not crit")
    };
    let (mut realm, _, mob, mut view) = world(4, &[(ARCANE_MOMENTUM, 3)]);
    assert_eq!(hit(&mut realm, &mut view, &mob), 4);
    go(&mut realm, &mut view, BLINK, None);
    view.run(&mut realm, 0.1);
    assert_eq!(hit(&mut realm, &mut view, &mob), 6, "45% more, rounded");
    view.run(&mut realm, 5.0);
    assert_eq!(hit(&mut realm, &mut view, &mob), 4, "gone after 5 s");
}
