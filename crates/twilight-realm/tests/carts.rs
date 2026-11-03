//! Carts in the realm (#564, spec #560) on the shipped `content/carts.toml`: the wagon's
//! timetable pose, the wagon, horse and driver it streams, its seats, a rider's stealth and the
//! map's stop marks.

mod common;

use benilla_protocol::{MapIcon, MapMark, SessionEvent, START_ELWYNN};
use common::*;
use twilight_realm::carts::{self, CartState, Route};
use twilight_realm::{Audience, Realm, RealmCommand};

const KOBOLD_VERMIN: u32 = 6;

fn first() -> &'static Route {
    &carts::routes()[0]
}

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// A realm whose catalogue sells Carts instead of giving it free: nothing is bought, so it is
/// locked. Carts is the catalogue's last row.
fn locked() -> Realm {
    let text = include_str!("../content/unlocks.toml");
    let (head, tail) = text.rsplit_once("free = true").expect("carts is free");
    realm().with_unlock_catalogue(&format!("{head}free = false{tail}"))
}

/// Makes Elwynn warriors named `names` and returns their guids.
fn elwynn(realm: &mut Realm, names: &[&str]) -> Vec<u64> {
    for name in names {
        let RealmCommand::CreateCharacter(mut req) = create_cmd(name, HUMAN, 0, WARRIOR) else {
            unreachable!()
        };
        req.start = START_ELWYNN;
        realm.handle(RealmCommand::CreateCharacter(req));
    }
    let list = roster(realm);
    names
        .iter()
        .map(|n| list.iter().find(|c| c.name == *n).expect("created").guid)
        .collect()
}

/// A realm at wall time `wall_ms` with one Elwynn warrior in the world beside the first route's
/// first stop.
fn at_the_stop(mut realm: Realm, wall_ms: u64) -> (Realm, u64, Vec<SessionEvent>) {
    let me = elwynn(&mut realm, &["Rider"])[0];
    realm.set_wall_clock(wall_ms);
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    let mut events = drain(&mut realm);
    go(&mut realm, first().stops[0].position, 2.0);
    events.extend(drain(&mut realm));
    (realm, me, events)
}

fn go(realm: &mut Realm, at: [f32; 3], east: f32) {
    realm.handle(RealmCommand::DevTeleport {
        x: at[0],
        y: at[1] - east,
        facing: None,
    });
    realm.tick(0.0, &FlatGround);
}

fn lines(events: &[SessionEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Chat(m) => Some(m.text.clone()),
            _ => None,
        })
        .collect()
}

fn seated(events: &[SessionEvent]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::Seated { .. }))
}

fn use_wagon(realm: &mut Realm) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::UseGameObject {
        guid: first().wagon_guid(),
    });
    drain(realm)
}

fn marks(events: &[SessionEvent]) -> Vec<MapMark> {
    events
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::MapMarks { marks } => Some(marks.clone()),
            _ => None,
        })
        .expect("map marks")
}

#[test]
fn the_wagon_stands_rolls_stands_and_comes_back_on_its_timetable() {
    let r = first();
    assert_eq!(r.period_ms, 2 * (r.leg_ms + r.dwell_ms));
    let start = carts::pose(r, 0);
    assert_eq!(start.state, CartState::Standing(0));
    assert!(dist(start.point, r.path[0]) < 0.01);

    let half = carts::pose(r, r.dwell_ms + r.leg_ms / 2);
    assert_eq!(half.state, CartState::Rolling);
    let to_start = dist(half.point, r.path[0]);
    let to_end = dist(half.point, r.path[r.path.len() - 1]);
    assert!(to_start > 50.0 && to_end > 50.0, "{half:?} is on the way");
    let on_path = r
        .path
        .iter()
        .map(|p| dist(*p, half.point))
        .fold(f32::MAX, f32::min);
    assert!(on_path < 8.0, "{half:?} is {on_path} yd off the path");

    let last = r.stops.len() - 1;
    let end = carts::pose(r, r.dwell_ms + r.leg_ms);
    assert_eq!(end.state, CartState::Standing(last));
    assert!(dist(end.point, r.path[r.path.len() - 1]) < 0.01);

    assert_eq!(carts::pose(r, r.period_ms), start);
}

#[test]
fn login_streams_each_route_then_its_wagon_horse_and_driver_on_their_transport() {
    let (_realm, _, events) = at_the_stop(realm(), 1_000);
    let on_map: Vec<&Route> = carts::routes().iter().filter(|r| r.map == 0).collect();
    let routes: Vec<u64> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::CartRoute {
                wagon_guid, map, ..
            } => Some((*wagon_guid, *map)),
            _ => None,
        })
        .map(|(g, map)| {
            assert_eq!(map, 0);
            g
        })
        .collect();
    assert_eq!(
        routes.len(),
        on_map.len(),
        "one CartRoute per route on map 0"
    );
    for r in on_map {
        let at = |guid: u64| {
            events
                .iter()
                .position(|e| matches!(e, SessionEvent::ObjectCreate { guid: g, .. } if *g == guid))
                .unwrap_or_else(|| panic!("{} streams {guid:#x}", r.key))
        };
        let route_at = events
            .iter()
            .position(|e| matches!(e, SessionEvent::CartRoute { wagon_guid, .. } if *wagon_guid == r.wagon_guid()))
            .unwrap();
        assert!(
            route_at < at(r.wagon_guid()),
            "the route comes before the wagon"
        );
        let SessionEvent::ObjectCreate {
            transport_progress, ..
        } = &events[at(r.wagon_guid())]
        else {
            unreachable!()
        };
        assert!(transport_progress.is_some(), "the wagon is a transport");
        for unit in [r.horse_guid(), r.driver_guid()] {
            let SessionEvent::ObjectCreate { transport, .. } = &events[at(unit)] else {
                unreachable!()
            };
            assert_eq!(transport.map(|t| t.guid), Some(r.wagon_guid()));
        }
    }
}

