//! The Initiate's active talents: Mana Surge refills half the bar, readies Blink and Catalyst,
//! and makes Ley-Spark instant for 10 s; Elemental Convergence puts an Ember and a Cold on the
//! target and makes the next Catalyst a critical strike. Played through the realm's seam with a
//! test Mage Calling.

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
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
meta = { cast_time_ms = 1500 }
effects = [{ kind = "school_damage", school = "arcane", amount = { base = 1.0 } }]

[[calling.skill]]
key = "biting_cold"
spell_id = 116
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [
    { kind = "school_damage", school = "frost", amount = { base = 1.0 } },
    { kind = "mod_run_speed", pct = -50, duration_ms = 5000 },
]

[[calling.skill]]
key = "blink"
spell_id = 1953
power_cost = 0
cooldown_ms = 15000
gcd_ms = 0
target = "self"
effects = [{ kind = "blink", distance_yd = 1.0 }]

[[calling.skill]]
key = "catalyst"
spell_id = 8921
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "consume_aura", aura_spell_id = 116, duration_ms = 5000, amount = { base = 20.0 }, school = "frost", amount_pct = 200 }]

[[calling.skill]]
key = "slow_catalyst"
spell_id = 133
power_cost = 0
cooldown_ms = 12000
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "school_damage", school = "arcane", amount = { base = 1.0 } }]

[[calling.skill]]
key = "drain"
spell_id = 5
power_cost = 50
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = []

[[calling.aura]]
key = "surge"
spell_id = 12043
duration_ms = 10000
effects = [{ kind = "next_cast", skill = "ley_spark", instant = true }]

[[calling.aura]]
key = "convergence"
spell_id = 11366
duration_ms = 6000
charges = 1
effects = [{ kind = "skill_crit_pct", skill = "catalyst", pct = 100 }]

[[calling.talent_tab]]
id = 906
name = "Test"
background = "MageArcane"

[[calling.talent_tab.talent]]
key = "mana_surge"
talent_id = 86
tier = 1
column = 1
rank_spells = [12043]
spell_name = "Mana Surge"
effects = []

[calling.talent_tab.talent.skill]
key = "mana_surge"
spell_id = 12043
power_cost = 0
cooldown_ms = 180000
gcd_ms = 0
target = "self"
effects = [
    { kind = "energize", power = "mana", amount = { base = 0.0 }, pct_of_max = 50 },
    { kind = "reset_cooldowns", spell_ids = [1953, 133] },
    { kind = "grant_aura", aura_spell_id = 12043 },
]

[[calling.talent_tab.talent]]
key = "elemental_convergence"
talent_id = 29
tier = 1
column = 2
rank_spells = [11366]
spell_name = "Elemental Convergence"
effects = []

[calling.talent_tab.talent.skill]
key = "elemental_convergence"
spell_id = 11366
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [
    { kind = "periodic_damage", tick_ms = 3000, duration_ms = 12000, tick = { base = 1.0 }, aura_spell_id = 2120 },
    { kind = "mod_run_speed", pct = -50, duration_ms = 5000, aura_spell_id = 116 },
    { kind = "grant_aura", aura_spell_id = 11366 },
]
"#;

const LEY_SPARK: u32 = 2912;
const BITING_COLD: u32 = 116;
const BLINK: u32 = 1953;
const CATALYST: u32 = 8921;
/// A second skill on a cooldown for Mana Surge to reset, standing in for Catalyst's.
const SLOW: u32 = 133;
const DRAIN: u32 = 5;
const MANA_SURGE: u32 = 12043;
const ELEMENTAL_CONVERGENCE: u32 = 11366;
/// The burn Elemental Convergence leaves, under its aura row's own spell id.
const EMBER: u32 = 2120;

/// A level 10 test Mage with both active talents, 20 yd from a camp mob, at `seed`: the realm,
/// their guid, the mob and the view.
fn world(seed: u64) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Weaver", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_mage\"")
        .replace("money = 0\n", "money = 0\ntalents = [[86, 1], [29, 1]]\n");
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
        orientation: 0.0,
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

/// Casts `spell_id`: the events, or the refusal's reason.
fn try_cast(
    realm: &mut Realm,
    view: &mut View,
    spell_id: u32,
    target: Option<u64>,
) -> Result<Vec<SessionEvent>, u8> {
    realm.handle(cast(spell_id, target));
    let events = view.drain(realm);
    match refusal(&events, spell_id) {
        Some(reason) => Err(reason),
        None => Ok(events),
    }
}

fn mana(view: &View, me: u64) -> u32 {
    view.fields(me).unit_power(MANA).unwrap()
}

