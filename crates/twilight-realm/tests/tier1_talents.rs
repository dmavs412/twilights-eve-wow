//! The Vanguard's two-tab tree, played through the realm's seam: 29 points at the cap, enough to
//! reach Flurry, the tabs' tier gates, and a WIP talent that still opens the next row.

mod common;
mod gear_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::*;

// `Talent.dbc` ids in the Vanguard tree (`content/callings.toml`).
// Weaponmaster.
const IMPROVED_OVERPOWER: u32 = 131;
const IMPROVED_REND: u32 = 127;
const IMPROVED_CHARGE: u32 = 126;
const DEEP_WOUNDS: u32 = 121;
const TWO_HANDED_WEAPON_SPECIALIZATION: u32 = 136;
const SWORD_SPECIALIZATION: u32 = 123;
/// WIP: no effect yet.
const ANGER_MANAGEMENT: u32 = 137;
const FLURRY: u32 = 156;
const FLURRY_RANKS: [u32; 5] = [12319, 12971, 12972, 12973, 12974];
// Warlord.
const CRUELTY: u32 = 157;
const ENRAGE: u32 = 155;
/// A Basic Warrior talent the Vanguard tree does not have.
const IMPROVED_HEROIC_STRIKE: u32 = 124;

/// A human Vanguard of `level` in the world, with the entry burst.
fn vanguard(level: u8) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with(level, 0, &[]);
    let blob = blob.replace("calling = \"basic_warrior\"", "calling = \"vanguard\"");
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

/// Nineteen points down Weaponmaster without Anger Management: one short of Flurry's row.
fn nineteen_in_weaponmaster(realm: &mut Realm, events: &mut Vec<SessionEvent>) {
    learn(realm, events, IMPROVED_OVERPOWER, 2);
    learn(realm, events, IMPROVED_REND, 3);
    learn(realm, events, DEEP_WOUNDS, 3);
    learn(realm, events, IMPROVED_CHARGE, 2);
    learn(realm, events, TWO_HANDED_WEAPON_SPECIALIZATION, 5);
    learn(realm, events, SWORD_SPECIALIZATION, 4);
}

#[test]
fn a_vanguard_at_the_cap_has_29_points_across_two_tabs() {
    let (mut realm, me, mut events) = vanguard(30);
    assert_eq!(points(&events, me), Some(29));
    // Not the Basic Warrior's tree any more.
    learn(&mut realm, &mut events, IMPROVED_HEROIC_STRIKE, 1);
    assert_eq!(points(&events, me), Some(29));
    // Enrage sits in Warlord's third row: 10 points there first.
    learn(&mut realm, &mut events, ENRAGE, 1);
    assert_eq!(points(&events, me), Some(29));
    learn(&mut realm, &mut events, CRUELTY, 5);
    assert_eq!(points(&events, me), Some(24));
}

#[test]
fn twenty_points_in_weaponmaster_reach_flurry_with_nine_left_for_warlord() {
    let (mut realm, me, mut events) = vanguard(30);
    nineteen_in_weaponmaster(&mut realm, &mut events);
    // Sword Specialization's fifth rank.
    realm.handle(RealmCommand::LearnTalent {
        talent_id: SWORD_SPECIALIZATION,
        rank: 4,
    });
    events.extend(drain(&mut realm));
    assert_eq!(points(&events, me), Some(9));
    learn(&mut realm, &mut events, FLURRY, 5);
    assert_eq!(points(&events, me), Some(4));
    assert!(knows_any(&events, &FLURRY_RANKS));
    learn(&mut realm, &mut events, CRUELTY, 4);
    assert_eq!(points(&events, me), Some(0));
}

#[test]
fn a_wip_talent_is_learnable_and_opens_the_next_tier() {
    let (mut realm, me, mut events) = vanguard(30);
    nineteen_in_weaponmaster(&mut realm, &mut events);
    assert_eq!(points(&events, me), Some(10));
    learn(&mut realm, &mut events, FLURRY, 1);
    assert_eq!(points(&events, me), Some(10), "Flurry's row needs 20");
    // Anger Management does nothing yet, but takes its point and counts toward the row.
    learn(&mut realm, &mut events, ANGER_MANAGEMENT, 1);
    assert_eq!(points(&events, me), Some(9));
    learn(&mut realm, &mut events, FLURRY, 1);
    assert_eq!(points(&events, me), Some(8));
}
