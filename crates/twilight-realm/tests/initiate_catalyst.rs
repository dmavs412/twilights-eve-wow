//! Catalyst consumes the Initiate's auras on a target: a Cold shatters for a multiple of Biting
//! Cold's hit, an Ember refunds mana. Thermal Shock raises both, Catalytic Chain spreads the
//! consumed aura to a second mob, and Catalytic Resonance raises spell damage after a consume.
//! Played through the realm's seam with a test Mage Calling whose numbers are easy to read.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::callings::{self, Effect};
use twilight_realm::rules::school::School;
use twilight_realm::{Realm, RealmCommand};

const MANA: u8 = 0;

/// `cold` a hit of 1 with a snare, `ember` a burn, `catalyst` the consume (a shatter of 200% of a
/// 20 hit, a 20% refund), `drain` spends 10 mana, `bolt` an arcane hit of 50. Thermal Shock,
/// Catalytic Chain and Catalytic Resonance (10% at every rank) on the talent tab.
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
key = "cold"
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
key = "ember"
spell_id = 133
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "periodic_damage", tick_ms = 3000, duration_ms = 12000, tick = { base = 2.0 }, max_stacks = 3 }]

[[calling.skill]]
key = "catalyst"
spell_id = 8921
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [
    { kind = "consume_aura", aura_spell_id = 116, duration_ms = 5000, amount = { base = 20.0 }, school = "frost", amount_pct = 200 },
    { kind = "consume_aura", aura_spell_id = 133, duration_ms = 12000, energize_pct_of_max = 20 },
]

[[calling.skill]]
key = "drain"
spell_id = 2912
power_cost = 10
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = []

[[calling.skill]]
key = "bolt"
spell_id = 2136
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "school_damage", school = "arcane", amount = { base = 50.0 } }]

[[calling.aura]]
key = "resonance"
spell_id = 12042
duration_ms = 8000
effects = [{ kind = "mod_spell_damage_pct", pct = 10 }]

[[calling.talent_tab]]
id = 906
name = "Test"
background = "MageArcane"

[[calling.talent_tab.talent]]
key = "thermal_shock"
talent_id = 27
tier = 1
column = 1
rank_spells = [11078, 11080, 12342]
spell_name = "Thermal Shock"
effects = [{ kind = "consume_amount_pct", skill = "catalyst", per_rank = [10, 20, 30] }]

[[calling.talent_tab.talent]]
key = "catalytic_chain"
talent_id = 23
tier = 1
column = 2
rank_spells = [11083, 12351]
spell_name = "Catalytic Chain"
effects = [{ kind = "proc", on = "consume", skill = "catalyst", spread = 10.0, chances = [50, 100], per_rank = [0, 0] }]

[[calling.talent_tab.talent]]
key = "catalytic_resonance"
talent_id = 80
tier = 1
column = 3
rank_spells = [11237, 12463, 12464, 16769, 16770]
spell_name = "Catalytic Resonance"
effects = [{ kind = "proc", on = "consume", skill = "catalyst", per_rank = [0, 0, 0, 0, 0], auras = [["resonance"], ["resonance"], ["resonance"], ["resonance"], ["resonance"]] }]
"#;

const COLD: u32 = 116;
const EMBER: u32 = 133;
const CATALYST: u32 = 8921;
const DRAIN: u32 = 2912;
const BOLT: u32 = 2136;

const THERMAL_SHOCK: u32 = 27;
const CATALYTIC_CHAIN: u32 = 23;
const CATALYTIC_RESONANCE: u32 = 80;

/// A level 10 test Mage with `talents` at the scorpid camp, at `seed`: the realm, their guid,
/// the camp's mobs and the view.
fn world(seed: u64, talents: &[(u32, u8)]) -> (Realm, u64, Vec<Mob>, View) {
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
    assert!(!mobs.is_empty(), "the camp is in view");
    (realm, guid, mobs, view)
}

/// Casts `spell_id` at `target` (or at the caster): the events, after asserting it went.
fn go(realm: &mut Realm, view: &mut View, spell_id: u32, target: Option<u64>) -> Vec<SessionEvent> {
    realm.handle(cast(spell_id, target));
    let events = view.drain(realm);
    assert_eq!(refusal(&events, spell_id), None, "spell {spell_id} is cast");
    events
}

/// The damage `spell_id` logged on `target`.
fn damage(events: &[SessionEvent], spell_id: u32, target: u64) -> Option<u32> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id && l.target == target => {
            Some(l.damage)
        }
        _ => None,
    })
}

/// The mana `spell_id` gave back.
fn refund(events: &[SessionEvent], spell_id: u32) -> Option<u32> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellEnergizeLog(l) if l.spell_id == spell_id => Some(l.amount),
        _ => None,
    })
}

/// Catalyst on a Cold, then on an Ember (mana spent first so the refund shows), with
/// `talents`: the shatter's damage and the refund, as a share of max mana in percent.
fn consume_both(talents: &[(u32, u8)]) -> (u32, f32) {
    let (mut realm, me, mobs, mut view) = world(7, talents);
    let mob = &mobs[0];
    stand_at(&mut realm, mob, 20.0);
    go(&mut realm, &mut view, COLD, Some(mob.guid));
    assert!(view.aura(mob.guid, COLD).is_some(), "the Cold is on");
    let shatter = go(&mut realm, &mut view, CATALYST, Some(mob.guid));
    assert!(view.aura(mob.guid, COLD).is_none(), "the Cold is consumed");
    let shatter = damage(&shatter, CATALYST, mob.guid).expect("a shatter");
    for _ in 0..20 {
        go(&mut realm, &mut view, DRAIN, None);
    }
    go(&mut realm, &mut view, EMBER, Some(mob.guid));
    let refunded = go(&mut realm, &mut view, CATALYST, Some(mob.guid));
    assert!(
        view.aura(mob.guid, EMBER).is_none(),
        "the Ember is consumed"
    );
    let max = view.fields(me).unit_max_power(MANA).unwrap();
    let back = refund(&refunded, CATALYST).expect("a refund");
    (shatter, back as f32 * 100.0 / max as f32)
}

