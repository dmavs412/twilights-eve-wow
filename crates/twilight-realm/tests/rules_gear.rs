//! Gear: what a Calling may wear (its own armour, every Warrior weapon, whatever the level),
//! where it goes, and what it adds to the character sheet.

use twilight_realm::callings::{self, CallingDef};
use twilight_realm::rules::gear::{can_equip, equip_slots, is_gear_slot, Refusal};
use twilight_realm::rules::power::class::WARRIOR;
use twilight_realm::rules::stats::CharacterStats;

const HUMAN: u8 = 1;

const FINE_SCIMITAR: u32 = 4560; // white 1H sword, 3-6 at 1.9 s
const CROOKED_STAFF: u32 = 1388; // white staff, two-handed
const FIRE_WAND: u32 = 5069; // required level 7
const PITCHFORK: u32 = 1485; // polearm, required level 20
const GNOLL_WAR_HARNESS: u32 = 1211; // leather chest, 70 armour, +2 str +2 sta, required level 10
const LARGE_OGRE_CHAIN_ARMOR: u32 = 914; // mail chest, required level 25
const JOUSTERS_CHESTPLATE: u32 = 8157; // plate chest, required level 40
const RING_OF_DEFENSE: u32 = 12985; // Equip: +4 Defense
const KRESHS_BACK: u32 = 13245; // shield, Equip: +4 Defense
const HUNTING_BOW: u32 = 8180;
const CRUDE_THROWING_AXE: u32 = 3111;
const SNOWY_ROBE: u32 = 2114;
const WOLFPACK_MEDALLION: u32 = 5754; // amulet
                                      // The Wardrobe's look templates for the starter Chests' pants and boots (content/gear.toml).
const LOOK_LEGS: u32 = 990000;
const LOOK_FEET: u32 = 990001;
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
    // vmangos `EquipmentSlots`: 1 neck, 4 chest, 10-11 fingers, 12-13 trinkets, 15 main hand,
    // 16 off hand, 17 ranged; 19-22 the bag slots. Only the ten Gear Slots and the bags.
    let vectors: [(u32, &[u8]); 15] = [
        (2, &[1]),               // neck
        (5, &[4]),               // chest
        (20, &[4]),              // robe
        (11, &[10, 11]),         // finger
        (12, &[12, 13]),         // trinket
        (13, &[15]),             // one-hand: no Dual Wield yet
        (17, &[15]),             // two-hand
        (21, &[15]),             // main hand
        (14, &[16]),             // shield
        (23, &[16]),             // held in off hand
        (15, &[17]),             // bow
        (25, &[17]),             // thrown
        (26, &[17]),             // gun, crossbow, wand
        (18, &[19, 20, 21, 22]), // bag: the four bag slots
        (0, &[]),                // not equippable
    ];
    for (inventory_type, slots) in vectors {
        assert_eq!(equip_slots(inventory_type), slots, "{inventory_type}");
    }
}

#[test]
fn only_the_ten_gear_slots_take_gear() {
    // Weapon (main hand, one-hand, two-hand), Offhand (off-hand weapon, shield, held), Ranged
    // (bow, thrown, gun, relic), Ammo, Ring, Trinket, Chest (and robe), Amulet.
    for inventory_type in [21, 13, 17, 22, 14, 23, 15, 25, 26, 28, 24, 11, 12, 5, 20, 2] {
        assert!(is_gear_slot(inventory_type), "{inventory_type}");
    }
    // Head, shoulders, shirt, waist, legs, feet, wrists, hands, back, tabard; and bags.
    for inventory_type in [1, 3, 4, 6, 7, 8, 9, 10, 16, 19, 18] {
        assert!(!is_gear_slot(inventory_type), "{inventory_type}");
        if inventory_type != 18 {
            assert_eq!(
                equip_slots(inventory_type),
                &[] as &[u8],
                "{inventory_type}"
            );
        }
    }
}

#[test]
fn every_gear_slot_kind_equips() {
    // The sheet holds no trinket or ammo yet; `only_the_ten_gear_slots_take_gear` covers them.
    let bw = basic_warrior();
    for (entry, what) in [
        (FINE_SCIMITAR, "weapon"),
        (WARRIORS_SHIELD, "offhand"),
        (HUNTING_BOW, "ranged"),
        (CRUDE_THROWING_AXE, "thrown"),
        (RING_OF_DEFENSE, "ring"),
        (GNOLL_WAR_HARNESS, "chest"),
        (SNOWY_ROBE, "robe"),
        (WOLFPACK_MEDALLION, "amulet"),
    ] {
        assert_eq!(can_equip(entry, HUMAN, bw), Ok(()), "{what}");
    }
}

#[test]
fn a_body_part_outside_the_gear_slots_is_refused() {
    // The Wardrobe's look templates are the only items left for those parts: legs and feet.
    for look in [LOOK_LEGS, LOOK_FEET] {
        assert_eq!(
            can_equip(look, HUMAN, basic_warrior()),
            Err(Refusal::NoGearSlot)
        );
    }
    // `EQUIP_ERR_ITEM_DOESNT_GO_TO_SLOT`.
    assert_eq!(Refusal::NoGearSlot.code(), 3);
}

#[test]
fn armour_and_stats_add_to_the_sheet() {
    let naked = CharacterStats::naked(WARRIOR, HUMAN, 10);
    let worn = CharacterStats::geared(WARRIOR, HUMAN, 10, &[GNOLL_WAR_HARNESS]);
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
    let s = CharacterStats::geared(WARRIOR, HUMAN, 1, &[FINE_SCIMITAR]);
    assert_eq!(s.attack_time_ms, 1900);
    assert!((s.min_damage - 6.936).abs() < 1e-3, "{}", s.min_damage);
    assert!((s.max_damage - 9.936).abs() < 1e-3, "{}", s.max_damage);
}

#[test]
fn equip_spells_add_defense() {
    let naked = CharacterStats::naked(WARRIOR, HUMAN, 27);
    let s = CharacterStats::geared(WARRIOR, HUMAN, 27, &[RING_OF_DEFENSE, KRESHS_BACK]);
    assert_eq!(s.attack_power_bonus, 0);
    assert_eq!(s.defense, naked.defense + 8);
}

#[test]
fn naked_is_geared_with_nothing() {
    assert_eq!(
        CharacterStats::geared(WARRIOR, HUMAN, 7, &[]),
        CharacterStats::naked(WARRIOR, HUMAN, 7)
    );
}
