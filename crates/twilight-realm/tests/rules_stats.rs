//! Rage and a warrior's derived stats against vmangos's values
//! (`docs/reference/vanilla-combat-rules.md` §4 and §5).

use twilight_realm::rules::rage;
use twilight_realm::rules::stats::WarriorStats;

const HUMAN: u8 = 1;
const ORC: u8 = 2;
const TAUREN: u8 = 6;

#[test]
fn rage_from_damage_dealt_and_taken_is_stored_in_tenths() {
    // (level, damage, dealt, taken), §4's table.
    let cases = [
        (1, 10, 99, 33), // c(1) is a hair over 7.5, so 9.99… truncates
        (1, 50, 499, 166),
        (10, 10, 20, 6),
        (10, 50, 100, 33),
        (20, 100, 103, 34),
        (30, 50, 34, 11),
        (60, 100, 32, 10),
    ];
    for (level, damage, dealt, taken) in cases {
        assert_eq!(
            rage::from_damage_dealt(damage, level),
            dealt,
            "L{level} deals {damage}"
        );
        assert_eq!(
            rage::from_damage_taken(damage, level),
            taken,
            "L{level} takes {damage}"
        );
    }
}

#[test]
fn rage_decays_two_points_every_two_seconds_out_of_combat() {
    assert_eq!(rage::DECAY_PER_TICK, 20);
    assert_eq!(rage::DECAY_TICK_SECS, 2.0);
}

#[test]
fn a_naked_warriors_sheet_matches_vmangos() {
    // (race, level, attack power, crit %, max health, armour)
    let cases = [
        (HUMAN, 1, 29, 5.0, Some(60), Some(40)),
        (HUMAN, 10, 76, 4.9998, Some(231), Some(52)),
        (HUMAN, 20, 134, 4.4870, None, None),
        (HUMAN, 30, 194, 4.2328, None, None),
        (HUMAN, 60, 400, 4.0, Some(2609), Some(160)),
        (ORC, 1, 35, 4.25, None, Some(34)),
        (ORC, 10, 82, 23.0 / 5.20021, Some(251), None),
        (TAUREN, 10, 86, 4.0383, None, None),
    ];
    for (race, level, ap, crit, hp, armor) in cases {
        let s = WarriorStats::naked(race, level);
        assert_eq!(s.attack_power, ap, "race {race} L{level} AP");
        assert!(
            (s.crit_pct - crit).abs() < 1e-3,
            "race {race} L{level} crit {}",
            s.crit_pct
        );
        // A warrior's dodge divisor is its crit divisor.
        assert!(
            (s.dodge_pct - crit).abs() < 1e-3,
            "race {race} L{level} dodge"
        );
        if let Some(hp) = hp {
            assert_eq!(s.max_health, hp, "race {race} L{level} health");
        }
        if let Some(armor) = armor {
            assert_eq!(s.armor, armor, "race {race} L{level} armour");
        }
    }
}

#[test]
fn skills_sit_at_the_levels_maximum() {
    let s = WarriorStats::naked(HUMAN, 10);
    assert_eq!(s.weapon_skill, 50);
    assert_eq!(s.defense, 50);
}

#[test]
fn bare_fists_hit_for_one_to_two_plus_attack_power_over_a_two_second_swing() {
    let s = WarriorStats::naked(HUMAN, 1);
    assert_eq!(s.attack_time_ms, 2000);
    // AP 29: 29 / 14 × 2.0 = 4.143.
    assert!((s.min_damage - 5.143).abs() < 1e-3, "{}", s.min_damage);
    assert!((s.max_damage - 6.143).abs() < 1e-3, "{}", s.max_damage);
}

#[test]
fn a_mob_takes_its_stats_from_its_unit_class_and_level() {
    use twilight_realm::rules::stats::{MobStats, UnitClass};
    // §6: a level-10 normal warrior-class mob.
    let m = MobStats::new(UnitClass::Warrior, 10, 1.0, 1.0, 1.0);
    assert_eq!(m.max_health, 198);
    assert_eq!(m.armor, 492);
    assert!((m.min_damage - 13.11).abs() < 0.01, "{}", m.min_damage);
    assert!((m.max_damage - 17.37).abs() < 0.01, "{}", m.max_damage);
    // Template multipliers scale health, damage and armour (an elite Deviate Guardian).
    let elite = MobStats::new(UnitClass::Warrior, 18, 3.0, 1.7, 1.0);
    assert_eq!(elite.max_health, 1251);
    // A level-3 paladin-class mob, the Vile Familiar.
    let imp = MobStats::new(UnitClass::Paladin, 3, 1.0, 1.0, 1.0);
    assert_eq!((imp.max_health, imp.armor), (69, 16));
}

#[test]
fn the_tier_bonus_adds_to_the_base_stats_not_to_the_gear_bonus() {
    // Tier 1's bonus on a level 1 human (AP 29, 60 hp, 40 armour): +12 Str, +6 Agi, +12 Sta, +3 Spi.
    let bonus = [12, 6, 12, 0, 3];
    let naked = WarriorStats::naked(HUMAN, 1);
    let tier1 = WarriorStats::geared_at_tier(HUMAN, 1, bonus, &[]);
    assert_eq!(
        tier1.base,
        std::array::from_fn(|i| naked.base[i] + bonus[i])
    );
    assert_eq!(
        tier1.bonus, [0; 5],
        "the sheet's green numbers are gear's alone"
    );
    // 24 attack power from 12 Strength, 12 armour from 6 Agility, 120 health from 12 Stamina.
    assert_eq!(tier1.attack_power, 53);
    assert_eq!(tier1.armor, 52);
    assert_eq!(tier1.max_health, 180);
    assert!(tier1.crit_pct > naked.crit_pct);
    assert_eq!(WarriorStats::geared_at_tier(HUMAN, 1, [0; 5], &[]), naked);
}

#[test]
fn a_dodged_or_parried_swing_earns_the_rage_of_three_quarters_of_its_damage() {
    // L10 would have dealt 40: 30 dealt is 30 / c(10) × 7.5 = 6.01 rage.
    assert_eq!(rage::from_avoided(40, 10), 60);
    // L1 would have dealt 10: 7 dealt (rounded down) is a hair under 7 rage, c(1) a hair over 7.5.
    assert_eq!(rage::from_avoided(10, 1), 69);
}
