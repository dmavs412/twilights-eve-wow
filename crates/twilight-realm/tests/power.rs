//! Energy, mana and combo points played through the realm's seam, on test-defined Callings
//! (`Realm::with_callings`): no shipped Calling sits on a Rogue or mana Class yet.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::*;
use common::*;

const SEED: u64 = 0x5EED_0062;

/// `Powers`.
const MANA: u8 = 0;
const ENERGY: u8 = 3;

/// `SPELL_FAILED_NO_COMBO_POINTS`.
const NO_COMBO_POINTS: u8 = 0x46;

/// The test Rogue's builder, two finishers, and the test mage's bolt.
const BUILDER: u32 = 1752;
const FINISHER: u32 = 2098;
const BIG_FINISHER: u32 = 408;
const BOLT: u32 = 116;

/// A Rogue-Class Calling with a builder that always lands and a finisher whose damage grows a
/// combo point at a time, and a mana-Class Calling with one bolt.
const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_rogue"
name = "Test Rogue"
tier = 0
class = 4
cap = 10
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth", "leather"]

[[calling.skill]]
key = "builder"
spell_id = 1752
power_cost = 10
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 5.0]
effects = [{ kind = "add_combo_points", amount = 1 }]

[[calling.skill]]
key = "finisher"
spell_id = 2098
power_cost = 20
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 5.0]
effects = [{ kind = "school_damage", amount = { base = 5.0 }, per_combo = { base = 5.0 } }]

[[calling.skill]]
key = "big_finisher"
spell_id = 408
power_cost = 20
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 5.0]
effects = [{ kind = "school_damage", amount = { base = 1.0 }, per_combo = { base = 500.0 } }]

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
key = "bolt"
spell_id = 116
power_cost = 20
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "school_damage", amount = { base = 5.0 } }]
"#;

/// A level 1 human of `calling` in the world beside the scorpid camp south of the Crossroads,
/// the camp's mobs, and the view.
fn at_the_camp(calling: &str) -> (Realm, u64, Vec<Mob>, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker.export_character(guid).unwrap().replace(
        "calling = \"basic_warrior\"",
        &format!("calling = \"{calling}\""),
    );
    let mut realm = Realm::with_seed(SEED).with_callings(TEST_CALLINGS);
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
    let streamed = view.run(&mut realm, 1.0);
    let camp: Vec<Mob> = streamed
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: benilla_protocol::EntityKind::Unit,
                position,
                ..
            } if flat(*position, CAMP) < 15.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .collect();
    assert!(camp.len() >= 2, "the scorpid camp streams in");
    (realm, guid, camp, view)
}

fn spell_damage(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id => Some(l.damage),
            _ => None,
        })
        .collect()
}

fn combo(view: &View, me: u64) -> (u8, u64) {
    let f = view.fields(me);
    (
        f.player_combo_points().unwrap_or(0),
        f.player_combo_target(),
    )
}

// ── Energy ──────────────────────────────────────────────────────────────────────────────────────

#[test]
fn a_rogue_class_calling_enters_with_a_full_energy_bar() {
    let (_, me, _, view) = at_the_camp("test_rogue");
    let f = view.fields(me);
    assert_eq!(f.unit_power_type(), ENERGY);
    assert_eq!(f.unit_power(ENERGY), Some(100));
    assert_eq!(f.unit_max_power(ENERGY), Some(100));
    assert_eq!(
        f.unit_bytes_0().map(|b| (b >> 8) & 0xff),
        Some(4),
        "a Rogue"
    );
}

