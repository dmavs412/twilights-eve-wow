//! A run-time spawn stands where its body fits (ADR 0021, #354): `Realm::spawn_for` asks the
//! host's `body_fits` and nudges a blocked spot out in rings a yard apart.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::*;
use twilight_realm::Audience;

/// Flat ground at one height.
const GROUND: f32 = 96.0;
/// A creature template the realm knows (the Crossroads hand spawn in `content/mobs.toml`).
const ENTRY: u32 = 3101;

/// Flat ground everywhere, and a pillar of `radius` yd round `pillar` that no body fits in.
struct Pillar {
    pillar: [f32; 2],
    radius: f32,
}

impl Host for Pillar {
    fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
        Some(GROUND)
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
    fn body_fits(&self, _: u32, at: [f32; 3]) -> bool {
        (at[0] - self.pillar[0]).hypot(at[1] - self.pillar[1]) > self.radius
    }
}

/// Where `guid`'s create in `events` put it.
fn created_at(events: &[SessionEvent], guid: u64) -> [f32; 3] {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid: g,
                kind: EntityKind::Unit,
                position,
                ..
            } if *g == guid => Some(*position),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{guid:#x} is created"))
}

#[test]
fn a_creature_wanted_inside_a_pillar_stands_a_yard_out() {
    let mut realm = Realm::with_seed(1);
    assert_eq!(create(&mut realm, "Stander", ORC, 0), CHAR_CREATE_SUCCESS);
    let me = roster(&mut realm)[0].guid;
    realm.handle(RealmCommand::EnterWorld { guid: me });
    let (map, at) = realm.player_positions()[0];
    let wanted = [at[0] + 5.0, at[1], 0.0];
    let host = Pillar {
        pillar: [wanted[0], wanted[1]],
        radius: 0.5,
    };
    realm.tick(0.1, &host);
    drain(&mut realm);
    let made = realm
        .spawn_for(&host, ENTRY, 1, wanted, map, Audience::of(&[me]))
        .expect("a spot a yard out");
    realm.tick(0.1, &host);
    let got = created_at(&drain(&mut realm), made);
    assert!(
        (got[0] - (wanted[0] + 1.0)).abs() < 1e-3 && (got[1] - wanted[1]).abs() < 1e-3,
        "{got:?}: the first ring's +x point"
    );
    assert_eq!(got[2], GROUND);
}

#[test]
fn a_creature_with_no_spot_that_fits_is_not_made() {
    let mut realm = Realm::with_seed(1);
    assert_eq!(create(&mut realm, "Stander", ORC, 0), CHAR_CREATE_SUCCESS);
    let me = roster(&mut realm)[0].guid;
    realm.handle(RealmCommand::EnterWorld { guid: me });
    let (map, at) = realm.player_positions()[0];
    let wanted = [at[0] + 5.0, at[1], 0.0];
    let host = Pillar {
        pillar: [wanted[0], wanted[1]],
        radius: 6.0,
    };
    assert_eq!(
        realm.spawn_for(&host, ENTRY, 1, wanted, map, Audience::of(&[me])),
        None
    );
}
