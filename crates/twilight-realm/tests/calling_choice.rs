//! The Calling Choice (#416): a Basic Warrior at the Tier Cap is sent the popup, once, until a pick
//! is made; picking puts that Calling's Northshire Advancement Quest in the log.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::*;
use gear_common::*;
use quest_common::*;
use twilight_realm::DevMultipliers;

const JORIK: u32 = 190051;
const THALYSSA: u32 = 190053;

/// The options of every `CallingChoice` in `events`.
fn popups(events: &[SessionEvent]) -> Vec<Vec<benilla_protocol::CallingOption>> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::CallingChoice { options } => Some(options.clone()),
            _ => None,
        })
        .collect()
}

/// A Basic Warrior at `level` who starts in Northshire, with these quests in the log, not yet in
/// the world.
fn northshire(level: u8, quests: &[u32]) -> (Realm, u64) {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Pilgrim", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    assert_eq!(create_result(&mut maker), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(me)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"));
    for (slot, id) in quests.iter().enumerate() {
        blob.push_str(&format!(
            "\n[[quests]]\nid = {id}\nslot = {slot}\nkills = [0, 0, 0, 0]\n"
        ));
    }
    let mut realm = Realm::with_seed(SEED);
    realm
        .import_character(&blob)
        .unwrap_or_else(|e| panic!("{e:?}: {blob}"));
    (realm, me)
}

fn enter(realm: &mut Realm, me: u64) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    drain(realm)
}

/// Stands beside the Hub NPC `entry` and lets the world stream in.
fn walk_to(realm: &mut Realm, entry: u32) {
    let [x, y, _] = hub_home(hub_guid(entry));
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [x + 1.5, y, 0.0],
        orientation: std::f32::consts::PI,
    });
    realm.tick(0.0, &FlatGround);
    drain(realm);
}

fn xp_times(xp: f32) -> DevMultipliers {
    DevMultipliers {
        xp,
        ..DevMultipliers::default()
    }
}

/// A level 9 Basic Warrior at the Crossroads with Zun'jin's skins quest done but for the hand-in.
fn skinner(xp: f32) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(
        9,
        &[(23, CROCOLISK_SKIN, 8)],
        0,
        &[(CROCOLISK_SKINS, [0; 4])],
    );
    let mut realm = Realm::with_seed(SEED).with_dev(xp_times(xp));
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

#[test]
fn reaching_the_tier_cap_sends_the_five_options() {
    let (mut realm, _, events) = skinner(50.0);
    let zunjin = giver(&events, ZUNJIN);
    let turned_in = hand_in(&mut realm, zunjin, CROCOLISK_SKINS);
    assert!(turned_in
        .iter()
        .any(|e| matches!(e, SessionEvent::LevelUp(_))));
    let sent = popups(&turned_in);
    assert_eq!(sent.len(), 1, "{turned_in:?}");
    let options = &sent[0];
    assert_eq!(
        options.iter().map(|o| o.quest).collect::<Vec<_>>(),
        [64009, 64010, 64011, 64012, 64013]
    );
    for o in options {
        assert!(!o.blurb.is_empty(), "{}", o.calling);
        assert!(!o.skills.is_empty(), "{}", o.calling);
        assert!(!o.objectives.is_empty(), "{}", o.calling);
    }
}

#[test]
fn xp_short_of_the_tier_cap_sends_nothing() {
    let (mut realm, _, events) = skinner(0.01);
    let zunjin = giver(&events, ZUNJIN);
    let turned_in = hand_in(&mut realm, zunjin, CROCOLISK_SKINS);
    assert!(!turned_in
        .iter()
        .any(|e| matches!(e, SessionEvent::LevelUp(_))));
    assert!(popups(&turned_in).is_empty());
}

#[test]
fn a_level_10_basic_warrior_who_has_not_picked_gets_it_at_login() {
    let (mut realm, me) = northshire(10, &[]);
    assert_eq!(popups(&enter(&mut realm, me)).len(), 1);
}

#[test]
fn an_advancement_quest_in_the_log_keeps_the_popup_away_at_login() {
    for quest in [64001, 64009] {
        let (mut realm, me) = northshire(10, &[quest]);
        assert!(popups(&enter(&mut realm, me)).is_empty(), "{quest}");
    }
}

