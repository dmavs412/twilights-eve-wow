//! Echo Ridge Mine and the Idol chain (#85, #89): the deep tunnels' cultists and their elite,
//! Geomancer Vurx, whose Smoldering Twilight Idol begins The Idol in the Deep (to Marshal
//! McBride), which opens Into the Abbey (to Brother Neals inside Northshire Abbey).

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::*;
use gear_common::item_creates;
use quest_common::*;
use twilight_realm::rules::quest::hunting_reward_xp;

const HIGHGUID_UNIT: u64 = 0xF130;
const INITIATE: u32 = 96001;
const VURX_ENTRY: u32 = 96002;
const VURX: u64 = HIGHGUID_UNIT << 48 | (VURX_ENTRY as u64) << 24 | 960101;
const VURX_AT: [f32; 3] = [-8533.0, -204.0, 83.85];
const MCBRIDE: u64 = HIGHGUID_UNIT << 48 | 197 << 24 | 79970;
const MCBRIDE_AT: [f32; 3] = [-8912.0, -140.5, 82.0];
const NEALS: u64 = HIGHGUID_UNIT << 48 | 952 << 24 | 780005;
const NEALS_AT: [f32; 3] = [-8891.8, -186.8, 82.4];
const IDOL: u32 = 980001;
const THE_IDOL_IN_THE_DEEP: u32 = 94001;
const INTO_THE_ABBEY: u32 = 94002;
/// The Creature Spell Vurx casts.
static CORRUPTION_SPELL: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| creature_spell("corruption_creature"));
/// The Stone Gnoll Hammer (item level 9): a level 8's weapon.
const HAMMER: u32 = 781;
/// The starting Worn Shortsword it replaces.
const WORN_SHORTSWORD: u32 = 25;
/// South-east of Vurx, inside his notice radius and clear of the Initiates' and the kobolds'.
const PULL_FROM: [f32; 3] = [-8521.0, -214.0, 83.85];

/// The mine's and the Abbey's floor: every spot here stands at about this height.
struct Floor;

impl Host for Floor {
    fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
        Some(83.0)
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
}

/// A human warrior of `level` who began in Northshire, with the starting Chest and the Stone
/// Gnoll Hammer in hand, in the world.
fn pilgrim(level: u8, seed: u64) -> (Realm, u64) {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Pilgrim", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    let me = roster(&mut maker)[0].guid;
    let blob = maker.export_character(me).unwrap();
    let sword = format!("slot = 15\nentry = {WORN_SHORTSWORD}\n");
    assert!(blob.contains(&sword), "the starting sword in hand: {blob}");
    let blob = blob
        .replace(&sword, &format!("slot = 15\nentry = {HAMMER}\n"))
        .replace("level = 1\n", &format!("level = {level}\n"));
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &Floor);
    drain(&mut realm);
    (realm, me)
}

/// Stands at `at` facing `toward` and ticks `secs`, collecting the events.
fn stand(realm: &mut Realm, at: [f32; 3], toward: [f32; 3], secs: f32) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::DevTeleport {
        x: at[0],
        y: at[1],
        facing: Some((toward[1] - at[1]).atan2(toward[0] - at[0])),
    });
    tick(realm, secs)
}

fn tick(realm: &mut Realm, secs: f32) -> Vec<SessionEvent> {
    let mut events = Vec::new();
    for _ in 0..(secs * 10.0).round().max(1.0) as u32 {
        realm.tick(0.1, &Floor);
        events.extend(drain(realm));
    }
    events
}

/// Hands `quest` in at `npc`, where the player stands, the stock way (complete, continue, choose).
fn turn_in(realm: &mut Realm, npc: u64, quest: u32) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::QuestGiverCompleteQuest { npc, quest });
    realm.handle(RealmCommand::QuestGiverRequestReward { npc, quest });
    realm.handle(RealmCommand::QuestGiverChooseReward {
        npc,
        quest,
        choice: 0,
    });
    drain(realm)
}

fn health(events: &[SessionEvent], guid: u64) -> Option<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectValues { guid: g, fields } if *g == guid => fields.unit_health(),
            _ => None,
        })
        .last()
}

/// Pulls Vurx alone and fights him with white swings until one falls: whether the player won,
/// and the events.
fn fight_vurx(realm: &mut Realm, me: u64) -> (bool, Vec<SessionEvent>) {
    let mut events = stand(realm, PULL_FROM, VURX_AT, 0.1);
    realm.handle(RealmCommand::AttackSwing { guid: VURX });
    for _ in 0..1800 {
        if health(&events, VURX) == Some(0) {
            return (true, events);
        }
        if health(&events, me) == Some(0) {
            return (false, events);
        }
        realm.tick(0.1, &Floor);
        events.extend(drain(realm));
    }
    panic!("the fight with Vurx outlived three minutes");
}

/// How many of `seeds` fights a level `level` wins.
fn wins(level: u8, seeds: std::ops::Range<u64>) -> usize {
    seeds
        .filter(|&seed| {
            let (mut realm, me) = pilgrim(level, seed);
            fight_vurx(&mut realm, me).0
        })
        .count()
}

