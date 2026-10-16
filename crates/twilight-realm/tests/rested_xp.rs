//! Rested XP (spec #28): the Crossroads inn is a rest area. Time logged out in it, or spent
//! standing in it, banks rested XP at vanilla's rate up to vanilla's cap, and a kill pays its XP
//! again out of the pool until it runs out. Played through the realm's front door, on its wall
//! clock.

mod common;
mod gear_common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::{stand_at, Mob};
use common::*;
use gear_common::{save_with, SEED};
use twilight_realm::CROSSROADS;

const PLAYER_FLAGS: u16 = 190;
const PLAYER_BYTES_2: u16 = 194;
const PLAYER_REST_STATE_EXPERIENCE: u16 = 1175;
const PLAYER_FLAGS_RESTING: u32 = 0x20;
const REST_STATE_RESTED: u32 = 0x01;
const REST_STATE_NORMAL: u32 = 0x02;

const HOUR_MS: u64 = 3_600_000;
/// Some evening in 2026, on the host's clock.
const EVENING_MS: u64 = 1_790_000_000_000;
/// Far out past The Barrens' camps, nowhere near an inn.
const FAR_AWAY: [f32; 3] = [CROSSROADS[0] + 3000.0, CROSSROADS[1], 0.0];
/// Level 5 kills level 3: 70 × 3/5 = 42 XP at ×1.
const KILL_XP: u32 = 42;

#[test]
fn eight_hours_logged_out_in_the_inn_bank_five_percent_of_a_level() {
    let mut realm = realm();
    realm.set_wall_clock(EVENING_MS);
    let (me, events) = enter_new(&mut realm, "Thrall", ORC);
    assert_eq!(rest_shown(&events, me), Some((0, REST_STATE_NORMAL)));
    let events = away(&mut realm, me, 8);
    // Level 1's 400 XP × 5%: 28800 s × 400 / 72000 × 0.125.
    assert_eq!(rest_shown(&events, me), Some((20, REST_STATE_RESTED)));
}

#[test]
fn eight_hours_logged_out_outside_bank_a_quarter_as_much() {
    let mut realm = realm();
    realm.set_wall_clock(EVENING_MS);
    let (me, _) = enter_new(&mut realm, "Thrall", ORC);
    walk_to(&mut realm, FAR_AWAY);
    let events = away(&mut realm, me, 8);
    // 28800 s × 400 / 72000 × 0.031 = 4.96.
    assert_eq!(rest_shown(&events, me), Some((4, REST_STATE_RESTED)));
}

#[test]
fn the_pool_stops_at_three_quarters_of_a_level() {
    let mut realm = realm();
    realm.set_wall_clock(EVENING_MS);
    let (me, _) = enter_new(&mut realm, "Thrall", ORC);
    let events = away(&mut realm, me, 24 * 30);
    assert_eq!(rest_shown(&events, me), Some((300, REST_STATE_RESTED)));
}

#[test]
fn standing_in_the_inn_rests_and_fills_the_pool() {
    let mut realm = realm();
    let (me, events) = enter_new(&mut realm, "Thrall", ORC);
    assert_eq!(flags(&events, me), Some(PLAYER_FLAGS_RESTING));
    let events = idle(&mut realm, 3600);
    // An hour: 3600 × 400 / 72000 × 0.125 = 2.5.
    assert_eq!(rest_shown(&events, me), Some((2, REST_STATE_RESTED)));
}

#[test]
fn standing_outside_rests_nothing() {
    let mut realm = realm();
    let (me, _) = enter_new(&mut realm, "Thrall", ORC);
    let events = walk_to(&mut realm, FAR_AWAY);
    assert_eq!(flags(&events, me), Some(0));
    let events = idle(&mut realm, 3600);
    assert_eq!(rest_shown(&events, me), None);
}

#[test]
fn a_rested_kill_pays_double_and_spends_the_pool() {
    let (mut realm, me, events) = in_world_as("basic_warrior", 100);
    assert_eq!(rest_shown(&events, me), Some((100, REST_STATE_RESTED)));
    let events = kill(&mut realm, first_mob(&events));
    assert_eq!(kill_xp(&events), (2 * KILL_XP, KILL_XP));
    assert_eq!(
        rest_shown(&events, me),
        Some((100 - KILL_XP, REST_STATE_RESTED))
    );
}

