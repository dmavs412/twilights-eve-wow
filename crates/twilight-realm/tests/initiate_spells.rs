//! The Initiate's spell mechanics, played through the realm's seam. Most tests use a test Mage
//! Calling whose skills isolate one mechanic: free, instant, tiny hits, so a camp mob outlives
//! many casts.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

/// A Mage-Class Calling: `bolt` an arcane hit of 1 with Ley-Spark's mana-fill crit, `drain` a
/// self cast that only spends 10 mana.
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
spell_id = 133
power_cost = 10
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = []
"#;

const BOLT: u32 = 2912;
const DRAIN: u32 = 133;

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
