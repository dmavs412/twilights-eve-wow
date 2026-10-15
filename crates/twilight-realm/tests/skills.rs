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
