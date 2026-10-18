//! The Hemomancer: Vitality raises stamina and then max health; Masochism heals back a share
//! of health spent; Coagulation shields a share of Dark Mending's heal; Crimson Surge speeds
//! casts after Blood Lance; Will to Survive raises healing below half health; Sanguine Shell
//! trades 30% of current health for a shield three times that. Played through the realm's seam
//! with a test Priest Calling, each talent at its top rank.

mod common;

use benilla_protocol::messages::PeriodicTick;
use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::callings::{self, TalentEffect};
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
key = "blood_lance"
spell_id = 8092
power_cost = 5
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
meta = { cast_time_ms = 1500, cost_kind = "health" }
effects = [{ kind = "school_damage", school = "shadow", amount = { base = 1.0 } }]

[[calling.skill]]
key = "dark_mending"
spell_id = 2061
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "heal", amount = { base = 40.0 } }]

[[calling.skill]]
key = "bleed"
spell_id = 5
power_cost = 60
cooldown_ms = 0
gcd_ms = 0
target = "self"
meta = { cost_kind = "health" }
effects = []

[[calling.skill]]
key = "sanguine_shell"
spell_id = 15237
power_cost = 30
cooldown_ms = 0
gcd_ms = 0
target = "self"
meta = { cost_kind = "health_current" }
effects = [{ kind = "absorb", pct = 300, pct_of = "health_cost", duration_ms = 15000, radius_yd = 30.0 }]

[[calling.skill]]
key = "toll"
spell_id = 589
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "periodic_damage", tick_ms = 100, duration_ms = 100, tick = { base = 10000.0 }, school = "shadow" }]

[[calling.aura]]
key = "masochism"
spell_id = 15356
duration_ms = 5000
effects = [{ kind = "periodic_heal", tick_ms = 1000, pct_of = "health_cost", pct = 50 }]

[[calling.aura]]
key = "surge"
spell_id = 15012
duration_ms = 6000
effects = [{ kind = "mod_cast_speed_pct", pct = 20 }]

[[calling.talent_tab]]
id = 911
name = "Test"
background = "PriestHoly"

[[calling.talent_tab.talent]]
key = "vitality"
talent_id = 401
tier = 1
column = 1
rank_spells = [14889, 15008, 15009, 15010, 15011]
spell_name = "Vitality"
effects = [
    { kind = "mod_stat", stat = "stamina", per_rank = [2, 4, 6, 8, 10] },
    { kind = "max_health_pct", per_rank = [2, 4, 6, 8, 10] },
]

[[calling.talent_tab.talent]]
key = "masochism"
talent_id = 404
tier = 1
column = 2
rank_spells = [14898, 15349, 15354, 15355, 15356]
spell_name = "Masochism"
effects = [{ kind = "proc", on = "cast", cost_kind = "health", per_rank = [0, 0, 0, 0, 0], auras = [["masochism"], ["masochism"], ["masochism"], ["masochism"], ["masochism"]] }]

[[calling.talent_tab.talent]]
key = "coagulation"
talent_id = 408
tier = 1
column = 3
rank_spells = [14912, 15013, 15014]
spell_name = "Coagulation"
effects = [{ kind = "proc", on = "cast", skill = "dark_mending", per_rank = [0, 0, 0], absorb_cost_pct = 30, absorb_of = "heal", absorb_school = "physical", absorb_duration_ms = 10000 }]

[[calling.talent_tab.talent]]
key = "crimson_surge"
talent_id = 410
tier = 1
column = 4
rank_spells = [14913, 15012]
spell_name = "Crimson Surge"
effects = [{ kind = "proc", on = "cast", skill = "blood_lance", per_rank = [0, 0], auras = [["surge"], ["surge"]] }]

[[calling.talent_tab.talent]]
key = "will_to_survive"
talent_id = 406
tier = 2
column = 1
rank_spells = [14908, 15020, 17191]
spell_name = "Will to Survive"
effects = [{ kind = "healing_taken_pct", per_rank = [10, 20, 30], below_health_pct = 50 }]
"#;

const BLOOD_LANCE: u32 = 8092;
const DARK_MENDING: u32 = 2061;
const BLEED: u32 = 5;
const SANGUINE_SHELL: u32 = 15237;
const TOLL: u32 = 589;
const MASOCHISM_AURA: u32 = 15356;
/// Coagulation's rank 3 spell: its shield is under it.
const COAGULATION_3: u32 = 15014;

const VITALITY: u32 = 401;
const MASOCHISM: u32 = 404;
const COAGULATION: u32 = 408;
const CRIMSON_SURGE: u32 = 410;
const WILL_TO_SURVIVE: u32 = 406;