#[test]
fn the_rest_state_returns_to_normal_when_the_pool_runs_out() {
    let (mut realm, me, events) = in_world_as("basic_warrior", 30);
    let events = kill(&mut realm, first_mob(&events));
    assert_eq!(kill_xp(&events), (KILL_XP + 30, KILL_XP));
    assert_eq!(rest_shown(&events, me), Some((0, REST_STATE_NORMAL)));
}

#[test]
fn rested_xp_doubles_the_tier_xp_rate() {
    let (mut realm, _, events) = in_world_as("vanguard", 1000);
    let events = kill(&mut realm, first_mob(&events));
    assert_eq!(kill_xp(&events), (2 * 3 * KILL_XP, 3 * KILL_XP));
}

#[test]
fn a_save_without_a_pool_loads_with_none() {
    let (guid, blob) = save_with(5, 0, &[]);
    assert!(!blob.contains("rested"), "{blob}");
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    assert_eq!(rest_shown(&events, guid), Some((0, REST_STATE_NORMAL)));
}

// ── Helpers ─────────────────────────────────────────────────────────────────────────────────────

/// A level 5 `calling` whose save holds `rested` XP, in the world at the Hub, with the entry burst.
fn in_world_as(calling: &str, rested: u32) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with(5, 0, &[]);
    let blob = blob
        .replace(
            "calling = \"basic_warrior\"",
            &format!("calling = \"{calling}\""),
        )
        .replace(
            "left_in_inn = false\n",
            &format!("left_in_inn = false\nrested = {rested}.0\n"),
        );
    let mut realm = Realm::with_seed(SEED);
    realm
        .import_character(&blob)
        .unwrap_or_else(|e| panic!("{e:?}: {blob}"));
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

/// Logs out, moves the wall clock on `hours`, and logs back in: the entry burst.
fn away(realm: &mut Realm, me: u64, hours: u64) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::Logout);
    drain(realm);
    realm.set_wall_clock(EVENING_MS + hours * HOUR_MS);
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    drain(realm)
}

fn walk_to(realm: &mut Realm, position: [f32; 3]) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::Move {
        position,
        orientation: 0.0,
    });
    realm.tick(0.1, &FlatGround);
    drain(realm)
}

/// Stands still `secs` seconds, a second a tick, collecting the events.
fn idle(realm: &mut Realm, secs: u32) -> Vec<SessionEvent> {
    let mut events = Vec::new();
    for _ in 0..secs {
        realm.tick(1.0, &FlatGround);
        events.extend(drain(realm));
    }
    events
}

/// The value the player's own fields last showed at `index`, create or update.
fn last_field(events: &[SessionEvent], me: u64, index: u16) -> Option<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate { guid, fields, .. }
            | SessionEvent::ObjectValues { guid, fields }
                if *guid == me =>
            {
                fields
                    .raw_fields()
                    .find_map(|(i, v)| (i == index).then_some(v))
            }
            _ => None,
        })
        .next_back()
}

/// The pool and the rest state, as the player's fields last showed them.
fn rest_shown(events: &[SessionEvent], me: u64) -> Option<(u32, u32)> {
    let pool = last_field(events, me, PLAYER_REST_STATE_EXPERIENCE)?;
    let state = last_field(events, me, PLAYER_BYTES_2)? >> 24;
    Some((pool, state))
}

fn flags(events: &[SessionEvent], me: u64) -> Option<u32> {
    last_field(events, me, PLAYER_FLAGS)
}

/// The kill's XP as the combat log reports it: `(total, without the rest)`.
fn kill_xp(events: &[SessionEvent]) -> (u32, u32) {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::XpGain(x) if x.kill => Some((x.total, x.base)),
            _ => None,
        })
        .expect("the kill pays XP")
}

/// The first mob to stream in with the world: a level 3 scorpid by the Crossroads.
fn first_mob(events: &[SessionEvent]) -> Mob {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                ..
            } => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .expect("a mob streams in with the world")
}

/// Stands by `mob` and auto-attacks it until it dies, collecting the events.
fn kill(realm: &mut Realm, mob: Mob) -> Vec<SessionEvent> {
    stand_at(realm, &mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let mut events = Vec::new();
    for _ in 0..1200 {
        realm.tick(0.1, &FlatGround);
        events.extend(drain(realm));
        let dead = events.iter().any(|e| matches!(e,
            SessionEvent::ObjectValues { guid, fields } if *guid == mob.guid && fields.unit_health() == Some(0)));
        if dead {
            return events;
        }
    }
    panic!("the mob outlived two minutes of fighting");
}
