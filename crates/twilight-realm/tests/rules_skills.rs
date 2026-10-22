//! The Basic Warrior's skill numbers against `docs/reference/skills-and-talents.md` (its Tier 0
//! table's L1/5/10 column): each skill's rage cost, cooldown, and an effect that grows with level.

mod common;

use common::spell;
use twilight_realm::callings::{self, Effect};
use twilight_realm::rules::skills::Linear;

/// The skill's main amount: the strike bonus, the damage, the tick, the rage or the attack power.
fn amount(spell_id: u32) -> Linear {
    let skill = callings::skill(spell_id).expect("a Calling skill");
    skill
        .effects
        .iter()
        .find_map(|e| match *e {
            Effect::WeaponStrike { bonus, .. } => Some(bonus),
            Effect::SchoolDamage { amount, .. } => Some(amount),
            Effect::PeriodicDamage { tick, .. } => Some(tick),
            Effect::Energize { amount, .. } => Some(amount),
            Effect::AuraAttackPower { amount, .. } => Some(amount),
            _ => None,
        })
        .expect("an amount")
}

fn at_levels(spell_id: u32, attack_power: i32) -> [u32; 3] {
    let a = amount(spell_id);
    [1, 5, 10].map(|level| a.at(level, attack_power))
}

#[test]
fn amounts_round_half_away_from_zero_and_respect_their_floor() {
    let a = Linear {
        base: 2.0,
        per_level: 0.5,
        per_ap: 0.0,
        min: 0,
    };
    assert_eq!(a.at(1, 0), 3, "2.5 rounds up");
    assert_eq!(a.at(2, 0), 3);
    let floored = Linear {
        base: -5.0,
        per_level: 0.0,
        per_ap: 0.0,
        min: 1,
    };
    assert_eq!(floored.at(10, 0), 1);
    let with_ap = Linear {
        base: 4.0,
        per_level: 0.24,
        per_ap: 0.01,
        min: 1,
    };
    // 4 + 0.24 + 0.29 = 4.53.
    assert_eq!(with_ap.at(1, 29), 5);
}

#[test]
fn heroic_strike_costs_15_rage_and_adds_11_17_24() {
    let id = spell("basic_warrior", "heroic_strike");
    let s = callings::skill(id).unwrap();
    assert_eq!((s.power_cost, s.cooldown_ms, s.gcd_ms), (15, 0, 0));
    assert_eq!(at_levels(id, 0), [11, 17, 24]);
}

#[test]
fn charge_is_free_on_15_s_and_gives_9_9_10_rage() {
    let id = spell("basic_warrior", "charge");
    let s = callings::skill(id).unwrap();
    assert_eq!((s.power_cost, s.cooldown_ms), (0, 15_000));
    assert_eq!(s.range_yd, [8.0, 25.0]);
    assert!(s.requires_out_of_combat);
    assert_eq!(at_levels(id, 0), [9, 9, 10]);
}

/// An ability from the front at a mob that parries: miss 500 + dodge 500 + parry 500 (the flat
/// table), no block.
#[test]
fn an_ability_rolls_avoidance_then_crit_on_its_own() {
    use twilight_realm::rules::melee::{player_vs_mob, MobDefense, PlayerOffense};
    use twilight_realm::rules::skills::ability_table;
    let p = PlayerOffense { crit_pct: 5.0 };
    let mob = MobDefense {
        parries: true,
        blocks: true,
    };
    let (table, crit) = ability_table(p, mob, true, false);
    assert_eq!((table.miss, table.dodge, table.parry), (500, 500, 500));
    assert_eq!((table.block, table.crit), (0, 0));
    // The crit chance is the white table's.
    assert_eq!(crit, 500);
    assert_eq!(crit, player_vs_mob(p, mob, true).crit);

    // Overpower cannot be dodged or parried: it only misses.
    let (table, _) = ability_table(p, mob, true, true);
    assert_eq!((table.miss, table.dodge, table.parry), (500, 0, 0));
}

#[test]
fn every_skill_is_stronger_at_level_10_than_at_level_1() {
    for id in [
        spell("basic_warrior", "heroic_strike"),
        spell("basic_warrior", "charge"),
    ] {
        let a = amount(id);
        assert!(a.at(10, 50) > a.at(1, 50), "spell {id}");
    }
}

#[test]
fn a_normalized_strike_trades_the_weapons_speed_for_two_point_four() {
    use twilight_realm::rules::skills::normalized_roll;
    // 140 AP on a 2.0 s weapon added 20 to the roll; at 2.4 s it adds 24.
    assert_eq!(normalized_roll(50.0, 140, 2.0), 54.0);
    // On a slow 3.6 s weapon it added 36: normalizing takes 12 off.
    assert_eq!(normalized_roll(80.0, 140, 3.6), 68.0);
}
