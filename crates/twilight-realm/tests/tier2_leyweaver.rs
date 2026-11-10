//! The Leyweaver (#690), the Initiate Line's second Tier 2 Calling: the Calling Choice offers it
//! beside the Pyromancer at the Initiate's cap, Teodric Lumen waits unmarked at a ley spring on
//! the Darkened Bank until he is met, Elsa Brightwood, Zaldimar Wefhellt and Watcher Callahan
//! point the way, his two-quest Trial gates the Advancement, and the sheet gives the Calling its
//! ten Talent Rows. Arcane Surge pours the bar into one blow, and Ley Tap, a channel on the caster,
//! fills it again.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::messages::GossipOption;
use benilla_protocol::{EntityKind, SessionEvent, START_ELWYNN};
use common::fight::{cast, flat, refusal, stand_at, Mob, View, CAMP};
use common::initiate::{mana, max_mana};
use common::*;
use gear_common::{Carried, SEED};
use quest_common::*;
use twilight_realm::{Realm, RealmCommand};

/// Teodric Lumen, Elsa Brightwood (the Northshire Initiate Trainer), Zaldimar Wefhellt in
/// Goldshire (a Hub NPC) and Watcher Callahan (a friendly Duskwood spawn, not a Hub NPC).
const TEODRIC: u32 = 190115;
const ELSA: u32 = 190052;
const ZALDIMAR: u32 = 328;
const CALLAHAN: u32 = 2142;
/// Watcher Callahan's spawn point (`content/duskwood.toml`).
const CALLAHAN_AT: [f32; 2] = [-10071.1, -1378.51];
/// Duskwood's zone.
const DUSKWOOD: u32 = 10;

/// The Pyromancer's Advancement Quest, offered beside this one.
const FLAME: u32 = 64403;
/// The Advancement Quest and its Trial, in order.
const LEY: u32 = 64404;
const QUIET: u32 = 64436;
const RECLUSES: u32 = 64437;

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

/// An Initiate of 30 who picked at the Tier 0 Calling Choice, entered in the world.
fn capped_initiate() -> Vec<SessionEvent> {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Listener", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    assert_eq!(create_result(&mut maker), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("calling = \"basic_warrior\"", "calling = \"initiate\"")
        .replacen("\nlevel = 1\n", "\nlevel = 30\nchosen_tiers = [0]\n", 1);
    let mut realm = Realm::with_seed(SEED);
    realm
        .import_character(&blob)
        .unwrap_or_else(|e| panic!("{e:?}: {blob}"));
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    drain(&mut realm)
}

/// An Initiate of 30 standing beside Teodric at the spring, carrying `items`, with `quests` under
/// way and `done` handed in, entered in the world.
fn by_the_spring(
    items: &[Carried],
    quests: &[Underway],
    done: &[u32],
) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(30, items, 0, quests);
    let [x, y, z] = hub_home(hub_guid(TEODRIC));
    let (head, tail) = blob.split_once("\n[").unwrap_or((&blob, ""));
    let head: Vec<String> = head
        .lines()
        .map(|line| {
            if line.starts_with("map = ") {
                "map = 0".to_string()
            } else if line.starts_with("zone = ") {
                format!("zone = {DUSKWOOD}")
            } else if line.starts_with("position = [") {
                format!("position = [{}, {y:?}, {z:?}]", x + 1.5)
            } else if line == "xp = 0" && !done.is_empty() {
                format!("xp = 0\ncompleted = {done:?}")
            } else {
                line.to_string()
            }
        })
        .collect();
    let mut blob = head
        .join("\n")
        .replace("calling = \"basic_warrior\"", "calling = \"initiate\"");
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

