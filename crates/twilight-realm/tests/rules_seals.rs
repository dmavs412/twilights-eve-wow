//! Damage schools and the Zealot's Seals, as pure rules (ADR 0002): holy damage skips armour,
//! Seal of Command's proc chance and damage, and Judgement of Command's holy hit by level and
//! attack power.

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
    let skill = callings::skill(20375).expect("Seal of Command");
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
    let skill = callings::skill(20375).unwrap();
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
