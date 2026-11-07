//! The Deeprun Tram's shell (spec #586, #587, ADR 0032 amended 2026-11-04, ADR 0033): an
//! instance map, 369 (`content/instance_maps.toml`), holding one Phased Dungeon, 18
//! (`content/deeprun_tram.toml`), the Stormwind station and the tunnel up to the mist wall.
//! Trigger 2173 in the Dwarven District's station hall loads the tram behind a loading screen and
//! trigger 2171 inside sends the character back out to the hall. The arrival point is open world;
//! a walk west onto the platform enters the Phase. The mist wall across the tunnel
//! (`data/twilights-eve/mist_walls.json`) refuses any step through it, and the station bell is a
//! lever whose pull is Run state. Ringing it starts the tram line (#589): empty cars roll in on
//! the timetable, the lamps and sounds warn of them, and a moving car hits a player in its lane.

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

// The tram line (#589, spec "The trams", ADR 0033): every number below is the `[tram]` table's in
// `content/deeprun_tram.toml`.

/// A tram car's create: vanilla's Subway (entry 176080), a `HIGHGUID_TRANSPORT` GameObject.
const CAR_ENTRY: u64 = 176080;
/// The tracks' x: track A between the west and centre strips, track B between the centre and
/// east strips.
const TRACK_A: f32 = -45.4;
const TRACK_B: f32 = 4.53;
/// The lamps at the platform's south end, a GameObject each (the entry is the guid).
const LAMP_A: u64 = 0xF110 << 48 | 983027;
/// BellTollDwarfGnome and SubwayStart.
const BELL_SOUND: u32 = 7234;
const WHISTLE_SOUND: u32 = 6757;
/// Tram Strike, the root a lane hit puts on.
const TRAM_STRIKE: u32 = 1900054;
/// A car rolls 150 yd in at 30 yd/s and stands 20 s at the platform.
const LEG_S: f32 = 5.0;
const DWELL_MS: u32 = 20_000;
/// On track A, where tram 1's single car passes; and on the centre strip beside it.
const ON_TRACK_A: [f32; 3] = [-45.4, 2470.0, -4.3];
const ON_THE_CENTRE_STRIP: [f32; 3] = [-30.0, 2470.0, -4.3];

fn is_car(guid: u64) -> bool {
    guid >> 48 == u64::from(benilla_protocol::guid::HIGH_TRANSPORT)
        && (guid >> 24) & 0xFF_FFFF == CAR_ENTRY
}

/// Rings the bell from beside it, with no tick after it: the line starts on the next tick.
fn ring_now(party: &mut Party) {
    party.cmd(0, RealmCommand::UseGameObject { guid: BELL });
}

/// Ticks `secs` in 0.1 s steps; each event each player's client got, with the seconds since the
/// start it came at. Run straight after [`ring_now`], the line's clock and these seconds agree.
fn ticks(party: &mut Party, secs: f32) -> Vec<Vec<(f32, SessionEvent)>> {
    let mut timed = vec![Vec::new(); party.sessions.len()];
    let steps = (secs * 10.0).round() as u32;
    for step in 1..=steps {
        let at = step as f32 / 10.0;
        for (i, events) in party.run(0.1).into_iter().enumerate() {
            timed[i].extend(events.into_iter().map(|e| (at, e)));
        }
    }
    timed
}

/// Rings the bell and ticks `secs`; what player 0's client got, timed.
fn after_the_bell(party: &mut Party, secs: f32) -> Vec<(f32, SessionEvent)> {
    ring_now(party);
    ticks(party, secs).swap_remove(0)
}

/// Whether `a` and `b` are the same moment, give or take a tick.
fn at(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.05
}

/// The tram cars created in `timed`: `(seconds, guid, position, transport_progress)`.
fn car_creates(timed: &[(f32, SessionEvent)]) -> Vec<(f32, u64, [f32; 3], Option<u32>)> {
    timed
        .iter()
        .filter_map(|(t, e)| match e {
            SessionEvent::ObjectCreate {
                guid,
                position,
                transport_progress,
                ..
            } if is_car(*guid) => Some((*t, *guid, *position, *transport_progress)),
            _ => None,
        })
        .collect()
}

/// When `guid` was removed in `timed`.
fn removed_at(timed: &[(f32, SessionEvent)], guid: u64) -> Option<f32> {
    timed.iter().find_map(|(t, e)| match e {
        SessionEvent::ObjectsRemoved(gone) if gone.contains(&guid) => Some(*t),
        _ => None,
    })
}