/// The reply to the hint line `line` of `npc`'s menu, standing beside it already.
fn hint(realm: &mut Realm, npc: u64, line: &str) -> String {
    let options = menu(realm, npc);
    let option = options
        .iter()
        .find(|o| o.message == line)
        .unwrap_or_else(|| panic!("the hint line {line:?} in {options:?}"));
    realm.handle(RealmCommand::GossipSelectOption {
        guid: npc,
        option: option.index,
    });
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
fn a_level_30_initiate_is_offered_the_pyromancer_and_the_leyweaver() {
    let offers = offered(&capped_initiate());
    assert_eq!(offers.len(), 1, "{offers:?}");
    assert!(offers[0].contains(&FLAME), "{offers:?}");
    assert!(offers[0].contains(&LEY), "{offers:?}");
}

#[test]
fn teodric_is_unmarked_until_met_and_elsa_zaldimar_and_callahan_give_the_hints() {
    let (mut realm, _, events) = by_the_spring(&[], &[(LEY, [0; 4])], &[]);
    let teodric = hub_guid(TEODRIC);
    assert!(!marked(&events, teodric), "not met yet");
    walk_up_to(&mut realm, teodric);
    menu(&mut realm, teodric);
    realm.tick(0.0, &FlatGround);
    assert!(
        marked(&drain(&mut realm), teodric),
        "met at his gossip menu"
    );

    let elsa = hub_guid(ELSA);
    walk_up_to(&mut realm, elsa);
    assert_eq!(
        hint(&mut realm, elsa, "Who still studies the old ley lines?"),
        "Zaldimar Wefhellt, in Goldshire. He maps them for Azora, though Azora stopped asking."
    );

    let zaldimar = hub_guid(ZALDIMAR);
    walk_up_to(&mut realm, zaldimar);
    assert_eq!(
        hint(&mut realm, zaldimar, "Where does the ley rise near here?"),
        "South, across the river, on Duskwood's bank. The Night Watch has a post down there. \
         Their watcher sees blue lights on the water and pretends he doesn't. Ask him."
    );

    let [x, y] = CALLAHAN_AT;
    let streamed = walk_to(&mut realm, [x + 1.5, y, 0.0]);
    let callahan = streamed
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                fields,
                ..
            } if fields.object_entry() == Some(CALLAHAN) => Some(*guid),
            _ => None,
        })
        .expect("Watcher Callahan streams in");
    assert_eq!(
        hint(&mut realm, callahan, "Have you seen lights on the river?"),
        "West of my post, along the bank, past the spiders. Blue lights at night, and a man who \
         sits by them. He doesn't sleep. I don't ask."
    );
}

#[test]
fn teodric_sets_the_trial_in_order_and_its_end_makes_a_level_1_leyweaver() {
    let teodric = hub_guid(TEODRIC);
    let (mut realm, _, _) = by_the_spring(&[], &[], &[]);
    assert!(
        !on_offer(&mut realm, teodric).contains(&QUIET),
        "64404 not taken"
    );

    let (mut realm, _, _) = by_the_spring(&[], &[(LEY, [0; 4])], &[]);
    assert_eq!(on_offer(&mut realm, teodric), [QUIET]);
    assert!(completed(&hand_in(&mut realm, teodric, LEY)).is_none());

    let (mut realm, _, _) = by_the_spring(&[], &[(LEY, [0; 4]), (QUIET, [8, 0, 0, 0])], &[]);
    assert!(completed(&hand_in(&mut realm, teodric, QUIET)).is_some());
    assert_eq!(on_offer(&mut realm, teodric), [RECLUSES]);
    assert!(completed(&hand_in(&mut realm, teodric, LEY)).is_none());

    let (mut realm, me, entered) =
        by_the_spring(&[], &[(LEY, [0; 4]), (RECLUSES, [6, 0, 0, 0])], &[QUIET]);
    assert!(completed(&hand_in(&mut realm, teodric, LEY)).is_none());
    assert_eq!(realm.current_calling().unwrap().key, "initiate");
    assert!(completed(&hand_in(&mut realm, teodric, RECLUSES)).is_some());
    let advanced = hand_in(&mut realm, teodric, LEY);
    assert!(completed(&advanced).is_some(), "{advanced:?}");
    assert_eq!(realm.current_calling().unwrap().key, "leyweaver");
    let saved = realm.export_character(me).unwrap();
    assert!(saved.contains("\nlevel = 1\n"), "{saved}");

    let mut events = entered;
    events.extend(advanced);
    let known = spellbook(&events);
    for key in [
        "ley_spark",
        "arcane_missiles",
        "ley_ward",
        "arcane_surge",
        "counterspell",
    ] {
        assert!(known.contains(&spell("leyweaver", key)), "{key}: {known:?}");
    }
    for key in ["ley_tap", "attunement", "ley_bind"] {
        assert!(!known.contains(&spell("leyweaver", key)), "{key} at 1");
    }

    // At level 10 the login's spell book has Ley Tap.
    let blob = saved.replacen("\nlevel = 1\n", "\nlevel = 10\n", 1);
    let mut later = Realm::with_seed(SEED);
    later.import_character(&blob).unwrap();
    later.handle(RealmCommand::EnterWorld { guid: me });
    later.tick(0.0, &FlatGround);
    let known = spellbook(&drain(&mut later));
    assert!(known.contains(&spell("leyweaver", "ley_tap")), "{known:?}");
    assert!(!known.contains(&spell("leyweaver", "attunement")));
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

/// The hit of `calling`'s Ley-Spark at level `l`, read from its formula.
fn spark_hit(calling: &str, l: f64) -> f64 {
    let c = sheet(calling);
    let spark = c["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["key"] == "ley_spark")
        .expect("a Ley-Spark");
    let amount = &spark["effects"][0]["params"]["amount"];
    amount["base"].as_f64().unwrap() + amount["per_level"].as_f64().unwrap() * l
}

#[test]
fn the_sheet_gives_ten_rows_and_a_ley_spark_that_starts_stronger() {
    let c = sheet("leyweaver");
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
    assert!(spark_hit("leyweaver", 1.0) > spark_hit("initiate", 30.0));
}

/// A Leyweaver of `level` 20 yd from a camp mob east of the Crossroads, at `seed`: the realm, their
/// guid, the mob and the view.
fn world(seed: u64, level: u8) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Teodra", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"leyweaver\"");
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    view.run(&mut realm, 1.0);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [CAMP[0] + 40.0, CAMP[1], 0.0],
        orientation: std::f32::consts::PI,
    });
    let mob = view
        .run(&mut realm, 1.0)
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                ..
            } if flat(*position, CAMP) < 15.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .expect("a camp mob in view");
    stand_at(&mut realm, &mob, 20.0);
    view.drain(&mut realm);
    (realm, guid, mob, view)
}

