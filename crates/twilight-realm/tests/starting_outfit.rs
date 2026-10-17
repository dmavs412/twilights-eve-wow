//! A new character starts dressed in the Basic Warrior's outfit (`content/starting_outfits.toml`,
//! ADR 0007): the Recruit's Mail Vest, whose Wardrobe is the vanilla Recruit's pants and boots,
//! with a sword, a shield and throwing axes, each worn in its slot, so the skills that need a
//! melee weapon can be cast from level 1. Every race starts the same.

mod common;
mod gear_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::descriptor;

const WORN_SHORTSWORD: u32 = 25;
const RECRUITS_MAIL_VEST: u32 = 900000;
const WORN_WOODEN_SHIELD: u32 = 2362;
const CRUDE_THROWING_AXE: u32 = 3111;

// `EquipmentSlots`.
const CHEST: u8 = 4;
const LEGS: u8 = 6;
const FEET: u8 = 7;
const MAIN_HAND: u8 = 15;
const OFF_HAND: u8 = 16;
const RANGED: u8 = 17;

/// The entry worn in each equipment slot, as the client sees the player's create.
fn worn(events: &[SessionEvent], me: u64) -> Vec<(u8, u32)> {
    let player = descriptor(events, me);
    (0..19)
        .filter_map(|slot| {
            let guid = player.player_inv_slot(slot).filter(|g| *g != 0)?;
            Some((slot, descriptor(events, guid).object_entry()?))
        })
        .collect()
}

/// The Basic Warrior's outfit, as the client sees a new character's create.
const OUTFIT: [(u8, u32); 4] = [
    (CHEST, RECRUITS_MAIL_VEST),
    (MAIN_HAND, WORN_SHORTSWORD),
    (OFF_HAND, WORN_WOODEN_SHIELD),
    (RANGED, CRUDE_THROWING_AXE),
];

#[test]
fn a_new_warrior_wears_the_basic_warriors_outfit_whatever_its_race() {
    for (name, race) in [("Anduin", HUMAN), ("Cairne", TAUREN)] {
        let mut realm = realm();
        let (me, events) = enter_new(&mut realm, name, race);
        assert_eq!(worn(&events, me), OUTFIT, "{name}");
        // The sword's 1.9 s swing, not the 2 s of bare fists.
        assert_eq!(descriptor(&events, me).unit_base_attack_time(0), Some(1900));
    }
}

#[test]
fn the_throwing_axes_come_as_a_whole_stack() {
    let mut realm = realm();
    let (me, events) = enter_new(&mut realm, "Anduin", HUMAN);
    let axes = descriptor(&events, me).player_inv_slot(RANGED).unwrap();
    assert_eq!(descriptor(&events, axes).item_stack_count(), Some(200));
}

#[test]
fn the_hearthstone_stays_in_the_backpack() {
    let mut realm = realm();
    let (me, events) = enter_new(&mut realm, "Cairne", TAUREN);
    let stone = descriptor(&events, me)
        .player_pack_slot(0)
        .filter(|g| *g != 0)
        .expect("the backpack's first slot holds something");
    assert_eq!(descriptor(&events, stone).object_entry(), Some(6948));
}

#[test]
fn the_select_screen_dresses_the_doll_in_the_outfit() {
    let mut realm = realm();
    assert_eq!(create(&mut realm, "Cairne", TAUREN, 0), CHAR_CREATE_SUCCESS);
    assert_eq!(create(&mut realm, "Anduin", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let list = roster(&mut realm);
    let doll = |name: &str| -> Vec<(u8, u32, u8)> {
        let c = list.iter().find(|c| c.name == name).expect("listed");
        (0..19u8)
            .map(|s| (s, &c.equipment[usize::from(s)]))
            .filter(|(_, e)| e.display_id != 0)
            .map(|(s, e)| (s, e.display_id, e.inventory_type))
            .collect()
    };
    // The vest's own look on the chest and its Wardrobe below: the Recruit's Shirt, Pants and
    // Boots' displays.
    let dressed = [
        (CHEST, 9891, 5),
        (LEGS, 9892, 7),
        (FEET, 10141, 8),
        (MAIN_HAND, 1542, 21),
        (OFF_HAND, 18730, 14),
        (RANGED, 20777, 25),
    ];
    assert_eq!(doll("Cairne"), dressed);
    assert_eq!(doll("Anduin"), dressed);
}

#[test]
fn the_sword_answers_its_item_query() {
    let mut realm = realm();
    realm.handle(RealmCommand::ItemQuery {
        entry: WORN_SHORTSWORD,
    });
    let info = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::ItemTemplate { info, .. } => info,
            _ => None,
        })
        .expect("its template");
    assert_eq!(info.name, "Worn Shortsword");
    assert_eq!((info.dmg_min, info.dmg_max), (1.0, 3.0));
}
