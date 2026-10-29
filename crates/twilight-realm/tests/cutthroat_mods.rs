//! The Cutthroat's Rupture. A test Calling's Rupture lasts longer for each combo point, played
//! through the realm's seam; its builder deals no damage, so the target outlives the bleed. The
//! shipped sheet's Rupture totals hold near the old ones.

mod common;

use benilla_protocol::messages::PeriodicTick;
use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

const SEED: u64 = 0x5EED_0111;

/// The test Calling's builder and Rupture, as `TEST_CALLINGS` numbers them.
const TEST_STRIKE: u32 = 1752;
const TEST_RUPTURE: u32 = 1943;

/// A Rogue-Class Calling whose builder always lands and deals nothing, with Rupture as the
/// shipped sheet defines it but with a tick every 5 s, so a few ticks tell its length and the
/// target outlives them.
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
key = "rupture"
spell_id = 1943
power_cost = 25
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 5.0]
effects = [{ kind = "periodic_damage", tick_ms = 5000, duration_ms = 6000, tick = { base = 4.0 }, per_combo = { base = 0.0 }, duration_per_combo = { base = 2000.0 } }]
"#;

/// A level 10 test Rogue at the scorpid camp, in the world at [`SEED`]: the realm, the first camp
/// mob in view and what the client has seen.
fn test_rogue() -> (Realm, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Garrote", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_rogue\"");
    let mut realm = Realm::with_seed(SEED).with_callings(TEST_CALLINGS);
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
    (realm, mob, view)
}

/// The damage of each tick of `spell_id`.
fn ticks(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == spell_id => Some(&l.ticks),
            _ => None,
        })
        .flatten()
        .filter_map(|t| match t {
            PeriodicTick::Damage { amount, .. } => Some(*amount),
            _ => None,
        })
        .collect()
}

/// Builds `points` on a camp mob, casts Rupture, and watches it for half a minute: each tick's
/// damage.
fn rupture(points: u32) -> Vec<u32> {
    let (mut realm, mob, mut view) = test_rogue();
    stand_at(&mut realm, &mob, 2.0);
    for _ in 0..points {
        realm.handle(cast(TEST_STRIKE, Some(mob.guid)));
    }
    realm.handle(cast(TEST_RUPTURE, Some(mob.guid)));
    let mut seen = view.drain(&mut realm);
    assert_eq!(refusal(&seen, TEST_RUPTURE), None);
    seen.extend(view.run(&mut realm, 30.0));
    ticks(&seen, TEST_RUPTURE)
}

#[test]
fn a_rupture_lasts_longer_for_each_combo_point() {
    let one = rupture(1);
    let five = rupture(5);
    // A tick every 5 s: 6 s + 2 s a point is one tick at 1 point and three at 5.
    assert_eq!((one.len(), five.len()), (1, 3));
}

#[test]
fn the_sheets_rupture_totals_hold_near_the_old_ones() {
    let root = env!("CARGO_MANIFEST_DIR");
    let path = format!("{root}/../../data/twilights-eve/server_spells.json");
    let text = std::fs::read_to_string(path).unwrap();
    let all: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mut params = serde_json::Value::Null;
    for calling in all.as_array().unwrap() {
        for skill in calling["skills"].as_array().unwrap() {
            if calling["key"] == "cutthroat" && skill["key"] == "rupture" {
                params = skill["effects"][0]["params"].clone();
            }
        }
    }
    let n = |v: &serde_json::Value| v.as_f64().unwrap().round();
    let tick_ms = n(&params["tick_ms"]);
    let base = n(&params["duration_ms"]);
    let stretch = n(&params["duration_per_combo"]["base"]);
    // At level 10, where the retune was balanced.
    let at_10 = |v: &serde_json::Value| n(&v["base"]) + 10.0 * v["per_level"].as_f64().unwrap();
    let tick = at_10(&params["tick"]);
    let more = params["per_combo"]["base"].as_f64().unwrap()
        + 10.0 * params["per_combo"]["per_level"].as_f64().unwrap();
    let total = |p: f64| ((base + stretch * p) / tick_ms).floor() * (tick + more * p);
    // 52 for one point and 140 for five, which the leech heals back 40% of.
    assert!(
        (48.0..=60.0).contains(&total(1.0)),
        "1 point: {}",
        total(1.0)
    );
    assert!(
        (130.0..=150.0).contains(&total(5.0)),
        "5 points: {}",
        total(5.0)
    );
    assert_eq!(params["leech_pct"].as_f64(), Some(40.0));
    assert!(total(1.0) < total(5.0));
}
