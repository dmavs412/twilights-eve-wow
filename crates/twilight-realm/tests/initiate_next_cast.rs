//! Timed casts and next-cast buffs: a skill with a cast bar goes off when the bar fills; Arcane
//! Substrate's riders shorten the next Biting Cold or lengthen the next Ember's burn, and using
//! one takes both; Ley-Line Attunement's third charge makes the next Biting Cold free. Played
//! through the realm's seam with a test Mage Calling whose skills hit for 1.

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
power_cost = 20
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
meta = { cast_time_ms = 2500 }
effects = [{ kind = "school_damage", school = "frost", amount = { base = 1.0 } }]

[[calling.skill]]
key = "searing_ember"
spell_id = 133
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "periodic_damage", tick_ms = 3000, duration_ms = 12000, tick = { base = 1.0 } }]

[[calling.aura]]
key = "substrate_cold"
spell_id = 12536
duration_ms = 15000
charges = 1
row = { exclusive_group = "substrate" }
effects = [{ kind = "next_cast", skill = "biting_cold", cast_time_ms = -500, family_bits = [5] }]

[[calling.aura]]
key = "substrate_ember"
spell_id = 11129
duration_ms = 15000
charges = 1
row = { exclusive_group = "substrate" }
effects = [{ kind = "next_cast", skill = "searing_ember", duration_ms = 5000, family_bits = [0] }]

[[calling.aura]]
key = "attunement"
spell_id = 18462
duration_ms = 30000
row = { max_stacks = 3, stack_rule = "stack" }
effects = [{ kind = "at_max_stacks", becomes = "attuned" }]

[[calling.aura]]
key = "attuned"
spell_id = 18463
duration_ms = 30000
charges = 1
effects = [{ kind = "next_cast", skill = ["biting_cold", "catalyst"], cost_pct = -100 }]

[[calling.talent_tab]]
id = 906
name = "Test"
background = "MageArcane"

[[calling.talent_tab.talent]]
key = "arcane_substrate"
talent_id = 30
tier = 1
column = 1
rank_spells = [11103, 12357, 12358, 12359, 12360]
spell_name = "Arcane Substrate"
effects = [{ kind = "proc", on = "cast", skill = "ley_spark", per_rank = [0, 0, 0, 0, 0], auras = [["substrate_cold", "substrate_ember"], ["substrate_cold", "substrate_ember"], ["substrate_cold", "substrate_ember"], ["substrate_cold", "substrate_ember"], ["substrate_cold", "substrate_ember"]] }]

[[calling.talent_tab.talent]]
key = "ley_line_attunement"
talent_id = 1142
tier = 1
column = 2
rank_spells = [18464, 18465, 18466]
spell_name = "Ley-Line Attunement"
effects = [{ kind = "proc", on = "cast", skill = "ley_spark", per_rank = [0, 0, 0], auras = [["attunement"], ["attunement"], ["attunement"]] }]
"#;

const LEY_SPARK: u32 = 2912;
const BITING_COLD: u32 = 116;
const SEARING_EMBER: u32 = 133;
const SUBSTRATE_COLD: u32 = 12536;
const SUBSTRATE_EMBER: u32 = 11129;
const ATTUNED: u32 = 18463;

const ARCANE_SUBSTRATE: u32 = 30;
const LEY_LINE_ATTUNEMENT: u32 = 1142;