#[test]
fn a_level_9_basic_warrior_gets_nothing_at_login() {
    let (mut realm, me) = northshire(9, &[]);
    assert!(popups(&enter(&mut realm, me)).is_empty());
}

#[test]
fn a_pick_logs_the_quest_already_met_and_jorik_makes_a_cutthroat() {
    let (mut realm, me) = northshire(10, &[]);
    enter(&mut realm, me);
    realm.handle(RealmCommand::ChooseCalling { quest: 64010 });
    let events = drain(&mut realm);
    assert!(
        events.iter().any(|e| matches!(
            e,
            SessionEvent::Chat(m) if m.text == "Your path is chosen. Speak with Jorik Vane to begin."
        )),
        "{events:?}"
    );
    let saved = realm.export_character(me).unwrap();
    assert!(saved.contains("id = 64010"), "{saved}");
    assert!(saved.contains("calling_chosen = true"), "{saved}");

    let mut again = Realm::with_seed(SEED);
    again.import_character(&saved).unwrap();
    assert!(popups(&enter(&mut again, me)).is_empty());

    walk_to(&mut again, JORIK);
    let done = hand_in(&mut again, hub_guid(JORIK), 64010);
    assert!(completed(&done).is_some(), "{done:?}");
    assert_eq!(again.current_calling().unwrap().key, "cutthroat");
}

#[test]
fn an_abandoned_pick_is_not_offered_again_and_the_trainer_still_offers_it() {
    let (mut realm, me) = northshire(10, &[]);
    enter(&mut realm, me);
    realm.handle(RealmCommand::ChooseCalling { quest: 64012 });
    drain(&mut realm);
    realm.handle(RealmCommand::QuestLogRemoveQuest { slot: 0 });
    drain(&mut realm);
    let saved = realm.export_character(me).unwrap();
    assert!(!saved.contains("id = 64012"), "{saved}");

    let mut again = Realm::with_seed(SEED);
    again.import_character(&saved).unwrap();
    assert!(popups(&enter(&mut again, me)).is_empty());
    walk_to(&mut again, THALYSSA);
    assert_eq!(on_offer(&mut again, hub_guid(THALYSSA)), [64012]);
}

#[test]
fn a_pick_that_is_not_an_option_or_comes_from_a_vanguard_changes_nothing() {
    let (mut realm, me) = northshire(10, &[]);
    enter(&mut realm, me);
    let before = realm.export_character(me).unwrap();
    realm.handle(RealmCommand::ChooseCalling { quest: 64001 });
    assert!(drain(&mut realm).is_empty());
    assert_eq!(realm.export_character(me).unwrap(), before);

    let (maker, guid) = northshire(10, &[]);
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("calling = \"basic_warrior\"", "calling = \"vanguard\"");
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    enter(&mut realm, guid);
    let before = realm.export_character(guid).unwrap();
    realm.handle(RealmCommand::ChooseCalling { quest: 64009 });
    assert!(drain(&mut realm).is_empty());
    assert_eq!(realm.export_character(guid).unwrap(), before);
}

#[test]
fn a_full_quest_log_refuses_the_pick_and_the_next_login_offers_it_again() {
    // Twenty quests that are not Advancement Quests fill the log (`LOG_SLOTS`).
    let filler: Vec<u32> = (91001..=91005)
        .chain([91006, 91007, 91008])
        .chain(91041..=91049)
        .chain([91051, 91052, 91053])
        .take(20)
        .collect();
    let (mut realm, me) = northshire(10, &filler);
    assert_eq!(filler.len(), 20);
    let entered = enter(&mut realm, me);
    assert_eq!(popups(&entered).len(), 1);
    realm.handle(RealmCommand::ChooseCalling { quest: 64010 });
    let events = drain(&mut realm);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SessionEvent::QuestLogFull)),
        "{events:?}"
    );
    let saved = realm.export_character(me).unwrap();
    assert!(!saved.contains("calling_chosen"), "{saved}");
    assert!(!saved.contains("id = 64010"), "{saved}");

    let mut again = Realm::with_seed(SEED);
    again.import_character(&saved).unwrap();
    assert_eq!(popups(&enter(&mut again, me)).len(), 1);
}
