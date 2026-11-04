//! The Stockade (spec #575, ADR 0032): a Phased Dungeon on its own instance map, 34. Trigger 101
//! in the city's jail door loads the jail behind a loading screen, and the whole map is the
//! Phase; trigger 503 inside sends the character back out to the door, and with no one left the
//! Run resets. Dying inside and a save made inside both come out at the door. The Stockade is
//! dungeon 16 (`content/stockade.toml`).

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

/// `AreaTrigger.dbc` 101, "Stormwind Stockades - Entrance": a box in the city's jail door.
const ENTRANCE: u32 = 101;
const DOOR: [f32; 3] = [-8761.85, 848.557, 87.805];
/// `AreaTrigger.dbc` 503, "Stormwind Stockades - Exit": an 8 yd sphere inside, by the gate.
const EXIT: u32 = 503;
/// In the exit's sphere, on the gate hall's floor.
const AT_THE_EXIT: [f32; 3] = [45.0, 0.8, -16.4];

/// `Map.dbc` 34, and Eastern Kingdoms.
const STOCKADE: u32 = 34;
const EASTERN_KINGDOMS: u32 = 0;
/// Where the entrance puts the player inside, and the exit outside (`areatrigger_teleport`).
const ARRIVAL: [f32; 3] = [49.0, 0.5, -16.4];
const OUTSIDE: [f32; 3] = [-8766.1, 845.5, 88.0];
const OUTSIDE_FACING: f32 = 3.84;

/// The one creature of the shell's Run.
const PHASE_TESTER: u32 = 97100;

/// A level-30 character standing in the city's jail door.
fn at_the_door() -> Party {
    let mut party = Party::starting(Realm::with_seed(1), &[30], START_ELWYNN, Box::new(NoGround));
    party.go(0, DOOR, 0.66);
    party.run(0.5);
    party
}

/// What the client has been sent since its `from`th event.
fn since(party: &Party, from: usize) -> Vec<SessionEvent> {
    party.seen(0)[from..].to_vec()
}

/// Walks into the entrance and loads the jail, as the client does; the events from the ack on.
fn enter(party: &mut Party) -> Vec<SessionEvent> {
    party.go(0, DOOR, 0.66);
    party.cmd(0, RealmCommand::AreaTrigger { id: ENTRANCE });
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::WorldportAck);
    party.run(0.5);
    since(party, from)
}

fn in_phase(party: &Party) -> bool {
    party.realm.run_elapsed_as(party.sessions[0]).is_some()
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

/// Asserts that `events` worldport the player out to the jail door on Eastern Kingdoms.
fn out_at_the_door(events: &[SessionEvent]) {
    assert_eq!(transfer_pending(events), Some(EASTERN_KINGDOMS));
    let (map, at, facing, needs_ack) = worldport(events).expect("a worldport out");
    assert_eq!((map, needs_ack), (EASTERN_KINGDOMS, true));
    assert!(near(at, OUTSIDE), "{at:?}");
    assert_eq!(facing, OUTSIDE_FACING);
}

#[test]
fn walking_into_the_jail_door_loads_the_stockade_behind_a_loading_screen() {
    let mut party = at_the_door();
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::AreaTrigger { id: ENTRANCE });
    let events = since(&party, from);
    assert_eq!(transfer_pending(&events), Some(STOCKADE));
    let (map, at, facing, needs_ack) = worldport(&events).expect("a worldport into the jail");
    assert_eq!(map, STOCKADE);
    assert!(near(at, ARRIVAL), "{at:?}");
    assert_eq!(facing, 0.0);
    assert!(
        needs_ack,
        "the realm waits for the client to have loaded the jail"
    );

    // Until the client says the jail is loaded, nothing of it is sent.
    let from = party.seen(0).len();
    party.run(2.0);
    assert!(unit_creates(&since(&party, from)).is_empty());
    assert!(!in_phase(&party));
}

#[test]
fn a_trigger_claimed_from_far_away_does_nothing() {
    let mut party = at_the_door();
    party.go(0, OUTSIDE.map(|v| v + 40.0), 0.0);
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::AreaTrigger { id: ENTRANCE });
    assert_eq!(worldport(&since(&party, from)), None);
}

#[test]
fn after_the_ack_the_character_is_in_the_stockade_s_phase() {
    let mut party = at_the_door();
    assert!(
        !unit_creates(party.seen(0)).is_empty(),
        "the city's people are in view at the door"
    );
    let events = enter(&mut party);
    assert!(in_phase(&party));
    let entries: Vec<u32> = unit_creates(&events).iter().map(|u| u.1).collect();
    assert_eq!(
        entries,
        vec![PHASE_TESTER],
        "the Phase Tester comes, and none of the city's people"
    );
}

#[test]
fn the_exit_takes_the_character_out_of_the_door_and_the_run_resets() {
    let mut party = at_the_door();
    enter(&mut party);
    party.run(3.0);
    let first = party.realm.run_elapsed_as(party.sessions[0]);
    assert!(first.is_some_and(|s| s > 2.5), "{first:?}");

    party.go(0, AT_THE_EXIT, 3.14);
    assert!(in_phase(&party), "the gate hall is in the Phase");
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::AreaTrigger { id: EXIT });
    out_at_the_door(&since(&party, from));
    party.cmd(0, RealmCommand::WorldportAck);
    party.run(0.3);
    assert!(!in_phase(&party));

    // No one is left inside: the next entry starts a fresh Run.
    let events = enter(&mut party);
    assert!(
        unit_creates(&events).iter().any(|u| u.1 == PHASE_TESTER),
        "a fresh Phase Tester"
    );
    let second = party.realm.run_elapsed_as(party.sessions[0]);
    assert!(second.is_some_and(|s| s < 1.0), "{second:?}");
}

#[test]
fn dying_inside_and_releasing_comes_out_at_the_door() {
    let mut party = at_the_door();
    enter(&mut party);
    party.realm.kill_player();
    party.run(0.2);
    assert!(!in_phase(&party), "the dead are in no dungeon's Phase");
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::RepopRequest);
    out_at_the_door(&since(&party, from));
}

#[test]
fn a_save_made_inside_logs_back_in_at_the_door() {
    let mut party = at_the_door();
    enter(&mut party);
    let me = party.guids[0];
    let saved = party.realm.export_character(me).unwrap();
    assert!(saved.contains("map = 34\n"), "{saved}");

    // The save comes back into a realm with no Run, as after a crash.
    let mut realm = Realm::with_seed(2);
    realm.import_character(&saved).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    let events = drain(&mut realm);
    let (map, at, facing, needs_ack) = worldport(&events).expect("the login's worldport");
    assert_eq!((map, needs_ack), (EASTERN_KINGDOMS, false));
    assert!(near(at, OUTSIDE), "{at:?}");
    assert_eq!(facing, OUTSIDE_FACING);
}
