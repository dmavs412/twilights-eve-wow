//! Run-time GameObjects (ADR 0014): a use object and a filled chest made for an Audience are
//! created for it only, clicked by it only, and go when used, emptied or despawned.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::party::*;
use common::*;
use twilight_realm::{Audience, ObjectKind, ObjectSpec, Realm, CHEST_DISPLAY, CROSSROADS};

/// Far out past The Barrens' camps: nothing else streams in.
const FAR_AWAY: [f32; 3] = [CROSSROADS[0] + 3000.0, CROSSROADS[1], 0.0];
const GATHER: u32 = 99301;
const ROOT: u32 = 990301;
const CHEST: u32 = 990302;
const TRINKET: u32 = 900300;

/// A quest's use objective must name a `gameobject` row; this one stands far from the test.
const TEST_QUESTS: &str = r#"
[[gameobject]]
guid = 990301
entry = 990301
display = 1
map = 0
position = [-8911.0, -140.5, 82.0]

[[quest]]
id = 99301
kind = "story"
giver = 197
level = 5
title = "Gather"
objectives = "Dig up a root."
details = "-"
progress = "-"
completion = "-"
money = 0
tokens = 0
use = [[990301, 1]]
"#;

fn at(dx: f32) -> [f32; 3] {
    [FAR_AWAY[0] + dx, FAR_AWAY[1], 0.0]
}

/// Two characters 5 yards apart, the first holding the Gather quest.
fn scene() -> (Party, u32, Audience) {
    let realm = Realm::with_seed(1).with_quests(TEST_QUESTS);
    let mut party = Party::edited(realm, &[5, 5], 0, Box::new(NoGround), |i, blob| {
        if i == 0 {
            format!("{blob}\n[[quests]]\nid = {GATHER}\nslot = 0\nkills = [0, 0, 0, 0]\n")
        } else {
            blob
        }
    });
    party.go(0, at(0.0), 0.0);
    party.go(1, at(5.0), 0.0);
    party.run(0.5);
    let map = party.realm.player_positions()[0].0;
    let audience = Audience::of(&[party.guids[0]]);
    (party, map, audience)
}

fn spec(entry: u32, kind: ObjectKind, map: u32) -> ObjectSpec {
    ObjectSpec {
        entry,
        display: if matches!(kind, ObjectKind::Chest(_)) {
            CHEST_DISPLAY
        } else {
            1
        },
        kind,
        map,
        position: at(2.0),
        orientation: 0.0,
    }
}

fn object_created(events: &[SessionEvent], guid: u64) -> bool {
    events.iter().any(|e| {
        matches!(e, SessionEvent::ObjectCreate { guid: g, kind: EntityKind::GameObject, .. } if *g == guid)
    })
}

fn removed(events: &[SessionEvent], guid: u64) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::ObjectsRemoved(gone) if gone.contains(&guid)))
}

fn loot_items(events: &[SessionEvent]) -> Option<usize> {
    events.iter().find_map(|e| match e {
        SessionEvent::LootResponse { items, .. } => Some(items.len()),
        _ => None,
    })
}

#[test]
fn a_use_object_is_created_for_its_audience_and_counts_for_their_quest() {
    let (mut party, map, audience) = scene();
    let host = party.host.as_ref();
    let guid = party
        .realm
        .spawn_object(host, spec(ROOT, ObjectKind::Use, map), audience)
        .expect("a spot");
    let got = party.run(1.0);
    assert!(object_created(&got[0], guid));
    assert!(!object_created(&got[1], guid));

    // The click of one it does not admit does nothing.
    party.cmd(1, RealmCommand::UseGameObject { guid });
    let got = party.run(0.5);
    assert!(!removed(&got[0], guid));

    party.cmd(0, RealmCommand::UseGameObject { guid });
    let got = party.run(0.5);
    assert!(party.seen(0).iter().any(|e| matches!(
        e,
        SessionEvent::QuestObjectiveKill {
            quest_id: GATHER,
            count: 1,
            required: 1,
            ..
        }
    )));
    assert!(removed(&got[0], guid));
    assert!(!object_created(&party.run(5.0)[0], guid));
}

#[test]
fn a_chest_opens_the_loot_window_for_its_audience_and_goes_when_emptied() {
    let (mut party, map, audience) = scene();
    let host = party.host.as_ref();
    let guid = party
        .realm
        .spawn_object(
            host,
            spec(CHEST, ObjectKind::Chest(vec![TRINKET]), map),
            audience,
        )
        .expect("a spot");
    let got = party.run(1.0);
    assert!(object_created(&got[0], guid));
    assert!(!object_created(&got[1], guid));

    party.cmd(1, RealmCommand::UseGameObject { guid });
    assert_eq!(loot_items(party.seen(1)), None);

    let before = party.seen(0).len();
    party.cmd(0, RealmCommand::UseGameObject { guid });
    assert_eq!(loot_items(&party.seen(0)[before..]), Some(1));
    party.cmd(0, RealmCommand::AutostoreLootItem { slot: 0 });
    let got = party.run(0.5);
    assert!(removed(&got[0], guid));
    let blob = party.realm.export_character(party.guids[0]).unwrap();
    assert!(blob.contains(&TRINKET.to_string()), "{blob}");
}

#[test]
fn despawning_the_audience_removes_a_chest_nobody_opened() {
    let (mut party, map, audience) = scene();
    let host = party.host.as_ref();
    let guid = party
        .realm
        .spawn_object(
            host,
            spec(CHEST, ObjectKind::Chest(vec![TRINKET]), map),
            audience.clone(),
        )
        .expect("a spot");
    assert!(object_created(&party.run(1.0)[0], guid));
    party.realm.despawn_audience(&audience);
    let got = party.run(0.5);
    assert!(removed(&got[0], guid));
    let before = party.seen(0).len();
    party.cmd(0, RealmCommand::UseGameObject { guid });
    assert_eq!(loot_items(&party.seen(0)[before..]), None);
}
