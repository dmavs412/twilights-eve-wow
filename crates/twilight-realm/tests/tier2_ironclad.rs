//! The Ironclad (#685), the Vanguard Line's Tier 2 Calling: the Calling Choice offers it at the
//! Vanguard's cap, Sergeant Maera Holt waits unmarked at the Lakeshire bridge until she is met,
//! Commander Wray points the way, her four-quest Trial gates the Advancement, and the sheet gives
//! the Calling its ten Talent Rows, a Retort answered only after a block, and a Hold the Line
//! whose stacks each raise the block chance.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::messages::GossipOption;
use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::fight::{cast, refusal, stand_at};
use common::vanguard::{scene_of, vanguard_save, Scene, BLOCKS, STURDY};
use common::*;
use gear_common::*;
use quest_common::*;

/// Sergeant Maera Holt and Commander Aldous Wray.
const HOLT: u32 = 190110;
const WRAY: u32 = 190071;
/// Redridge Mountains' zone, and Stormwind's.
const REDRIDGE: u32 = 44;
const STORMWIND: u32 = 1519;

/// The Advancement Quest, its Trial in order, the Zealot's Vow and the Templar's Charge.
const OATH: u32 = 64002;
const HIGHWAY: u32 = 64411;
const SHIELDS: u32 = 64412;
const SHORE: u32 = 64413;
const HOLD: u32 = 64414;
const ZEALOT_VOW: u32 = 64003;
const TEMPLAR: u32 = 64004;
/// The Blackrock Shield 64412 collects.
const BLACKROCK_SHIELD: u32 = 980811;

/// `SPELL_FAILED_CASTER_AURASTATE`: the cast needs a state the caster is not in.
const CASTER_AURASTATE: u8 = 0x12;

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

/// A Vanguard of 30 who picked at the Tier 0 Calling Choice, entered in the world.
fn capped_vanguard() -> Vec<SessionEvent> {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Bulwark", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    assert_eq!(create_result(&mut maker), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("calling = \"basic_warrior\"", "calling = \"vanguard\"")
        .replacen("\nlevel = 1\n", "\nlevel = 30\nchosen_tiers = [0]\n", 1);
    let mut realm = Realm::with_seed(SEED);
    realm
        .import_character(&blob)
        .unwrap_or_else(|e| panic!("{e:?}: {blob}"));
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    drain(&mut realm)
}

