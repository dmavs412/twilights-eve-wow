//! The Rot-Weaver: Creeping Blight grows a stack each tick and leeches half its ticks; Blood
//! Lance and Settle grow a worn Blight; Necrotic Detonation consumes the stacks for damage per
//! stack and heals around the target. Played through the realm's seam with a test Priest
//! Calling.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
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
"#;

const BLIGHT: u32 = 589;
const BLOOD_LANCE: u32 = 8092;
const BLEED: u32 = 2061;
const SETTLE: u32 = 15487;
const DETONATION: u32 = 15286;

/// A level 10 test Priest at `seed`, `yards` from the first camp mob, at half health so heals
/// show: the realm, the camp's mobs and the view.
fn world(seed: u64, yards: f32) -> (Realm, Vec<Mob>, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Vellune", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_priest\"");
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
    (realm, mobs, view)
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
    let (mut realm, mobs, mut view) = world(1, 20.0);
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
    let (mut realm, mobs, mut view) = world(5, 20.0);
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

    let (mut realm, mobs, mut view) = world(6, 20.0);
    go(&mut realm, &mut view, BLOOD_LANCE, mobs[0].guid);
    view.run(&mut realm, 1.6);
    assert!(
        view.aura(mobs[0].guid, BLIGHT).is_none(),
        "a lance puts no Blight on a mob without one"
    );
}

#[test]
fn settle_grows_a_worn_blight_and_puts_on_a_missing_one() {
    let (mut realm, mobs, mut view) = world(7, 20.0);
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
fn necrotic_detonation_at_five_stacks_deals_five_times_heals_and_clears_the_blight() {
    let detonate = |stacks: usize| {
        let (mut realm, mobs, mut view) = world(3, 8.0);
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
        (dealt, heals(&events, DETONATION))
    };
    let (one, _) = detonate(1);
    let (five, healed) = detonate(5);
    assert_eq!(five, one * 5, "five times one stack");
    assert_eq!(healed, [five * 150 / 100], "150% of it heals the caster");
}