#[test]
fn the_cultists_and_their_elite_load() {
    let mut realm = realm();
    for (entry, name, rank) in [
        (INITIATE, "Twilight Initiate", 0),
        (VURX_ENTRY, "Geomancer Vurx", 1),
    ] {
        realm.handle(RealmCommand::CreatureQuery { entry });
        let got = drain(&mut realm).into_iter().find_map(|e| match e {
            SessionEvent::CreatureName {
                entry: e,
                name,
                rank,
                ..
            } if e == entry => Some((name, rank)),
            _ => None,
        });
        assert_eq!(got, Some((Some(name.to_string()), rank)), "{entry}");
    }
}

#[test]
fn vurx_casts_corruption() {
    let (mut realm, me) = pilgrim(8, 1);
    let (_, events) = fight_vurx(&mut realm, me);
    assert!(events.iter().any(|e| matches!(e,
        SessionEvent::SpellStart { caster, spell_id, .. }
            if *caster == VURX && *spell_id == *CORRUPTION_SPELL)));
}

#[test]
fn the_idol_chain_from_vurx_to_brother_neals() {
    // Under the Tier Cap, so the quests pay XP: the first fight a level 9 wins.
    let (mut realm, me) = (0..10)
        .map(|seed| pilgrim(9, seed))
        .find_map(|(mut realm, me)| fight_vurx(&mut realm, me).0.then_some((realm, me)))
        .expect("a level 9 beats Vurx in some fight");

    // The idol drops, and begins The Idol in the Deep.
    realm.handle(RealmCommand::Loot { guid: VURX });
    let loot = drain(&mut realm).into_iter().find_map(|e| match e {
        SessionEvent::LootResponse { items, .. } => Some(items),
        _ => None,
    });
    let slot = loot
        .expect("Vurx is lootable")
        .iter()
        .find(|i| i.item_id == IDOL)
        .expect("the idol drops")
        .slot;
    realm.handle(RealmCommand::AutostoreLootItem { slot });
    let idol = item_creates(&drain(&mut realm))
        .into_iter()
        .find(|(_, f)| f.object_entry() == Some(IDOL))
        .map(|(g, _)| g)
        .expect("the idol is carried");
    realm.handle(RealmCommand::ItemQuery { entry: IDOL });
    let starts = drain(&mut realm).into_iter().find_map(|e| match e {
        SessionEvent::ItemTemplate {
            info: Some(info), ..
        } => Some(info.start_quest),
        _ => None,
    });
    assert_eq!(starts, Some(THE_IDOL_IN_THE_DEEP));
    realm.handle(RealmCommand::QuestGiverAcceptQuest {
        npc: idol,
        quest: THE_IDOL_IN_THE_DEEP,
    });
    drain(&mut realm);

    let blob = realm.export_character(me).unwrap();
    assert!(
        blob.contains(&format!("id = {THE_IDOL_IN_THE_DEEP}")),
        "taken: {blob}"
    );

    // McBride takes it in for XP and a choice of three, and opens Into the Abbey.
    let seen = stand(
        &mut realm,
        [MCBRIDE_AT[0] + 2.0, MCBRIDE_AT[1], 82.0],
        MCBRIDE_AT,
        0.5,
    );
    assert!(
        seen.iter()
            .any(|e| matches!(e, SessionEvent::ObjectCreate { guid, .. } if *guid == MCBRIDE)),
        "McBride streams in"
    );
    let done = turn_in(&mut realm, MCBRIDE, THE_IDOL_IN_THE_DEEP);
    let c = completed(&done).expect("McBride takes the idol");
    assert_eq!(c.xp, hunting_reward_xp(9, 7));
    assert_eq!(c.money, 0);
    assert_eq!(c.items, [(900414, 1)], "the first of its three choices");
    assert!(done
        .iter()
        .any(|e| matches!(e, SessionEvent::QuestDetail(d) if d.quest_id == INTO_THE_ABBEY)));
    realm.handle(RealmCommand::QuestGiverAcceptQuest {
        npc: MCBRIDE,
        quest: INTO_THE_ABBEY,
    });
    drain(&mut realm);

    // Brother Neals takes it in, inside the Abbey.
    stand(
        &mut realm,
        [NEALS_AT[0] + 2.0, NEALS_AT[1], 82.4],
        NEALS_AT,
        0.5,
    );
    let done = turn_in(&mut realm, NEALS, INTO_THE_ABBEY);
    assert_eq!(
        completed(&done).map(|c| c.quest_id),
        Some(INTO_THE_ABBEY),
        "Neals takes it in"
    );
    let blob = realm.export_character(me).unwrap();
    assert!(
        !blob.contains(&format!("entry = {IDOL}")),
        "the idol went: {blob}"
    );
}

/// The tuning (recorded on Vurx's template): white swings only, as the realm's other fight tests
/// run, so a player using skills does better. The attack table is flat (ADR 0012), so the level
/// gap no longer decides the fight; a geared character beats Vurx most of the time.
#[test]
fn a_geared_character_usually_beats_vurx() {
    let at_9 = wins(9, 0..40);
    assert!(at_9 >= 24, "level 9 wins {at_9}/40");
}
