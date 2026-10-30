//! Hammer of the Vanguard, Bulwark Charge's choice (#504): the charge ends in a strike on its
//! target for 150% of a weapon roll plus 10 + 1 a Power Level, in Holy, so armour takes nothing.
//! The strike is the choice's fourth effect, realm-only, and lands after the charge has moved
//! the caster. Played through the realm's seam by a level 16 Vanguard charging the first mob of
//! the scorpid camp south of the Crossroads from 25 yd.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::CAMP;
use common::vanguard::*;
use common::*;
use twilight_realm::callings;

/// Bulwark Charge's row opens at 16.
const LEVEL: u8 = 16;
/// The client's school index for Holy.
const HOLY: u8 = 1;
/// `SpellDamageLog::hit_info` of a crit.
const CRIT: u32 = 0x2;

static BULWARK_CHARGE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("vanguard", "bulwark_charge"));
static HAMMER: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| choice("vanguard", "bulwark_charge", "hammer_of_vanguard"));

/// The strikes `charge` logged under its own spell id when cast at the camp's first mob on
/// `seed`'s dice: (target, damage, crit, school), and the swing's white damage range.
fn charge(seed: u64, charge: u32) -> (Vec<(u64, u32, bool, u8)>, u64, [f32; 2]) {
    let rows: &[(&str, u32)] = if charge == *BULWARK_CHARGE {
        &[]
    } else {
        &[("bulwark_charge", charge)]
    };
    let mut s = scene(LEVEL, true, seed, rows, CAMP);
    let sheet = s.view.fields(s.me);
    let range = [
        sheet.unit_min_damage().unwrap(),
        sheet.unit_max_damage().unwrap(),
    ];
    let events = s.cast_at_mob(charge);
    let strikes = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == charge => {
                Some((l.target, l.damage, l.hit_info & CRIT != 0, l.school))
            }
            _ => None,
        })
        .collect();
    (strikes, s.mob.guid, range)
}

#[test]
fn the_hammers_charge_ends_in_a_holy_strike_of_half_again_a_weapon_roll_and_a_bonus() {
    let tier = callings::calling("vanguard").unwrap().tier;
    let bonus = 10.0 + f32::from(twilight_realm::rules::stats::power_level(tier, LEVEL));
    let mut plain = 0;
    for seed in 0..8 {
        let (strikes, mob, [min, max]) = charge(seed, *HAMMER);
        // A miss logs no damage; a hit logs one strike on the charge's target.
        assert!(strikes.len() <= 1, "seed {seed}: {strikes:?}");
        for (target, damage, crit, school) in strikes {
            assert_eq!(target, mob, "seed {seed}: the charge's target");
            assert_eq!(school, HOLY, "seed {seed}");
            if crit {
                continue;
            }
            // Holy skips armour, so a plain hit is the roll's 150% and the bonus, no less.
            let d = damage as f32;
            assert!(
                d >= (1.5 * min + bonus).floor() - 1.0 && d <= (1.5 * max + bonus).ceil() + 1.0,
                "seed {seed}: {d} outside 1.5 x {min}..{max} + {bonus}"
            );
            plain += 1;
        }
    }
    assert!(plain >= 3, "{plain} plain hits");
}

#[test]
fn bulwark_charge_strikes_nothing_of_its_own() {
    for seed in 0..3 {
        let (strikes, _, _) = charge(seed, *BULWARK_CHARGE);
        assert!(strikes.is_empty(), "seed {seed}: {strikes:?}");
    }
}