/// A level 10 test Mage with `talents`, 20 yd from a camp mob: the realm, their guid, the mob
/// and the view.
fn world(talents: &[(u32, u8)]) -> (Realm, u64, Mob, View) {
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

/// Casts Ley-Spark at `mob` and waits for it to land.
fn ley_spark(realm: &mut Realm, view: &mut View, mob: &Mob) -> Vec<SessionEvent> {
    assert_eq!(start(realm, view, LEY_SPARK, mob), 1500);
    let events = view.run(realm, 1.6);
    assert!(landed(&events, LEY_SPARK), "Ley-Spark lands");
    events
}

#[test]
fn a_cast_bar_goes_off_when_it_fills_and_moving_interrupts_it() {
    let (mut realm, me, mob, mut view) = world(&[]);
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

#[test]
fn arcane_substrate_shortens_the_next_biting_cold_and_takes_the_ember_rider_with_it() {
    let (mut realm, me, mob, mut view) = world(&[(ARCANE_SUBSTRATE, 5)]);
    let sparked = ley_spark(&mut realm, &mut view, &mob);
    assert!(view.aura(me, SUBSTRATE_COLD).is_some() && view.aura(me, SUBSTRATE_EMBER).is_some());
    // The client's cast bar is told too.
    assert!(sparked.iter().any(|e| matches!(
        e,
        SessionEvent::SpellModifier {
            flat: true,
            mask_bit: 5,
            op: 10,
            value: -500
        }
    )));
    assert_eq!(start(&mut realm, &mut view, BITING_COLD, &mob), 2000);
    assert!(!landed(&view.run(&mut realm, 1.9), BITING_COLD));
    let done = view.run(&mut realm, 0.2);
    assert!(landed(&done, BITING_COLD), "lands after 2.0 s");
    assert!(
        view.aura(me, SUBSTRATE_COLD).is_none(),
        "the Cold rider is used"
    );
    assert!(
        view.aura(me, SUBSTRATE_EMBER).is_none(),
        "the Ember rider goes with it"
    );
    assert!(done.iter().any(|e| matches!(
        e,
        SessionEvent::SpellModifier {
            flat: true,
            mask_bit: 5,
            op: 10,
            value: 0
        }
    )));
    assert_eq!(
        start(&mut realm, &mut view, BITING_COLD, &mob),
        2500,
        "back to 2.5 s"
    );
}

#[test]
fn arcane_substrate_makes_the_next_embers_burn_last_17_seconds() {
    let (mut realm, me, mob, mut view) = world(&[(ARCANE_SUBSTRATE, 5)]);
    ley_spark(&mut realm, &mut view, &mob);
    realm.handle(cast(SEARING_EMBER, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(&mut realm), SEARING_EMBER), None);
    assert!(
        view.aura(me, SUBSTRATE_COLD).is_none(),
        "the Cold rider goes with it"
    );
    view.run(&mut realm, 16.5);
    assert!(
        view.aura(mob.guid, SEARING_EMBER).is_some(),
        "still burning at 16.5 s"
    );
    view.run(&mut realm, 1.0);
    assert!(
        view.aura(mob.guid, SEARING_EMBER).is_none(),
        "out by 17.5 s"
    );
}

#[test]
fn three_ley_sparks_with_attunement_make_the_next_biting_cold_free() {
    let (mut realm, me, mob, mut view) = world(&[(LEY_LINE_ATTUNEMENT, 1)]);
    for _ in 0..2 {
        ley_spark(&mut realm, &mut view, &mob);
    }
    assert!(
        view.aura(me, ATTUNED).is_none(),
        "two charges are not enough"
    );
    ley_spark(&mut realm, &mut view, &mob);
    assert!(view.aura(me, ATTUNED).is_some(), "the third makes it ready");

    let mana = view.fields(me).unit_power(MANA).unwrap();
    start(&mut realm, &mut view, BITING_COLD, &mob);
    assert!(landed(&view.run(&mut realm, 2.6), BITING_COLD));
    assert_eq!(view.fields(me).unit_power(MANA).unwrap(), mana, "free");
    assert!(view.aura(me, ATTUNED).is_none(), "used up");

    start(&mut realm, &mut view, BITING_COLD, &mob);
    assert!(landed(&view.run(&mut realm, 2.6), BITING_COLD));
    assert_eq!(
        view.fields(me).unit_power(MANA).unwrap(),
        mana - 20,
        "the next one costs again"
    );
}

#[test]
fn the_shipped_substrate_riders_share_a_group_and_name_their_skills() {
    let cold = callings::aura("aura_arcane_substrate_cold_5").expect("modelled");
    let ember = callings::aura("aura_arcane_substrate_ember_5").expect("modelled");
    assert_eq!(cold.row.exclusive_group, ember.row.exclusive_group);
    assert!(cold.row.exclusive_group.is_some());
    assert!(matches!(
        &cold.effects[0],
        AuraEffect::NextCast(n) if n.skill == ["biting_cold"] && n.cast_time_ms == -500 && !n.family_bits.is_empty()
    ));
    assert!(matches!(
        &ember.effects[0],
        AuraEffect::NextCast(n) if n.skill == ["searing_ember"] && n.duration_ms == 5000
    ));
    let ready = callings::aura("aura_ley_line_attunement_ready").expect("modelled");
    assert!(matches!(
        &ready.effects[0],
        AuraEffect::NextCast(n) if n.cost_pct == -100 && n.skill.len() == 2
    ));
}