/// The trams in `timed`, by their cars' creates: `(arrival, cars, track x)`.
fn trams(timed: &[(f32, SessionEvent)]) -> Vec<(f32, usize, f32)> {
    let mut trams: Vec<(f32, usize, f32)> = Vec::new();
    for (t, _, position, _) in car_creates(timed) {
        match trams.last_mut() {
            Some(last) if at(last.0, t + LEG_S) => {
                last.1 += 1;
                assert_eq!(last.2, position[0], "one tram, one track");
            }
            _ => trams.push((t + LEG_S, 1, position[0])),
        }
    }
    trams
}

/// The sounds played on `guid` in `timed`: `(seconds, sound)`.
fn sounds_on(timed: &[(f32, SessionEvent)], guid: u64) -> Vec<(f32, u32)> {
    timed
        .iter()
        .filter_map(|(t, e)| match e {
            SessionEvent::PlayObjectSound { sound_id, guid: g } if *g == guid => {
                Some((*t, *sound_id))
            }
            _ => None,
        })
        .collect()
}

/// When `guid` was created, and when removed, in `timed`, in order.
fn shown_and_gone(timed: &[(f32, SessionEvent)], guid: u64) -> Vec<(f32, bool)> {
    timed
        .iter()
        .filter_map(|(t, e)| match e {
            SessionEvent::ObjectCreate { guid: g, .. } if *g == guid => Some((*t, true)),
            SessionEvent::ObjectsRemoved(gone) if gone.contains(&guid) => Some((*t, false)),
            _ => None,
        })
        .collect()
}

/// Players `levels.len()` level-38 characters, each walked into the tram and onto the platform
/// beside the bell.
fn by_the_bell(levels: &[u8]) -> Party {
    let mut party = Party::edited(
        Realm::with_seed(1),
        levels,
        START_ELWYNN,
        Box::new(NoGround),
        |_, blob| blob,
    );
    for i in 0..levels.len() {
        party.go(i, STATION, 0.0);
        party.run(0.2);
        party.cmd(i, RealmCommand::AreaTrigger { id: ENTRANCE });
        party.cmd(i, RealmCommand::WorldportAck);
        party.run(0.2);
        party.go(i, BY_THE_BELL, 0.0);
        party.run(0.2);
    }
    assert!(party.realm.run_elapsed_as(party.sessions[0]).is_some());
    party
}

#[test]
fn with_every_tram_cleared_the_trams_come_faster_and_longer_then_the_boss_tram() {
    let mut party = by_the_bell(&[38]);
    let timed = after_the_bell(&mut party, 240.0);
    let trams = trams(&timed);
    let want = [
        (10.0, 1, TRACK_A),
        (50.0, 2, TRACK_B),
        (75.0, 3, TRACK_A),
        (100.0, 3, TRACK_B),
        (125.0, 3, TRACK_A),
        (150.0, 3, TRACK_B),
        (175.0, 1, TRACK_A),
    ];
    assert_eq!(trams.len(), want.len(), "{trams:?}");
    for (got, want) in trams.iter().zip(want) {
        assert!(at(got.0, want.0), "arrives at {}: {got:?}", want.0);
        assert_eq!((got.1, got.2), (want.1, want.2), "{got:?}");
    }
}

#[test]
fn a_car_s_route_comes_before_its_create_and_the_car_is_gone_at_its_period_s_end() {
    let mut party = by_the_bell(&[38]);
    let timed = after_the_bell(&mut party, 105.0);
    let mut cars = car_creates(&timed);
    // Trams 1 to 3, which have backed out by now; tram 4 is still rolling.
    cars.truncate(6);
    assert!(cars.iter().all(|c| c.0 < 75.0), "{cars:?}");
    let mut guids: Vec<u64> = cars.iter().map(|c| c.1).collect();
    guids.sort_unstable();
    guids.dedup();
    assert_eq!(guids.len(), 6, "no car's guid is used twice");
    for &(t, guid, position, progress) in &cars {
        let create = timed
            .iter()
            .position(
                |(_, e)| matches!(e, SessionEvent::ObjectCreate { guid: g, .. } if *g == guid),
            )
            .unwrap();
        let route = timed[..create].iter().rev().find_map(|(_, e)| match e {
            SessionEvent::CartRoute {
                wagon_guid,
                points,
                backs_out,
                dwell_ms,
                period_ms,
                map,
                ..
            } if *wagon_guid == guid => {
                Some((points.clone(), *backs_out, *dwell_ms, *period_ms, *map))
            }
            _ => None,
        });
        let (points, backs_out, dwell_ms, period_ms, map) = route.expect("its route first");
        assert!(backs_out, "a car never turns round");
        assert_eq!(map, TRAM);
        assert_eq!(points.len(), 2);
        assert!(points.iter().all(|p| p[0] == position[0]), "{points:?}");
        assert!([TRACK_A, TRACK_B].contains(&position[0]));
        assert!(
            (points[1][1] - points[0][1] - 150.0).abs() < 0.01,
            "{points:?}"
        );
        assert!(near(points[0], position), "it starts at its route's start");
        assert_eq!(progress, Some(DWELL_MS), "it rolls at once");
        assert_eq!(dwell_ms, DWELL_MS);
        assert_eq!(period_ms, 2 * (5_000 + DWELL_MS));
        let gone = removed_at(&timed, guid).expect("removed");
        assert!(at(gone, t + 30.0), "removed at {gone}, made at {t}");
    }
}

