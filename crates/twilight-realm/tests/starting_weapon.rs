//! A new character starts armed: vanilla's Warrior starting weapon, worn in the main hand, so
//! the skills that need a melee weapon can be cast from level 1.

mod common;
mod gear_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::descriptor;

const WORN_SHORTSWORD: u32 = 25;
const MAIN_HAND: u8 = 15;

#[test]
fn a_new_character_wields_a_worn_shortsword_in_the_main_hand() {
    let mut realm = realm();
    let (me, events) = enter_new(&mut realm, "Garrosh", ORC);
    let sword = descriptor(&events, me)
        .player_inv_slot(MAIN_HAND)
        .filter(|g| *g != 0)
        .expect("a weapon in the main hand");
    assert_eq!(
        descriptor(&events, sword).object_entry(),
        Some(WORN_SHORTSWORD)
    );
    // Its 1.9 s swing, not the 2 s of bare fists.
    assert_eq!(descriptor(&events, me).unit_base_attack_time(0), Some(1900));
}

#[test]
fn the_select_screen_shows_the_sword() {
    let mut realm = realm();
    assert_eq!(create(&mut realm, "Garrosh", ORC, 0), CHAR_CREATE_SUCCESS);
    let c = &roster(&mut realm)[0];
    // `INVTYPE_WEAPONMAINHAND`.
    assert_eq!(c.equipment[usize::from(MAIN_HAND)].inventory_type, 21);
    assert_ne!(c.equipment[usize::from(MAIN_HAND)].display_id, 0);
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
