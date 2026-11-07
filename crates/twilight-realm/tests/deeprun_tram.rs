//! The Deeprun Tram's shell (spec #586, #587, ADR 0032 amended 2026-11-04, ADR 0033): an
//! instance map, 369 (`content/instance_maps.toml`), holding one Phased Dungeon, 18
//! (`content/deeprun_tram.toml`), the Stormwind station and the tunnel up to the mist wall.
//! Trigger 2173 in the Dwarven District's station hall loads the tram behind a loading screen and
//! trigger 2171 inside sends the character back out to the hall. The arrival point is open world;
//! a walk west onto the platform enters the Phase. The mist wall across the tunnel
//! (`data/twilights-eve/mist_walls.json`) refuses any step through it, and the station bell is a
//! lever whose pull is Run state.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

/// `AreaTrigger.dbc` 2173, "Deeprun Tram Stormwind - Entrance": a 10 yd sphere in the station
/// hall.
const ENTRANCE: u32 = 2173;
const STATION: [f32; 3] = [-8346.46, 514.031, 96.5989];
/// `AreaTrigger.dbc` 2171, "Deeprun Tram Stormwind - Exit": a 10 yd sphere inside.
const EXIT: u32 = 2171;
/// In the exit's sphere, on the floor at the foot of the stairs.
const AT_THE_EXIT: [f32; 3] = [78.0, 2490.9, -4.29649];

/// `Map.dbc` 369, and Eastern Kingdoms.
const TRAM: u32 = 369;
const EASTERN_KINGDOMS: u32 = 0;
/// Where the entrance puts the player inside, and the exit outside (`areatrigger_teleport`).
const ARRIVAL: [f32; 3] = [67.7607, 2490.98, -4.29649];
const ARRIVAL_FACING: f32 = 3.14159;
const OUTSIDE: [f32; 3] = [-8354.23, 524.068, 91.7971];
const OUTSIDE_FACING: f32 = 2.35619;

/// On the platform, inside the footprint: the probe's drop.
const PLATFORM: [f32; 3] = [30.0, 2491.0, -4.29649];

/// On track A, the wall's side and short of it, and a step through it.
const SHORT_OF_THE_WALL: [f32; 3] = [-45.4, 2410.0, -4.3];
const AT_THE_WALL: [f32; 3] = [-45.4, 2405.0, -4.3];
const PAST_THE_WALL: [f32; 3] = [-45.4, 2395.0, -4.3];
const REFUSED: &str = "You cannot go that way.";

/// The station bell, a lever (the entry is the guid), what ringing it says, and a spot on the
/// centre strip in its reach.
const BELL: u64 = 0xF110 << 48 | 983012;
const RUNG: &str = "The bell rings down the tunnel. Something far off answers it.";
const BY_THE_BELL: [f32; 3] = [-18.5, 2486.0, -4.29649];

/// A level-38 character standing in the station hall.
fn at_the_station() -> Party {
    let mut party = Party::edited(
        Realm::with_seed(1),
        &[38],
        START_ELWYNN,
        Box::new(NoGround),
        |_, blob| blob,
    );
    party.go(0, STATION, 0.0);
    party.run(0.5);
    party
}

/// What the client has been sent since its `from`th event.
fn since(party: &Party, from: usize) -> Vec<SessionEvent> {
    party.seen(0)[from..].to_vec()
}

/// Walks into the entrance and loads the tram, as the client does; the events from the ack on.
fn enter(party: &mut Party) -> Vec<SessionEvent> {
    party.go(0, STATION, 0.0);
    party.cmd(0, RealmCommand::AreaTrigger { id: ENTRANCE });
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::WorldportAck);
    party.run(0.5);
    since(party, from)
}

/// Walks to `at` and lets the realm tick; the events that came.
fn walk(party: &mut Party, at: [f32; 3]) -> Vec<SessionEvent> {
    let from = party.seen(0).len();
    party.go(0, at, 0.0);
    party.run(0.5);
    since(party, from)
}

fn in_phase(party: &Party) -> bool {
    party.realm.run_elapsed_as(party.sessions[0]).is_some()
}

/// Whether the client holds the GameObject `guid`, by what it has been sent.
fn shows(party: &Party, guid: u64) -> bool {
    let mut shown = false;
    for e in party.seen(0) {
        match e {
            SessionEvent::ObjectCreate { guid: g, .. } if *g == guid => shown = true,
            SessionEvent::ObjectsRemoved(gone) if gone.contains(&guid) => shown = false,
            SessionEvent::ObjectDestroyed(g) if *g == guid => shown = false,
            _ => {}
        }
    }
    shown
}

/// The worldport in `events`: `(map, position, orientation, needs_ack)`.
fn worldport(events: &[SessionEvent]) -> Option<(u32, [f32; 3], f32, bool)> {
    events.iter().find_map(|e| match e {
        SessionEvent::Worldport {
            map_id,
            position,
            orientation,
            needs_ack,
        } => Some((*map_id, *position, *orientation, *needs_ack)),
        _ => None,
    })
}

fn transfer_pending(events: &[SessionEvent]) -> Option<u32> {
    events.iter().find_map(|e| match e {
        SessionEvent::TransferPending { map_id, .. } => Some(*map_id),
        _ => None,
    })
}