#[test]
fn catalyst_shatters_a_cold_for_twice_its_hit_and_refunds_a_fifth_of_mana_for_an_ember() {
    let (shatter, refund) = consume_both(&[]);
    assert_eq!(shatter, 40, "200% of a 20 hit");
    assert!((refund - 20.0).abs() < 0.5, "{refund}% of max mana");
}

#[test]
fn thermal_shock_makes_the_consume_260_percent_and_26_percent() {
    let (shatter, refund) = consume_both(&[(THERMAL_SHOCK, 3)]);
    assert_eq!(shatter, 52, "260% of a 20 hit");
    assert!((refund - 26.0).abs() < 0.5, "{refund}% of max mana");
}

#[test]
fn catalyst_with_nothing_to_consume_does_nothing() {
    let (mut realm, me, mobs, mut view) = world(8, &[]);
    let mob = &mobs[0];
    stand_at(&mut realm, mob, 20.0);
    for _ in 0..5 {
        go(&mut realm, &mut view, DRAIN, None);
    }
    let mana = view.fields(me).unit_power(MANA);
    let events = go(&mut realm, &mut view, CATALYST, Some(mob.guid));
    assert_eq!(damage(&events, CATALYST, mob.guid), None);
    assert_eq!(refund(&events, CATALYST), None);
    assert_eq!(view.fields(me).unit_power(MANA), mana);
}

#[test]
fn catalytic_chain_spreads_the_consumed_aura_to_a_second_mob() {
    let (mut realm, _, mut mobs, mut view) = world(9, &[(CATALYTIC_CHAIN, 2)]);
    // Two mobs pulled onto the caster stand side by side.
    stand_at(&mut realm, &mobs[0], 20.0);
    mobs.truncate(2);
    for m in &mobs {
        go(&mut realm, &mut view, COLD, Some(m.guid));
    }
    for e in view.run(&mut realm, 8.0) {
        if let SessionEvent::MonsterMove { guid, path, .. } = e {
            if let (Some(m), Some(end)) = (mobs.iter_mut().find(|m| m.guid == guid), path.last()) {
                m.position = *end;
            }
        }
    }
    let (mob, other) = (&mobs[0], &mobs[1]);
    assert!(
        flat(mob.position, other.position) <= 10.0,
        "pulled together"
    );
    go(&mut realm, &mut view, EMBER, Some(mob.guid));
    go(&mut realm, &mut view, CATALYST, Some(mob.guid));
    assert!(
        view.aura(mob.guid, EMBER).is_none(),
        "consumed on the target"
    );
    let copy = view
        .aura(other.guid, EMBER)
        .expect("a copy on the second mob");
    assert_eq!(copy.stacks, 1);
}

#[test]
fn catalytic_resonance_raises_the_next_spell_by_ten_percent() {
    let bolt = |talents: &[(u32, u8)]| {
        let (mut realm, _, mobs, mut view) = world(10, talents);
        let mob = &mobs[0];
        stand_at(&mut realm, mob, 20.0);
        go(&mut realm, &mut view, EMBER, Some(mob.guid));
        go(&mut realm, &mut view, CATALYST, Some(mob.guid));
        // Several bolts, for one that does not crit.
        (0..10)
            .find_map(|_| {
                let events = go(&mut realm, &mut view, BOLT, Some(mob.guid));
                events.iter().find_map(|e| match e {
                    SessionEvent::SpellDamageLog(l) if l.spell_id == BOLT && l.hit_info == 0 => {
                        Some(l.damage)
                    }
                    _ => None,
                })
            })
            .expect("a bolt that does not crit")
    };
    assert_eq!(bolt(&[]), 50);
    assert_eq!(bolt(&[(CATALYTIC_RESONANCE, 5)]), 55);
}

#[test]
fn the_shipped_catalyst_shatters_for_twice_biting_colds_frost_hit() {
    let initiate = callings::calling("initiate").unwrap();
    let skill = |key: &str| initiate.skills.iter().find(|s| s.key == key).unwrap();
    let cold = skill("biting_cold")
        .effects
        .iter()
        .find_map(|e| match e {
            Effect::SchoolDamage { amount, .. } => Some(*amount),
            _ => None,
        })
        .unwrap();
    let consumes: Vec<&Effect> = skill("catalyst").effects.iter().collect();
    assert!(matches!(
        consumes[0],
        Effect::ConsumeAura { aura_spell_id, amount: Some(a), school: School::Frost, amount_pct: 200, .. }
            if *a == cold && *aura_spell_id == spell("initiate", "aura_biting_cold")
    ));
    assert!(matches!(
        consumes[1],
        Effect::ConsumeAura {
            aura_spell_id,
            amount: None,
            energize_pct_of_max: 10,
            ..
        } if *aura_spell_id == spell("initiate", "aura_searing_ember")
    ));
}