#[test]
fn a_seat_drops_threat_refuses_a_swing_and_a_move_stands_the_rider_up_beside_the_wagon() {
    let (mut realm, me, _) = at_the_stop(realm(), 1_000);
    let stop = first().stops[0].position;
    let kobold = realm
        .spawn_for(
            &FlatGround,
            KOBOLD_VERMIN,
            2,
            [stop[0] + 8.0, stop[1], 0.0],
            0,
            Audience::of(&[me]),
        )
        .expect("a kobold");
    realm.add_creature_threat(kobold, me, 100.0);
    assert!(realm.creature_threat(kobold, me) > 0.0);

    let events = use_wagon(&mut realm);
    assert!(seated(&events), "{events:?}");
    assert_eq!(realm.creature_threat(kobold, me), 0.0);

    realm.handle(RealmCommand::AttackSwing { guid: kobold });
    assert!(lines(&drain(&mut realm)).contains(&"You are riding.".to_string()));

    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: stop,
        orientation: 0.0,
    });
    realm.tick(0.1, &FlatGround);
    let events = drain(&mut realm);
    let position = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::Unseated { position } => Some(*position),
            _ => None,
        })
        .expect("Unseated");
    let wagon = carts::pose(first(), 1_100).point;
    let beside = dist(position, wagon);
    assert!((1.5..=2.5).contains(&beside), "{beside} yd from the wagon");
}

#[test]
fn a_rolling_or_distant_wagon_refuses_a_seat() {
    let r = first();
    let (mut rolling, _, _) = at_the_stop(realm(), r.dwell_ms + 1_000);
    let events = use_wagon(&mut rolling);
    assert!(!seated(&events));
    assert!(lines(&events).contains(&"The cart is moving.".to_string()));

    let (mut far, _, _) = at_the_stop(realm(), 1_000);
    go(&mut far, first().stops[0].position, 30.0);
    drain(&mut far);
    let events = use_wagon(&mut far);
    assert!(!seated(&events));
    assert!(lines(&events).contains(&"You are too far away.".to_string()));
}

#[test]
fn a_sixth_rider_finds_the_cart_full() {
    let mut realm = realm();
    let names = ["Anna", "Bert", "Carl", "Dora", "Emil", "Fern"];
    let guids = elwynn(&mut realm, &names);
    realm.set_wall_clock(1_000);
    let mut sessions = Vec::new();
    for guid in guids {
        let s = realm.connect();
        realm.handle_as(s, RealmCommand::EnterWorld { guid });
        let at = first().stops[0].position;
        realm.handle_as(
            s,
            RealmCommand::DevTeleport {
                x: at[0],
                y: at[1] - 2.0,
                facing: None,
            },
        );
        sessions.push(s);
    }
    realm.tick(0.0, &FlatGround);
    realm.drain_session_events();
    for &s in &sessions {
        realm.handle_as(
            s,
            RealmCommand::UseGameObject {
                guid: first().wagon_guid(),
            },
        );
    }
    let events = realm.drain_session_events();
    let of = |s| -> Vec<SessionEvent> {
        events
            .iter()
            .filter(|(id, _)| *id == s)
            .map(|(_, e)| e.clone())
            .collect()
    };
    for &s in &sessions[..5] {
        assert!(seated(&of(s)), "{s:?} sits");
    }
    let last = of(sessions[5]);
    assert!(!seated(&last));
    assert!(lines(&last).contains(&"The cart is full.".to_string()));
}

#[test]
fn a_locked_server_parks_the_wagon_and_greys_the_stops() {
    let (mut realm, _, events) = at_the_stop(locked(), 1_000);
    assert!(!realm.unlock_on("carts"));
    assert!(!events
        .iter()
        .any(|e| matches!(e, SessionEvent::CartRoute { .. })));
    let r = first();
    for guid in [r.wagon_guid(), r.horse_guid(), r.driver_guid()] {
        let (position, progress) = events
            .iter()
            .find_map(|e| match e {
                SessionEvent::ObjectCreate {
                    guid: g,
                    position,
                    transport_progress,
                    ..
                } if *g == guid => Some((*position, *transport_progress)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{guid:#x} is created"));
        assert_eq!(progress, None);
        assert!(dist(position, r.stops[0].position) < 6.0);
    }
    let stops: Vec<MapMark> = marks(&events)
        .into_iter()
        .filter(|m| matches!(m.icon, MapIcon::CartStop | MapIcon::CartStopLocked))
        .collect();
    let count: usize = carts::routes().iter().map(|r| r.stops.len()).sum();
    assert_eq!(stops.len(), count);
    assert!(stops.iter().all(|m| m.icon == MapIcon::CartStopLocked));

    let events = use_wagon(&mut realm);
    assert!(!seated(&events));
    assert!(lines(&events).contains(&carts::cart().locked_line));
}
