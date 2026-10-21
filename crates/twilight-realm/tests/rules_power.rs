//! Power caps, max mana, regeneration and a mana-Class sheet against vmangos's values
//! (`docs/reference/vanilla-combat-rules.md` §5.5).

use twilight_realm::rules::power::{self, class, PowerKind};
use twilight_realm::rules::rage;
use twilight_realm::rules::stats::CharacterStats;

const HUMAN: u8 = 1;
const ORC: u8 = 2;
const TAUREN: u8 = 6;

/// A Calling's bar is its own `power` (callings.csv, ADR 0011), not its Class's; the shipped ones
/// keep the kind and table their old Class gave them.
#[test]
fn each_shipped_calling_runs_on_its_own_power_and_profile() {
    use twilight_realm::callings::{self, StatProfile};
    for (key, power, profile) in [
        ("basic_warrior", PowerKind::Rage, StatProfile::Warrior),
        ("vanguard", PowerKind::Rage, StatProfile::Warrior),
        ("zealot", PowerKind::Rage, StatProfile::Warrior),
        ("cutthroat", PowerKind::Energy, StatProfile::Rogue),
        ("initiate", PowerKind::Mana, StatProfile::Mage),
        ("ranger", PowerKind::Mana, StatProfile::Hunter),
        ("acolyte", PowerKind::Mana, StatProfile::Priest),
    ] {
        let c = callings::calling(key).unwrap();
        assert_eq!((c.power, c.stat_profile), (power, profile), "{key}");
        assert_eq!((c.health_pct, c.mana_pct), (100, 100), "{key}");
    }
    assert_eq!(StatProfile::Rogue.class(), class::ROGUE);
}

#[test]
fn focus_caps_at_a_hundred_and_fills_like_a_pets() {
    assert_eq!(power::cap(PowerKind::Focus, 0, 0), 100);
    assert_eq!(PowerKind::Focus.id(), 2, "the client's POWER_FOCUS");
    assert_eq!(PowerKind::Focus.stored(7), 7, "whole points");
    assert_eq!(power::FOCUS_PER_TICK, 10, "five a second");
}

#[test]
fn rage_and_energy_cap_at_a_hundred_and_mana_at_its_max() {
    assert_eq!(power::cap(PowerKind::Rage, 0, 0), rage::MAX);
    assert_eq!(rage::MAX, 1000, "rage is stored in tenths");
    assert_eq!(power::cap(PowerKind::Energy, 0, 0), 100);
    assert_eq!(power::cap(PowerKind::Mana, 2808, 0), 2808);
}

#[test]
fn the_first_twenty_intellect_give_one_mana_each_and_the_rest_fifteen() {
    for (int, mana) in [(0, 0), (20, 20), (21, 35), (23, 65), (125, 1595)] {
        assert_eq!(power::mana_from_intellect(int), mana, "int {int}");
    }
}

#[test]
fn max_mana_is_the_class_base_for_the_level_plus_intellect() {
    // (class, level, intellect, max mana), §5.5's vectors.
    let cases = [
        (class::MAGE, 1, 23, 165),
        (class::MAGE, 10, 34, 426),
        (class::MAGE, 60, 125, 2808),
        (class::PRIEST, 30, 61, 1322),
        (class::SHAMAN, 60, 87, 2545),
    ];
    for (class, level, int, want) in cases {
        assert_eq!(
            power::max_mana(class, level, int),
            want,
            "class {class} L{level}"
        );
    }
    assert_eq!(power::max_mana(class::WARRIOR, 60, 125), 0);
    assert_eq!(power::max_mana(class::ROGUE, 60, 125), 0);
}

#[test]
fn energy_regen_is_the_callings_own_per_two_second_tick() {
    use twilight_realm::callings;
    assert_eq!(callings::calling("cutthroat").unwrap().energy_regen(), 25);
    assert_eq!(
        callings::calling("cutthroat").unwrap().energy_per_tick,
        Some(25)
    );
    for key in [
        "basic_warrior",
        "vanguard",
        "zealot",
        "initiate",
        "ranger",
        "acolyte",
    ] {
        assert_eq!(
            callings::calling(key).unwrap().energy_per_tick,
            None,
            "{key}"
        );
    }
    assert_eq!(power::ENERGY_PER_TICK, 20, "the vanilla fallback");
    assert_eq!(power::REGEN_TICK_SECS, 2.0);
    assert_eq!(
        power::REGEN_TICK_SECS,
        rage::DECAY_TICK_SECS,
        "one timer for every kind"
    );
}

