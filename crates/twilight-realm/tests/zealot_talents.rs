//! The Zealot's three-tab tree of Paladin talents, played through the realm's seam: 49 points at
//! the cap, Holy Power behind 30 in Crusade, Benediction's cheaper Seals and Judgement with the
//! spell modifiers that tell the client, what the modelled talents do to the sheet and the
//! swings, and the respec that wipes it all.

mod common;
mod gear_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::*;

// `Talent.dbc` ids in the Zealot tree (`content/callings.toml`).
// Crusade.
const BENEDICTION: u32 = 1407;
const DEFLECTION: u32 = 1403;
const CONVICTION: u32 = 1411;
const VINDICATION: u32 = 1633;
const PURSUIT_OF_JUSTICE: u32 = 1634;
const EYE_FOR_AN_EYE: u32 = 1632;
const TWO_HANDED_WEAPON_SPECIALIZATION: u32 = 1410;
const IMPROVED_HAMMER_OF_JUSTICE: u32 = 1521;
const VENGEANCE: u32 = 1402;
const HOLY_POWER: u32 = 1627;
const HOLY_POWER_RANKS: [u32; 5] = [5923, 5924, 5925, 5926, 25829];

/// A human Zealot of `level` in the world, with the entry burst.
fn zealot(level: u8) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with(level, 0, &[]);
    let blob = blob.replace("calling = \"basic_warrior\"", "calling = \"zealot\"");
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

fn learn(realm: &mut Realm, events: &mut Vec<SessionEvent>, talent_id: u32, ranks: u32) {
    for rank in 0..ranks {
        realm.handle(RealmCommand::LearnTalent { talent_id, rank });
    }
    events.extend(drain(realm));
}

fn points(events: &[SessionEvent], me: u64) -> Option<u32> {
    descriptor(events, me).player_talent_points()
}

/// Whether the spellbook ends up holding any of `ranks`.
fn knows_any(events: &[SessionEvent], ranks: &[u32]) -> bool {
    let mut known = false;
    for e in events {
        match e {
            SessionEvent::SpellLearned { spell_id } if ranks.contains(spell_id) => known = true,
            SessionEvent::SpellRemoved { spell_id } if ranks.contains(spell_id) => known = false,
            _ => {}
        }
    }
    known
}

/// Twenty-nine points down Crusade's first six rows: one short of Holy Power's.
fn twenty_nine_in_crusade(realm: &mut Realm, events: &mut Vec<SessionEvent>) {
    learn(realm, events, BENEDICTION, 5);
    learn(realm, events, DEFLECTION, 5);
    learn(realm, events, CONVICTION, 5);
    learn(realm, events, VINDICATION, 3);
    learn(realm, events, PURSUIT_OF_JUSTICE, 2);
    learn(realm, events, EYE_FOR_AN_EYE, 2);
    learn(realm, events, TWO_HANDED_WEAPON_SPECIALIZATION, 3);
    learn(realm, events, IMPROVED_HAMMER_OF_JUSTICE, 3);
    learn(realm, events, VENGEANCE, 1);
}

// Blade.
const IMPROVED_CHARGE: u32 = 126;
const IMPALE: u32 = 662;
const DEEP_WOUNDS: u32 = 121;
const SWORD_SPECIALIZATION: u32 = 123;
const FLURRY: u32 = 156;

#[test]
fn blade_opens_its_second_row_from_its_first_and_flurry_at_ten() {
    let (mut realm, me, mut events) = zealot(50);
    learn(&mut realm, &mut events, IMPROVED_CHARGE, 2);
    learn(&mut realm, &mut events, IMPALE, 2);
    learn(&mut realm, &mut events, SWORD_SPECIALIZATION, 1);
    assert_eq!(points(&events, me), Some(45), "the second row needs 5");
    learn(&mut realm, &mut events, DEEP_WOUNDS, 1);
    learn(&mut realm, &mut events, SWORD_SPECIALIZATION, 4);
    assert_eq!(points(&events, me), Some(40));
    learn(&mut realm, &mut events, FLURRY, 1);
    assert_eq!(points(&events, me), Some(40), "Flurry's row needs 10");
    learn(&mut realm, &mut events, SWORD_SPECIALIZATION, 5);
    learn(&mut realm, &mut events, FLURRY, 1);
    assert_eq!(points(&events, me), Some(38));
}

#[test]
fn a_zealot_at_the_cap_has_49_points_and_the_31st_in_crusade_reaches_holy_power() {
    let (mut realm, me, mut events) = zealot(50);
    assert_eq!(points(&events, me), Some(49));
    twenty_nine_in_crusade(&mut realm, &mut events);
    assert_eq!(points(&events, me), Some(20));
    learn(&mut realm, &mut events, HOLY_POWER, 1);
    assert_eq!(points(&events, me), Some(20), "Holy Power's row needs 30");
    learn(&mut realm, &mut events, VENGEANCE, 2);
    assert_eq!(points(&events, me), Some(19));
    learn(&mut realm, &mut events, HOLY_POWER, 5);
    assert_eq!(points(&events, me), Some(14));
    assert!(knows_any(&events, &HOLY_POWER_RANKS));
}
