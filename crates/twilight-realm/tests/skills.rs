//! The Basic Warrior's eight skills, played through the realm's seam: the spellbook at login,
//! casts from the action bar, what they cost, what they refuse, and what they do to a mob.

mod common;

use benilla_protocol::SessionEvent;
use common::*;

const HEROIC_STRIKE: u32 = 78;
const CHARGE: u32 = 100;
const REND: u32 = 772;
const THUNDER_CLAP: u32 = 6343;
const BATTLE_SHOUT: u32 = 6673;
const HAMSTRING: u32 = 1715;
const OVERPOWER: u32 = 7384;
const BLOODRAGE: u32 = 2687;

const EIGHT: [u32; 8] = [
    HEROIC_STRIKE,
    CHARGE,
    REND,
    THUNDER_CLAP,
    BATTLE_SHOUT,
    HAMSTRING,
    OVERPOWER,
    BLOODRAGE,
];

#[test]
fn a_new_basic_warriors_spellbook_holds_exactly_the_eight_skills() {
    let mut realm = realm();
    let (_, events) = enter_new(&mut realm, "Garrosh", ORC);

    let books: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellBook {
                spell_ids,
                cooldowns,
            } => Some((spell_ids.clone(), cooldowns.len())),
            _ => None,
        })
        .collect();
    assert_eq!(books.len(), 1, "one spellbook at login");
    let (mut ids, cooldowns) = books[0].clone();
    ids.sort_unstable();
    let mut want = EIGHT.to_vec();
    want.sort_unstable();
    assert_eq!(ids, want);
    assert_eq!(cooldowns, 0, "a fresh character has nothing cooling down");
}

#[test]
fn a_warrior_stands_in_battle_stance_so_the_client_allows_its_stance_skills() {
    let mut realm = realm();
    let (guid, events) = enter_new(&mut realm, "Garrosh", ORC);
    assert_eq!(own_create(&events, guid).unit_shapeshift_form(), BATTLE_STANCE);
}

#[test]
fn a_new_character_finds_the_eight_on_its_battle_stance_bar() {
    let mut realm = realm();
    let (_, events) = enter_new(&mut realm, "Garrosh", ORC);
    let buttons = action_buttons(&events);
    // Battle Stance pages the main bar to slots 73-84 (0-based 72-83).
    let on_bar: Vec<(u8, u32)> = buttons.iter().map(|b| (b.slot, b.action)).collect();
    let want: Vec<(u8, u32)> = (72..).zip(EIGHT).collect();
    assert_eq!(on_bar, want);
    assert!(buttons.iter().all(|b| b.kind == 0), "spell buttons");
}

#[test]
fn the_bar_the_player_arranges_comes_back_at_the_next_login() {
    let mut realm = realm();
    let (guid, _) = enter_new(&mut realm, "Garrosh", ORC);
    // Charge moved to button 1 of the stance bar; Heroic Strike's old button cleared.
    realm.handle(RealmCommand::SetActionButton {
        slot: 72,
        packed: CHARGE,
    });
    realm.handle(RealmCommand::SetActionButton {
        slot: 73,
        packed: 0,
    });
    realm.handle(RealmCommand::Logout);
    realm.handle(RealmCommand::EnterWorld { guid });
    let events = drain(&mut realm);
    let on_bar: Vec<(u8, u32)> = action_buttons(&events)
        .iter()
        .map(|b| (b.slot, b.action))
        .collect();
    assert_eq!(&on_bar[..2], &[(72, CHARGE), (74, REND)]);

    // And it survives the save.
    let blob = realm.export_character(guid).unwrap();
    let mut other = Realm::new();
    other.import_character(&blob).unwrap();
    other.handle(RealmCommand::EnterWorld { guid });
    let events = drain(&mut other);
    assert_eq!(action_buttons(&events)[0].action, CHARGE);
}

// ── Helpers ──────────────────────────────────────────────────────────────────────────────────

/// `UNIT_FIELD_BYTES_1` byte 2's Battle Stance.
const BATTLE_STANCE: u8 = 17;

fn own_create(events: &[SessionEvent], guid: u64) -> benilla_protocol::ObjectFields {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate { guid: g, fields, .. } if *g == guid => {
                Some(fields.clone())
            }
            _ => None,
        })
        .expect("the player's own create")
}

fn action_buttons(events: &[SessionEvent]) -> Vec<benilla_protocol::messages::ActionButton> {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ActionButtons { buttons } => Some(buttons.clone()),
            _ => None,
        })
        .expect("the action bar at login")
}