#[test]
fn mana_per_tick_comes_from_spirit_by_the_class_constants() {
    let cases = [
        (class::MAGE, 22, 18.0),
        (class::MAGE, 126, 44.0),
        (class::PRIEST, 67, 29.25),
        (class::SHAMAN, 103, 37.6),
        (class::PALADIN, 50, 25.0),
        (class::WARLOCK, 50, 25.0),
        (class::HUNTER, 50, 25.0),
        (class::DRUID, 50, 25.0),
        (class::WARRIOR, 50, 0.0),
        (class::ROGUE, 50, 0.0),
    ];
    for (class, spirit, want) in cases {
        let got = power::mana_per_tick(class, spirit);
        assert!(
            (got - want).abs() < 1e-9,
            "class {class} spirit {spirit}: {got}"
        );
    }
}

#[test]
fn spirit_regen_pauses_for_five_seconds_after_a_cast_spends_mana() {
    assert_eq!(power::MANA_PAUSE_SECS, 5.0);
}

#[test]
fn a_naked_mages_sheet_matches_vmangos() {
    // L1 human mage: base health 32 + sta 20; base mana 100 + int 23; AP str 20 - 10;
    // crit agi 20 / 11.1111; spirit 22 / 4 + 12.5.
    let s = CharacterStats::naked(class::MAGE, HUMAN, 1);
    assert_eq!(s.class, class::MAGE);
    assert_eq!(s.base, [20, 20, 20, 23, 22]);
    assert_eq!(s.max_health, 52);
    assert_eq!(s.max_mana, 165);
    assert_eq!(s.attack_power, 10);
    assert!((s.crit_pct - 1.8).abs() < 1e-3, "{}", s.crit_pct);
    assert!((s.mana_regen - 18.0).abs() < 1e-9, "{}", s.mana_regen);
    // L60 human mage: 1370 + 20 + 25 × 10 health; 1213 + 1595 mana; spirit 126.
    let s = CharacterStats::naked(class::MAGE, HUMAN, 60);
    assert_eq!(s.max_health, 1640);
    assert_eq!(s.max_mana, 2808);
    assert!((s.mana_regen - 44.0).abs() < 1e-9, "{}", s.mana_regen);
}

#[test]
fn a_rogue_uses_its_own_attack_power_and_agility_rates() {
    // L10 orc rogue: str 30, agi 32: AP 20 + 30 + 32 - 20; crit 32 / 3.50017, dodge 32 / 1.75009.
    let s = CharacterStats::naked(class::ROGUE, ORC, 10);
    assert_eq!(s.attack_power, 62);
    assert!((s.crit_pct - 9.1424).abs() < 1e-3, "{}", s.crit_pct);
    assert!((s.dodge_pct - 18.2848).abs() < 1e-3, "{}", s.dodge_pct);
    assert_eq!((s.max_mana, s.mana_regen), (0, 0.0));
}

#[test]
fn a_race_vanilla_never_paired_with_the_class_has_synthesised_rows() {
    // No Tauren mage exists in player_levelstats: its rows are the undead mage's (the Class's
    // first non-human race) plus the Tauren's offset from the undead in the Warrior table.
    const UNDEAD: u8 = 5;
    let tauren = CharacterStats::naked(class::MAGE, TAUREN, 10).base;
    let undead = CharacterStats::naked(class::MAGE, UNDEAD, 10).base;
    let tauren_w = CharacterStats::naked(class::WARRIOR, TAUREN, 10).base;
    let undead_w = CharacterStats::naked(class::WARRIOR, UNDEAD, 10).base;
    let want: [u32; 5] = std::array::from_fn(|i| undead[i] + tauren_w[i] - undead_w[i]);
    assert_eq!(tauren, want);
    assert_ne!(tauren, CharacterStats::naked(class::MAGE, HUMAN, 10).base);
}
