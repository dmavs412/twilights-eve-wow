//! A strike's `block_value_pct` (#504): it adds that percent of the caster's shield Block Value to
//! the strike, and nothing without a shield. Iron Juggernaut, Righteous Smash's choice, carries
//! 150 on its next-swing strike; the shipped half plays it against the plain skill on the same
//! dice. The exact share is played on a test Calling (`Realm::with_callings`) whose instant Holy
//! strikes skip armour and roll no weapon damage, so each hit is its bonus and its share alone.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::*;
use common::vanguard::*;
use common::*;
use twilight_realm::callings::{self, Effect};

/// `SpellDamageLog::hit_info` of a crit.
const CRIT: u32 = 0x2;
/// Righteous Smash's row opens at 4.
const LEVEL: u8 = 4;

static SMASH: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("vanguard", "righteous_smash"));
static JUGGERNAUT: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| choice("vanguard", "righteous_smash", "iron_juggernaut"));
static CHARGE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("vanguard", "bulwark_charge"));

/// The first strike of `smash` (Righteous Smash or Iron Juggernaut) on the camp's first mob, by a
/// level 4 Vanguard on `seed`'s dice who charged it for the rage: its damage and whether it
/// crit, or `None` when it missed.
fn first_smash(seed: u64, shield: bool, smash: u32) -> Option<(u32, bool)> {
    let rows: &[(&str, u32)] = if smash == *SMASH {
        &[]
    } else {
        &[("righteous_smash", smash)]
    };
    let mut s = scene(LEVEL, shield, seed, rows, CAMP);
    s.cast_at_mob(*CHARGE);
    s.realm.handle(cast(smash, Some(s.mob.guid)));
    assert_eq!(refusal(&s.view.drain(&mut s.realm), smash), None, "queued");
    s.realm
        .handle(twilight_realm::RealmCommand::AttackSwing { guid: s.mob.guid });
    let events = s.run_until(10.0, "the smash", |_, events| {
        events
            .iter()
            .any(|e| matches!(e, SessionEvent::SpellGo { spell_id, .. } if *spell_id == smash))
    });
    events.iter().find_map(|e| match e {
        SessionEvent::SpellDamageLog(l) if l.spell_id == smash => {
            Some((l.damage, l.hit_info & CRIT != 0))
        }
        _ => None,
    })
}

#[test]
fn iron_juggernaut_strikes_harder_than_righteous_smash_with_a_shield_and_the_same_without() {
    let mut compared = 0;
    for seed in 0..6 {
        let (plain, juggernaut) = (
            first_smash(seed, true, *SMASH),
            first_smash(seed, true, *JUGGERNAUT),
        );
        match (plain, juggernaut) {
            (Some(plain), Some(juggernaut)) => {
                assert_eq!(plain.1, juggernaut.1, "seed {seed}: one roll for both");
                assert!(
                    juggernaut.0 > plain.0,
                    "seed {seed}: {juggernaut:?} over {plain:?}"
                );
                compared += 1;
            }
            (None, None) => {}
            other => panic!("seed {seed}: one landed and one missed: {other:?}"),
        }
        assert_eq!(
            first_smash(seed, false, *SMASH),
            first_smash(seed, false, *JUGGERNAUT),
            "seed {seed}: no shield, no Block Value"
        );
    }
    assert!(compared >= 3, "{compared} landed pairs");
}

#[test]
fn iron_juggernaut_is_righteous_smash_with_150_percent_of_block_value() {
    let vanguard = callings::calling("vanguard").unwrap();
    let row = vanguard
        .rows
        .iter()
        .find(|r| r.key == "righteous_smash")
        .unwrap();
    let strike = |spell_id: u32| {
        let skill = row
            .choices
            .iter()
            .filter_map(|c| c.skill())
            .chain(vanguard.skills.iter())
            .find(|s| s.spell_id == spell_id)
            .unwrap();
        skill.effects.iter().find_map(|e| match e {
            Effect::WeaponStrike {
                block_value_pct,
                on_next_swing,
                ..
            } => Some((*block_value_pct, *on_next_swing)),
            _ => None,
        })
    };
    assert_eq!(strike(*JUGGERNAUT), Some((150, true)));
    assert_eq!(strike(*SMASH), Some((0, true)));
}

