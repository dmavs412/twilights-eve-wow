//! The Acolyte's blood: Blood Lance and Dark Mending cost a share of max health, a cast that
//! would spend the last of it is refused, Dark Mending heals up to max and takes 15% of what it
//! healed back over 6 s, and Blood Lance's HoT lands on the friendly player with the least
//! health near the target (the caster, until other players exist). Played through the realm's
//! seam with the shipped Acolyte, and a test Calling for the refusal.

mod common;

use benilla_protocol::messages::PeriodicTick;
use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

const BLOOD_LANCE: u32 = 8092;
const DARK_MENDING: u32 = 2061;
const SPELL_FAILED_NO_POWER: u8 = 0x4d;

/// A Priest-Class Calling: `bleed` spends 96% of max health, `lance` 5%, both instant.
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
key = "bleed"
spell_id = 2061
power_cost = 96
cooldown_ms = 0
gcd_ms = 0
target = "self"
meta = { cost_kind = "health" }
effects = []

[[calling.skill]]
key = "lance"
spell_id = 8092
power_cost = 5
cooldown_ms = 0
gcd_ms = 0
target = "self"
meta = { cost_kind = "health" }
effects = []
"#;

/// A level 10 `calling`, 10 yd from a camp mob: the realm, their guid, the mob and the view.
fn world(calling: &str, callings: Option<&str>) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Vellune", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace(
            "calling = \"basic_warrior\"",
            &format!("calling = \"{calling}\""),
        );
    let mut realm = Realm::with_seed(3);
    if let Some(callings) = callings {
        realm = realm.with_callings(callings);
    }
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
    stand_at(&mut realm, &mob, 10.0);
    view.drain(&mut realm);
    (realm, guid, mob, view)
}

fn max_health(view: &View, me: u64) -> u32 {
    view.fields(me).unit_max_health().unwrap()
}

#[test]
fn blood_lance_costs_5_percent_of_max_health_and_its_hot_lands_on_the_caster() {
    let (mut realm, me, mob, mut view) = world("acolyte", None);
    let max = max_health(&view, me);
    assert_eq!(view.health(me), max);
    realm.handle(cast(BLOOD_LANCE, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(&mut realm), BLOOD_LANCE), None);
    let landed = view.run(&mut realm, 1.6);
    let hit = landed
        .iter()
        .find_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == BLOOD_LANCE => Some(l.damage),
            _ => None,
        })
        .expect("Blood Lance lands after its bar");
    assert_eq!(view.health(me), max - max * 5 / 100, "5% of max health");

    let hot: Vec<u32> = view
        .run(&mut realm, 6.1)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == BLOOD_LANCE => {
                assert_eq!(l.target, me, "the HoT is on the caster");
                match l.ticks[0] {
                    PeriodicTick::Heal { amount } => Some(amount),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        hot,
        vec![hit * 20 / 100 / 3; 3],
        "20% of the hit in three ticks"
    );
}

#[test]
fn a_health_cost_at_or_above_the_health_left_is_refused() {
    let (mut realm, me, _, mut view) = world("test_priest", Some(TEST_CALLINGS));
    let max = max_health(&view, me);
    realm.handle(cast(DARK_MENDING, None));
    view.drain(&mut realm);
    assert_eq!(view.health(me), max - max * 96 / 100, "4% left");
    realm.handle(cast(BLOOD_LANCE, None));
    assert_eq!(
        refusal(&view.drain(&mut realm), BLOOD_LANCE),
        Some(SPELL_FAILED_NO_POWER),
        "5% of max is more than is left"
    );
}

#[test]
fn dark_mending_heals_up_to_max_then_takes_15_percent_of_it_back() {
    let (mut realm, me, _, mut view) = world("acolyte", None);
    let max = max_health(&view, me);
    realm.handle(cast(DARK_MENDING, None));
    let mending = view.drain(&mut realm);
    assert_eq!(refusal(&mending, DARK_MENDING), None);
    let healed = mending
        .iter()
        .find_map(|e| match e {
            SessionEvent::SpellHealLog(l) if l.spell_id == DARK_MENDING => Some(l.amount),
            _ => None,
        })
        .expect("a heal");
    // It paid 10% of max health, so it can heal no more than that back.
    assert_eq!(healed, max * 10 / 100, "capped at max health");
    assert_eq!(view.health(me), max);
    let toll: u32 = view
        .run(&mut realm, 6.1)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == DARK_MENDING && l.target == me => {
                match l.ticks[0] {
                    PeriodicTick::Damage { amount, .. } => Some(amount),
                    _ => None,
                }
            }
            _ => None,
        })
        .sum();
    assert_eq!(toll, healed * 15 / 100 / 3 * 3, "15% of the heal over 6 s");
}
