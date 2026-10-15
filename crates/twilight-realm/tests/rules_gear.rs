//! Gear: what a Warrior may wear (vanilla proficiency, `docs/reference/items-and-vendors.md`),
//! where it goes, and what it adds to the character sheet.

use twilight_realm::rules::gear::{can_equip, equip_slots, Refusal};
use twilight_realm::rules::stats::WarriorStats;

const HUMAN: u8 = 1;

const FINE_SCIMITAR: u32 = 4560; // white 1H sword, 3-6 at 1.9 s
const CROOKED_STAFF: u32 = 1388; // white staff, two-handed
const FIRE_WAND: u32 = 5069; // required level 7
const PITCHFORK: u32 = 1485; // polearm, required level 20
const GNOLL_WAR_HARNESS: u32 = 1211; // leather chest, 70 armour, +2 str +2 sta, required level 10
const LARGE_OGRE_CHAIN_ARMOR: u32 = 914; // mail chest, required level 25
const GAUNTLETS_OF_OGRE_STRENGTH: u32 = 3341; // Equip: +16 attack power
const RING_OF_DEFENSE: u32 = 12985; // Equip: +4 Defense
const WARRIORS_SHIELD: u32 = 1438;
const DULL_FRENZY_SCALE: u32 = 537; // grey vendor trash

#[test]
fn a_warrior_wears_cloth_leather_mail_shields_and_most_weapons() {
    assert_eq!(can_equip(FINE_SCIMITAR, HUMAN, 1), Ok(()));
    assert_eq!(can_equip(CROOKED_STAFF, HUMAN, 1), Ok(()));
    assert_eq!(can_equip(WARRIORS_SHIELD, HUMAN, 4), Ok(()));
    assert_eq!(can_equip(GNOLL_WAR_HARNESS, HUMAN, 10), Ok(()));
}

#[test]
fn a_warrior_can_never_use_a_wand() {
    assert_eq!(can_equip(FIRE_WAND, HUMAN, 30), Err(Refusal::Proficiency));
}

#[test]
fn polearms_need_level_20() {
    assert_eq!(can_equip(PITCHFORK, HUMAN, 19), Err(Refusal::Proficiency));
    assert_eq!(can_equip(PITCHFORK, HUMAN, 20), Ok(()));
}

#[test]
fn an_item_above_the_characters_level_is_refused_with_its_level() {
    assert_eq!(
        can_equip(LARGE_OGRE_CHAIN_ARMOR, HUMAN, 24),
        Err(Refusal::Level(25))
    );
    assert_eq!(can_equip(LARGE_OGRE_CHAIN_ARMOR, HUMAN, 25), Ok(()));
}

#[test]
fn vendor_trash_is_not_equippable() {
    assert_eq!(
        can_equip(DULL_FRENZY_SCALE, HUMAN, 10),
        Err(Refusal::NotEquippable)
    );
}

#[test]
fn each_inventory_type_goes_to_its_equipment_slots() {
    // vmangos `EquipmentSlots`: 0 head … 15 main hand, 16 off hand, 17 ranged, 18 tabard.
    let vectors: [(u32, &[u8]); 14] = [
        (1, &[0]),       // head
        (5, &[4]),       // chest
        (20, &[4]),      // robe
        (6, &[5]),       // waist
        (11, &[10, 11]), // finger
        (12, &[12, 13]), // trinket
        (16, &[14]),     // cloak
        (13, &[15]),     // one-hand: no Dual Wield yet
        (17, &[15]),     // two-hand
        (21, &[15]),     // main hand
        (14, &[16]),     // shield
        (23, &[16]),     // held in off hand
        (15, &[17]),     // ranged
        (0, &[]),        // not equippable
    ];
    for (inventory_type, slots) in vectors {
        assert_eq!(equip_slots(inventory_type), slots, "{inventory_type}");
    }
}

#[test]
fn armour_and_stats_add_to_the_sheet() {
    let naked = WarriorStats::naked(HUMAN, 10);
    let worn = WarriorStats::geared(HUMAN, 10, &[GNOLL_WAR_HARNESS]);
    let [str, agi, sta, int, spi] = naked.base;
    assert_eq!(worn.base, [str + 2, agi, sta + 2, int, spi]);
    assert_eq!(worn.bonus, [2, 0, 2, 0, 0], "the green part of each stat");
    assert_eq!(worn.armor, naked.armor + 70);
    // Two strength is four attack power; two stamina past the first 20 is 20 health.
    assert_eq!(worn.attack_power, naked.attack_power + 4);
    assert_eq!(worn.max_health, naked.max_health + 20);
}

#[test]
fn a_weapon_sets_the_swing_damage_and_speed() {
    // Level 1 human: 29 attack power, 29 / 14 × 1.9 s = 3.936 on each end.
    let s = WarriorStats::geared(HUMAN, 1, &[FINE_SCIMITAR]);
    assert_eq!(s.attack_time_ms, 1900);
    assert!((s.min_damage - 6.936).abs() < 1e-3, "{}", s.min_damage);
    assert!((s.max_damage - 9.936).abs() < 1e-3, "{}", s.max_damage);
}

#[test]
fn equip_spells_add_attack_power_and_defense() {
    let naked = WarriorStats::naked(HUMAN, 27);
    let s = WarriorStats::geared(HUMAN, 27, &[GAUNTLETS_OF_OGRE_STRENGTH, RING_OF_DEFENSE]);
    assert_eq!(s.attack_power_bonus, 16);
    // The gauntlets carry no strength of their own: just the 16.
    assert_eq!(
        s.attack_power,
        naked.attack_power + 16 + 2 * s.bonus[0] as i32
    );
    assert_eq!(s.defense, naked.defense + 4);
}

#[test]
fn naked_is_geared_with_nothing() {
    assert_eq!(
        WarriorStats::geared(HUMAN, 7, &[]),
        WarriorStats::naked(HUMAN, 7)
    );
}
