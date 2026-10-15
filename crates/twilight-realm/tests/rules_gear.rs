//! Gear: what a Calling may wear (its own armour, every Warrior weapon, whatever the level),
//! where it goes, and what it adds to the character sheet.

use twilight_realm::callings::{self, CallingDef};
use twilight_realm::rules::gear::{can_equip, equip_slots, Refusal};
use twilight_realm::rules::stats::WarriorStats;

const HUMAN: u8 = 1;

const FINE_SCIMITAR: u32 = 4560; // white 1H sword, 3-6 at 1.9 s
const CROOKED_STAFF: u32 = 1388; // white staff, two-handed
const FIRE_WAND: u32 = 5069; // required level 7
const PITCHFORK: u32 = 1485; // polearm, required level 20
const GNOLL_WAR_HARNESS: u32 = 1211; // leather chest, 70 armour, +2 str +2 sta, required level 10
const LARGE_OGRE_CHAIN_ARMOR: u32 = 914; // mail chest, required level 25
const JOUSTERS_CHESTPLATE: u32 = 8157; // plate chest, required level 40
const GAUNTLETS_OF_OGRE_STRENGTH: u32 = 3341; // Equip: +16 attack power
const RING_OF_DEFENSE: u32 = 12985; // Equip: +4 Defense
const WARRIORS_SHIELD: u32 = 1438;
const DULL_FRENZY_SCALE: u32 = 537; // grey vendor trash

fn basic_warrior() -> &'static CallingDef {
    callings::calling("basic_warrior").unwrap()
}

fn vanguard() -> &'static CallingDef {
    callings::calling("vanguard").unwrap()
}

#[test]
fn a_basic_warrior_wears_cloth_leather_mail_shields_and_every_warrior_weapon() {
    let bw = basic_warrior();
    assert_eq!(can_equip(FINE_SCIMITAR, HUMAN, bw), Ok(()));
    assert_eq!(can_equip(CROOKED_STAFF, HUMAN, bw), Ok(()));
    assert_eq!(can_equip(PITCHFORK, HUMAN, bw), Ok(()), "polearms too");
    assert_eq!(can_equip(WARRIORS_SHIELD, HUMAN, bw), Ok(()));
    assert_eq!(can_equip(GNOLL_WAR_HARNESS, HUMAN, bw), Ok(()));
}

#[test]
fn a_basic_warrior_is_refused_plate_for_want_of_proficiency() {
    assert_eq!(
        can_equip(JOUSTERS_CHESTPLATE, HUMAN, basic_warrior()),
        Err(Refusal::Proficiency)
    );
}

#[test]
fn a_vanguard_wears_plate() {
    assert_eq!(can_equip(JOUSTERS_CHESTPLATE, HUMAN, vanguard()), Ok(()));
}

#[test]
fn no_calling_can_use_a_wand() {
    for c in [basic_warrior(), vanguard()] {
        assert_eq!(can_equip(FIRE_WAND, HUMAN, c), Err(Refusal::Proficiency));
    }
}

#[test]
fn an_items_required_level_never_refuses_it() {
    // Required level 25 mail, on a Calling whose level resets to 1.
    assert_eq!(
        can_equip(LARGE_OGRE_CHAIN_ARMOR, HUMAN, basic_warrior()),
        Ok(())
    );
}

#[test]
fn vendor_trash_is_not_equippable() {
    assert_eq!(
        can_equip(DULL_FRENZY_SCALE, HUMAN, basic_warrior()),
        Err(Refusal::NotEquippable)
    );
}

#[test]
fn each_inventory_type_goes_to_its_equipment_slots() {
    // vmangos `EquipmentSlots`: 0 head … 15 main hand, 16 off hand, 17 ranged, 18 tabard; 19-22
    // the bag slots.
    let vectors: [(u32, &[u8]); 15] = [
        (1, &[0]),               // head
        (5, &[4]),               // chest
        (20, &[4]),              // robe
        (6, &[5]),               // waist
        (11, &[10, 11]),         // finger
        (12, &[12, 13]),         // trinket
        (16, &[14]),             // cloak
        (13, &[15]),             // one-hand: no Dual Wield yet
        (17, &[15]),             // two-hand
        (21, &[15]),             // main hand
        (14, &[16]),             // shield
        (23, &[16]),             // held in off hand
        (15, &[17]),             // ranged
        (18, &[19, 20, 21, 22]), // bag: the four bag slots
        (0, &[]),                // not equippable
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
