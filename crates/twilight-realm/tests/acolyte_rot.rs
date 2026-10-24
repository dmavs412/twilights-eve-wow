//! The Rot-Weaver: Creeping Blight stacks to five and leeches half its ticks; Virulence speeds
//! the next Blood Lance off a tick; Epidemic spreads the Blight; Symbiotic Rot turns shadow
//! damage into healing; Festering Wounds marks what the Blight ticks on; Necrotic Detonation
//! consumes the stacks for damage per stack and heals around the target. Played through the
//! realm's seam with a test Priest Calling.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::callings::{self, AuraEffect};
use twilight_realm::rules::school::School;
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
key = "creeping_blight"
spell_id = 589
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "periodic_damage", tick_ms = 1000, duration_ms = 10000, tick = { base = 2.0 }, max_stacks = 255, stack_rule = "replace", stacks_per_tick = 1, leech_pct = 50, school = "shadow" }]

[[calling.skill]]
key = "blood_lance"
spell_id = 8092
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
meta = { cast_time_ms = 1500 }
effects = [{ kind = "school_damage", school = "shadow", amount = { base = 1.0 } }, { kind = "grow_aura", aura_spell_id = 589, stacks = 1 }]

[[calling.skill]]
key = "settle"
spell_id = 15487
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "periodic_damage", tick_ms = 1000, duration_ms = 10000, tick = { base = 2.0 }, max_stacks = 255, aura_spell_id = 589, stack_rule = "replace", stacks_per_tick = 1, leech_pct = 50, school = "shadow", if_worn = "grow" }]

[[calling.skill]]
key = "bleed"
spell_id = 2061
power_cost = 50
cooldown_ms = 0
gcd_ms = 0
target = "self"
meta = { cost_kind = "health" }
effects = []

[[calling.skill]]
key = "necrotic_detonation"
spell_id = 15286
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "consume_aura", aura_spell_id = 589, amount = { base = 10.0 }, school = "shadow", per_stack = true, heal_allies_pct = 150, radius_yd = 10.0 }]

[[calling.aura]]
key = "virulence"
spell_id = 15270
duration_ms = 15000
charges = 1
effects = [{ kind = "next_cast", skill = "blood_lance", cast_time_ms = -500 }]

[[calling.aura]]
key = "festering"
spell_id = 15311
duration_ms = 4000
effects = [{ kind = "mod_damage_taken_pct", pct = 10, school = "physical" }]

[[calling.talent_tab]]
id = 910
name = "Test"
background = "PriestShadow"

[[calling.talent_tab.talent]]
key = "virulence"
talent_id = 465
tier = 1
column = 1
rank_spells = [15270, 15335, 15336, 15337, 15338]
spell_name = "Virulence"
effects = [{ kind = "proc", on = "periodic_tick", skill = "creeping_blight", chances = [4, 8, 12, 16, 20], per_rank = [0, 0, 0, 0, 0], auras = [["virulence"], ["virulence"], ["virulence"], ["virulence"], ["virulence"]] }]

[[calling.talent_tab.talent]]
key = "epidemic"
talent_id = 466
tier = 1
column = 2
rank_spells = [15318, 15272, 15320]
spell_name = "Epidemic"
effects = [{ kind = "proc", on = "periodic_tick", skill = "creeping_blight", chances = [10, 20, 30], per_rank = [0, 0, 0], spread_aura = 10.0 }]

[[calling.talent_tab.talent]]
key = "symbiotic_rot"
talent_id = 1636
tier = 1
column = 3
rank_spells = [27811, 27815, 27816]
spell_name = "Symbiotic Rot"
effects = [{ kind = "proc", on = "damage_done", damage_school = "shadow", per_rank = [0, 0, 0], heal_allies_pct = [5, 10, 15], radius_yd = 20.0 }]

[[calling.talent_tab.talent]]
key = "festering_wounds"
talent_id = 483
tier = 1
column = 4
rank_spells = [15274, 15311]
spell_name = "Festering Wounds"
effects = [{ kind = "proc", on = "periodic_tick", damage_school = "shadow", per_rank = [0, 0], target_auras = [["festering"], ["festering"]] }]
"#;

const BLIGHT: u32 = 589;
const BLOOD_LANCE: u32 = 8092;
const BLEED: u32 = 2061;
const SETTLE: u32 = 15487;
const DETONATION: u32 = 15286;
const VIRULENCE_AURA: u32 = 15270;
const FESTERING_AURA: u32 = 15311;
/// Symbiotic Rot's rank 3 spell: its heals are logged under it.
const SYMBIOTIC_3: u32 = 27816;

const VIRULENCE: u32 = 465;
const EPIDEMIC: u32 = 466;
const SYMBIOTIC_ROT: u32 = 1636;
const FESTERING_WOUNDS: u32 = 483;

