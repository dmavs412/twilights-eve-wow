//! Audience (ADR 0014): a creature made at run time for some characters is created for them
//! only, notices and fights them only, refuses everyone else's attacks, and goes when its
//! Audience is despawned.

mod common;

use benilla_protocol::SessionEvent;
use common::party::*;
use common::*;
use twilight_realm::{Audience, CROSSROADS};

/// Far out past The Barrens' camps: nothing else streams in.
const FAR_AWAY: [f32; 3] = [CROSSROADS[0] + 3000.0, CROSSROADS[1], 0.0];
const PRAIRIE_WOLF: u32 = 2958;

fn at(dx: f32) -> [f32; 3] {
    [FAR_AWAY[0] + dx, FAR_AWAY[1], 0.0]
}

/// Two characters 10 yards apart, and a wolf 40 yards from the first made for the first alone.
fn scene() -> (Party, u64, Audience) {
    let mut party = Party::new(realm(), &[5, 5]);
    party.go(0, at(0.0), 0.0);
    party.go(1, at(10.0), 0.0);
    // A tick for the characters to come out on the map their position is on.
    party.run(0.5);
    let map = party.realm.player_positions()[0].0;
    let audience = Audience::of(&[party.guids[0]]);
    let wolf = party
        .realm
        .spawn_for(
            party.host.as_ref(),
            PRAIRIE_WOLF,
            5,
            at(40.0),
            map,
            audience.clone(),
        )
        .expect("a wolf");
    (party, wolf, audience)
}

fn created(events: &[SessionEvent], guid: u64) -> bool {
    unit_creates(events).iter().any(|(g, _, _)| *g == guid)
}

fn removed(events: &[SessionEvent], guid: u64) -> bool {
    events.iter().any(|e| match e {
        SessionEvent::ObjectsRemoved(gone) => gone.contains(&guid),
        SessionEvent::ObjectDestroyed(g) => *g == guid,
        _ => false,
    })
}

#[test]
fn only_its_audience_is_sent_the_wolf() {
    let (mut party, wolf, _) = scene();
    party.run(2.0);
    assert!(created(party.seen(0), wolf));
    // Walking to 2 yards from it changes nothing for the other.
    party.go(1, at(38.0), 0.0);
    party.run(2.0);
    assert!(!created(party.seen(1), wolf));
}

#[test]
fn it_aggroes_its_audience_and_never_the_others() {
    let (mut party, wolf, _) = scene();
    party.run(0.5);
    party.go(1, at(38.0), 0.0);
    let got = party.run(4.0);
    assert!(!swung_at(&got[1], wolf).contains(&party.guids[1]));
    assert!(swung_at(&got[0], wolf).is_empty());
    // The first walks into its range, with the second still closer.
    party.go_facing(0, at(30.0), at(40.0));
    let mut hit = false;
    for _ in 0..10 {
        let got = party.run(1.0);
        assert!(!swung_at(&got[1], wolf).contains(&party.guids[1]));
        hit |= swung_at(&got[0], wolf).contains(&party.guids[0]);
    }
    assert!(hit, "the wolf never swung at its audience");
}

#[test]
fn an_attack_from_outside_the_audience_is_refused() {
    let (mut party, wolf, _) = scene();
    party.run(0.5);
    party.go_facing(0, at(38.0), at(40.0));
    party.go_facing(1, at(38.5), at(40.0));
    party.run(0.2);
    let before = party.seen(1).len();
    party.cmd(1, RealmCommand::AttackSwing { guid: wolf });
    let refused = party.seen(1)[before..]
        .iter()
        .any(|e| matches!(e, SessionEvent::AttackSwingError(_)));
    let started = party.seen(1)[before..]
        .iter()
        .any(|e| matches!(e, SessionEvent::AttackStart { .. }));
    assert!(refused && !started);

    let before = party.seen(0).len();
    party.cmd(0, RealmCommand::AttackSwing { guid: wolf });
    let started = party.seen(0)[before..]
        .iter()
        .any(|e| matches!(e, SessionEvent::AttackStart { .. }));
    assert!(started);
}

#[test]
fn despawning_the_audience_destroys_the_wolf_for_it() {
    let (mut party, wolf, audience) = scene();
    party.run(1.0);
    assert!(party.units_held(0).contains(&wolf));
    party.realm.despawn_audience(&audience);
    let got = party.run(1.0);
    assert!(removed(&got[0], wolf));
    assert!(!party.units_held(0).contains(&wolf));
    party.run(5.0);
    assert!(!party.units_held(0).contains(&wolf));
}
