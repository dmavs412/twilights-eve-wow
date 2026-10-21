//! Northshire's five Calling trainers (#215): Hub NPCs with `map = 0`. A character on the
//! Eastern Kingdoms sees and reaches them and not the Crossroads trainers; a Kalimdor character
//! the reverse. Sergeant Hale sells the respec.

mod common;

use benilla_protocol::{EntityKind, SessionEvent, START_ELWYNN};
use common::*;

const HALE: u32 = 190050;
const NORTHSHIRE_TRAINERS: [u32; 5] = [190050, 190051, 190052, 190053, 190054];
const CROSSROADS_TRAINERS: [u32; 5] = [5810, 190030, 190031, 190032, 190033];

fn create_at(realm: &mut Realm, name: &str, start: u8) -> u64 {
    let RealmCommand::CreateCharacter(mut req) = create_cmd(name, HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = start;
    realm.handle(RealmCommand::CreateCharacter(req));
    assert_eq!(create_result(realm), CHAR_CREATE_SUCCESS);
    roster(realm)
        .into_iter()
        .find(|c| c.name == name)
        .unwrap()
        .guid
}

/// A character on the Eastern Kingdoms (`START_ELWYNN`) or on Kalimdor (`0`), in the world.
fn enter(start: u8, talents: bool) -> (Realm, u64, Vec<SessionEvent>) {
    let mut realm = realm();
    let me = create_at(&mut realm, "Pilgrim", start);
    if talents {
        let blob = realm
            .export_character(me)
            .unwrap()
            .replace("level = 1\n", "level = 10\n")
            .replace(
                "xp = 0\n",
                &format!(
                    "xp = 0\ntalents = [[{}, 3]]\n",
                    talent("basic_warrior", "field_medic")
                ),
            );
        let mut loaded = Realm::new();
        loaded.import_character(&blob).unwrap();
        realm = loaded;
    }
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    let entry = drain(&mut realm);
    (realm, me, entry)
}

/// Stands the character beside the Hub NPC `entry` and lets the world stream in.
fn walk_to(realm: &mut Realm, entry: u32) -> Vec<SessionEvent> {
    let [x, y, _] = hub_home(hub_guid(entry));
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [x + 1.5, y, 0.0],
        orientation: std::f32::consts::PI,
    });
    realm.tick(0.0, &FlatGround);
    drain(realm)
}

fn sees(events: &[SessionEvent], entry: u32) -> bool {
    events.iter().any(|e| match e {
        SessionEvent::ObjectCreate {
            kind: EntityKind::Unit,
            fields,
            ..
        } => fields.object_entry() == Some(entry),
        _ => false,
    })
}

/// The gossip menu a `GossipHello` at `entry` opens, if any.
fn menu(realm: &mut Realm, entry: u32) -> Option<Vec<String>> {
    realm.handle(RealmCommand::GossipHello {
        guid: hub_guid(entry),
    });
    drain(realm).into_iter().find_map(|e| match e {
        SessionEvent::GossipMenu { options, .. } => {
            Some(options.into_iter().map(|o| o.message).collect())
        }
        _ => None,
    })
}

#[test]
fn a_northshire_character_sees_and_reaches_the_five_trainers_and_not_the_crossroads_ones() {
    let (mut realm, _, mut seen) = enter(START_ELWYNN, false);
    for entry in NORTHSHIRE_TRAINERS {
        seen.extend(walk_to(&mut realm, entry));
        assert!(sees(&seen, entry), "{entry} streams in");
        assert!(menu(&mut realm, entry).is_some(), "{entry} opens a menu");
    }
    for entry in CROSSROADS_TRAINERS {
        seen.extend(walk_to(&mut realm, entry));
        assert!(menu(&mut realm, entry).is_none(), "{entry} is out of reach");
    }
    for entry in CROSSROADS_TRAINERS {
        assert!(!sees(&seen, entry), "{entry} is a Kalimdor NPC");
    }
}

#[test]
fn a_kalimdor_character_sees_and_reaches_the_crossroads_trainers_and_not_northshires() {
    let (mut realm, _, mut seen) = enter(0, false);
    for entry in CROSSROADS_TRAINERS {
        seen.extend(walk_to(&mut realm, entry));
        assert!(sees(&seen, entry), "{entry} streams in");
        assert!(menu(&mut realm, entry).is_some(), "{entry} opens a menu");
    }
    for entry in NORTHSHIRE_TRAINERS {
        seen.extend(walk_to(&mut realm, entry));
        assert!(menu(&mut realm, entry).is_none(), "{entry} is out of reach");
    }
    for entry in NORTHSHIRE_TRAINERS {
        assert!(!sees(&seen, entry), "{entry} is an Eastern Kingdoms NPC");
    }
}

#[test]
fn hales_unlearn_line_opens_the_talent_wipe_confirm() {
    let (mut realm, _, _) = enter(START_ELWYNN, true);
    walk_to(&mut realm, HALE);
    let lines = menu(&mut realm, HALE).expect("Hale's menu");
    assert_eq!(lines, ["I wish to unlearn my talents."]);
    realm.handle(RealmCommand::GossipSelectOption {
        guid: hub_guid(HALE),
        option: 0,
    });
    let events = drain(&mut realm);
    assert!(
        events.iter().any(|e| matches!(
            e,
            SessionEvent::TalentWipeConfirm { trainer, .. } if *trainer == hub_guid(HALE)
        )),
        "{events:?}"
    );
}

#[test]
fn the_other_northshire_trainers_sell_no_respec() {
    let (mut realm, _, _) = enter(START_ELWYNN, true);
    for entry in &NORTHSHIRE_TRAINERS[1..] {
        walk_to(&mut realm, *entry);
        assert_eq!(menu(&mut realm, *entry), Some(vec![]), "{entry}");
    }
}
