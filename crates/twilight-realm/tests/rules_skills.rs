//! The Basic Warrior's skill numbers against `docs/reference/skills-and-talents.md` (its Tier 0
//! table's L1/5/10 column): each skill's rage cost, cooldown, and an effect that grows with level.

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
            Effect::EnergizeRage { amount } => Some(amount),
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
    let s = callings::skill(78).unwrap();
    assert_eq!((s.rage_cost, s.cooldown_ms, s.gcd_ms), (15, 0, 0));
    assert_eq!(at_levels(78, 0), [11, 17, 24]);
}

#[test]
fn charge_is_free_on_15_s_and_gives_9_9_10_rage() {
    let s = callings::skill(100).unwrap();
    assert_eq!((s.rage_cost, s.cooldown_ms), (0, 15_000));
    assert_eq!(s.range_yd, [8.0, 25.0]);
    assert!(s.requires_out_of_combat);
    assert_eq!(at_levels(100, 0), [9, 9, 10]);
}

#[test]
fn rend_costs_10_rage_and_ticks_harder_with_level_and_attack_power() {
    let s = callings::skill(772).unwrap();
    assert_eq!((s.rage_cost, s.cooldown_ms, s.gcd_ms), (10, 0, 1500));
    // A naked level-1 human's 29 attack power: 4 + 0.24 + 0.29.
    assert_eq!(amount(772).at(1, 29), 5);
    assert_eq!(at_levels(772, 0), [4, 5, 6]);
    assert_eq!(amount(772).at(10, 100), 7, "4 + 2.4 + 1");
}

#[test]
fn thunder_clap_costs_20_rage_on_4_s() {
    let s = callings::skill(6343).unwrap();
    assert_eq!((s.rage_cost, s.cooldown_ms, s.gcd_ms), (20, 4000, 1500));
    assert_eq!((s.radius_yd, s.max_targets), (8.0, 4));
    // 2.6 + 1.23 L + 0.05 AP.
    assert_eq!(amount(6343).at(1, 29), 5);
    assert_eq!(amount(6343).at(5, 47), 11);
    assert_eq!(amount(6343).at(10, 90), 19);
}

#[test]
fn battle_shout_costs_10_rage_and_gives_15_17_20_attack_power() {
    let s = callings::skill(6673).unwrap();
    assert_eq!((s.rage_cost, s.cooldown_ms, s.gcd_ms), (10, 0, 1500));
    assert_eq!(at_levels(6673, 0), [15, 17, 20]);
}

#[test]
fn hamstring_costs_10_rage_and_hits_for_2_4_8() {
    let s = callings::skill(1715).unwrap();
    assert_eq!((s.rage_cost, s.cooldown_ms, s.gcd_ms), (10, 0, 1500));
    // 0.7 + 0.54 L + 0.02 AP.
    assert_eq!(amount(1715).at(1, 29), 2);
    assert_eq!(amount(1715).at(5, 47), 4);
    assert_eq!(amount(1715).at(10, 90), 8);
}

#[test]
fn overpower_costs_5_rage_on_5_s_and_adds_3_5_7() {
    let s = callings::skill(7384).unwrap();
    assert_eq!((s.rage_cost, s.cooldown_ms, s.gcd_ms), (5, 5000, 1500));
    assert!(s.requires_target_dodged);
    assert_eq!(at_levels(7384, 0), [3, 5, 7]);
}

#[test]
fn bloodrage_costs_a_fifth_of_base_health_on_60_s_and_gives_10_11_12_rage() {
    let s = callings::skill(2687).unwrap();
    assert_eq!(
        (s.rage_cost, s.health_cost_pct_of_base_health, s.cooldown_ms),
        (0, 20, 60_000)
    );
    assert_eq!(at_levels(2687, 0), [10, 11, 12]);
}

/// `vanilla-combat-rules.md` §1.5's worked two-roll vector: a level-10 player's ability at a
/// level-12 mob, from the front: miss 600 + dodge 600 + parry 700, no glancing, no block.
#[test]
fn an_ability_rolls_avoidance_without_glancing_then_crit_on_its_own() {
    use twilight_realm::rules::melee::{player_vs_mob, MobDefense, PlayerOffense};
    use twilight_realm::rules::skills::ability_table;
    let p = PlayerOffense {
        level: 10,
        weapon_skill: 50,
        crit_pct: 5.0,
    };
    let m12 = MobDefense {
        level: 12,
        parries: true,
        blocks: true,
    };
    let (table, crit) = ability_table(p, m12, true, false);
    assert_eq!((table.miss, table.dodge, table.parry), (600, 600, 700));
    assert_eq!(
        (table.glancing, table.block, table.crit, table.crushing),
        (0, 0, 0, 0)
    );
    // The crit chance is the white table's: 5% less 0.2% per point of skill short, 10 points.
    assert_eq!(crit, 300);
    assert_eq!(crit, player_vs_mob(p, m12, true).crit);

    // Overpower cannot be dodged or parried: it only misses.
    let (table, _) = ability_table(p, m12, true, true);
    assert_eq!((table.miss, table.dodge, table.parry), (600, 0, 0));
}

#[test]
fn every_skill_is_stronger_at_level_10_than_at_level_1() {
    for id in [78, 100, 772, 6343, 6673, 1715, 7384, 2687] {
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