/// A level 10 test Priest with `talents` at `seed`, `yards` from the first camp mob, at half
/// health so heals show: the realm, their guid, the camp's mobs and the view.
fn world(seed: u64, talents: &[(u32, u8)], yards: f32) -> (Realm, u64, Vec<Mob>, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Vellune", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_priest\"");
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
        motion: Default::default(),
        position: [CAMP[0] + 40.0, CAMP[1], 0.0],
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
    stand_at(&mut realm, &mobs[0], yards);
    realm.handle(cast(BLEED, None));
    view.drain(&mut realm);
    (realm, guid, mobs, view)
}

/// Casts `spell_id` at `target`, asserting it went: the events.
fn go(realm: &mut Realm, view: &mut View, spell_id: u32, target: u64) -> Vec<SessionEvent> {
    realm.handle(cast(spell_id, Some(target)));
    let events = view.drain(realm);
    assert_eq!(refusal(&events, spell_id), None, "spell {spell_id} is cast");
    events
}

/// The heals logged under `spell_id` in `events`.
fn heals(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellHealLog(l) if l.spell_id == spell_id => Some(l.amount),
            _ => None,
        })
        .collect()
}

#[test]
fn creeping_blight_grows_each_tick_and_a_recast_starts_it_over() {
    let (mut realm, _, mobs, mut view) = world(1, &[], 20.0);
    let mob = &mobs[0];
    let dealt = |events: &[SessionEvent]| -> Vec<u32> {
        events
            .iter()
            .filter_map(|e| match e {
                SessionEvent::PeriodicAuraLog(l) if l.spell_id == BLIGHT => match l.ticks[0] {
                    benilla_protocol::messages::PeriodicTick::Damage { amount, .. } => Some(amount),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    };
    go(&mut realm, &mut view, BLIGHT, mob.guid);
    assert_eq!(view.aura(mob.guid, BLIGHT).map(|a| a.stacks), Some(1));
    let mut log = Vec::new();
    let mut healed = Vec::new();
    for _ in 0..3 {
        let tick = view.run(&mut realm, 1.0);
        log.extend(dealt(&tick));
        healed.extend(heals(&tick, BLIGHT));
    }
    assert_eq!(log, [2, 4, 6], "one more stack after each tick");
    assert_eq!(healed, [1, 2, 3], "half of each tick back");
    assert_eq!(view.aura(mob.guid, BLIGHT).map(|a| a.stacks), Some(4));
    go(&mut realm, &mut view, BLIGHT, mob.guid);
    assert_eq!(
        view.aura(mob.guid, BLIGHT).map(|a| a.stacks),
        Some(1),
        "a recast starts over"
    );
    assert_eq!(dealt(&view.run(&mut realm, 1.0)), [2]);
}

#[test]
fn blood_lance_adds_a_stack_and_keeps_the_timer() {
    let (mut realm, _, mobs, mut view) = world(5, &[], 20.0);
    let mob = &mobs[0];
    go(&mut realm, &mut view, BLIGHT, mob.guid);
    realm.handle(cast(BLOOD_LANCE, Some(mob.guid)));
    view.drain(&mut realm);
    // The first tick at 1 s leaves two stacks; the lance lands at 1.5 s.
    view.run(&mut realm, 1.55);
    assert_eq!(view.aura(mob.guid, BLIGHT).map(|a| a.stacks), Some(3));
    let tick = view.run(&mut realm, 0.5);
    let dealt: Vec<u32> = tick
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == BLIGHT => match l.ticks[0] {
                benilla_protocol::messages::PeriodicTick::Damage { amount, .. } => Some(amount),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(dealt, [6], "three stacks of 2");
    view.run(&mut realm, 7.85);
    assert!(view.aura(mob.guid, BLIGHT).is_some(), "9.9 s in");
    view.run(&mut realm, 0.15);
    assert!(
        view.aura(mob.guid, BLIGHT).is_none(),
        "gone 10.05 s after its own cast"
    );

    let (mut realm, _, mobs, mut view) = world(6, &[], 20.0);
    go(&mut realm, &mut view, BLOOD_LANCE, mobs[0].guid);
    view.run(&mut realm, 1.6);
    assert!(
        view.aura(mobs[0].guid, BLIGHT).is_none(),
        "a lance puts no Blight on a mob without one"
    );
}

#[test]
fn settle_grows_a_worn_blight_and_puts_on_a_missing_one() {
    let (mut realm, _, mobs, mut view) = world(7, &[], 20.0);
    let mob = &mobs[0];
    go(&mut realm, &mut view, SETTLE, mob.guid);
    assert_eq!(view.aura(mob.guid, BLIGHT).map(|a| a.stacks), Some(1));
    view.run(&mut realm, 2.05);
    assert_eq!(view.aura(mob.guid, BLIGHT).map(|a| a.stacks), Some(3));
    go(&mut realm, &mut view, SETTLE, mob.guid);
    assert_eq!(view.aura(mob.guid, BLIGHT).map(|a| a.stacks), Some(4));
    view.run(&mut realm, 7.8);
    assert!(view.aura(mob.guid, BLIGHT).is_some(), "9.85 s in");
    view.run(&mut realm, 0.2);
    assert!(
        view.aura(mob.guid, BLIGHT).is_none(),
        "fades 10.05 s after the first application"
    );
}

#[test]
fn virulence_fires_on_about_a_fifth_of_ticks_and_shortens_the_next_blood_lance() {
    let mut fired = 0;
    for seed in 0..200 {
        let (mut realm, me, mobs, mut view) = world(seed, &[(VIRULENCE, 5)], 20.0);
        go(&mut realm, &mut view, BLIGHT, mobs[0].guid);
        view.run(&mut realm, 1.05);
        if view.aura(me, VIRULENCE_AURA).is_none() {
            continue;
        }
        fired += 1;
        realm.handle(cast(BLOOD_LANCE, Some(mobs[0].guid)));
        let start = view.drain(&mut realm);
        assert!(start.iter().any(|e| matches!(
            e,
            SessionEvent::SpellStart {
                spell_id: BLOOD_LANCE,
                cast_time_ms: 1000,
                ..
            }
        )));
    }
    assert!((25..=55).contains(&fired), "{fired} of 200 first ticks");
}

#[test]
fn epidemic_spreads_the_blight_to_a_mob_that_lacks_it() {
    let (mut realm, _, mut mobs, mut view) = world(2, &[(EPIDEMIC, 3)], 20.0);
    mobs.truncate(2);
    // Pull both onto the caster, so they stand side by side.
    for m in &mobs {
        pull(&mut realm, &mut view, m);
    }
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [CAMP[0] + 40.0, CAMP[1], 0.0],
        orientation: std::f32::consts::PI,
    });
    view.run(&mut realm, 8.0);
    go(&mut realm, &mut view, BLIGHT, mobs[0].guid);
    let spread = (0..60).any(|_| {
        view.run(&mut realm, 0.5);
        view.aura(mobs[1].guid, BLIGHT).is_some()
    });
    assert!(spread, "the Blight reached the second mob");
    let source = view.aura(mobs[0].guid, BLIGHT).map(|a| a.stacks);
    assert_eq!(
        view.aura(mobs[1].guid, BLIGHT).map(|a| a.stacks),
        source,
        "the copy carries the source's stacks"
    );
    assert!(source > Some(1), "the test Blight grows each tick");
    assert!(
        view.aura(mobs[0].guid, BLIGHT).is_some(),
        "and stayed on the first"
    );
}

#[test]
fn necrotic_detonation_at_five_stacks_deals_five_times_heals_and_clears_the_blight() {
    let detonate = |stacks: usize| {
        let (mut realm, _, mobs, mut view) = world(3, &[(SYMBIOTIC_ROT, 3)], 8.0);
        let mob = &mobs[0];
        go(&mut realm, &mut view, BLIGHT, mob.guid);
        view.run(&mut realm, (stacks - 1) as f32 + 0.05);
        let events = go(&mut realm, &mut view, DETONATION, mob.guid);
        assert!(
            view.aura(mob.guid, BLIGHT).is_none(),
            "the Blight is consumed"
        );
        let dealt = events
            .iter()
            .find_map(|e| match e {
                SessionEvent::SpellDamageLog(l) if l.spell_id == DETONATION => Some(l.damage),
                _ => None,
            })
            .expect("the detonation's damage");
        (
            dealt,
            heals(&events, DETONATION),
            heals(&events, SYMBIOTIC_3),
        )
    };
    let (one, _, _) = detonate(1);
    let (five, healed, symbiotic) = detonate(5);
    assert_eq!(five, one * 5, "five times one stack");
    assert_eq!(healed, [five * 150 / 100], "150% of it heals the caster");
    assert_eq!(
        symbiotic,
        [five * 15 / 100],
        "Symbiotic Rot: 15% of the shadow damage"
    );
}

#[test]
fn festering_wounds_marks_what_the_blight_ticks_on() {
    let (mut realm, _, mobs, mut view) = world(4, &[(FESTERING_WOUNDS, 2)], 20.0);
    go(&mut realm, &mut view, BLIGHT, mobs[0].guid);
    assert!(view.aura(mobs[0].guid, FESTERING_AURA).is_none());
    view.run(&mut realm, 1.05);
    assert!(
        view.aura(mobs[0].guid, FESTERING_AURA).is_some(),
        "on with the first tick"
    );
    let shipped = callings::aura("aura_festering_wounds_2").expect("the shipped row loads");
    assert!(matches!(
        shipped.effects[..],
        [AuraEffect::ModDamageTakenPct {
            pct: 10,
            school: School::Physical
        }]
    ));
}
