//! The Initiate's spell mechanics, played through the realm's seam. Most tests use a test Mage
//! Calling whose skills isolate one mechanic: free, instant, tiny hits, so a camp mob outlives
//! many casts.

mod common;

use benilla_protocol::messages::PeriodicTick;
use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::callings::Effect;
use twilight_realm::{Realm, RealmCommand};

/// A Mage-Class Calling: `bolt` an arcane hit of 1 with Ley-Spark's mana-fill crit, `drain` a
/// self cast that only spends 10 mana, `ember` a burn of 2 every 3 s for 12 s stacking to 3,
/// `frost` a hit of 1 with a 50% snare for 5 s.
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
key = "bolt"
spell_id = 2912
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "school_damage", school = "arcane", amount = { base = 1.0 }, crit_pct_at_full_mana = 15.0 }]

[[calling.skill]]
key = "drain"
spell_id = 8921
power_cost = 10
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = []

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
key = "frost"
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
"#;

const BOLT: u32 = 2912;
const DRAIN: u32 = 8921;
const EMBER: u32 = 133;
const FROST: u32 = 116;

/// A level 10 `calling` in the world at `seed`, with `callings` replacing the shipped ones when
/// given (the character then stands at the scorpid camp): the realm, the nearest mob, the view.
fn world(seed: u64, calling: &str, callings: Option<&str>) -> (Realm, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Weaver", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace(
            "calling = \"basic_warrior\"",
            &format!("calling = \"{calling}\""),
        );
    let mut realm = Realm::with_seed(seed);
    if let Some(callings) = callings {
        realm = realm.with_callings(callings);
    }
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let mut seen = view.drain(&mut realm);
    if callings.is_some() {
        view.run(&mut realm, 1.0);
        realm.handle(RealmCommand::Move {
            position: [CAMP[0] + 40.0, CAMP[1], 0.0],
            orientation: std::f32::consts::PI,
        });
        seen = view.run(&mut realm, 1.0);
    }
    let mob = seen
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                ..
            } if callings.is_none() || flat(*position, CAMP) < 15.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .expect("a mob in view");
    (realm, mob, view)
}

/// How many of `casts` bolts crit, over `seeds` worlds, with the bar full or emptied first.
fn bolt_crits(seeds: u64, casts: u32, empty: bool) -> (u32, u32) {
    let (mut crits, mut landed) = (0, 0);
    for seed in 0..seeds {
        let (mut realm, mob, mut view) =
            world(0x1A17_0000 + seed, "test_mage", Some(TEST_CALLINGS));
        stand_at(&mut realm, &mob, 10.0);
        view.drain(&mut realm);
        if empty {
            loop {
                realm.handle(cast(DRAIN, None));
                if refusal(&view.drain(&mut realm), DRAIN).is_some() {
                    break;
                }
            }
        }
        for _ in 0..casts {
            realm.handle(cast(BOLT, Some(mob.guid)));
            for e in view.drain(&mut realm) {
                if let SessionEvent::SpellDamageLog(l) = e {
                    if l.spell_id == BOLT {
                        landed += 1;
                        crits += u32::from(l.hit_info != 0);
                    }
                }
            }
        }
    }
    (crits, landed)
}

#[test]
fn ley_spark_crits_about_15_points_more_often_at_full_mana_than_at_empty() {
    let (full_crits, full) = bolt_crits(50, 40, false);
    let (empty_crits, empty) = bolt_crits(50, 40, true);
    assert!(
        full > 1500 && empty > 1500,
        "most bolts land: {full}, {empty}"
    );
    let full_rate = f64::from(full_crits) / f64::from(full);
    let empty_rate = f64::from(empty_crits) / f64::from(empty);
    let gap = (full_rate - empty_rate) * 100.0;
    assert!(
        (10.0..20.0).contains(&gap),
        "full {full_rate:.3}, empty {empty_rate:.3}: a gap of {gap:.1} points"
    );
}

