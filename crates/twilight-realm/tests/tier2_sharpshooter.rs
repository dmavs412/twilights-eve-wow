//! The Sharpshooter (#684), the Ranger Line's Tier 2 Calling: the Calling Choice offers it at the
//! Ranger's cap, Wren Ashcombe waits unmarked on the hill above Three Corners until she is met,
//! Thalyssa Windbough and Guard Parker point the way, her three-quest Trial gates the Advancement,
//! and the sheet gives the Calling its ten Talent Rows and a Sunder Arrow that sunders.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::messages::GossipOption;
use benilla_protocol::{EntityKind, SessionEvent, START_ELWYNN};
use common::ranger::{ranger_save, shots};
use common::vanguard::{scene_of, STURDY};
use common::*;
use gear_common::*;
use quest_common::*;

/// Wren Ashcombe, Thalyssa Windbough (the Northshire Ranger Trainer) and Guard Parker at Three
/// Corners (a friendly spawn, not a Hub NPC).
const WREN: u32 = 190116;
const THALYSSA: u32 = 190053;
const PARKER: u32 = 464;
/// Guard Parker's spawn point (`redridge_spawns.json`).
const PARKER_AT: [f32; 2] = [-9609.64, -1902.4];
/// Redridge Mountains' zone.
const REDRIDGE: u32 = 44;

/// The Advancement Quest and its Trial, in order.
const STILLNESS: u32 = 64405;
const HOLD_THE_HILL: u32 = 64441;
const SPIDERS: u32 = 64442;
const FAR_SHORE: u32 = 64443;

/// The options of every `CallingChoice` in `events`.
fn offered(events: &[SessionEvent]) -> Vec<Vec<u32>> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::CallingChoice { options } => {
                Some(options.iter().map(|o| o.quest).collect())
            }
            _ => None,
        })
        .collect()
}

/// A Ranger of 30 who picked at the Tier 0 Calling Choice, entered in the world.
fn capped_ranger() -> Vec<SessionEvent> {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Pilgrim", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    assert_eq!(create_result(&mut maker), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("calling = \"basic_warrior\"", "calling = \"ranger\"")
        .replacen("\nlevel = 1\n", "\nlevel = 30\nchosen_tiers = [0]\n", 1);
    let mut realm = Realm::with_seed(SEED);
    realm
        .import_character(&blob)
        .unwrap_or_else(|e| panic!("{e:?}: {blob}"));
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    drain(&mut realm)
}

/// A save of `calling` at `level` standing beside Wren on the hill, with `quests` under way and
/// `done` handed in, entered in the world.
fn on_the_hill(
    calling: &str,
    level: u8,
    quests: &[Underway],
    done: &[u32],
) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(level, &[], 0, quests);
    let [x, y, z] = hub_home(hub_guid(WREN));
    let (head, tail) = blob.split_once("\n[").unwrap_or((&blob, ""));
    let head: Vec<String> = head
        .lines()
        .map(|line| {
            if line.starts_with("map = ") {
                "map = 0".to_string()
            } else if line.starts_with("zone = ") {
                format!("zone = {REDRIDGE}")
            } else if line.starts_with("position = [") {
                format!("position = [{}, {y:?}, {z:?}]", x + 1.5)
            } else if line == "xp = 0" && !done.is_empty() {
                format!("xp = 0\ncompleted = {done:?}")
            } else {
                line.to_string()
            }
        })
        .collect();
    let mut blob = head.join("\n").replace(
        "calling = \"basic_warrior\"",
        &format!("calling = \"{calling}\""),
    );
    if !tail.is_empty() {
        blob = format!("{blob}\n[{tail}");
    }
    let mut realm = Realm::with_seed(SEED);
    realm
        .import_character(&blob)
        .unwrap_or_else(|e| panic!("{e:?}: {blob}"));
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

/// Whether the last `MapMarks` in `events` marks `npc`.
fn marked(events: &[SessionEvent], npc: u64) -> bool {
    let last = events
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::MapMarks { marks } => Some(marks),
            _ => None,
        })
        .expect("a map mark list");
    last.iter().any(|m| m.guid == npc)
}

/// Opens `npc`'s gossip menu, standing beside it already: its lines.
fn menu(realm: &mut Realm, npc: u64) -> Vec<GossipOption> {
    realm.handle(RealmCommand::GossipHello { guid: npc });
    drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { options, .. } => Some(options),
            _ => None,
        })
        .expect("a gossip menu")
}

/// Chooses gossip line `option` of `npc`: the text the reply shows.
fn choose(realm: &mut Realm, npc: u64, option: u32) -> String {
    realm.handle(RealmCommand::GossipSelectOption { guid: npc, option });
    let text_id = drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { text_id, .. } => Some(text_id),
            _ => None,
        })
        .expect("the reply");
    realm.handle(RealmCommand::NpcTextQuery { text_id });
    drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::NpcGreeting { blocks, .. } => Some(blocks[0].male.clone()),
            _ => None,
        })
        .expect("the reply's text")
}

