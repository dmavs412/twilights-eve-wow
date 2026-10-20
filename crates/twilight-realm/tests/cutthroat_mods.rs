//! The Cutthroat's builder and finisher modifiers: Lethality (crit damage for builders only),
//! Bloodletting (Rupture lasts longer and hits harder), Flurry of Blades' Energy cut, and
//! Rupture's length per combo point. Played through the realm's seam; the crit tests search fixed
//! seeds for a crit, the Rupture tests use a test Calling whose builder deals no damage so the
//! target outlives the bleed.

mod common;

use benilla_protocol::messages::PeriodicTick;
use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

const ENERGY: u8 = 3;
/// Seeds tried for a crit, and the first of them.
const SEEDS: u64 = 400;
const FIRST_SEED: u64 = 0x5EED_0111;

static SINISTER_STRIKE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "sinister_strike"));
static EVISCERATE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "eviscerate"));
/// The test Calling's builder and Rupture, as `TEST_CALLINGS` numbers them.
const TEST_STRIKE: u32 = 1752;
const TEST_RUPTURE: u32 = 1943;

/// `Talent.dbc` ids of the talents under test.
const LETHALITY: u32 = 269;
const FLURRY_OF_BLADES: u32 = 1122;
const BLOODLETTING: u32 = 1702;

/// A Rogue-Class Calling whose builder always lands and deals nothing, with Rupture as the
/// shipped sheet defines it but with a tick every 5 s, so a few ticks tell its length and the
/// target outlives them. Bloodletting is the sheet's.
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

[[calling.talent_tab]]
id = 904
name = "Test"
background = "WarriorArms"

[[calling.talent_tab.talent]]
key = "bloodletting"
talent_id = 1702
tier = 1
column = 1
rank_spells = [30902, 30903, 30904, 30905, 30906]
spell_name = "Bloodletting"
effects = [
    { kind = "skill_amount_pct", skill = "rupture", per_rank = [10, 20, 30, 40, 50] },
    { kind = "skill_duration_ms", skill = "rupture", per_rank = [2000, 4000, 6000, 8000, 10000] },
]
"#;

/// A level 10 `calling` with `talents` in its save, in the world at `seed`: the realm, its guid,
/// the mobs in view and what the client has seen. With `callings`, the test Callings replace the
/// shipped ones and the character stands at the scorpid camp.
fn world(
    seed: u64,
    calling: &str,
    callings: Option<&str>,
    talents: &[(u32, u8)],
) -> (Realm, u64, Vec<Mob>, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Garrote", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace(
            "calling = \"basic_warrior\"",
            &format!("calling = \"{calling}\""),
        );
    if !talents.is_empty() {
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        blob = blob.replace(
            "money = 0\n",
            &format!("money = 0\ntalents = [{}]\n", rows.join(", ")),
        );
    }
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
            motion: Default::default(),
            position: [CAMP[0] + 40.0, CAMP[1], 0.0],
            orientation: std::f32::consts::PI,
        });
        seen = view.run(&mut realm, 1.0);
    }
    let mobs: Vec<Mob> = seen
        .iter()
        .filter_map(|e| match e {
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
        .collect();
    assert!(!mobs.is_empty(), "a mob in view");
    (realm, guid, mobs, view)
}

fn cutthroat(seed: u64, talents: &[(u32, u8)]) -> (Realm, u64, Mob, View) {
    let (realm, guid, mut mobs, view) = world(seed, "cutthroat", None, talents);
    (realm, guid, mobs.swap_remove(0), view)
}

/// Casts each of `spells` at `mob`, a gap after each, and returns what happened, or `None` when
/// a cast was refused (no point, no target).
fn cast_all(
    realm: &mut Realm,
    view: &mut View,
    mob: &Mob,
    spells: &[u32],
) -> Option<Vec<SessionEvent>> {
    stand_at(realm, mob, 2.0);
    let mut seen = Vec::new();
    for &spell_id in spells {
        realm.handle(cast(spell_id, Some(mob.guid)));
        let events = view.drain(realm);
        if refusal(&events, spell_id).is_some() {
            return None;
        }
        seen.extend(events);
        seen.extend(view.run(realm, 1.5));
    }
    Some(seen)
}

/// The direct damage `spell_id` did, and whether it crit.
fn hit(events: &[SessionEvent], spell_id: u32) -> Option<(u32, bool)> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id => {
            Some((l.damage, l.hit_info != 0))
        }
        _ => None,
    })
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