/// The burn ticks `spell_id` dealt on `mob` in `events`.
fn ticks(events: &[SessionEvent], mob: u64, spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.target == mob && l.spell_id == spell_id => {
                match l.ticks[0] {
                    PeriodicTick::Damage { amount, .. } => Some(amount),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect()
}

#[test]
fn three_embers_stack_to_three_and_tick_for_three_times_one() {
    let (mut realm, mob, mut view) = world(1, "test_mage", Some(TEST_CALLINGS));
    stand_at(&mut realm, &mob, 20.0);
    realm.handle(cast(EMBER, Some(mob.guid)));
    assert_eq!(
        refusal(&view.drain(&mut realm), EMBER),
        None,
        "the first ember"
    );
    let one = ticks(&view.run(&mut realm, 3.0), mob.guid, EMBER);
    assert_eq!(one, [2], "one stack ticks once");
    for _ in 0..2 {
        realm.handle(cast(EMBER, Some(mob.guid)));
        assert_eq!(refusal(&view.drain(&mut realm), EMBER), None);
    }
    assert_eq!(view.aura(mob.guid, EMBER).map(|a| a.stacks), Some(3));
    let three = ticks(&view.run(&mut realm, 3.0), mob.guid, EMBER);
    assert_eq!(three, [6], "three stacks tick for 3x");
}

#[test]
fn a_fourth_ember_keeps_three_stacks_and_starts_the_burn_over() {
    let (mut realm, mob, mut view) = world(2, "test_mage", Some(TEST_CALLINGS));
    stand_at(&mut realm, &mob, 20.0);
    for _ in 0..3 {
        realm.handle(cast(EMBER, Some(mob.guid)));
    }
    view.drain(&mut realm);
    view.run(&mut realm, 10.0);
    realm.handle(cast(EMBER, Some(mob.guid)));
    view.drain(&mut realm);
    assert_eq!(view.aura(mob.guid, EMBER).map(|a| a.stacks), Some(3));
    // 15 s after the first: the burn would be gone but for the fourth.
    let later = ticks(&view.run(&mut realm, 5.0), mob.guid, EMBER);
    assert!(
        later.iter().all(|&t| t == 6) && !later.is_empty(),
        "{later:?}"
    );
    assert!(view.aura(mob.guid, EMBER).is_some(), "still burning");
}

/// The yards a second `mob`'s chase moves cover in `events`, from their paths and durations.
fn chase_speed(events: &[SessionEvent], mob: u64) -> Option<f32> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::MonsterMove {
            guid,
            start,
            path,
            duration_ms,
            ..
        } if *guid == mob && *duration_ms > 0 => {
            let mut from = *start;
            let mut yards = 0.0;
            for p in path {
                yards += flat(from, *p);
                from = *p;
            }
            Some(yards / (*duration_ms as f32 / 1000.0))
        }
        _ => None,
    })
}

#[test]
fn biting_cold_halves_a_chasing_mobs_speed_for_five_seconds() {
    let (mut realm, mob, mut view) = world(3, "test_mage", Some(TEST_CALLINGS));
    stand_at(&mut realm, &mob, 25.0);
    realm.handle(cast(BOLT, Some(mob.guid)));
    let free = chase_speed(&view.run(&mut realm, 0.5), mob.guid).expect("the mob gives chase");
    let (mut realm, mob, mut view) = world(3, "test_mage", Some(TEST_CALLINGS));
    stand_at(&mut realm, &mob, 25.0);
    realm.handle(cast(FROST, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(&mut realm), FROST), None, "biting cold");
    let mut seen = view.run(&mut realm, 0.5);
    let slowed = chase_speed(&seen, mob.guid).expect("a snared chase");
    let ratio = slowed / free;
    assert!((0.45..0.55).contains(&ratio), "{slowed} against {free}");
    assert!(view.aura(mob.guid, FROST).is_some(), "the snare shows");
    seen.extend(view.run(&mut realm, 5.0));
    assert!(view.aura(mob.guid, FROST).is_none(), "the snare fades");
    let here = seen
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::MonsterMove { guid, path, .. } if *guid == mob.guid => path.last(),
            _ => None,
        })
        .copied()
        .expect("the mob moved");
    let mob = Mob {
        guid: mob.guid,
        position: here,
    };
    stand_at(&mut realm, &mob, 20.0);
    let after = chase_speed(&view.run(&mut realm, 1.0), mob.guid).expect("the chase goes on");
    assert!((after / free - 1.0).abs() < 0.05, "{after} against {free}");
}

#[test]
fn the_shipped_ember_stacks_to_three_and_biting_cold_snares_by_half() {
    let initiate = twilight_realm::callings::calling("initiate").unwrap();
    let skill = |key: &str| initiate.skills.iter().find(|s| s.key == key).unwrap();
    assert!(skill("searing_ember")
        .effects
        .iter()
        .any(|e| matches!(e, Effect::PeriodicDamage { max_stacks: 3, .. })));
    assert!(skill("biting_cold").effects.iter().any(|e| matches!(
        e,
        Effect::ModRunSpeed {
            pct: -50,
            duration_ms: 5000
        }
    )));
}
