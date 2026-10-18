//! One world per map: two players on The Barrens see the same mobs, a kill is dead for both and
//! comes back once for both, and each client is sent only what is in its own player's view.

mod common;

use std::collections::{HashMap, HashSet};

use benilla_protocol::{EntityKind, SessionEvent};
use common::*;
use twilight_realm::{SessionId, CROSSROADS};

/// The Vile Familiar in front of the Crossroads inn.
const FAMILIAR: u32 = 3101;

struct Two {
    realm: Realm,
    a: SessionId,
    b: SessionId,
    /// Every event so far, by session.
    log: HashMap<SessionId, Vec<SessionEvent>>,
}

impl Two {
    /// Two level 20 warriors in the world, each in their own session, standing at the Hub.
    fn at_the_hub() -> Self {
        let mut maker = realm();
        assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
        assert_eq!(create(&mut maker, "Anduin", HUMAN, 0), CHAR_CREATE_SUCCESS);
        let mut realm = Realm::with_seed(7);
        let guids: Vec<u64> = roster(&mut maker).iter().map(|c| c.guid).collect();
        for &guid in &guids {
            let blob = maker
                .export_character(guid)
                .unwrap()
                .replace("level = 1\n", "level = 20\n");
            realm.import_character(&blob).unwrap();
        }
        let (a, b) = (realm.connect(), realm.connect());
        realm.handle_as(a, RealmCommand::EnterWorld { guid: guids[0] });
        realm.handle_as(b, RealmCommand::EnterWorld { guid: guids[1] });
        let mut two = Self {
            realm,
            a,
            b,
            log: HashMap::new(),
        };
        two.run(0.0);
        // Away first, so both come to the Hub together.
        two.go(a, north_of(HUB, 500.0));
        two.go(b, north_of(HUB, 500.0));
        two.run(0.0);
        two.go(a, HUB);
        two.go(b, HUB);
        two.log.clear();
        two.run(0.0);
        two
    }

    fn go(&mut self, s: SessionId, at: [f32; 3]) {
        self.realm.handle_as(
            s,
            RealmCommand::Move {
                position: at,
                orientation: 0.0,
            },
        );
    }

    /// Ticks `secs` in 0.1 s steps (the first at once); the events each session got.
    fn run(&mut self, secs: f32) -> HashMap<SessionId, Vec<SessionEvent>> {
        let mut got: HashMap<SessionId, Vec<SessionEvent>> = HashMap::new();
        let mut t = -0.1;
        while t < secs - 1e-3 {
            let dt = if t < 0.0 { 0.0 } else { 0.1 };
            self.realm.tick(dt, &NoGround);
            for (s, e) in self.realm.drain_session_events() {
                got.entry(s).or_default().push(e.clone());
                self.log.entry(s).or_default().push(e);
            }
            t += 0.1;
        }
        got
    }

    fn seen(&self, s: SessionId) -> Vec<SessionEvent> {
        self.log.get(&s).cloned().unwrap_or_default()
    }
}

const HUB: [f32; 3] = [CROSSROADS[0], CROSSROADS[1], 95.9];

fn north_of(p: [f32; 3], yd: f32) -> [f32; 3] {
    [p[0] + yd, p[1], p[2]]
}

/// A host with no ground loaded anywhere: the realm keeps each spawn's own height.
struct NoGround;

impl Host for NoGround {
    fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
        None
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
}

/// The units created, by guid: their entry and where they stand.
fn creates(events: &[SessionEvent]) -> HashMap<u64, (u32, [f32; 3])> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                fields,
                ..
            } => Some((*guid, (fields.object_entry().unwrap_or(0), *position))),
            _ => None,
        })
        .collect()
}

fn create_count(events: &[SessionEvent], guid: u64) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, SessionEvent::ObjectCreate { guid: g, .. } if *g == guid))
        .count()
}