/// The reply to the hint line `line` in `options`, chosen at `npc`.
fn hint(realm: &mut Realm, npc: u64, options: &[GossipOption], line: &str) -> String {
    let option = options
        .iter()
        .find(|o| o.message == line)
        .unwrap_or_else(|| panic!("the hint line {line:?} in {options:?}"));
    choose(realm, npc, option.index)
}

/// The spells the client knows after `events`.
fn spellbook(events: &[SessionEvent]) -> std::collections::BTreeSet<u32> {
    let mut known = std::collections::BTreeSet::new();
    for e in events {
        match e {
            SessionEvent::SpellBook { spell_ids, .. } => {
                known = spell_ids.iter().copied().collect()
            }
            SessionEvent::SpellLearned { spell_id } => {
                known.insert(*spell_id);
            }
            SessionEvent::SpellRemoved { spell_id } => {
                known.remove(spell_id);
            }
            _ => {}
        }
    }
    known
}

#[test]
fn a_level_30_ranger_is_offered_the_sharpshooter() {
    let offers = offered(&capped_ranger());
    assert_eq!(offers.len(), 1, "{offers:?}");
    assert!(offers[0].contains(&STILLNESS), "{offers:?}");
}

#[test]
fn wren_is_unmarked_until_met_and_thalyssa_and_parker_give_the_hints() {
    let (mut realm, _, events) = on_the_hill("ranger", 30, &[(STILLNESS, [0; 4])], &[]);
    let wren = hub_guid(WREN);
    assert!(!marked(&events, wren), "not met yet");
    walk_up_to(&mut realm, wren);
    menu(&mut realm, wren);
    realm.tick(0.0, &FlatGround);
    assert!(marked(&drain(&mut realm), wren), "met at her gossip menu");

    let thalyssa = hub_guid(THALYSSA);
    walk_up_to(&mut realm, thalyssa);
    let lines = menu(&mut realm, thalyssa);
    assert_eq!(
        hint(&mut realm, thalyssa, &lines, "Who shoots without moving?"),
        "Wren Ashcombe. She went east to Redridge and stopped writing. Guard Parker at Three \
         Corners will know where she stands. His gnolls keep dying before they reach him."
    );

    let [x, y] = PARKER_AT;
    let streamed = walk_to(&mut realm, [x + 1.5, y, 0.0]);
    let parker = streamed
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                fields,
                ..
            } if fields.object_entry() == Some(PARKER) => Some(*guid),
            _ => None,
        })
        .expect("Guard Parker streams in");
    let lines = menu(&mut realm, parker);
    assert_eq!(
        hint(
            &mut realm,
            parker,
            &lines,
            "Who's been shooting the gnolls?"
        ),
        "Somebody on the hill east of my post, above the road. One arrow each. Never two. I \
         stopped asking who. You go up and ask."
    );
}

#[test]
fn wren_sets_the_trial_in_order_and_its_end_makes_a_level_1_sharpshooter() {
    let wren = hub_guid(WREN);
    let (mut realm, _, _) = on_the_hill("ranger", 30, &[], &[]);
    assert!(
        !on_offer(&mut realm, wren).contains(&HOLD_THE_HILL),
        "64405 not taken"
    );

    let (mut realm, _, _) = on_the_hill("ranger", 30, &[(STILLNESS, [0; 4])], &[]);
    assert_eq!(on_offer(&mut realm, wren), [HOLD_THE_HILL]);
    assert!(completed(&hand_in(&mut realm, wren, STILLNESS)).is_none());

    let (mut realm, _, _) = on_the_hill(
        "ranger",
        30,
        &[(STILLNESS, [0; 4]), (HOLD_THE_HILL, [10, 0, 0, 0])],
        &[],
    );
    assert!(completed(&hand_in(&mut realm, wren, HOLD_THE_HILL)).is_some());
    assert_eq!(on_offer(&mut realm, wren), [SPIDERS]);

    let (mut realm, _, _) = on_the_hill(
        "ranger",
        30,
        &[(STILLNESS, [0; 4]), (SPIDERS, [8, 0, 0, 0])],
        &[HOLD_THE_HILL],
    );
    assert!(completed(&hand_in(&mut realm, wren, SPIDERS)).is_some());
    assert_eq!(on_offer(&mut realm, wren), [FAR_SHORE]);
    assert!(completed(&hand_in(&mut realm, wren, STILLNESS)).is_none());

    let (mut realm, me, entered) = on_the_hill(
        "ranger",
        30,
        &[(STILLNESS, [0; 4]), (FAR_SHORE, [6, 0, 0, 0])],
        &[HOLD_THE_HILL, SPIDERS],
    );
    assert!(completed(&hand_in(&mut realm, wren, STILLNESS)).is_none());
    assert_eq!(realm.current_calling().unwrap().key, "ranger");
    assert!(completed(&hand_in(&mut realm, wren, FAR_SHORE)).is_some());
    let advanced = hand_in(&mut realm, wren, STILLNESS);
    assert!(completed(&advanced).is_some(), "{advanced:?}");
    assert_eq!(realm.current_calling().unwrap().key, "sharpshooter");
    let saved = realm.export_character(me).unwrap();
    assert!(saved.contains("\nlevel = 1\n"), "{saved}");

    let mut events = entered;
    events.extend(advanced);
    let known = spellbook(&events);
    for key in [
        "auto_shot",
        "puncturing_shaft",
        "steady_shot",
        "rapid_fire",
        "aimed_shot",
        "silencing_shot",
    ] {
        assert!(
            known.contains(&spell("sharpshooter", key)),
            "{key}: {known:?}"
        );
    }
    for key in ["sunder_arrow", "hold_your_ground", "pinning_shot"] {
        assert!(!known.contains(&spell("sharpshooter", key)), "{key} at 1");
    }

    // At level 10 the login's spell book has Sunder Arrow.
    let blob = saved.replacen("\nlevel = 1\n", "\nlevel = 10\n", 1);
    let mut later = Realm::with_seed(SEED);
    later.import_character(&blob).unwrap();
    later.handle(RealmCommand::EnterWorld { guid: me });
    later.tick(0.0, &FlatGround);
    let known = spellbook(&drain(&mut later));
    assert!(
        known.contains(&spell("sharpshooter", "sunder_arrow")),
        "{known:?}"
    );
    assert!(!known.contains(&spell("sharpshooter", "hold_your_ground")));
}

