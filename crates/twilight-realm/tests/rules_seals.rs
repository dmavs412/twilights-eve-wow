//! Damage schools and the Zealot's Seals, as pure rules (ADR 0002): holy damage skips armour,
//! Seal of Command's proc chance and damage, and Judgement of Command's holy hit by level and
//! attack power.

mod common;

use common::spell;
use twilight_realm::callings::{self, Effect, Seal};
use twilight_realm::rules::school::{mitigated, School};

#[test]
fn holy_damage_ignores_armour() {
    assert_eq!(mitigated(School::Holy, 100, 0, 10), 100);
    assert_eq!(mitigated(School::Holy, 100, 5000, 10), 100);
    assert_eq!(mitigated(School::Holy, 0, 5000, 10), 0);
}

#[test]
fn physical_damage_takes_armour_off() {
    // 1000 / (1000 + 85·10 + 400) = 44.4% off.
    assert_eq!(mitigated(School::Physical, 100, 1000, 10), 55);
    assert_eq!(mitigated(School::Physical, 100, 0, 10), 100);
}

#[test]
fn schools_log_as_the_client_numbers_them() {
    assert_eq!(School::Physical.index(), 0);
    assert_eq!(School::Holy.index(), 1);
}

/// vmangos gives Seal of Command 7 procs a minute: a slow weapon procs more often a swing.
#[test]
fn seal_of_command_procs_seven_times_a_minute() {
    use twilight_realm::rules::seals::ppm_chance;
    assert_eq!(ppm_chance(7.0, 2000), 2333, "23.33% at 2.0 s");
    assert_eq!(ppm_chance(7.0, 3600), 4200, "42% at 3.6 s");
    assert_eq!(ppm_chance(7.0, 1500), 1750);
    assert_eq!(ppm_chance(0.0, 2000), 0);
}

/// `$20424s1`: 70% of a weapon roll, rounded.
#[test]
fn seal_of_command_deals_seventy_percent_of_weapon_damage() {
    use twilight_realm::rules::seals::weapon_pct_damage;
    assert_eq!(weapon_pct_damage(100.0, 70), 70);
    assert_eq!(weapon_pct_damage(57.0, 70), 40, "39.9");
    assert_eq!(weapon_pct_damage(43.5, 70), 30, "30.45");
    assert_eq!(weapon_pct_damage(0.0, 70), 0);
}

/// The Zealot's Seal of Command, as its content says.
fn seal_of_command() -> Seal {
    let skill = callings::skill(spell("zealot", "seal_of_command")).expect("Seal of Command");
    skill
        .effects
        .iter()
        .find_map(|e| match *e {
            Effect::Seal { seal, .. } => Some(seal),
            _ => None,
        })
        .expect("a Seal")
}

#[test]
fn seal_of_command_is_a_thirty_second_seal_of_seven_procs_a_minute_at_seventy_percent() {
    let skill = callings::skill(spell("zealot", "seal_of_command")).unwrap();
    assert!(skill.effects.contains(&Effect::Seal {
        duration_ms: 30_000,
        seal: seal_of_command()
    }));
    let Seal::Command {
        procs_per_minute,
        weapon_pct,
        proc_spell_id,
        judgement_spell_id,
        ..
    } = seal_of_command()
    else {
        panic!("Seal of Command's seal");
    };
    assert_eq!(procs_per_minute, 7.0);
    assert_eq!(weapon_pct, 70);
    assert_eq!(
        proc_spell_id, 20424,
        "Spell.dbc's Seal of Command proc, holy"
    );
    assert_eq!(
        judgement_spell_id, 20467,
        "Judgement of Command Rank 1, holy"
    );
}

/// Judgement of Command's holy hit: 30 + 3 a level + 0.15 an attack power point.
#[test]
fn judgement_of_command_grows_with_level_and_attack_power() {
    let Seal::Command { judgement, .. } = seal_of_command() else {
        panic!("Seal of Command's seal");
    };
    assert_eq!(judgement.at(1, 100), 48);
    assert_eq!(judgement.at(30, 400), 180);
    assert_eq!(judgement.at(50, 600), 270);
    assert_eq!(judgement.at(1, 0), 33);
}

/// Judgement of the Crusader's debuff: holy damage taken, raised by its percent and rounded.
#[test]
fn judgement_of_the_crusader_raises_holy_damage_taken() {
    use twilight_realm::rules::seals::holy_taken;
    assert_eq!(holy_taken(100, 20), 120);
    assert_eq!(holy_taken(33, 20), 40, "39.6");
    assert_eq!(holy_taken(7, 20), 8, "8.4");
    assert_eq!(holy_taken(0, 20), 0);
    assert_eq!(holy_taken(50, 0), 50);
}

/// The Zealot's Seal of the Crusader, as its content says.
fn seal_of_the_crusader() -> Seal {
    let skill =
        callings::skill(spell("zealot", "seal_of_the_crusader")).expect("Seal of the Crusader");
    assert!(skill.effects.iter().any(|e| matches!(
        e,
        Effect::Seal {
            duration_ms: 30_000,
            ..
        }
    )));
    skill
        .effects
        .iter()
        .find_map(|e| match *e {
            Effect::Seal { seal, .. } => Some(seal),
            _ => None,
        })
        .expect("a Seal")
}

/// 20 + 4 attack power a level (vanilla's Rank 3 gives 94 at 22, Rank 6 306 at 52); 40% faster
/// at every level, so every level's white swings are cut alike.
#[test]
fn seal_of_the_crusaders_attack_power_haste_and_cut_by_level() {
    use twilight_realm::rules::haste::{haste_cut_damage, hasted_attack_time};
    let Seal::Crusader {
        attack_power,
        haste_pct,
        ..
    } = seal_of_the_crusader()
    else {
        panic!("Seal of the Crusader's seal");
    };
    assert_eq!(attack_power.at(1, 0), 24);
    assert_eq!(attack_power.at(10, 0), 60);
    assert_eq!(attack_power.at(30, 0), 140);
    assert_eq!(attack_power.at(50, 0), 220);
    assert_eq!(attack_power.at(50, 1000), 220, "not from attack power");
    assert_eq!(haste_pct, 40);
    // No level in either: the same at every level.
    assert_eq!(hasted_attack_time(2000, haste_pct), 1429);
    assert_eq!(haste_cut_damage(140.0, haste_pct), 100.0);
}

/// Judged, Seal of the Crusader puts Judgement of the Crusader Rank 3 (20300) on the target for
/// its 10 s: 20% more holy damage taken.
#[test]
fn judgement_of_the_crusader_is_ten_seconds_of_twenty_percent_more_holy_damage() {
    let Seal::Crusader {
        judgement_spell_id,
        judgement_duration_ms,
        holy_taken_pct,
        ..
    } = seal_of_the_crusader()
    else {
        panic!("Seal of the Crusader's seal");
    };
    assert_eq!(judgement_spell_id, 20300);
    assert_eq!(judgement_duration_ms, 10_000);
    assert_eq!(holy_taken_pct, 20);
}
