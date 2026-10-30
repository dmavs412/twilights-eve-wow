//! The Initiate's Ley Ward: an instant self cast that absorbs damage equal to 10% of the caster's
//! maximum mana, with a 30 s cooldown. Played through the realm's seam with a test Mage Calling.

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
key = "ley_ward"
spell_id = 3000
power_cost = 0
cooldown_ms = 30000
gcd_ms = 0
target = "self"
effects = [{ kind = "absorb", pct = 10, pct_of = "max_mana", duration_ms = 20000, radius_yd = 0.0 }]

[[calling.skill]]
key = "toll"
spell_id = 3001
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "periodic_damage", tick_ms = 100, duration_ms = 100, tick = { base = 100.0 }, school = "shadow" }]
"#;

const LEY_WARD: u32 = 3000;
const TOLL: u32 = 3001;
/// The damage one toll deals.
const HIT: u32 = 100;

/// A level 10 test Mage in a camp: the realm, their guid and the view.
fn world() -> (Realm, u64, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Vellune", HUMAN, 0), CHAR_CREATE_SUCCESS);
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
    (realm, guid, view)
}

/// Casts `spell_id` at nothing, returning the events.
fn go(realm: &mut Realm, view: &mut View, spell_id: u32) -> Vec<SessionEvent> {
    realm.handle(cast(spell_id, None));
    view.drain(realm)
}

#[test]
fn ley_ward_absorbs_a_tenth_of_max_mana_then_drops_and_waits_out_its_cooldown() {
    let (mut realm, me, mut view) = world();
    // A tenth of the bar, rounded as the realm rounds a share.
    let ward = (view.fields(me).unit_max_power(MANA).unwrap() * 10 + 50) / 100;
    assert!(
        ward > 0 && ward < HIT,
        "a ward of {ward} against a hit of {HIT}"
    );

    let events = go(&mut realm, &mut view, LEY_WARD);
    assert_eq!(refusal(&events, LEY_WARD), None, "Ley Ward is cast");
    assert!(view.aura(me, LEY_WARD).is_some(), "the ward is up");

    // The first hit loses `ward` to it and takes the ward down.
    let before = view.health(me);
    go(&mut realm, &mut view, TOLL);
    view.run(&mut realm, 0.15);
    assert_eq!(view.health(me), before - (HIT - ward), "{ward} soaked");
    assert!(view.aura(me, LEY_WARD).is_none(), "the ward is spent");

    // The next hit lands whole.
    let before = view.health(me);
    go(&mut realm, &mut view, TOLL);
    view.run(&mut realm, 0.15);
    assert_eq!(view.health(me), before - HIT);

    // A second cast inside 30 s is refused.
    view.run(&mut realm, 10.0);
    let events = go(&mut realm, &mut view, LEY_WARD);
    assert!(refusal(&events, LEY_WARD).is_some(), "on cooldown");
}

// Ley-Spark's Warding Spark and Ley Ward's Skill Choices (#506), played by a shipped Initiate at
// 24 on the level 8 or 9 mob south of the Crossroads.

use common::cutthroat::{damage_of, energized};
use common::initiate::*;
use common::vanguard::{Scene, STURDY};

/// Seeds enough for a crit, a miss of the crit, and a burst that does not crit.
const SEEDS: u64 = 24;

/// The Initiate's scene with `rows` set, standing 6 yd from the mob, outside its notice.
fn close(seed: u64, rows: &[(&str, u32)]) -> Scene {
    let mut s = initiate_scene(seed, rows, STURDY);
    let mob = s.mob.clone();
    stand_at(&mut s.realm, &mob, 6.0);
    s.view.run(&mut s.realm, 0.5);
    s
}

/// The Initiate's scene with `rows` set, the mob pulled onto them with a swing.
fn fighting(seed: u64, rows: &[(&str, u32)]) -> Scene {
    let mut s = initiate_scene(seed, rows, STURDY);
    let mob = s.mob.clone();
    pull(&mut s.realm, &mut s.view, &mob);
    s
}

/// Casts the self-targeted `spell_id`: what it sent, after asserting it went off.
fn ward(s: &mut Scene, spell_id: u32) -> Vec<SessionEvent> {
    s.realm.handle(cast(spell_id, None));
    let events = s.view.drain(&mut s.realm);
    assert_eq!(refusal(&events, spell_id), None, "the ward goes up");
    assert!(s.view.aura(s.me, spell_id).is_some(), "the ward is worn");
    events
}

/// A Warding Spark that crits wards the Initiate for half of what it dealt: the ward soaks that
/// much of the mob's blows and breaks. One that does not crit puts up no ward.
#[test]
fn warding_sparks_crit_wards_for_half_its_damage_and_a_plain_hit_wards_nothing() {
    let spark = choice("initiate", "ley_spark", "warding_spark");
    let (mut crits, mut hits) = (0, 0);
    for seed in 0..SEEDS {
        let mut s = close(seed, &[("ley_spark", spark)]);
        let (me, mob) = (s.me, s.mob.guid);
        let events = cast_through(&mut s.realm, &mut s.view, spark, mob);
        let [(_, dealt, crit)] = damage_of(&events, spark)[..] else {
            continue;
        };
        if !crit {
            hits += 1;
            assert!(s.view.aura(me, spark).is_none(), "seed {seed}: no ward");
            continue;
        }
        crits += 1;
        let blows = blows(&mut s.realm, &mut s.view, me, 8.0, |v| {
            v.aura(me, spark).is_some()
        });
        let soaked: u32 = blows
            .iter()
            .filter(|b| b.watched)
            .map(|b| b.reported - b.lost)
            .sum();
        assert_eq!(soaked, (dealt as f32 / 2.0).round() as u32, "seed {seed}");
    }
    assert!(crits > 0 && hits > 0, "{crits} crits, {hits} hits");

    // Plain Ley-Spark wards nothing, crit or not.
    let plain = initiate("ley_spark");
    for seed in 0..SEEDS {
        let mut s = close(seed, &[]);
        cast_through(&mut s.realm, &mut s.view, plain, s.mob.guid);
        assert!(s.view.aura(s.me, plain).is_none(), "seed {seed}");
    }
}

