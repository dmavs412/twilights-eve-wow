//! The Cutthroat's Rupture. A test Calling's Rupture lasts longer for each combo point, played
//! through the realm's seam; its builder deals no damage, so the target outlives the bleed. The
//! shipped sheet's Rupture totals hold near the old ones.
//!
//! Three of Cutthroat's row choices (#505), played by the shipped Cutthroat at the sturdy mob
//! south of the Crossroads: Serrated Strikes' point more on a bleeding target, Open Wounds' half
//! again on one, and Kidney Shot's stun of a second per combo point.

mod common;

use benilla_protocol::messages::PeriodicTick;
use benilla_protocol::{EntityKind, SessionEvent};
use common::cutthroat::*;
use common::fight::*;
use common::vanguard::STURDY;
use common::*;
use twilight_realm::callings::{self, Effect};
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

// ── Cutthroat's rows (#505), on the shipped Cutthroat ───────────────────────────────────────────

static SINISTER_STRIKE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "sinister_strike"));
static RUPTURE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "rupture"));
static EVISCERATE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "eviscerate"));
static SERRATED_STRIKES: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| choice("cutthroat", "sinister_strike", "serrated_strikes"));
static OPEN_WOUNDS: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| choice("cutthroat", "eviscerate", "open_wounds"));
static KIDNEY_SHOT: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| choice("cutthroat", "eviscerate", "kidney_shot"));

/// Kidney Shot's stun aura, as the sheet names it.
static KIDNEY_STUN: std::sync::LazyLock<u32> = std::sync::LazyLock::new(|| {
    let cutthroat = callings::calling("cutthroat").unwrap();
    let row = cutthroat
        .rows
        .iter()
        .find(|r| r.key == "eviscerate")
        .unwrap();
    let kidney = row
        .choices
        .iter()
        .filter_map(|c| c.skill())
        .find(|s| s.spell_id == *KIDNEY_SHOT)
        .unwrap();
    kidney
        .effects
        .iter()
        .find_map(|e| match e {
            Effect::Stun { aura_spell_id, .. } => Some(*aura_spell_id),
            _ => None,
        })
        .expect("Kidney Shot stuns")
});

/// The Eviscerate row opens at 12. At 24 a Sinister Strike takes 84 of a mob's few hundred, so
/// five of them and a Kidney Shot kill the sturdy mob before the stun can show.
const EVISCERATE_ROW: u8 = 12;

/// Casts each of `spells` at the scene's mob in turn, each when the bar can pay and 1.1 s after the
/// last; `false` when one was refused.
fn cast_in_turn(s: &mut common::vanguard::Scene, spells: &[(u32, u32)]) -> bool {
    for &(spell_id, cost) in spells {
        wait_for_energy(&mut s.realm, &mut s.view, s.me, cost);
        if try_cast(&mut s.realm, &mut s.view, spell_id, s.mob.guid).is_none() {
            return false;
        }
        s.view.run(&mut s.realm, 1.1);
    }
    true
}

/// Serrated Strikes, Rupture and Serrated Strikes again at the sturdy mob, by a level 24
/// Cutthroat with fists on `seed`'s dice: the combo points the first strike left on the clean
/// mob and the last on the bleeding one, or `None` when a blow missed.
fn serrated_points(seed: u64) -> Option<(u8, u8)> {
    let rows = [("sinister_strike", *SERRATED_STRIKES)];
    let mut s = cutthroat_scene(ROWS_OPEN, true, seed, &rows, STURDY);
    let (me, mob) = (s.me, s.mob.guid);
    let mut points = Vec::new();
    for (spell_id, cost) in [
        (*SERRATED_STRIKES, 35),
        (*RUPTURE, 25),
        (*SERRATED_STRIKES, 35),
    ] {
        if !cast_in_turn(&mut s, &[(spell_id, cost)]) {
            return None;
        }
        let (held, on) = combo(&s.view, me);
        if spell_id == *RUPTURE {
            // A Rupture that landed spent the point and bleeds the mob.
            if held != 0 || s.view.aura(mob, *RUPTURE).is_none() {
                return None;
            }
        } else if held == 0 || on != mob {
            return None;
        } else {
            points.push(held);
        }
    }
    Some((points[0], points[1]))
}

#[test]
fn serrated_strikes_awards_two_points_on_a_ruptured_target_and_one_on_a_clean_one() {
    let mut checked = 0;
    for seed in 0..8 {
        if let Some(points) = serrated_points(seed) {
            assert_eq!(points, (1, 2), "seed {seed}");
            checked += 1;
        }
    }
    assert!(checked >= 3, "only {checked} seeds landed every blow");
}