#[test]
fn mana_surge_refills_half_the_bar_readies_blink_and_makes_ley_spark_instant() {
    let (mut realm, me, mob, mut view) = world(1);
    let max = view.fields(me).unit_max_power(MANA).unwrap();
    while mana(&view, me) > max / 2 {
        try_cast(&mut realm, &mut view, DRAIN, None).unwrap();
    }
    try_cast(&mut realm, &mut view, BLINK, None).unwrap();
    try_cast(&mut realm, &mut view, SLOW, Some(mob.guid)).unwrap();
    view.run(&mut realm, 0.2);
    assert!(
        try_cast(&mut realm, &mut view, BLINK, None).is_err(),
        "Blink cools down"
    );
    let before = mana(&view, me);

    let surge = try_cast(&mut realm, &mut view, MANA_SURGE, None).unwrap();
    assert_eq!(
        mana(&view, me),
        (before + max / 2).min(max),
        "half the bar back"
    );
    for id in [BLINK, SLOW] {
        assert!(
            surge.iter().any(
                |e| matches!(e, SessionEvent::ClearCooldown { spell_id, .. } if *spell_id == id)
            ),
            "the client is told {id} is ready"
        );
    }
    try_cast(&mut realm, &mut view, BLINK, None).expect("Blink is ready");
    try_cast(&mut realm, &mut view, SLOW, Some(mob.guid)).expect("the other skill is ready");
    view.run(&mut realm, 0.2);
    stand_at(&mut realm, &mob, 20.0);

    let spark = try_cast(&mut realm, &mut view, LEY_SPARK, Some(mob.guid)).unwrap();
    assert!(
        spark
            .iter()
            .any(|e| matches!(e, SessionEvent::SpellDamageLog(l) if l.spell_id == LEY_SPARK)),
        "instant: it lands at once"
    );
    view.run(&mut realm, 10.0);
    let later = try_cast(&mut realm, &mut view, LEY_SPARK, Some(mob.guid)).unwrap();
    assert!(
        later.iter().any(|e| matches!(
            e,
            SessionEvent::SpellStart {
                spell_id: LEY_SPARK,
                cast_time_ms: 1500,
                ..
            }
        )),
        "after 10 s it has its bar again"
    );
}

#[test]
fn elemental_convergence_burns_and_chills_and_makes_the_next_catalyst_crit() {
    let (mut realm, me, mob, mut view) = world(2);
    try_cast(&mut realm, &mut view, ELEMENTAL_CONVERGENCE, Some(mob.guid)).unwrap();
    assert!(view.aura(mob.guid, EMBER).is_some(), "the Ember is on");
    assert!(view.aura(mob.guid, BITING_COLD).is_some(), "the Cold is on");
    assert!(
        view.aura(me, ELEMENTAL_CONVERGENCE).is_some(),
        "the crit buff is up"
    );

    let shatter = |events: &[SessionEvent]| {
        events.iter().find_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == CATALYST => Some(l.hit_info != 0),
            _ => None,
        })
    };
    let first = try_cast(&mut realm, &mut view, CATALYST, Some(mob.guid)).unwrap();
    assert_eq!(shatter(&first), Some(true), "a critical shatter");
    assert!(
        view.aura(me, ELEMENTAL_CONVERGENCE).is_none(),
        "the buff is spent"
    );

    try_cast(&mut realm, &mut view, BITING_COLD, Some(mob.guid)).unwrap();
    let second = try_cast(&mut realm, &mut view, CATALYST, Some(mob.guid)).unwrap();
    assert_eq!(
        shatter(&second),
        Some(false),
        "the next shatter rolls as usual"
    );
}

#[test]
fn the_shipped_actives_name_the_real_auras_and_cooldowns() {
    use twilight_realm::callings::{self, Effect};
    let initiate = callings::calling("initiate").unwrap();
    let skill = |key: &str| {
        initiate
            .talent_tabs
            .iter()
            .flat_map(|t| &t.talents)
            .find(|t| t.key == key)
            .and_then(|t| t.skill.as_ref())
            .unwrap_or_else(|| panic!("{key} teaches a skill"))
    };
    let id = |key: &str| spell("initiate", key);
    let surge = &skill("mana_surge").effects;
    assert!(surge
        .iter()
        .any(|e| matches!(e, Effect::Energize { pct_of_max: 50, .. })));
    assert!(surge.iter().any(
        |e| matches!(e, Effect::ResetCooldowns { spell_ids } if spell_ids[..2] == [id("blink"), id("catalyst")])
    ));
    let convergence = &skill("elemental_convergence").effects;
    assert!(convergence.iter().any(|e| matches!(
        e,
        Effect::PeriodicDamage {
            aura_spell_id: Some(a),
            max_stacks: 3,
            ..
        } if *a == id("aura_searing_ember")
    )));
    assert!(convergence.iter().any(|e| matches!(
        e,
        Effect::ModRunSpeed {
            aura_spell_id: Some(a),
            ..
        } if *a == id("aura_biting_cold")
    )));
}