/// A level 10 test Priest with `talents`, 20 yd from a camp mob: the realm, their guid, the mob
/// and the view.
fn world(talents: &[(u32, u8)]) -> (Realm, u64, Mob, View) {
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
    let mut realm = Realm::with_seed(5).with_callings(TEST_CALLINGS);
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

/// Casts `spell_id`, asserting it went: the events.
fn go(realm: &mut Realm, view: &mut View, spell_id: u32, target: Option<u64>) -> Vec<SessionEvent> {
    realm.handle(cast(spell_id, target));
    let events = view.drain(realm);
    assert_eq!(refusal(&events, spell_id), None, "spell {spell_id} is cast");
    events
}

fn max_health(view: &View, me: u64) -> u32 {
    view.fields(me).unit_max_health().unwrap()
}

#[test]
fn vitality_raises_stamina_then_max_health_by_ten_percent() {
    let (_, me, _, view) = world(&[]);
    let plain = max_health(&view, me);
    let (_, me, _, view) = world(&[(VITALITY, 5)]);
    let raised = max_health(&view, me);
    // Stamina's 10% adds health, then the whole is 10% more.
    assert!(raised * 100 >= plain * 110, "{raised} against {plain}");
    assert!(raised * 100 <= plain * 125, "{raised} against {plain}");
}

#[test]
fn masochism_heals_back_half_the_health_spent_over_five_seconds() {
    let (mut realm, me, mob, mut view) = world(&[(MASOCHISM, 5)]);
    go(&mut realm, &mut view, BLOOD_LANCE, Some(mob.guid));
    view.run(&mut realm, 1.6);
    assert!(
        view.aura(me, MASOCHISM_AURA).is_some(),
        "the heal over time is on"
    );
    let spent = max_health(&view, me) * 5 / 100;
    let ticks: Vec<u32> = view
        .run(&mut realm, 5.1)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == MASOCHISM_AURA => match l.ticks[0] {
                PeriodicTick::Heal { amount } => Some(amount),
                _ => None,
            },
            _ => None,
        })
        .collect();
    // Half the cost, rounded as the realm rounds a share, in five ticks.
    assert_eq!(
        ticks,
        vec![(spent * 50 + 50) / 100 / 5; 5],
        "half of {spent}"
    );
}

#[test]
fn coagulation_shields_30_percent_of_dark_mendings_heal() {
    let (mut realm, me, _, mut view) = world(&[(COAGULATION, 3)]);
    go(&mut realm, &mut view, BLEED, None);
    let mending = go(&mut realm, &mut view, DARK_MENDING, None);
    assert!(mending
        .iter()
        .any(|e| matches!(e, SessionEvent::SpellHealLog(l) if l.spell_id == DARK_MENDING && l.amount == 40)));
    assert!(
        view.aura(me, COAGULATION_3).is_some(),
        "a shield of 12 is up"
    );
}

#[test]
fn crimson_surge_makes_the_next_blood_lance_a_fifth_faster() {
    let (mut realm, _, mob, mut view) = world(&[(CRIMSON_SURGE, 2)]);
    let bar = |events: &[SessionEvent]| {
        events.iter().find_map(|e| match e {
            SessionEvent::SpellStart {
                spell_id: BLOOD_LANCE,
                cast_time_ms,
                ..
            } => Some(*cast_time_ms),
            _ => None,
        })
    };
    assert_eq!(
        bar(&go(&mut realm, &mut view, BLOOD_LANCE, Some(mob.guid))),
        Some(1500)
    );
    view.run(&mut realm, 1.6);
    assert_eq!(
        bar(&go(&mut realm, &mut view, BLOOD_LANCE, Some(mob.guid))),
        Some(1250),
        "1500 / 1.2"
    );
}

#[test]
fn will_to_survive_raises_healing_by_30_percent_below_half_health() {
    let (mut realm, _, _, mut view) = world(&[(WILL_TO_SURVIVE, 3)]);
    let healed = |events: &[SessionEvent]| {
        events.iter().find_map(|e| match e {
            SessionEvent::SpellHealLog(l) if l.spell_id == DARK_MENDING => Some(l.amount),
            _ => None,
        })
    };
    // At 40% of max health the heal is 30% larger.
    go(&mut realm, &mut view, BLEED, None);
    assert_eq!(
        healed(&go(&mut realm, &mut view, DARK_MENDING, None)),
        Some(52)
    );
}

#[test]
fn sanguine_shell_costs_30_percent_of_current_health_and_shields_three_times_that() {
    let (mut realm, me, _, mut view) = world(&[]);
    let before = view.health(me);
    go(&mut realm, &mut view, SANGUINE_SHELL, None);
    let cost = before * 30 / 100;
    let after = view.health(me);
    assert_eq!(after, before - cost, "30% of current health");
    assert!(view.aura(me, SANGUINE_SHELL).is_some(), "the shield is up");
    // A toll of everything but 1 health: the shield soaks 3x the cost of it.
    go(&mut realm, &mut view, TOLL, None);
    view.run(&mut realm, 0.15);
    let toll = after - 1;
    assert_eq!(
        view.health(me),
        after - toll.saturating_sub(3 * cost),
        "{} soaked",
        3 * cost
    );
}

#[test]
fn the_shipped_hemomancer_talents_carry_their_top_ranks() {
    let acolyte = callings::calling("acolyte").unwrap();
    let talent = |key: &str| {
        acolyte
            .talent_tabs
            .iter()
            .flat_map(|t| &t.talents)
            .find(|t| t.key == key)
            .unwrap_or_else(|| panic!("{key}"))
    };
    let vitality = &talent("vitality").effects;
    assert!(vitality
        .iter()
        .any(|e| matches!(e, TalentEffect::MaxHealthPct { per_rank } if per_rank[4] == 10)));
    assert!(talent("will_to_survive").effects.iter().any(|e| matches!(
        e,
        TalentEffect::HealingTakenPct { per_rank, below_health_pct: 50 } if per_rank[2] == 30
    )));
    for key in ["masochism", "coagulation", "crimson_surge"] {
        assert!(!talent(key).effects.is_empty(), "{key} plays");
    }
    assert!(
        talent("sanguine_shell").skill.is_some(),
        "Sanguine Shell is cast"
    );
}
