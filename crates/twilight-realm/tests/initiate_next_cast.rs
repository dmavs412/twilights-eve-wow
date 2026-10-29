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
