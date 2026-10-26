//! The Crossroads as a player meets it: a new character starts in the inn, and the townsfolk
//! each say one line, offer nothing else, and some walk a few yards about their spot.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::*;
use twilight_realm::CROSSROADS;

/// The town's NPCs as their creates show them: `(guid, entry, npc flags, position)`.
fn npcs(events: &[SessionEvent]) -> Vec<(u64, u32, u32, [f32; 3])> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                fields,
                ..
            } if fields.unit_npc_flags() != 0 => Some((
                *guid,
                fields.object_entry()?,
                fields.unit_npc_flags(),
                *position,
            )),
            _ => None,
        })
        .collect()
}

/// `content/hub.toml`'s townsfolk on Kalimdor, `(name, guid, greeting, wander)`. Goldshire's
/// (#459) stand on the Eastern Kingdoms.
fn townsfolk() -> Vec<(String, u64, String, f32)> {
    let hub: toml::Table = include_str!("../content/hub.toml").parse().unwrap();
    hub["npc"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["role"].as_str() == Some("ambient"))
        .filter(|n| n.get("map").and_then(|m| m.as_integer()).unwrap_or(1) == 1)
        .map(|n| {
            (
                n["name"].as_str().unwrap().to_string(),
                0xF130 << 48
                    | (n["entry"].as_integer().unwrap() as u64) << 24
                    | n["guid"].as_integer().unwrap() as u64,
                n["greeting"].as_str().unwrap().to_string(),
                n.get("wander").and_then(|w| w.as_float()).unwrap_or(0.0) as f32,
            )
        })
        .collect()
}

/// Walks the player to `at` and lets the town stream in around them.
fn walk_to(realm: &mut Realm, at: [f32; 3]) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: at,
        orientation: 0.0,
    });
    realm.tick(0.0, &FlatGround);
    drain(realm)
}

/// The middle of Camp Taurajo.
const TAURAJO: [f32; 3] = [-2370.0, -1950.0, 0.0];

/// Every Hub NPC the player meets walking the town: from the inn, then its corners, then Camp
/// Taurajo.

fn meet_the_town(realm: &mut Realm, entry: Vec<SessionEvent>) -> Vec<(u64, u32, u32, [f32; 3])> {
    let mut met = npcs(&entry);
    let [x, y, ..] = CROSSROADS;
    for (dx, dy) in [
        (-60.0, 0.0),
        (60.0, 0.0),
        (0.0, 60.0),
        (0.0, -60.0),
        (-120.0, 0.0),
    ] {
        for n in npcs(&walk_to(realm, [x + dx, y + dy, 0.0])) {
            if !met.iter().any(|m| m.0 == n.0) {
                met.push(n);
            }
        }
    }
    // Camp Taurajo, the Outpost, keeps townsfolk of its own.
    for n in npcs(&walk_to(realm, TAURAJO)) {
        if !met.iter().any(|m| m.0 == n.0) {
            met.push(n);
        }
    }
    met
}

#[test]
fn a_new_character_starts_in_the_inn_beside_barg_and_the_innkeeper() {
    let mut realm = realm();
    let (_, events) = enter_new(&mut realm, "Thrall", ORC);
    let position = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::Worldport { position, .. } => Some(*position),
            _ => None,
        })
        .expect("the login's spot");
    assert_eq!([position[0], position[1]], [CROSSROADS[0], CROSSROADS[1]]);
    // Barg keeps the inn's goods: the first NPC a new character can deal with.
    let barg = npcs(&events)
        .into_iter()
        .find(|n| n.1 == 190001)
        .expect("Barg stands in view");
    let d = ((barg.3[0] - position[0]).powi(2) + (barg.3[1] - position[1]).powi(2)).sqrt();
    assert!(d <= 10.0, "Barg stands {d:.1} yd from the start");
    // The innkeeper, at the bar.
    let innkeeper = npcs(&events)
        .into_iter()
        .find(|n| n.1 == 3934)
        .expect("the innkeeper stands in view");
    let d =
        ((innkeeper.3[0] - position[0]).powi(2) + (innkeeper.3[1] - position[1]).powi(2)).sqrt();
    assert!(d <= 12.0, "the innkeeper stands {d:.1} yd from the start");
}

