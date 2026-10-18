//! The Vanguard's two-tab tree, played through the realm's seam: 29 points at the cap, enough to
//! reach Hammer of the Vanguard, the tabs' tier gates, and a WIP talent that still opens the next
//! row.

mod common;
mod gear_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::*;

// `Talent.dbc` ids in the Vanguard tree (the master CSV).
// The Righteous.
const DIVINE_STRENGTH: u32 = 1401;
/// WIP: no effect yet.
const WRATHFUL_IRON: u32 = 1463;
const HALLOWED_GROUND: u32 = 128;
const SPIRITUAL_FOCUS: u32 = 1405;
const RIGHTEOUS_FURY: u32 = 157;
const HAMMER_OF_THE_VANGUARD: u32 = 152;
const HAMMER_RANKS: [u32; 1] = [12809];
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

/// Fifteen points down The Righteous: enough for Hammer of the Vanguard's row.
fn fifteen_in_righteous(realm: &mut Realm, events: &mut Vec<SessionEvent>) {
    learn(realm, events, DIVINE_STRENGTH, 5);
    learn(realm, events, WRATHFUL_IRON, 5);
    learn(realm, events, HALLOWED_GROUND, 3);
    learn(realm, events, SPIRITUAL_FOCUS, 2);
}

#[test]
fn a_vanguard_at_the_cap_has_29_points_across_two_tabs() {
    let (mut realm, me, mut events) = vanguard(30);
    assert_eq!(points(&events, me), Some(29));
    // Not the Basic Warrior's tree any more.
    learn(&mut realm, &mut events, IMPROVED_HEROIC_STRIKE, 1);
    assert_eq!(points(&events, me), Some(29));
    // Hammer of the Vanguard sits in The Righteous' fourth row: 15 points there first.
    learn(&mut realm, &mut events, HAMMER_OF_THE_VANGUARD, 1);
    assert_eq!(points(&events, me), Some(29));
    learn(&mut realm, &mut events, DIVINE_STRENGTH, 5);
    assert_eq!(points(&events, me), Some(24));
}

#[test]
fn fifteen_points_in_righteous_reach_the_hammer() {
    let (mut realm, me, mut events) = vanguard(30);
    fifteen_in_righteous(&mut realm, &mut events);
    assert_eq!(points(&events, me), Some(14));
    learn(&mut realm, &mut events, HAMMER_OF_THE_VANGUARD, 1);
    assert_eq!(points(&events, me), Some(13));
    assert!(knows_any(&events, &HAMMER_RANKS));
}

#[test]
fn a_wip_talent_is_learnable_and_opens_the_next_tier() {
    let (mut realm, me, mut events) = vanguard(30);
    learn(&mut realm, &mut events, DIVINE_STRENGTH, 5);
    learn(&mut realm, &mut events, HALLOWED_GROUND, 3);
    learn(&mut realm, &mut events, WRATHFUL_IRON, 1);
    assert_eq!(points(&events, me), Some(20));
    learn(&mut realm, &mut events, RIGHTEOUS_FURY, 1);
    assert_eq!(
        points(&events, me),
        Some(20),
        "Righteous Fury's row needs 10"
    );
    // Wrathful Iron does nothing yet, but takes its point and counts toward the row.
    // Its second rank.
    realm.handle(RealmCommand::LearnTalent {
        talent_id: WRATHFUL_IRON,
        rank: 1,
    });
    events.extend(drain(&mut realm));
    assert_eq!(points(&events, me), Some(19));
    learn(&mut realm, &mut events, RIGHTEOUS_FURY, 1);
    assert_eq!(points(&events, me), Some(18));
}