/// `server_spells.json`'s Calling `key`.
fn sheet(key: &str) -> serde_json::Value {
    let all: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/twilights-eve/server_spells.json"
    ))
    .unwrap();
    all.as_array()
        .unwrap()
        .iter()
        .find(|c| c["key"] == key)
        .unwrap_or_else(|| panic!("{key} is a Calling of the sheet"))
        .clone()
}

/// The bonus of `calling`'s Puncturing Shaft at level `l`, read from its formula.
fn shaft_bonus(calling: &str, l: f64) -> f64 {
    let c = sheet(calling);
    let shaft = c["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["key"] == "puncturing_shaft")
        .expect("a Puncturing Shaft");
    let bonus = &shaft["effects"][0]["params"]["bonus"];
    bonus["base"].as_f64().unwrap() + bonus["per_level"].as_f64().unwrap() * l
}

#[test]
fn the_sheet_gives_ten_rows_and_a_shaft_that_starts_stronger() {
    let c = sheet("sharpshooter");
    assert_eq!(c["tier"], 2);
    let rows = c["rows"].as_array().unwrap();
    let levels: Vec<u64> = rows.iter().map(|r| r["level"].as_u64().unwrap()).collect();
    assert_eq!(levels, [5, 10, 15, 20, 25, 30, 35, 40, 45, 50]);
    for row in rows {
        assert_eq!(
            row["choices"].as_array().unwrap().len(),
            3,
            "{}",
            row["key"]
        );
    }
    assert!(shaft_bonus("sharpshooter", 1.0) > shaft_bonus("ranger", 30.0));
}

/// Sunder Arrow puts a stack of Sundering on the mob it hits, Shred two.
#[test]
fn sunder_arrow_sunders_the_mob_it_hits_and_shred_twice() {
    let arrow = spell("sharpshooter", "sunder_arrow");
    let shred = choice("sharpshooter", "sunder_arrow", "shred");
    let sundering = spell("sharpshooter", "aura_sundering");
    for (rows, cast, stacks) in [
        (vec![], arrow, 1),
        (vec![("sunder_arrow", shred)], shred, 2),
    ] {
        let mut landed = 0;
        for seed in 0..6 {
            let (me, blob) = ranger_save(30);
            let blob = blob.replace("calling = \"ranger\"", "calling = \"sharpshooter\"");
            let mut s = scene_of(me, &blob, 0x684_0000 + seed, &rows, STURDY);
            let mob = s.mob.guid;
            let mut events = s.cast_at_mob(cast);
            events.extend(s.view.run(&mut s.realm, 0.2));
            let hit = shots(&events, cast)
                .iter()
                .any(|&(t, d, _)| t == mob && d > 0);
            let worn = s.view.aura(mob, sundering).map(|slot| slot.stacks);
            if hit {
                landed += 1;
                assert_eq!(worn, Some(stacks), "seed {seed}");
            }
            assert!(s.view.aura(me, sundering).is_none(), "never on the shooter");
        }
        assert!(landed > 0, "no arrow of six landed");
    }
}