fn removed(events: &[SessionEvent]) -> HashSet<u64> {
    events
        .iter()
        .flat_map(|e| match e {
            SessionEvent::ObjectsRemoved(guids) => guids.clone(),
            SessionEvent::ObjectDestroyed(guid) => vec![*guid],
            _ => Vec::new(),
        })
        .collect()
}

fn died(events: &[SessionEvent], guid: u64) -> bool {
    events.iter().any(|e| match e {
        SessionEvent::ObjectValues { guid: g, fields } => {
            *g == guid && fields.unit_health() == Some(0)
        }
        _ => false,
    })
}

#[test]
fn two_players_together_see_the_same_mobs() {
    let two = Two::at_the_hub();
    let a: HashSet<u64> = creates(&two.seen(two.a)).into_keys().collect();
    let b: HashSet<u64> = creates(&two.seen(two.b)).into_keys().collect();
    assert!(a.len() > 3, "the Hub's mobs and NPCs: {}", a.len());
    assert_eq!(a, b);
}

#[test]
fn a_kill_is_dead_for_both_and_comes_back_once_for_both() {
    let mut two = Two::at_the_hub();
    let (guid, imp) = creates(&two.seen(two.a))
        .into_iter()
        .find(|(_, (entry, _))| *entry == FAMILIAR)
        .map(|(g, (_, at))| (g, at))
        .expect("the familiar");
    let at = north_of(imp, 2.0);
    let (a, b) = (two.a, two.b);
    two.realm.handle_as(
        a,
        RealmCommand::Move {
            position: at,
            orientation: std::f32::consts::PI,
        },
    );
    two.realm.handle_as(a, RealmCommand::AttackSwing { guid });
    let mut dead = false;
    for _ in 0..300 {
        let got = two.run(0.1);
        if got.get(&b).is_some_and(|e| died(e, guid)) {
            dead = true;
            break;
        }
    }
    assert!(dead, "the other player sees the familiar die");
    assert!(died(&two.seen(a), guid), "and so does its killer");
    // The loot is the killer's: the other client's corpse is not lootable.
    let lootable = |s| {
        two.seen(s).iter().any(|e| {
            matches!(e, SessionEvent::ObjectValues { guid: g, fields }
                if *g == guid && fields.unit_dynamic_flags() & 1 != 0)
        })
    };
    assert!(lootable(a) && !lootable(b));

    // Both stand 70 yd off, in view, for the minute it takes.
    two.go(a, north_of(imp, 70.0));
    two.go(b, north_of(imp, 70.0));
    let got = two.run(62.0);
    for s in [a, b] {
        let events = &got[&s];
        let gone = events
            .iter()
            .position(|e| matches!(e, SessionEvent::ObjectDestroyed(g) if *g == guid))
            .expect("the corpse goes");
        assert_eq!(create_count(&events[gone..], guid), 1, "back once");
    }
}

#[test]
fn walking_away_takes_the_view_with_the_player_alone() {
    let mut two = Two::at_the_hub();
    let (a, b) = (two.a, two.b);
    let hub: HashSet<u64> = creates(&two.seen(a)).into_keys().collect();

    two.go(b, north_of(HUB, 500.0));
    let got = two.run(0.0);
    let b_lost = removed(&got[&b]);
    assert!(
        !b_lost.is_empty(),
        "the walker's client drops the Hub's mobs"
    );
    assert!(b_lost.is_subset(&hub));
    let a_lost = got.get(&a).map(|e| removed(e)).unwrap_or_default();
    assert!(a_lost.is_disjoint(&b_lost), "the one who stayed keeps them");

    // What the far player sees is in the world for them alone, and stays.
    let far: HashSet<u64> = creates(&got[&b]).into_keys().collect();
    assert!(!far.is_empty(), "the far player's own mobs");
    let got = two.run(5.0);
    assert!(removed(got.get(&b).map_or(&[][..], |e| e)).is_disjoint(&far));
    let a_seen: HashSet<u64> = creates(&two.seen(a)).into_keys().collect();
    assert!(a_seen.is_disjoint(&far), "never sent to the one at the Hub");
}