/// Each blow Siphoning Ward soaks whole gives back half of it as mana, rounded down, while the
/// bar has room; plain Ley Ward gives none.
#[test]
fn siphoning_ward_gives_back_half_of_a_soaked_swing_as_mana() {
    let siphon = choice("initiate", "ley_ward", "siphoning_ward");
    let mut s = fighting(3, &[("ley_ward", siphon)]);
    let me = s.me;
    ward(&mut s, siphon);
    let blows = blows(&mut s.realm, &mut s.view, me, 10.0, |v| {
        v.aura(me, siphon).is_some() && mana(v, me) < max_mana(v, me)
    });
    let whole: Vec<&Blow> = blows.iter().filter(|b| b.watched && b.lost == 0).collect();
    assert!(!whole.is_empty(), "{blows:?}");
    for b in whole {
        assert_eq!(energized(&b.events, siphon), [b.reported / 2], "{b:?}");
    }

    let plain = initiate("ley_ward");
    let mut s = fighting(3, &[]);
    ward(&mut s, plain);
    let events = s.view.run(&mut s.realm, 10.0);
    assert!(energized(&events, plain).is_empty());
}

/// The mob whose blow Frost Ward soaks wears Frost Ward's slow; under plain Ley Ward it wears
/// none.
#[test]
fn frost_ward_slows_the_mob_whose_swing_it_soaks() {
    let frost = choice("initiate", "ley_ward", "frost_ward");
    let slow = initiate("aura_frost_ward");
    let mut s = fighting(3, &[("ley_ward", frost)]);
    let (me, mob) = (s.me, s.mob.guid);
    ward(&mut s, frost);
    assert!(s.view.aura(mob, slow).is_none(), "not before a soak");
    let blows = blows(&mut s.realm, &mut s.view, me, 6.0, |v| {
        v.aura(me, frost).is_some()
    });
    let soak = blows
        .iter()
        .find(|b| b.watched)
        .expect("the ward soaks a blow");
    assert!(
        soak.events
            .iter()
            .any(|e| matches!(e, SessionEvent::ObjectValues { guid, .. } if *guid == mob)),
        "the mob's auras change"
    );
    assert!(s.view.aura(mob, slow).is_some(), "the attacker is slowed");

    let plain = initiate("ley_ward");
    let mut s = fighting(3, &[]);
    ward(&mut s, plain);
    s.view.run(&mut s.realm, 6.0);
    assert!(s.view.aura(s.mob.guid, slow).is_none());
}

/// When the mob's blows break Volatile Ward, the mob beside the Initiate takes the ward's whole
/// amount as Arcane damage (more on a crit, nothing on a miss); plain Ley Ward bursts for nothing.
#[test]
fn volatile_ward_broken_damages_a_mob_within_8_yards() {
    let volatile = choice("initiate", "ley_ward", "volatile_ward");
    let mut plain_hits = 0;
    for seed in 0..SEEDS {
        let mut s = fighting(seed, &[("ley_ward", volatile)]);
        let (me, mob) = (s.me, s.mob.guid);
        ward(&mut s, volatile);
        let blows = blows(&mut s.realm, &mut s.view, me, 20.0, |v| {
            v.aura(me, volatile).is_some()
        });
        assert!(
            s.view.aura(me, volatile).is_none(),
            "seed {seed}: the ward broke"
        );
        let soaked: u32 = blows
            .iter()
            .filter(|b| b.watched)
            .map(|b| b.reported - b.lost)
            .sum();
        let bursts: Vec<(u64, u32, bool)> = blows
            .iter()
            .flat_map(|b| damage_of(&b.events, volatile))
            .collect();
        match bursts[..] {
            [] => {}
            [(target, dealt, crit)] => {
                assert_eq!(target, mob, "seed {seed}");
                if crit {
                    assert!(dealt > soaked, "seed {seed}: a crit deals more");
                } else {
                    assert_eq!(dealt, soaked, "seed {seed}: the ward's whole amount");
                    plain_hits += 1;
                }
            }
            ref more => panic!("seed {seed}: one burst, saw {more:?}"),
        }
    }
    assert!(plain_hits > 0, "a burst that did not crit");

    let plain = initiate("ley_ward");
    let mut s = fighting(3, &[]);
    ward(&mut s, plain);
    let events = s.view.run(&mut s.realm, 20.0);
    assert!(damage_of(&events, plain).is_empty());
}

/// A Volatile Ward that times out with a mob 7 yd away, idle, bursts for nothing.
#[test]
fn volatile_ward_timed_out_damages_nothing() {
    let volatile = choice("initiate", "ley_ward", "volatile_ward");
    let mut s = initiate_scene(3, &[("ley_ward", volatile)], STURDY);
    let mob = s.mob.clone();
    stand_at(&mut s.realm, &mob, 7.0);
    s.view.run(&mut s.realm, 0.5);
    ward(&mut s, volatile);
    let events = s.view.run(&mut s.realm, 21.0);
    assert!(s.view.aura(s.me, volatile).is_none(), "the ward ran out");
    assert!(damage_of(&events, volatile).is_empty(), "no burst");
    assert_eq!(
        s.view.health(mob.guid),
        s.view.fields(mob.guid).unit_max_health().unwrap()
    );
}
