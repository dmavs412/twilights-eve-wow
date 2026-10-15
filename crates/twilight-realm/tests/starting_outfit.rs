//! A new character starts dressed: vanilla's Warrior starting outfit for its race (the client's
//! `CharStartOutfit.dbc`, the same entries as vmangos `playercreateinfo_item`), each item worn in
//! its slot, so the skills that need a melee weapon can be cast from level 1.

mod common;
mod gear_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::descriptor;

const WORN_SHORTSWORD: u32 = 25;

// `EquipmentSlots`.
const BODY: u8 = 3;
const LEGS: u8 = 6;
const FEET: u8 = 7;
const MAIN_HAND: u8 = 15;
const OFF_HAND: u8 = 16;

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

#[test]
fn a_new_human_warrior_wears_the_recruits_outfit_with_sword_and_shield() {
    let mut realm = realm();
    let (me, events) = enter_new(&mut realm, "Anduin", HUMAN);
    // vmangos `playercreateinfo_item` (race 1, class 1): Recruit's Shirt, Pants and Boots, the
    // Worn Shortsword and the Worn Wooden Shield.
    assert_eq!(
        worn(&events, me),
        [
            (BODY, 38),
            (LEGS, 39),
            (FEET, 40),
            (MAIN_HAND, WORN_SHORTSWORD),
            (OFF_HAND, 2362)
        ]
    );
    // The sword's 1.9 s swing, not the 2 s of bare fists.
    assert_eq!(descriptor(&events, me).unit_base_attack_time(0), Some(1900));
}

#[test]
fn a_new_tauren_warrior_wears_the_brawlers_outfit_with_a_two_handed_hammer() {
    let mut realm = realm();
    let (me, events) = enter_new(&mut realm, "Cairne", TAUREN);
    // vmangos `playercreateinfo_item` (race 6, class 1): Brawler's Harness and Pants (no boots
    // for hooves) and the Battleworn Hammer.
    assert_eq!(
        worn(&events, me),
        [(BODY, 6125), (LEGS, 139), (MAIN_HAND, 2361)]
    );
    // The hammer's 2.9 s swing.
    assert_eq!(descriptor(&events, me).unit_base_attack_time(0), Some(2900));
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
    // Display ids and inventory types from the client's Item.dbc and CharStartOutfit.dbc.
    assert_eq!(
        doll("Cairne"),
        [(BODY, 9995, 4), (LEGS, 9988, 7), (MAIN_HAND, 8690, 17)]
    );
    assert_eq!(
        doll("Anduin"),
        [
            (BODY, 9891, 4),
            (LEGS, 9892, 7),
            (FEET, 10141, 8),
            (MAIN_HAND, 1542, 21),
            (OFF_HAND, 18730, 14)
        ]
    );
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