#[test]
fn every_townsperson_says_one_line_and_offers_nothing_else() {
    let mut realm = realm();
    let (_, entry) = enter_new(&mut realm, "Thrall", ORC);
    let met = meet_the_town(&mut realm, entry);
    let folk = townsfolk();
    assert!(folk.len() >= 18);
    for (name, id, line, _) in &folk {
        let (guid, _, flags, _) = *met
            .iter()
            .find(|n| n.0 == *id)
            .unwrap_or_else(|| panic!("{name} lives in the town"));
        // Gossip only: no quest giver, vendor or trainer flag.
        assert_eq!(flags, 0x1, "{name}");
        walk_up_to(&mut realm, guid);

        realm.handle(RealmCommand::GossipHello { guid });
        let (text_id, options, quests) = drain(&mut realm)
            .into_iter()
            .find_map(|e| match e {
                SessionEvent::GossipMenu {
                    npc,
                    text_id,
                    options,
                    quests,
                } if npc == guid => Some((text_id, options, quests)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{name}'s gossip"));
        assert!(options.is_empty() && quests.is_empty(), "{name}");

        realm.handle(RealmCommand::NpcTextQuery { text_id });
        let said = drain(&mut realm).into_iter().find_map(|e| match e {
            SessionEvent::NpcGreeting { text_id: t, blocks } if t == text_id => {
                Some(blocks[0].male.clone())
            }
            _ => None,
        });
        assert_eq!(said.as_deref(), Some(line.as_str()), "{name}");

        // Asked for quests or wares, they have none.
        realm.handle(RealmCommand::QuestGiverHello { npc: guid });
        realm.handle(RealmCommand::ListInventory { guid });
        let rest = drain(&mut realm);
        assert!(
            !rest.iter().any(|e| matches!(
                e,
                SessionEvent::QuestDetail(_)
                    | SessionEvent::QuestGreeting(_)
                    | SessionEvent::VendorInventory { .. }
            )),
            "{name}: {rest:?}"
        );
    }
}

#[test]
fn a_wanderer_walks_a_few_yards_about_its_spot() {
    let mut realm = realm();
    let (_, entry) = enter_new(&mut realm, "Thrall", ORC);
    let met = meet_the_town(&mut realm, entry);
    let (name, id, _, wander) = townsfolk()
        .into_iter()
        .find(|f| f.3 > 0.0)
        .expect("a wanderer");
    let guid = met.iter().find(|n| n.0 == id).expect("met").0;
    let home = hub_home(guid);
    walk_up_to(&mut realm, guid);
    let mut walks = Vec::new();
    for _ in 0..120 {
        realm.tick(0.5, &FlatGround);
        for e in drain(&mut realm) {
            if let SessionEvent::MonsterMove {
                guid: g,
                path,
                run_mode,
                ..
            } = e
            {
                if g == guid {
                    walks.push((path, run_mode));
                }
            }
        }
    }
    assert!(
        walks.len() >= 2,
        "{name} walked {} times in a minute",
        walks.len()
    );
    for (path, run_mode) in walks {
        assert!(!run_mode, "{name} walks, not runs");
        for p in path {
            let d = ((p[0] - home[0]).powi(2) + (p[1] - home[1]).powi(2)).sqrt();
            assert!(d <= wander + 0.01, "{name} strayed {d:.1} yd");
        }
    }
}

#[test]
fn the_others_stand_still() {
    let mut realm = realm();
    let (_, entry) = enter_new(&mut realm, "Thrall", ORC);
    let still: Vec<u64> = npcs(&entry)
        .into_iter()
        .filter(|n| !townsfolk().iter().any(|f| f.1 == n.0 && f.3 > 0.0))
        .map(|n| n.0)
        .collect();
    assert!(!still.is_empty());
    for _ in 0..120 {
        realm.tick(0.5, &FlatGround);
        for e in drain(&mut realm) {
            if let SessionEvent::MonsterMove { guid, .. } = e {
                assert!(!still.contains(&guid), "{guid:#x} moved");
            }
        }
    }
}
