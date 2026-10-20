//! The Initiate's damage modifiers: Combustion Engine raises Biting Cold against a target the
//! caster's Ember burns, and Volatile Embers makes the Ember cost more and burn harder while its
//! hit stays as it was. Played through the realm's seam with a test Mage Calling.

mod common;

use benilla_protocol::messages::PeriodicTick;
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
key = "biting_cold"
spell_id = 116
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "school_damage", school = "frost", amount = { base = 20.0 } }]

[[calling.skill]]
key = "searing_ember"
spell_id = 133
power_cost = 20
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [
    { kind = "school_damage", school = "fire", amount = { base = 10.0 } },
    { kind = "periodic_damage", tick_ms = 3000, duration_ms = 12000, tick = { base = 10.0 } },
]

[[calling.aura]]
key = "ember_burn"
spell_id = 133
duration_ms = 12000
effects = []

[[calling.talent_tab]]
id = 906
name = "Test"
background = "MageArcane"

[[calling.talent_tab.talent]]
key = "combustion_engine"
talent_id = 25
tier = 1
column = 1
rank_spells = [11095, 12872, 12873]
spell_name = "Combustion Engine"
effects = [{ kind = "skill_amount_pct", skill = "biting_cold", per_rank = [15, 30, 45], requires_target_aura = "ember_burn" }]

[[calling.talent_tab.talent]]
key = "volatile_embers"
talent_id = 76
tier = 1
column = 2
rank_spells = [11222, 12839, 12840, 12841, 12842]
spell_name = "Volatile Embers"
effects = [
    { kind = "skill_cost", skill = "searing_ember", per_rank = [10, 20, 30, 40, 50], flat = false },
    { kind = "skill_amount_pct", skill = "searing_ember", per_rank = [10, 20, 30, 40, 50], applies_to = "tick" },
]
"#;

const BITING_COLD: u32 = 116;
const SEARING_EMBER: u32 = 133;
const COMBUSTION_ENGINE: u32 = 25;
const VOLATILE_EMBERS: u32 = 76;

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
    let mut realm = Realm::with_seed(6).with_callings(TEST_CALLINGS);
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

/// The damage of the first of up to ten `spell_id` casts at `mob` that does not crit.
fn plain_hit(realm: &mut Realm, view: &mut View, spell_id: u32, mob: &Mob) -> u32 {
    (0..10)
        .find_map(|_| {
            realm.handle(cast(spell_id, Some(mob.guid)));
            view.drain(realm).iter().find_map(|e| match e {
                SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id && l.hit_info == 0 => {
                    Some(l.damage)
                }
                _ => None,
            })
        })
        .expect("a hit that does not crit")
}

/// The Ember's burn ticks in `events`.
fn ticks(events: &[SessionEvent]) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == SEARING_EMBER => match l.ticks[0] {
                PeriodicTick::Damage { amount, .. } => Some(amount),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[test]
fn combustion_engine_raises_biting_cold_by_45_percent_on_a_burning_target() {
    let (mut realm, _, mob, mut view) = world(&[(COMBUSTION_ENGINE, 3)]);
    assert_eq!(
        plain_hit(&mut realm, &mut view, BITING_COLD, &mob),
        20,
        "clean"
    );
    realm.handle(cast(SEARING_EMBER, Some(mob.guid)));
    view.drain(&mut realm);
    assert!(view.aura(mob.guid, SEARING_EMBER).is_some(), "burning");
    assert_eq!(
        plain_hit(&mut realm, &mut view, BITING_COLD, &mob),
        29,
        "45% more"
    );
}

#[test]
fn volatile_embers_costs_half_again_and_burns_half_again_with_the_same_hit() {
    let ember = |talents: &[(u32, u8)]| {
        let (mut realm, me, mob, mut view) = world(talents);
        let before = view.fields(me).unit_power(MANA).unwrap();
        let hit = plain_hit(&mut realm, &mut view, SEARING_EMBER, &mob);
        let cost = before - view.fields(me).unit_power(MANA).unwrap();
        let burn = ticks(&view.run(&mut realm, 3.1));
        (cost, hit, burn)
    };
    let (cost, hit, burn) = ember(&[]);
    assert_eq!((cost, hit, burn), (20, 10, vec![10]));
    let (cost, hit, burn) = ember(&[(VOLATILE_EMBERS, 5)]);
    assert_eq!(cost, 30, "50% more mana");
    assert_eq!(hit, 10, "the hit is unchanged");
    assert_eq!(burn, [15], "each tick 50% bigger");
}

/// Combustion Engine looks the burn up by its aura row; the shipped row must load.
#[test]
fn the_shipped_ember_aura_loads_for_combustion_engine_to_find() {
    let ember = twilight_realm::callings::aura("aura_searing_ember").expect("the aura row loads");
    assert_eq!(ember.spell_id, spell("initiate", "aura_searing_ember"));
}