#[test]
fn the_bell_and_the_lamp_warn_of_the_arrival_and_the_whistle_of_the_departure() {
    let mut party = by_the_bell(&[38]);
    let timed = after_the_bell(&mut party, 40.0);
    let sounds = sounds_on(&timed, LAMP_A);
    assert_eq!(sounds.len(), 2, "{sounds:?}");
    assert!(
        at(sounds[0].0, 5.0) && sounds[0].1 == BELL_SOUND,
        "{sounds:?}"
    );
    assert!(
        at(sounds[1].0, 27.0) && sounds[1].1 == WHISTLE_SOUND,
        "{sounds:?}"
    );
    let lamp = shown_and_gone(&timed, LAMP_A);
    assert_eq!(lamp.len(), 4, "{lamp:?}");
    let want = [(5.0, true), (10.0, false), (27.0, true), (30.0, false)];
    for (got, want) in lamp.iter().zip(want) {
        assert!(at(got.0, want.0) && got.1 == want.1, "{lamp:?}");
    }
    // The bell rings on a lamp the client already holds.
    let created = timed
        .iter()
        .position(|(_, e)| matches!(e, SessionEvent::ObjectCreate { guid, .. } if *guid == LAMP_A))
        .unwrap();
    let rang = timed
        .iter()
        .position(|(_, e)| matches!(e, SessionEvent::PlayObjectSound { .. }))
        .unwrap();
    assert!(created < rang);
}

/// The Tram Strike hits on `guid` in `timed`: `(seconds, damage)`.
fn strikes(timed: &[(f32, SessionEvent)], guid: u64) -> Vec<(f32, u32)> {
    timed
        .iter()
        .filter_map(|(t, e)| match e {
            SessionEvent::SpellDamageLog(l) if l.target == guid && l.spell_id == TRAM_STRIKE => {
                Some((*t, l.damage))
            }
            _ => None,
        })
        .collect()
}

/// The roots put on `guid` in `timed`, by when.
fn roots(timed: &[(f32, SessionEvent)], guid: u64) -> Vec<f32> {
    timed
        .iter()
        .filter_map(|(t, e)| match e {
            SessionEvent::MoveMode {
                guid: g,
                mode: benilla_protocol::messages::MoveMode::Root,
                apply: true,
                ..
            } if *g == guid => Some(*t),
            _ => None,
        })
        .collect()
}

/// The last maximum health player `i`'s client was shown for `guid`.
fn max_health(party: &Party, i: usize, guid: u64) -> u32 {
    party
        .seen(i)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid: g, fields, ..
            }
            | SessionEvent::ObjectValues { guid: g, fields }
                if *g == guid =>
            {
                fields.unit_max_health()
            }
            _ => None,
        })
        .last()
        .expect("a maximum health")
}

#[test]
fn a_car_hits_a_player_in_its_lane_once_each_way_and_not_one_on_the_centre_strip() {
    let mut party = by_the_bell(&[38, 38]);
    let (on_track, beside) = (party.guids[0], party.guids[1]);
    ring_now(&mut party);
    party.go(0, ON_TRACK_A, 0.0);
    party.go(1, ON_THE_CENTRE_STRIP, 0.0);
    let mut timed = ticks(&mut party, 34.0);
    let theirs = timed.pop().unwrap();
    let timed = timed.pop().unwrap();
    let max = max_health(&party, 0, on_track);
    let hit = strikes(&timed, on_track);
    assert_eq!(hit.len(), 2, "once in and once out: {hit:?}");
    assert!(
        hit.iter().all(|&(_, damage)| damage == max * 40 / 100),
        "{hit:?} of {max}"
    );
    assert!(hit[0].0 > 5.0 && hit[0].0 < 10.0, "rolling in: {hit:?}");
    assert!(hit[1].0 > 30.0, "backing out: {hit:?}");
    let rooted = roots(&timed, on_track);
    assert_eq!(rooted.len(), 2, "{rooted:?}");
    assert!(
        rooted.iter().zip(&hit).all(|(r, h)| at(*r, h.0)),
        "{rooted:?}"
    );

    assert!(
        strikes(&theirs, beside).is_empty(),
        "the centre strip is no lane"
    );
    assert!(roots(&theirs, beside).is_empty());
}