#[test]
fn lethality_raises_sinister_strike_crits_by_thirty_percent() {
    for seed in FIRST_SEED..FIRST_SEED + SEEDS {
        let (mut realm, _, mob, mut view) = cutthroat(seed, &[]);
        let Some(bare) = cast_all(&mut realm, &mut view, &mob, &[*SINISTER_STRIKE]) else {
            continue;
        };
        let Some((plain, true)) = hit(&bare, *SINISTER_STRIKE) else {
            continue;
        };
        let (mut realm, _, mob, mut view) = cutthroat(seed, &[(LETHALITY, 5)]);
        let trained = cast_all(&mut realm, &mut view, &mob, &[*SINISTER_STRIKE]).unwrap();
        let (more, crit) = hit(&trained, *SINISTER_STRIKE).unwrap();
        assert!(crit, "the same swing crits");
        // 200% of the hit becomes 260%.
        assert!(
            more.abs_diff(plain * 13 / 10) <= 1,
            "seed {seed:#x}: crit {plain} -> {more}"
        );
        return;
    }
    panic!("no Sinister Strike crit in {SEEDS} seeds");
}

#[test]
fn lethality_leaves_eviscerate_crits_as_they_were() {
    for seed in FIRST_SEED..FIRST_SEED + SEEDS {
        let (mut realm, _, mob, mut view) = cutthroat(seed, &[]);
        let spells = [*SINISTER_STRIKE, *EVISCERATE];
        let Some(bare) = cast_all(&mut realm, &mut view, &mob, &spells) else {
            continue;
        };
        // The builder's own crit would change the mob's health, not the roll after it.
        if matches!(hit(&bare, *SINISTER_STRIKE), Some((_, true))) {
            continue;
        }
        let Some((plain, true)) = hit(&bare, *EVISCERATE) else {
            continue;
        };
        let (mut realm, _, mob, mut view) = cutthroat(seed, &[(LETHALITY, 5)]);
        let trained = cast_all(&mut realm, &mut view, &mob, &spells).unwrap();
        assert_eq!(
            hit(&trained, *EVISCERATE),
            Some((plain, true)),
            "seed {seed:#x}: Eviscerate's crit"
        );
        return;
    }
    panic!("no Eviscerate crit in {SEEDS} seeds");
}

#[test]
fn flurry_of_blades_cuts_sinister_strikes_cost() {
    let cost = |talents: &[(u32, u8)]| {
        let (mut realm, me, mob, mut view) = cutthroat(FIRST_SEED, talents);
        stand_at(&mut realm, &mob, 2.0);
        realm.handle(cast(*SINISTER_STRIKE, Some(mob.guid)));
        let events = view.drain(&mut realm);
        assert_eq!(refusal(&events, *SINISTER_STRIKE), None);
        100 - view.fields(me).unit_power(ENERGY).unwrap()
    };
    assert_eq!(cost(&[]), 40);
    assert_eq!(cost(&[(FLURRY_OF_BLADES, 1)]), 38);
    assert_eq!(cost(&[(FLURRY_OF_BLADES, 3)]), 34);
}

/// Builds `points` on a camp mob, casts Rupture, and watches it for half a minute: each tick's
/// damage.
fn rupture(points: u32, talents: &[(u32, u8)]) -> Vec<u32> {
    let (mut realm, _, camp, mut view) =
        world(FIRST_SEED, "test_rogue", Some(TEST_CALLINGS), talents);
    let mob = &camp[0];
    stand_at(&mut realm, mob, 2.0);
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
    let one = rupture(1, &[]);
    let five = rupture(5, &[]);
    // A tick every 5 s: 6 s + 2 s a point is one tick at 1 point and three at 5.
    assert_eq!((one.len(), five.len()), (1, 3));
}

#[test]
fn bloodletting_lengthens_rupture_by_ten_seconds_and_hits_fifty_percent_harder() {
    let bare = rupture(5, &[]);
    let trained = rupture(5, &[(BLOODLETTING, 5)]);
    // Ten more seconds is two more ticks of a 5 s period.
    assert_eq!((bare.len(), trained.len()), (3, 5));
    assert_eq!(bare, [4, 4, 4]);
    assert_eq!(trained, [6, 6, 6, 6, 6]);
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
    let tick = n(&params["tick"]["base"]);
    let more = n(&params["per_combo"]["base"]);
    let total = |p: f64| ((base + stretch * p) / tick_ms).floor() * (tick + more * p);
    // Today's totals: 52 for one point and 146 for five.
    assert!(
        (48.0..=60.0).contains(&total(1.0)),
        "1 point: {}",
        total(1.0)
    );
    assert!(
        (136.0..=156.0).contains(&total(5.0)),
        "5 points: {}",
        total(5.0)
    );
    assert!(total(1.0) < total(5.0));
}