// ── The exact share, on a test Calling ──────────────────────────────────────────────────────────

const SEED: u64 = 0x5EED_0505;
const WHOLE: u32 = 1752;
const HALF_AGAIN: u32 = 2098;
const NONE: u32 = 78;

const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_juggernaut"
name = "Test Juggernaut"
tier = 0
class = 1
cap = 30
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth", "leather", "mail", "plate", "shield"]

[[calling.skill]]
key = "whole"
spell_id = 1752
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 5.0]
effects = [{ kind = "weapon_strike", weapon = "normal", bonus = { base = 1.0 }, weapon_pct = 0, school = "holy", block_value_pct = 100 }]

[[calling.skill]]
key = "half_again"
spell_id = 2098
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 5.0]
effects = [{ kind = "weapon_strike", weapon = "normal", bonus = { base = 1.0 }, weapon_pct = 0, school = "holy", block_value_pct = 150 }]

[[calling.skill]]
key = "none"
spell_id = 78
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 5.0]
effects = [{ kind = "weapon_strike", weapon = "normal", bonus = { base = 1.0 }, weapon_pct = 0, school = "holy" }]
"#;

/// The first plain (not crit) hit of each of the test Calling's three strikes on the camp's first
/// mob, by a level 20 caster with or without the starting shield: `[whole, half_again, none]`.
fn shares(shield: bool) -> [u32; 3] {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("level = 1\n", "level = 20\n")
        .replace(
            "calling = \"basic_warrior\"",
            "calling = \"test_juggernaut\"",
        );
    let blob = if shield {
        blob
    } else {
        drop_slot(&blob, OFF_HAND)
    };
    let (mut realm, mut view, _) = {
        let mut realm = Realm::with_seed(SEED).with_callings(TEST_CALLINGS);
        realm.import_character(&blob).unwrap();
        realm.handle(RealmCommand::EnterWorld { guid: me });
        realm.tick(0.0, &FlatGround);
        let mut view = View::default();
        view.drain(&mut realm);
        (realm, view, ())
    };
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
                kind: benilla_protocol::EntityKind::Unit,
                position,
                ..
            } if flat(*position, CAMP) < 1.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .expect("the camp's first mob streams in");
    stand_at(&mut realm, &mob, 2.0);
    let mut got = [None; 3];
    for _ in 0..20 {
        for (i, spell_id) in [WHOLE, HALF_AGAIN, NONE].into_iter().enumerate() {
            if got[i].is_some() {
                continue;
            }
            realm.handle(cast(spell_id, Some(mob.guid)));
            let events = view.drain(&mut realm);
            assert_eq!(refusal(&events, spell_id), None, "spell {spell_id}");
            got[i] = events.iter().find_map(|e| match e {
                SessionEvent::SpellDamageLog(l)
                    if l.spell_id == spell_id && l.hit_info & CRIT == 0 =>
                {
                    Some(l.damage)
                }
                _ => None,
            });
        }
        if got.iter().all(Option::is_some) {
            return got.map(Option::unwrap);
        }
    }
    panic!("no plain hit of each: {got:?}");
}

#[test]
fn a_strike_adds_its_percent_of_the_shields_block_value() {
    let [whole, half_again, none] = shares(true);
    // The bonus alone, 1, and the Block Value on top.
    assert_eq!(none, 1);
    let block_value = whole - none;
    assert!(block_value > 0, "a shield has a Block Value");
    assert_eq!(half_again - none, block_value * 150 / 100);
}

#[test]
fn without_a_shield_no_strike_adds_anything() {
    assert_eq!(shares(false), [1, 1, 1]);
}