fn near(a: [f32; 3], b: [f32; 3]) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 0.01)
}

/// Asserts that `events` worldport the player out to the station hall on Eastern Kingdoms.
fn out_at_the_station(events: &[SessionEvent]) {
    assert_eq!(transfer_pending(events), Some(EASTERN_KINGDOMS));
    let (map, at, facing, needs_ack) = worldport(events).expect("a worldport out");
    assert_eq!((map, needs_ack), (EASTERN_KINGDOMS, true));
    assert!(near(at, OUTSIDE), "{at:?}");
    assert_eq!(facing, OUTSIDE_FACING);
}

/// The last teleport in `events`.
fn last_teleport(events: &[SessionEvent]) -> Option<[f32; 3]> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Teleport { position, .. } => Some(*position),
            _ => None,
        })
        .last()
}

/// The notifications saying `said` in `events`.
fn notes(events: &[SessionEvent], said: &str) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, SessionEvent::Notification { text } if text == said))
        .count()
}

/// Where the character stands, by the realm's record.
fn standing(party: &Party) -> [f32; 3] {
    party.realm.player_positions()[0].1
}

/// Rings the bell from beside it; the events that came.
fn ring(party: &mut Party) -> Vec<SessionEvent> {
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::UseGameObject { guid: BELL });
    party.run(0.3);
    since(party, from)
}

#[test]
fn walking_into_the_station_trigger_loads_the_tram_behind_a_loading_screen() {
    let mut party = at_the_station();
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::AreaTrigger { id: ENTRANCE });
    let events = since(&party, from);
    assert_eq!(transfer_pending(&events), Some(TRAM));
    let (map, at, facing, needs_ack) = worldport(&events).expect("a worldport into the tram");
    assert_eq!(map, TRAM);
    assert!(near(at, ARRIVAL), "{at:?}");
    assert_eq!(facing, ARRIVAL_FACING);
    assert!(
        needs_ack,
        "the realm waits for the client to have loaded the tram"
    );
}

#[test]
fn the_exit_takes_the_character_back_to_the_station_hall() {
    let mut party = at_the_station();
    enter(&mut party);
    walk(&mut party, AT_THE_EXIT);
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::AreaTrigger { id: EXIT });
    out_at_the_station(&since(&party, from));
    party.cmd(0, RealmCommand::WorldportAck);
    party.run(0.3);
    assert!(!in_phase(&party));
}

#[test]
fn the_arrival_is_open_world_and_the_platform_is_the_tram_s_phase() {
    let mut party = at_the_station();
    enter(&mut party);
    assert!(!in_phase(&party), "the arrival point is open world");
    assert!(!shows(&party, BELL), "the bell outside the Phase");

    walk(&mut party, PLATFORM);
    assert!(in_phase(&party), "the platform is the tram's");
    assert!(
        shows(&party, BELL),
        "the bell, which stands only in dungeon 18's Phase"
    );
}

#[test]
fn dying_on_the_platform_and_releasing_comes_out_at_the_station() {
    let mut party = at_the_station();
    enter(&mut party);
    walk(&mut party, PLATFORM);
    assert!(in_phase(&party));
    party.realm.kill_player();
    party.run(0.2);
    assert!(!in_phase(&party), "the dead are in no dungeon's Phase");
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::RepopRequest);
    out_at_the_station(&since(&party, from));
}

#[test]
fn the_mist_wall_refuses_a_step_through_it_and_not_one_short_of_it() {
    let mut party = at_the_station();
    enter(&mut party);
    walk(&mut party, PLATFORM);
    let events = walk(&mut party, SHORT_OF_THE_WALL);
    assert_eq!(notes(&events, REFUSED), 0);

    let events = walk(&mut party, AT_THE_WALL);
    assert_eq!(notes(&events, REFUSED), 0, "a step short of the wall");
    assert_eq!(last_teleport(&events), None);
    assert!(near(standing(&party), AT_THE_WALL));
    assert!(in_phase(&party));

    let events = walk(&mut party, PAST_THE_WALL);
    assert_eq!(notes(&events, REFUSED), 1, "{events:?}");
    assert_eq!(last_teleport(&events), Some(AT_THE_WALL), "snapped back");
    assert!(
        near(standing(&party), AT_THE_WALL),
        "{:?}",
        standing(&party)
    );
}

#[test]
fn ringing_the_bell_is_run_state_and_the_run_s_reset_clears_it() {
    let mut party = at_the_station();
    enter(&mut party);
    walk(&mut party, BY_THE_BELL);
    assert!(in_phase(&party));
    assert_eq!(notes(&ring(&mut party), RUNG), 1, "the first ring");
    assert_eq!(notes(&ring(&mut party), RUNG), 0, "rung twice in one Run");

    // The last living player leaves the footprint: the Run resets.
    walk(&mut party, ARRIVAL);
    assert!(!in_phase(&party));
    party.run(1.0);
    walk(&mut party, BY_THE_BELL);
    let fresh = party.realm.run_elapsed_as(party.sessions[0]);
    assert!(fresh.is_some_and(|s| s < 1.0), "{fresh:?}");
    assert_eq!(notes(&ring(&mut party), RUNG), 1, "the bell stayed rung");
}
