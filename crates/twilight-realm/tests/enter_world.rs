//! Entering the world: the events a client needs to stand a character in the Crossroads.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::*;
use twilight_realm::CROSSROADS;

#[test]
fn entering_announces_the_character_then_ports_it_to_the_crossroads() {
    let mut realm = realm();
    let (guid, events) = enter_new(&mut realm, "Cairne", TAUREN);

    let connected = events
        .iter()
        .position(|e| matches!(e, SessionEvent::Connected { self_guid, name, .. } if *self_guid == guid && name == "Cairne"))
        .expect("Connected for the picked character");
    let port = events
        .iter()
        .position(|e| matches!(e, SessionEvent::Worldport { .. }))
        .expect("a login worldport");
    assert!(
        connected < port,
        "the IO thread announces the entry before any world packet"
    );

    let SessionEvent::Worldport {
        map_id,
        position,
        needs_ack,
        ..
    } = &events[port]
    else {
        unreachable!()
    };
    assert_eq!(*map_id, KALIMDOR);
    assert!((position[0] - CROSSROADS[0]).abs() < 1.0);
    assert!((position[1] - CROSSROADS[1]).abs() < 1.0);
    assert!(!needs_ack, "the login worldport is not acked");
}

/// Twilight's Eve is co-op: every race reports the one player faction template, so the client
/// reads a player of the other faction as friendly.
#[test]
fn every_race_reports_one_player_faction_template() {
    let mut templates = Vec::new();
    for (name, race) in ["Ra", "Rb", "Rc", "Rd", "Re", "Rf", "Rg", "Rh"]
        .into_iter()
        .zip([HUMAN, ORC, 3, 4, 5, TAUREN, 7, 8])
    {
        let mut realm = realm();
        let (guid, events) = enter_new(&mut realm, name, race);
        let template = events
            .iter()
            .find_map(|e| match e {
                SessionEvent::ObjectCreate {
                    guid: g, fields, ..
                } if *g == guid => fields.unit_faction_template(),
                _ => None,
            })
            .expect("our own player's create");
        templates.push(template);
    }
    assert!(
        templates.iter().all(|&t| t == templates[0]),
        "{templates:?}"
    );
}

#[test]
fn the_entered_character_streams_in_as_a_level_1_warrior_of_its_race() {
    let mut realm = realm();
    let (guid, events) = enter_new(&mut realm, "Cairne", TAUREN);

    let (display_id, fields, speeds) = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid: g,
                kind: EntityKind::Player,
                display_id,
                fields,
                speeds,
                ..
            } if *g == guid => Some((*display_id, fields.clone(), *speeds)),
            _ => None,
        })
        .expect("our own player's create");

    // Tauren male's native display (`ChrRaces.dbc`).
    assert_eq!(display_id, Some(59));
    assert_eq!(fields.unit_level(), Some(1));
    // Every player reports the one co-op faction template, Tauren included.
    assert_eq!(fields.unit_faction_template(), Some(2));
    assert_eq!(fields.unit_race(), Some(TAUREN));
    assert_eq!(fields.unit_class(), Some(WARRIOR));
    assert_eq!(fields.unit_gender(), Some(0));
    // A Warrior's power is rage (power type 1).
    assert_eq!(fields.unit_power_type(), 1);
    assert!(fields.unit_health().unwrap() > 0);
    // vanilla's base run speed.
    assert_eq!(speeds.expect("a living block").run, 7.0);
}

#[test]
fn a_character_that_is_not_on_the_roster_is_refused() {
    let mut realm = realm();
    realm.handle(RealmCommand::EnterWorld { guid: 42 });
    assert!(drain(&mut realm)
        .iter()
        .any(|e| matches!(e, SessionEvent::CharacterLoginFailed { .. })));
}

#[test]
fn logging_out_returns_to_the_roster_where_the_character_stood() {
    let mut realm = realm();
    enter_new(&mut realm, "Baine", TAUREN);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [-400.0, -2600.0, 92.0],
        orientation: 0.5,
    });
    realm.handle(RealmCommand::Logout);
    let events = drain(&mut realm);
    assert!(events.iter().any(|e| matches!(e, SessionEvent::LoggedOut)));
    let list = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::CharacterList { characters, .. } => Some(characters.clone()),
            _ => None,
        })
        .expect("the roster after a logout");
    assert_eq!(list[0].position.x, -400.0);
    assert_eq!(list[0].position.y, -2600.0);
}

#[test]
fn the_login_burst_holds_the_sky_at_dusk() {
    let mut realm = realm();
    let (_, events) = enter_new(&mut realm, "Cairne", TAUREN);

    let (hours, minutes, timescale) = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::TimeSpeed {
                hours,
                minutes,
                timescale,
                ..
            } => Some((*hours, *minutes, *timescale)),
            _ => None,
        })
        .expect("the login burst sets the game clock");
    // Evening twilight: after the 21:00 sunset, before full night at 22:00.
    let minute = u32::from(hours) * 60 + u32::from(minutes);
    assert!(
        (21 * 60..22 * 60).contains(&minute),
        "dusk, got {hours:02}:{minutes:02}"
    );
    assert_eq!(timescale, 0.0, "the game clock does not advance");
}