#[test]
fn energy_refills_twenty_a_tick_to_the_cap() {
    let (mut realm, me, camp, mut view) = at_the_camp("test_rogue");
    stand_at(&mut realm, &camp[0], 2.0);
    realm.handle(cast(BUILDER, Some(camp[0].guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, BUILDER), None);
    assert_eq!(view.fields(me).unit_power(ENERGY), Some(90));
    view.run(&mut realm, 2.1);
    assert_eq!(view.fields(me).unit_power(ENERGY), Some(100), "capped");
    for _ in 0..5 {
        realm.handle(cast(BUILDER, Some(camp[0].guid)));
    }
    view.drain(&mut realm);
    assert_eq!(view.fields(me).unit_power(ENERGY), Some(50));
    view.run(&mut realm, 2.1);
    assert_eq!(view.fields(me).unit_power(ENERGY), Some(70));
    view.run(&mut realm, 2.0);
    assert_eq!(view.fields(me).unit_power(ENERGY), Some(90));
}

// ── Combo points ────────────────────────────────────────────────────────────────────────────────

#[test]
fn a_builder_adds_combo_points_on_the_target_up_to_five() {
    let (mut realm, me, camp, mut view) = at_the_camp("test_rogue");
    let mob = &camp[0];
    stand_at(&mut realm, mob, 2.0);
    realm.handle(cast(BUILDER, Some(mob.guid)));
    view.drain(&mut realm);
    assert_eq!(combo(&view, me), (1, mob.guid));
    for _ in 0..5 {
        realm.handle(cast(BUILDER, Some(mob.guid)));
    }
    view.drain(&mut realm);
    assert_eq!(combo(&view, me), (5, mob.guid), "five at most");
}

#[test]
fn a_finisher_needs_points_and_spends_them_when_it_lands() {
    let (mut realm, me, camp, mut view) = at_the_camp("test_rogue");
    let mob = &camp[0];
    stand_at(&mut realm, mob, 2.0);
    realm.handle(cast(FINISHER, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, FINISHER), Some(NO_COMBO_POINTS));
    assert_eq!(
        view.fields(me).unit_power(ENERGY),
        Some(100),
        "nothing paid"
    );

    realm.handle(cast(BUILDER, Some(mob.guid)));
    realm.handle(cast(FINISHER, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, FINISHER), None);
    let one_point = spell_damage(&events, FINISHER);
    assert_eq!(one_point.len(), 1);
    assert_eq!(combo(&view, me), (0, 0), "spent");
    assert_eq!(view.fields(me).unit_power(ENERGY), Some(70));

    for _ in 0..3 {
        realm.handle(cast(BUILDER, Some(mob.guid)));
    }
    realm.handle(cast(FINISHER, Some(mob.guid)));
    let events = view.drain(&mut realm);
    let three_points = spell_damage(&events, FINISHER);
    assert_eq!(three_points.len(), 1);
    assert!(
        three_points[0] > one_point[0],
        "{three_points:?} > {one_point:?}: 5 + 5 a point, before armour"
    );
    assert_eq!(combo(&view, me), (0, 0));
}

#[test]
fn points_on_another_target_are_lost_when_a_new_target_is_built_on() {
    let (mut realm, me, camp, mut view) = at_the_camp("test_rogue");
    stand_at(&mut realm, &camp[0], 2.0);
    realm.handle(cast(BUILDER, Some(camp[0].guid)));
    realm.handle(cast(BUILDER, Some(camp[0].guid)));
    view.drain(&mut realm);
    assert_eq!(combo(&view, me), (2, camp[0].guid));
    stand_at(&mut realm, &camp[1], 2.0);
    realm.handle(cast(BUILDER, Some(camp[1].guid)));
    view.drain(&mut realm);
    assert_eq!(combo(&view, me), (1, camp[1].guid));
    realm.handle(cast(FINISHER, Some(camp[0].guid)));
    let events = view.drain(&mut realm);
    assert_eq!(
        refusal(&events, FINISHER),
        Some(NO_COMBO_POINTS),
        "no points on the old target"
    );
}

#[test]
fn a_dead_target_takes_its_points_with_it() {
    let (mut realm, me, camp, mut view) = at_the_camp("test_rogue");
    let mob = &camp[0];
    stand_at(&mut realm, mob, 2.0);
    for _ in 0..5 {
        realm.handle(cast(BUILDER, Some(mob.guid)));
    }
    view.drain(&mut realm);
    assert_eq!(combo(&view, me), (5, mob.guid));
    // 2501 before armour kills a level 3 scorpid; the kill, not the spend, is what is checked:
    // the fields read empty and the next build starts from nothing.
    realm.handle(cast(BIG_FINISHER, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, BIG_FINISHER), None);
    assert_eq!(view.health(mob.guid), 0, "dead");
    assert_eq!(combo(&view, me), (0, 0));
    stand_at(&mut realm, &camp[1], 2.0);
    realm.handle(cast(BUILDER, Some(camp[1].guid)));
    view.drain(&mut realm);
    assert_eq!(combo(&view, me), (1, camp[1].guid));
}

// ── Mana ────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn a_mana_class_calling_enters_with_a_full_bar_sized_by_class_level_and_intellect() {
    let (_, me, _, view) = at_the_camp("test_mage");
    let f = view.fields(me);
    assert_eq!(f.unit_power_type(), MANA);
    // A level 1 human mage: 100 base mana, 23 intellect (20 + 3 × 15).
    assert_eq!(f.unit_power(MANA), Some(165));
    assert_eq!(f.unit_max_power(MANA), Some(165));
    assert_eq!(f.unit_bytes_0().map(|b| (b >> 8) & 0xff), Some(8), "a Mage");
}

#[test]
fn spirit_regenerates_mana_except_for_five_seconds_after_a_cost() {
    let (mut realm, me, camp, mut view) = at_the_camp("test_mage");
    stand_at(&mut realm, &camp[0], 20.0);
    realm.handle(cast(BOLT, Some(camp[0].guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, BOLT), None);
    assert_eq!(view.fields(me).unit_power(MANA), Some(145));
    // The ticks at 2 s and 4 s fall inside the pause.
    view.run(&mut realm, 4.1);
    assert_eq!(view.fields(me).unit_power(MANA), Some(145), "paused");
    // The tick at 6 s: 22 spirit / 4 + 12.5 = 18.
    view.run(&mut realm, 2.0);
    assert_eq!(view.fields(me).unit_power(MANA), Some(163));
    view.run(&mut realm, 2.0);
    assert_eq!(view.fields(me).unit_power(MANA), Some(165), "capped");
}