/// A save of `calling` at `level` standing beside the Hub NPC `npc` in `zone`, carrying `items`,
/// with `quests` under way and `done` handed in, entered in the world.
fn beside(
    npc: u32,
    zone: u32,
    calling: &str,
    level: u8,
    items: &[Carried],
    quests: &[Underway],
    done: &[u32],
) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(level, items, 0, quests);
    let [x, y, z] = hub_home(hub_guid(npc));
    let (head, tail) = blob.split_once("\n[").unwrap_or((&blob, ""));
    let head: Vec<String> = head
        .lines()
        .map(|line| {
            if line.starts_with("map = ") {
                "map = 0".to_string()
            } else if line.starts_with("zone = ") {
                format!("zone = {zone}")
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

/// A Vanguard of 30 at the bridge beside Holt, carrying `items`, with `quests` under way and
/// `done` handed in.
fn at_the_bridge(
    items: &[Carried],
    quests: &[Underway],
    done: &[u32],
) -> (Realm, u64, Vec<SessionEvent>) {
    beside(HOLT, REDRIDGE, "vanguard", 30, items, quests, done)
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
fn a_level_30_vanguard_is_offered_the_ironclad_and_the_zealot() {
    let offers = offered(&capped_vanguard());
    assert_eq!(offers.len(), 1, "{offers:?}");
    assert!(offers[0].contains(&OATH), "{offers:?}");
    assert!(offers[0].contains(&ZEALOT_VOW), "{offers:?}");
}

#[test]
fn holt_is_unmarked_until_met_and_wray_gives_the_hint() {
    let (mut realm, _, events) = at_the_bridge(&[], &[(OATH, [0; 4])], &[]);
    let holt = hub_guid(HOLT);
    assert!(!marked(&events, holt), "not met yet");
    walk_up_to(&mut realm, holt);
    menu(&mut realm, holt);
    realm.tick(0.0, &FlatGround);
    assert!(marked(&drain(&mut realm), holt), "met at her gossip menu");

    let wray = hub_guid(WRAY);
    walk_up_to(&mut realm, wray);
    let lines = menu(&mut realm, wray);
    let line = lines
        .iter()
        .find(|o| o.message == "Where do I find the Ironclads?")
        .unwrap_or_else(|| panic!("the hint line in {lines:?}"));
    assert_eq!(
        choose(&mut realm, wray, line.index),
        "Lakeshire. They hold the bridge into town because nobody else will. Look for Sergeant \
         Holt at the west end, in plate older than you are."
    );
}

#[test]
fn holt_sets_the_trial_in_order_and_its_end_makes_a_level_1_ironclad() {
    let holt = hub_guid(HOLT);
    let (mut realm, _, _) = at_the_bridge(&[], &[], &[]);
    assert!(
        !on_offer(&mut realm, holt).contains(&HIGHWAY),
        "64002 not taken"
    );

    let (mut realm, _, _) = at_the_bridge(&[], &[(OATH, [0; 4])], &[]);
    assert_eq!(on_offer(&mut realm, holt), [HIGHWAY]);
    assert!(completed(&hand_in(&mut realm, holt, OATH)).is_none());

    let (mut realm, _, _) = at_the_bridge(&[], &[(OATH, [0; 4]), (HIGHWAY, [12, 0, 0, 0])], &[]);
    assert!(completed(&hand_in(&mut realm, holt, HIGHWAY)).is_some());
    assert_eq!(on_offer(&mut realm, holt), [SHIELDS]);

    let (mut realm, _, _) = at_the_bridge(
        &[(23, BLACKROCK_SHIELD, 8)],
        &[(OATH, [0; 4]), (SHIELDS, [0; 4])],
        &[HIGHWAY],
    );
    assert!(completed(&hand_in(&mut realm, holt, SHIELDS)).is_some());
    assert_eq!(on_offer(&mut realm, holt), [SHORE]);

    let (mut realm, _, _) = at_the_bridge(
        &[],
        &[(OATH, [0; 4]), (SHORE, [10, 0, 0, 0])],
        &[HIGHWAY, SHIELDS],
    );
    assert!(completed(&hand_in(&mut realm, holt, SHORE)).is_some());
    assert_eq!(on_offer(&mut realm, holt), [HOLD]);
    assert!(completed(&hand_in(&mut realm, holt, OATH)).is_none());

    let (mut realm, me, entered) = at_the_bridge(
        &[],
        &[(OATH, [0; 4]), (HOLD, [10, 0, 0, 0])],
        &[HIGHWAY, SHIELDS, SHORE],
    );
    assert!(completed(&hand_in(&mut realm, holt, OATH)).is_none());
    assert_eq!(realm.current_calling().unwrap().key, "vanguard");
    assert!(completed(&hand_in(&mut realm, holt, HOLD)).is_some());
    let advanced = hand_in(&mut realm, holt, OATH);
    assert!(completed(&advanced).is_some(), "{advanced:?}");
    assert_eq!(realm.current_calling().unwrap().key, "ironclad");
    let saved = realm.export_character(me).unwrap();
    assert!(saved.contains("\nlevel = 1\n"), "{saved}");

    let mut events = entered;
    events.extend(advanced);
    let known = spellbook(&events);
    for key in [
        "righteous_smash",
        "shield_slam",
        "shield_of_penitent",
        "retort",
        "rebuke",
    ] {
        assert!(known.contains(&spell("ironclad", key)), "{key}: {known:?}");
    }
    for key in ["iron_wall", "steel_resolve", "hold_the_line"] {
        assert!(!known.contains(&spell("ironclad", key)), "{key} at 1");
    }

    // At level 10 the login's spell book has Iron Wall.
    let blob = saved.replacen("\nlevel = 1\n", "\nlevel = 10\n", 1);
    let mut later = Realm::with_seed(SEED);
    later.import_character(&blob).unwrap();
    later.handle(RealmCommand::EnterWorld { guid: me });
    later.tick(0.0, &FlatGround);
    let known = spellbook(&drain(&mut later));
    assert!(known.contains(&spell("ironclad", "iron_wall")), "{known:?}");
    assert!(!known.contains(&spell("ironclad", "steel_resolve")));
}

#[test]
fn wray_still_offers_the_templars_charge_and_no_longer_the_ironclads_oath() {
    let wray = hub_guid(WRAY);
    let (mut realm, _, _) = beside(WRAY, STORMWIND, "zealot", 50, &[], &[], &[]);
    assert!(on_offer(&mut realm, wray).contains(&TEMPLAR));
    let (mut realm, _, _) = beside(WRAY, STORMWIND, "vanguard", 30, &[], &[], &[]);
    let offer = on_offer(&mut realm, wray);
    assert!(!offer.contains(&OATH), "{offer:?}");
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

/// The bonus of `calling`'s Righteous Smash at level `l`, read from its formula.
fn smash_bonus(calling: &str, l: f64) -> f64 {
    let c = sheet(calling);
    let smash = c["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["key"] == "righteous_smash")
        .expect("a Righteous Smash");
    let bonus = &smash["effects"][0]["params"]["bonus"];
    bonus["base"].as_f64().unwrap() + bonus["per_level"].as_f64().unwrap() * l
}

#[test]
fn the_sheet_gives_ten_rows_and_a_smash_that_starts_stronger() {
    let c = sheet("ironclad");
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
    assert!(smash_bonus("ironclad", 1.0) > smash_bonus("vanguard", 30.0));
}

/// An Ironclad of `level` with the starting shield and `rows` set, 2 yd from the sturdy mob.
fn ironclad_beside_mob(level: u8, seed: u64, rows: &[(&str, u32)]) -> Scene {
    let (me, blob) = vanguard_save(level, true);
    let blob = blob.replace("calling = \"vanguard\"", "calling = \"ironclad\"");
    let mut s = scene_of(me, &blob, seed, rows, STURDY);
    stand_at(&mut s.realm, &s.mob, 2.0);
    s.view.run(&mut s.realm, 0.2);
    s
}

/// The block chance the client shows for `me`.
fn block_pct(s: &Scene) -> f32 {
    s.view
        .fields(s.me)
        .player_block_percentage()
        .expect("the block chance")
}

/// Casts Hold the Line, or the Skill Choice of it `spell_id`, beside the mob: the client's block
/// chance before and after.
fn hold_the_line(s: &mut Scene, spell_id: u32) -> (f32, f32) {
    let before = block_pct(s);
    s.realm.handle(cast(spell_id, None));
    let events = s.view.drain(&mut s.realm);
    assert_eq!(refusal(&events, spell_id), None, "Hold the Line goes off");
    s.view.run(&mut s.realm, 0.2);
    (before, block_pct(s))
}

/// Hold the Line gives a stack of 5% block for the one enemy it taunts, Firm Line two.
#[test]
fn hold_the_line_raises_the_block_chance_a_stack_per_enemy_taunted() {
    let stacks = spell("ironclad", "aura_hold_the_line");
    let plain = spell("ironclad", "hold_the_line");
    let firm = choice("ironclad", "hold_the_line", "firm_line");
    for (rows, cast_id, want) in [
        (vec![], plain, 1u8),
        (vec![("hold_the_line", firm)], firm, 2),
    ] {
        let mut s = ironclad_beside_mob(50, SEED, &rows);
        let (before, after) = hold_the_line(&mut s, cast_id);
        let me = s.me;
        assert_eq!(s.view.aura(me, stacks).map(|a| a.stacks), Some(want));
        assert!(
            (after - before - 5.0 * f32::from(want)).abs() < 0.01,
            "{before} -> {after}"
        );
    }
}

/// Retort is refused until the Ironclad blocks a blow, then goes off.
#[test]
fn retort_answers_only_after_a_block() {
    let retort = spell("ironclad", "retort");
    let mut s = ironclad_beside_mob(30, SEED, &[]);
    s.realm.handle(cast(retort, Some(s.mob.guid)));
    let events = s.view.drain(&mut s.realm);
    assert_eq!(
        refusal(&events, retort),
        Some(CASTER_AURASTATE),
        "no block yet"
    );

    // Hold the Line turns the mob on the Ironclad, and its blows build the rage Retort costs.
    hold_the_line(&mut s, spell("ironclad", "hold_the_line"));
    let me = s.me;
    s.run_until(60.0, "a block with 5 rage", |view, events| {
        let blocked = events.iter().any(|e| {
            matches!(e, SessionEvent::AttackerState(a) if a.victim == me
                && (a.blocked > 0 || a.victim_state == BLOCKS))
        });
        blocked && view.rage(me) >= 5
    });
    s.cast_at_mob(retort);
}
