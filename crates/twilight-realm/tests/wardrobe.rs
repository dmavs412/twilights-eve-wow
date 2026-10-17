//! The Wardrobe (ADR 0007): a worn Chest dresses the whole body. The realm writes the look of
//! each hidden body part into its visible-item field, as if a full set were worn, while the
//! inventory fields for those parts stay empty: no item object, no guid.

mod common;
mod gear_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::*;
use twilight_realm::RealmCommand;

/// The Basic Warrior's starter Chest: the Recruit's pants on the legs, the boots on the feet.
const RECRUITS_MAIL_VEST: u32 = 900000;
/// Its look templates (`content/gear.toml`).
const LOOK_LEGS: u32 = 990000;
const LOOK_FEET: u32 = 990001;
/// A Chest with no Wardrobe: bare everywhere else.
const KNITTED_TUNIC: u32 = 795;

// `EquipmentSlots`.
const CHEST: u8 = 4;
const LEGS: u8 = 6;
const FEET: u8 = 7;
/// Head, shoulders, back, wrists, hands, waist, legs, feet: the parts the Wardrobe dresses.
const HIDDEN: [u8; 8] = [0, 2, 14, 8, 9, 5, 6, 7];
/// The backpack's second slot (the first holds nothing in these saves either).
const PACK: u8 = 24;

/// The visible-item entry of each hidden part, `(slot, entry)`, where one is shown.
fn shown(events: &[SessionEvent], me: u64) -> Vec<(u8, u32)> {
    let me = descriptor(events, me);
    HIDDEN
        .iter()
        .filter_map(|&s| Some((s, me.player_visible_item_entry(s)?)))
        .collect()
}

/// Whether any hidden part has an item guid in its inventory field.
fn any_hidden_guid(events: &[SessionEvent], me: u64) -> bool {
    let me = descriptor(events, me);
    HIDDEN
        .iter()
        .any(|&s| me.player_inv_slot(s).is_some_and(|g| g != 0))
}

#[test]
fn login_shows_the_worn_chests_wardrobe() {
    let (_, me, events) = warrior_with(1, 0, &[(CHEST, RECRUITS_MAIL_VEST, 1)]);
    assert_eq!(shown(&events, me), [(LEGS, LOOK_LEGS), (FEET, LOOK_FEET)]);
    assert!(
        !any_hidden_guid(&events, me),
        "no item stands behind a look"
    );
}

#[test]
fn unequipping_the_chest_bares_the_body_and_wearing_it_again_dresses_it() {
    let (mut realm, me, mut events) = warrior_with(1, 0, &[(CHEST, RECRUITS_MAIL_VEST, 1)]);
    realm.handle(RealmCommand::SwapInvItem {
        src: CHEST,
        dst: PACK,
    });
    let off = drain(&mut realm);
    // The update that empties the chest slot bares every part with it.
    let emptied = off
        .iter()
        .find(|e| match e {
            SessionEvent::ObjectValues { guid, fields } => {
                *guid == me && fields.player_inv_slot(CHEST).is_some()
            }
            _ => false,
        })
        .expect("the chest slot changes")
        .clone();
    let mut just_that = events.clone();
    just_that.push(emptied);
    assert_eq!(shown(&just_that, me), []);
    events.extend(off);
    assert_eq!(shown(&events, me), []);

    realm.handle(RealmCommand::SwapInvItem {
        src: PACK,
        dst: CHEST,
    });
    events.extend(drain(&mut realm));
    assert_eq!(shown(&events, me), [(LEGS, LOOK_LEGS), (FEET, LOOK_FEET)]);
    assert!(!any_hidden_guid(&events, me));
}

#[test]
fn swapping_to_a_chest_without_a_wardrobe_clears_the_looks() {
    let (mut realm, me, mut events) = warrior_with(
        1,
        0,
        &[(CHEST, RECRUITS_MAIL_VEST, 1), (PACK, KNITTED_TUNIC, 1)],
    );
    realm.handle(RealmCommand::SwapInvItem {
        src: PACK,
        dst: CHEST,
    });
    events.extend(drain(&mut realm));
    assert_eq!(shown(&events, me), []);
    let worn = descriptor(&events, me).player_visible_item_entry(CHEST);
    assert_eq!(worn, Some(KNITTED_TUNIC));
}

#[test]
fn the_select_screen_doll_wears_the_wardrobe() {
    let (guid, blob) = save_with(1, 0, &[(CHEST, RECRUITS_MAIL_VEST, 1)]);
    let mut realm = realm();
    realm.import_character(&blob).unwrap();
    let c = roster(&mut realm)
        .into_iter()
        .find(|c| c.guid == guid)
        .expect("listed");
    let legs = &c.equipment[usize::from(LEGS)];
    let feet = &c.equipment[usize::from(FEET)];
    // The Recruit's Pants' and Boots' displays.
    assert_eq!((legs.display_id, legs.inventory_type), (9892, 7));
    assert_eq!((feet.display_id, feet.inventory_type), (10141, 8));
}

/// Every look template answers the item query with its display and its body part's inventory
/// type, carries nothing, and no Calling may equip it.
#[test]
fn the_look_templates_answer_the_item_query_and_are_never_gear() {
    let mut realm = realm();
    for (entry, display, inventory_type) in [(LOOK_LEGS, 9892, 7), (LOOK_FEET, 10141, 8)] {
        realm.handle(RealmCommand::ItemQuery { entry });
        let info = drain(&mut realm)
            .into_iter()
            .find_map(|e| match e {
                SessionEvent::ItemTemplate { info, .. } => info,
                _ => None,
            })
            .unwrap_or_else(|| panic!("{entry} answers"));
        assert_eq!(
            (info.display_info_id, info.inventory_type),
            (display, inventory_type)
        );
        assert!(info.stats.is_empty() && info.armor == 0 && info.damages.is_empty());
        let basic = twilight_realm::callings::calling("basic_warrior").unwrap();
        assert!(twilight_realm::rules::gear::can_equip(entry, HUMAN, basic).is_err());
    }
}
