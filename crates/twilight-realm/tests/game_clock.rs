//! The game clock (ADR 0019): 12x the wall clock, sent once in the login burst.

mod common;

use benilla_protocol::SessionEvent;
use common::*;
use twilight_realm::rules::game_clock::{minute_of_day, TIMESCALE};

/// The login burst's game clock: hours, minutes and timescale.
fn time_speed(events: &[SessionEvent]) -> (u8, u8, f32) {
    events
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
        .expect("the login burst sets the game clock")
}

#[test]
fn five_real_minutes_are_a_game_hour_and_two_real_hours_a_day() {
    assert_eq!(minute_of_day(0.0), 0);
    assert_eq!(minute_of_day(300.0), 60);
    assert_eq!(minute_of_day(7200.0), 0);
}

#[test]
fn the_login_burst_sends_the_hour_of_the_wall_clock() {
    let mut realm = realm();
    realm.set_wall_clock(600_000);
    let (_, events) = enter_new(&mut realm, "Cairne", TAUREN);
    assert_eq!(time_speed(&events), (2, 0, 0.2));
    assert_eq!(TIMESCALE, 0.2);
}

#[test]
fn logins_five_real_minutes_apart_are_an_hour_apart() {
    let mut realm = realm();
    realm.set_wall_clock(600_000);
    let (guid, events) = enter_new(&mut realm, "Cairne", TAUREN);
    let (first, _, _) = time_speed(&events);

    realm.handle(RealmCommand::Logout);
    drain(&mut realm);
    realm.set_wall_clock(900_000);
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let (second, _, _) = time_speed(&drain(&mut realm));

    assert_eq!(second, first + 1);
}