/// Sinister Strike, Rupture when `bleed`, Sinister Strike, then `eviscerate` (the plain skill, or
/// a choice of `rows`) at the sturdy mob, by a level 24 Cutthroat with fists on `seed`'s dice:
/// Eviscerate's damage and whether it crit, or `None` when a blow missed.
fn eviscerate_hit(
    seed: u64,
    rows: &[(&str, u32)],
    eviscerate: u32,
    bleed: bool,
) -> Option<(u32, bool)> {
    let mut s = cutthroat_scene(ROWS_OPEN, true, seed, rows, STURDY);
    let (me, mob) = (s.me, s.mob.guid);
    let mut builders = vec![(*SINISTER_STRIKE, 35)];
    if bleed {
        builders.push((*RUPTURE, 25));
    }
    builders.push((*SINISTER_STRIKE, 35));
    if !cast_in_turn(&mut s, &builders) || combo(&s.view, me).0 != 1 {
        return None;
    }
    if bleed && s.view.aura(mob, *RUPTURE).is_none() {
        return None;
    }
    wait_for_energy(&mut s.realm, &mut s.view, me, 35);
    let events = try_cast(&mut s.realm, &mut s.view, eviscerate, mob)?;
    damage_of(&events, eviscerate)
        .into_iter()
        .find(|d| d.0 == mob)
        .map(|(_, damage, crit)| (damage, crit))
}

#[test]
fn open_wounds_deals_half_again_eviscerates_roll_to_a_bleeding_target() {
    let rows = [("eviscerate", *OPEN_WOUNDS)];
    let mut compared = 0;
    for seed in 0..6 {
        let plain = eviscerate_hit(seed, &[], *EVISCERATE, true);
        let open = eviscerate_hit(seed, &rows, *OPEN_WOUNDS, true);
        match (plain, open) {
            (Some(plain), Some(open)) => {
                assert_eq!(plain.1, open.1, "seed {seed}: one roll for both");
                // 1.5 times, give or take the rounding of the hit, its armour and its crit.
                assert!(
                    (open.0 * 2).abs_diff(plain.0 * 3) <= 6,
                    "seed {seed}: {open:?} against {plain:?}"
                );
                compared += 1;
            }
            (None, None) => {}
            other => panic!("seed {seed}: one landed and one missed: {other:?}"),
        }
        assert_eq!(
            eviscerate_hit(seed, &[], *EVISCERATE, false),
            eviscerate_hit(seed, &rows, *OPEN_WOUNDS, false),
            "seed {seed}: a clean target takes the same"
        );
    }
    assert!(compared >= 3, "only {compared} pairs landed");
}

/// `finisher` (the plain Eviscerate, or a choice of `rows`) at the sturdy mob after Sinister
/// Strikes put `points` on it, by a level 12 Cutthroat with fists on `seed`'s dice: how many
/// seconds Kidney Shot's stun stayed on the mob, or `None` when a blow missed or the mob died.
fn stun_after(seed: u64, points: u8, rows: &[(&str, u32)], finisher: u32) -> Option<f32> {
    let mut s = cutthroat_scene(EVISCERATE_ROW, true, seed, rows, STURDY);
    let (me, mob) = (s.me, s.mob.guid);
    for _ in 0..points {
        let before = combo(&s.view, me).0;
        if !cast_in_turn(&mut s, &[(*SINISTER_STRIKE, 35)]) || combo(&s.view, me).0 != before + 1 {
            return None;
        }
    }
    wait_for_energy(&mut s.realm, &mut s.view, me, 35);
    let events = try_cast(&mut s.realm, &mut s.view, finisher, mob)?;
    let landed = damage_of(&events, finisher).iter().any(|d| d.0 == mob);
    if !landed || s.view.health(mob) == 0 {
        return None;
    }
    let mut steps = 0;
    while s.view.aura(mob, *KIDNEY_STUN).is_some() && steps < 100 {
        s.view.run(&mut s.realm, 0.1);
        steps += 1;
    }
    Some(steps as f32 / 10.0)
}

#[test]
fn kidney_shot_stuns_the_target_a_second_per_combo_point() {
    let rows = [("eviscerate", *KIDNEY_SHOT)];
    for (points, secs) in [(1, 1.0), (5, 5.0)] {
        let lasted: Vec<f32> = (0..12)
            .filter_map(|seed| stun_after(seed, points, &rows, *KIDNEY_SHOT))
            .collect();
        assert!(
            !lasted.is_empty(),
            "no seed landed {points} points and the stun"
        );
        for l in lasted {
            assert!(
                (l - secs).abs() <= 0.15,
                "{points} points stunned for {l} s"
            );
        }
    }
    let plain: Vec<f32> = (0..4)
        .filter_map(|seed| stun_after(seed, 1, &[], *EVISCERATE))
        .collect();
    assert!(!plain.is_empty(), "no plain Eviscerate landed");
    assert!(
        plain.iter().all(|&l| l == 0.0),
        "Eviscerate stunned: {plain:?}"
    );
}