/// Each direct blow of `spell_id` in `events`: `(damage, crit)`.
fn hits(events: &[SessionEvent], spell_id: u32) -> Vec<(u32, bool)> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id && !l.periodic => {
                Some((l.damage, l.hit_info != 0))
            }
            _ => None,
        })
        .collect()
}

/// An Arcane Surge at level 30 from a bar of `bar` mana (the whole bar for `None`): the plain
/// blow it dealt, the bar before and the mana left after it.
fn surge_from(bar: Option<u32>) -> (u32, u32, u32) {
    let surge = spell("leyweaver", "arcane_surge");
    for seed in 0..20 {
        let (mut realm, me, mob, mut view) = world(0x690_0000 + seed, 30);
        if let Some(bar) = bar {
            realm.set_player_power(bar);
            view.drain(&mut realm);
        }
        let before = mana(&view, me);
        realm.handle(cast(surge, Some(mob.guid)));
        let events = view.drain(&mut realm);
        assert_eq!(refusal(&events, surge), None, "{events:?}");
        if let Some(&(damage, false)) = hits(&events, surge).first() {
            return (damage, before, mana(&view, me));
        }
    }
    panic!("no plain Arcane Surge in 20 worlds");
}

#[test]
fn arcane_surge_from_a_full_bar_empties_it_up_to_the_extra_and_hits_harder_than_from_a_low_one() {
    let (full, before, left) = surge_from(None);
    let (_, me, _, view) = world(0x690_0000, 30);
    let max = max_mana(&view, me);
    assert!(max > 700, "the bar holds the cost and the extra: {max}");
    assert_eq!(before, max, "a full bar");
    assert_eq!(left, max - 100 - 600, "100 and 600 more spent");

    let (low, before, left) = surge_from(Some(120));
    assert_eq!(before, 120);
    assert_eq!(left, 0, "100 and the 20 left spent");
    // 60% of 700 against 60% of 120.
    assert_eq!(full - low, 420 - 72, "full {full}, low {low}");
}

#[test]
fn ley_tap_is_a_channel_on_the_caster_that_gives_12_pct_of_max_mana_each_tick() {
    let tap = spell("leyweaver", "ley_tap");
    let (mut realm, me, _, mut view) = world(0x690, 10);
    // Well clear of the camp, so no mob's blow breaks the channel.
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [CAMP[0] + 120.0, CAMP[1], 0.0],
        orientation: 0.0,
    });
    view.run(&mut realm, 1.0);
    let max = max_mana(&view, me);
    realm.set_player_power(0);
    view.drain(&mut realm);
    realm.handle(cast(tap, None));
    let mut events = view.drain(&mut realm);
    assert_eq!(refusal(&events, tap), None, "{events:?}");
    events.extend(view.run(&mut realm, 7.0));
    let gains: Vec<u32> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellEnergizeLog(l) if l.spell_id == tap => Some(l.amount),
            _ => None,
        })
        .collect();
    assert_eq!(gains, [max * 12 / 100; 3], "three ticks of 12%");
    assert!(
        !events.iter().any(
            |e| matches!(e, SessionEvent::SpellFailedOther { spell_id, .. } if *spell_id == tap)
        ),
        "the channel ran out, not broken"
    );
}
