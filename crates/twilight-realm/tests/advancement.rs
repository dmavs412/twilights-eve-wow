//! Tiers and Advancement, played through the realm's seam: the Tier Cap that stops XP, the
//! Advancement Quest at the Warrior trainer, what Advancement replaces and what it keeps, the
//! Tier Bonus, and Champion's quest that cannot be finished.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::messages::dialog_status;
use benilla_protocol::SessionEvent;
use common::*;
use gear_common::*;
use quest_common::*;
use twilight_realm::DevMultipliers;

/// That warrior, as [`questing`] puts it in the world, with the dev multipliers set.
fn questing_with(
    dev: DevMultipliers,
    level: u8,
    items: &[Carried],
    quests: &[Underway],
) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(level, items, 0, quests);
    let mut realm = Realm::with_seed(SEED).with_dev(dev);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

fn xp_times(xp: f32) -> DevMultipliers {
    DevMultipliers {
        xp,
        ..DevMultipliers::default()
    }
}

/// Eight skins, enough for Zun'jin's Crocolisk Skins (level 10).
const EIGHT_SKINS: &[Carried] = &[(23, CROCOLISK_SKIN, 8)];

// ── The Tier Cap ────────────────────────────────────────────────────────────────────────────────

#[test]
fn xp_past_the_tier_cap_is_not_banked() {
    // Three levels' worth at level 9: a Basic Warrior stops at 10 with nothing over.
    let (mut realm, me, mut events) =
        questing_with(xp_times(3.0), 9, EIGHT_SKINS, &[(CROCOLISK_SKINS, [0; 4])]);
    let zunjin = giver(&events, ZUNJIN);
    events.extend(hand_in(&mut realm, zunjin, CROCOLISK_SKINS));
    let me_now = descriptor(&events, me);
    assert_eq!(me_now.unit_level(), Some(10));
    assert_eq!(me_now.player_xp(), Some(0));
    let blob = realm.export_character(me).unwrap();
    assert!(
        blob.contains("level = 10\n") && blob.contains("xp = 0\n"),
        "{blob}"
    );
}

#[test]
fn a_basic_warrior_at_the_tier_cap_gains_no_xp() {
    let (mut realm, me, mut events) = questing_with(
        xp_times(50.0),
        10,
        EIGHT_SKINS,
        &[(CROCOLISK_SKINS, [0; 4])],
    );
    let zunjin = giver(&events, ZUNJIN);
    let turned_in = hand_in(&mut realm, zunjin, CROCOLISK_SKINS);
    let done = completed(&turned_in).expect("the quest still completes");
    assert_eq!(done.xp, 0, "no XP at the cap, even with ?xp=");
    assert_eq!(done.money, 400, "the rest of the reward still comes");
    assert!(!turned_in
        .iter()
        .any(|e| matches!(e, SessionEvent::LevelUp(_))));
    events.extend(turned_in);
    let me_now = descriptor(&events, me);
    assert_eq!(me_now.unit_level(), Some(10));
    assert_eq!(me_now.player_xp(), Some(0));
}

#[test]
fn a_tier_1_warrior_levels_past_ten_and_stops_at_thirty() {
    let quest = &[(CROCOLISK_SKINS, [0; 4])];
    let (mut realm, me, mut events) = questing_as("warrior", 10, EIGHT_SKINS, 0, quest);
    events.extend(hand_in(&mut realm, giver(&events, ZUNJIN), CROCOLISK_SKINS));
    assert_eq!(descriptor(&events, me).unit_level(), Some(11));

    let (mut realm, me, mut events) = questing_as("warrior", 30, EIGHT_SKINS, 0, quest);
    let turned_in = hand_in(&mut realm, giver(&events, ZUNJIN), CROCOLISK_SKINS);
    assert_eq!(completed(&turned_in).unwrap().xp, 0, "Tier 1's cap is 30");
    events.extend(turned_in);
    assert_eq!(descriptor(&events, me).unit_level(), Some(30));
}

// ── The Tier Bonus ──────────────────────────────────────────────────────────────────────────────

#[test]
fn a_tier_1_warrior_has_the_tier_bonus_on_its_sheet() {
    let (_, me, basic) = questing(1, &[], 0, &[]);
    let (_, _, tier1) = questing_as("warrior", 1, &[], 0, &[]);
    let (basic, tier1) = (descriptor(&basic, me), descriptor(&tier1, me));
    let stats = |f: &benilla_protocol::ObjectFields| -> [u32; 5] {
        std::array::from_fn(|i| f.unit_stat(i as u8).unwrap())
    };
    let [s, a, st, i, sp] = stats(&basic);
    assert_eq!(stats(&tier1), [s + 12, a + 6, st + 12, i, sp + 3]);
    assert_eq!(
        tier1.unit_max_health().unwrap(),
        basic.unit_max_health().unwrap() + 120
    );
    assert_eq!(
        tier1.unit_attack_power().unwrap(),
        basic.unit_attack_power().unwrap() + 24
    );
}

// ── The Advancement Quest ───────────────────────────────────────────────────────────────────────

/// The Warrior trainer (Uzzek's entry), who gives the Advancement Quests.
const UZZEK: u32 = 5810;
/// Into Tier 1 Warrior, and Champion's, which cannot be finished.
const THE_WARRIORS_PATH: u32 = 64001;
const THE_CHAMPIONS_TRIAL: u32 = 64002;
/// QuestSort.dbc 81, "Warrior": the quest log's header, as a negative `ZoneOrSort`.
const SORT_WARRIOR: i32 = -81;

// The two Callings' skills (`content/callings.toml`).
const BASIC_SKILLS: [u32; 2] = [78, 100];
const TIER_1_SKILLS: [u32; 8] = [12294, 7887, 6178, 6546, 8198, 845, 5308, 6192];
/// Three of Cruelty's Recruit ranks.
const CRUELTY_RANKS: [u32; 3] = [12320, 12852, 12853];

/// The trainer's guid, from the entry burst.
fn trainer(events: &[SessionEvent]) -> u64 {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate { guid, fields, .. }
                if fields.object_entry() == Some(UZZEK) =>
            {
                Some(*guid)
            }
            _ => None,
        })
        .expect("the trainer stands at the Hub")
}

/// The quest rows of the trainer's gossip menu: `(quest, icon, level, title)`.
fn gossip_quests(realm: &mut Realm, npc: u64) -> Vec<(u32, u32, u32, String)> {
    realm.handle(RealmCommand::GossipHello { guid: npc });
    drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { quests, .. } => Some(quests),
            _ => None,
        })
        .expect("the trainer's menu")
}

/// Clicks an offered quest in the gossip menu and accepts it, as the stock frames do.
fn take(realm: &mut Realm, npc: u64, quest: u32) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::QuestGiverQueryQuest { npc, quest });
    let mut events = drain(realm);
    events.extend(accept(realm, npc, quest));
    events
}

/// A Basic Warrior at the cap with things to keep: 12345 copper, 7 Tokens, Cruelty 3/3, the worn
/// sword, a Hearthstone and skins in the bags, and a quest under way.
fn capped_warrior() -> (Realm, u64, Vec<SessionEvent>) {
    let items: &[Carried] = &[(15, 25, 1), (23, 6948, 1), (24, CROCOLISK_SKIN, 3)];
    let (guid, blob) = save_with_quests(10, items, 7, &[(THINNING_THE_HERDS, [1, 2, 0, 0])]);
    let blob = blob
        .replace("money = 0\n", "money = 12345\n")
        .replace("xp = 0\n", "xp = 0\ntalents = [[157, 3]]\n");
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

/// Takes the Warrior's Path and hands it in: the events of the whole Advancement.
fn advance(realm: &mut Realm, npc: u64) -> Vec<SessionEvent> {
    let mut events = take(realm, npc, THE_WARRIORS_PATH);
    events.extend(hand_in(realm, npc, THE_WARRIORS_PATH));
    events
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
fn the_trainer_offers_the_advancement_quest_only_at_the_tier_cap() {
    let (mut realm, _, events) = questing(9, &[], 0, &[]);
    let uzzek = trainer(&events);
    assert!(gossip_quests(&mut realm, uzzek).is_empty(), "not at 9");

    let (mut realm, _, events) = questing(10, &[], 0, &[]);
    let uzzek = trainer(&events);
    let fields = descriptor(&events, uzzek);
    assert_ne!(
        fields.unit_npc_flags() & NPC_FLAG_QUESTGIVER,
        0,
        "a quest giver too"
    );
    assert_eq!(status(&mut realm, uzzek), dialog_status::AVAILABLE);
    assert_eq!(
        gossip_quests(&mut realm, uzzek),
        [(
            THE_WARRIORS_PATH,
            dialog_status::AVAILABLE,
            10,
            "The Warrior's Path".to_string()
        )]
    );
    realm.handle(RealmCommand::QuestQuery {
        quest: THE_WARRIORS_PATH,
    });
    let template = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestTemplate(t) => Some(t),
            _ => None,
        })
        .expect("the quest template");
    assert_eq!(
        template.zone_or_sort, SORT_WARRIOR,
        "under the log's Warrior header"
    );
}

#[test]
fn a_tier_1_warrior_is_not_offered_the_warriors_path() {
    let (mut realm, _, events) = questing_as("warrior", 10, &[], 0, &[]);
    let uzzek = trainer(&events);
    assert!(gossip_quests(&mut realm, uzzek).is_empty());
    assert_eq!(status(&mut realm, uzzek), dialog_status::NONE);
}

#[test]
fn the_advancement_quest_is_finished_by_speaking_to_the_trainer() {
    let (mut realm, me, mut events) = questing(10, &[], 0, &[]);
    let uzzek = trainer(&events);
    events.extend(take(&mut realm, uzzek, THE_WARRIORS_PATH));
    // Nothing to kill or bring: complete as soon as it is in the log.
    let log = quest_log(&events, me);
    assert_eq!(log.len(), 1);
    assert_eq!((log[0].1, log[0].3), (THE_WARRIORS_PATH, 1));
    assert_eq!(status(&mut realm, uzzek), dialog_status::REWARD2);
    let rows = gossip_quests(&mut realm, uzzek);
    assert_eq!(rows[0].0, THE_WARRIORS_PATH);
    assert_eq!(rows[0].1, 4, "the menu's row to hand in");

    // The stock hand-in: straight to the reward panel, then the choice.
    realm.handle(RealmCommand::QuestGiverCompleteQuest {
        npc: uzzek,
        quest: THE_WARRIORS_PATH,
    });
    assert!(drain(&mut realm)
        .iter()
        .any(|e| matches!(e, SessionEvent::QuestOffer(o) if o.quest_id == THE_WARRIORS_PATH)));
    let done = completed(&hand_in(&mut realm, uzzek, THE_WARRIORS_PATH)).expect("it completes");
    assert_eq!(done.xp, 0);
}

#[test]
fn advancement_makes_a_level_1_tier_1_warrior_with_the_new_skills_and_tree() {
    let (mut realm, me, mut events) = capped_warrior();
    let before = descriptor(&events, me);
    assert_eq!(realm.current_calling().unwrap().key, "basic_warrior");
    events.extend(advance(&mut realm, trainer(&events)));

    assert_eq!(realm.current_calling().unwrap().key, "warrior");
    let now = descriptor(&events, me);
    assert_eq!(now.unit_level(), Some(1));
    assert_eq!(now.player_xp(), Some(0));
    assert_eq!(now.player_next_level_xp(), Some(400), "level 1's XP to go");
    assert_eq!(now.player_talent_points(), Some(0), "no points at level 1");
    assert_eq!(now.unit_health(), now.unit_max_health(), "at full health");

    // The kits swapped, and Cruelty's ranks gone with the old tree.
    let known = spellbook(&events);
    for s in TIER_1_SKILLS {
        assert!(known.contains(&s), "Tier 1 skill {s} learned");
    }
    for s in BASIC_SKILLS.iter().chain(&CRUELTY_RANKS) {
        assert!(!known.contains(s), "spell {s} forgotten");
    }
    let tabs: Vec<&str> = realm
        .current_calling()
        .unwrap()
        .talent_tabs
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(tabs, ["Arms", "Fury", "Protection"]);
    // The new Calling's default bar.
    let bar = events
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::ActionButtons { buttons } => Some(buttons.clone()),
            _ => None,
        })
        .expect("the bar is sent again");
    assert_eq!(bar.len(), 8);
    assert_eq!(bar[0].action, TIER_1_SKILLS[0]);

    // Stronger than a level 1 Basic Warrior: the Tier Bonus.
    let (_, _, fresh) = questing(1, &[], 0, &[]);
    let fresh = descriptor(&fresh, me);
    for i in 0..5 {
        assert!(now.unit_stat(i) >= fresh.unit_stat(i), "stat {i}");
    }
    assert!(now.unit_stat(0) > fresh.unit_stat(0));
    assert!(now.unit_max_health() > fresh.unit_max_health());
    assert!(
        now.unit_max_health() < before.unit_max_health(),
        "but not a level 10's"
    );
}

#[test]
fn advancement_keeps_gear_bags_gold_tokens_and_quests() {
    let (mut realm, me, mut events) = capped_warrior();
    let uzzek = trainer(&events);
    let gear_before = descriptor(&events, me).player_inv_slot(15);
    let bag_before = descriptor(&events, me).player_inv_slot(23);
    events.extend(advance(&mut realm, uzzek));
    let now = descriptor(&events, me);
    assert_eq!(now.player_inv_slot(15), gear_before, "the worn sword");
    assert_eq!(now.player_inv_slot(23), bag_before, "the Hearthstone");
    assert_eq!(now.player_money(), Some(12345));
    assert_eq!(
        quest_log(&events, me),
        [(0, THINNING_THE_HERDS, [1, 2, 0, 0], 0)],
        "the quest under way"
    );
    let blob = realm.export_character(me).unwrap();
    assert_eq!(carried(&events, me, TOKEN), 7, "the Tokens in the bags");
    assert!(blob.contains("money = 12345\n"), "{blob}");
    assert!(
        blob.contains(&format!("entry = {CROCOLISK_SKIN}\ncount = 3")),
        "{blob}"
    );
    assert!(
        blob.contains(&format!("completed = [{THE_WARRIORS_PATH}]")),
        "the completed quest is kept: {blob}"
    );
}

#[test]
fn completed_quests_carry_over_advancement() {
    let (mut realm, me, events) = questing(10, EIGHT_SKINS, 0, &[(CROCOLISK_SKINS, [0; 4])]);
    hand_in(&mut realm, giver(&events, ZUNJIN), CROCOLISK_SKINS);
    advance(&mut realm, trainer(&events));
    let blob = realm.export_character(me).unwrap();
    assert!(
        blob.contains(&format!(
            "completed = [{CROCOLISK_SKINS}, {THE_WARRIORS_PATH}]"
        )),
        "{blob}"
    );
}

#[test]
fn the_calling_survives_the_save() {
    let (mut realm, me, events) = capped_warrior();
    advance(&mut realm, trainer(&events));
    let blob = realm.export_character(me).unwrap();
    assert!(blob.contains("calling = \"warrior\"\n"), "{blob}");
    assert!(blob.contains("level = 1\n"), "{blob}");
    assert!(!blob.contains("talents"), "no talents left: {blob}");

    let mut reloaded = Realm::with_seed(SEED);
    reloaded.import_character(&blob).unwrap();
    reloaded.handle(RealmCommand::EnterWorld { guid: me });
    reloaded.tick(0.0, &FlatGround);
    let events = drain(&mut reloaded);
    assert_eq!(reloaded.current_calling().unwrap().key, "warrior");
    let known = spellbook(&events);
    assert!(TIER_1_SKILLS.iter().all(|s| known.contains(s)));
    assert_eq!(descriptor(&events, me).unit_level(), Some(1));
    assert_eq!(
        reloaded.export_character(me).unwrap(),
        blob,
        "the save reads back as it was written"
    );
}

#[test]
fn advancement_saves_the_character() {
    let (mut realm, me, events) = capped_warrior();
    realm.drain_storage();
    advance(&mut realm, trainer(&events));
    let saved = realm.drain_storage();
    assert!(saved.iter().any(|s| matches!(
        s,
        twilight_realm::Storage::Write { guid, blob } if *guid == me && blob.contains("calling = \"warrior\"")
    )));
}

// ── Champion ────────────────────────────────────────────────────────────────────────────────────

#[test]
fn at_thirty_champions_quest_is_offered_but_cannot_be_finished() {
    let (mut realm, _, events) = questing_as("warrior", 29, &[], 0, &[]);
    assert!(
        gossip_quests(&mut realm, trainer(&events)).is_empty(),
        "not at 29"
    );

    let (mut realm, me, mut events) = questing_as("warrior", 30, &[], 0, &[]);
    let uzzek = trainer(&events);
    let rows = gossip_quests(&mut realm, uzzek);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0].0, rows[0].1, rows[0].3.as_str()),
        (
            THE_CHAMPIONS_TRIAL,
            dialog_status::AVAILABLE,
            "The Champion's Trial"
        )
    );
    events.extend(take(&mut realm, uzzek, THE_CHAMPIONS_TRIAL));
    let log = quest_log(&events, me);
    assert_eq!(
        (log[0].1, log[0].3),
        (THE_CHAMPIONS_TRIAL, 0),
        "never complete"
    );
    assert_eq!(status(&mut realm, uzzek), dialog_status::INCOMPLETE);

    realm.handle(RealmCommand::QuestGiverCompleteQuest {
        npc: uzzek,
        quest: THE_CHAMPIONS_TRIAL,
    });
    let progress = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestProgress(p) => Some(p),
            _ => None,
        })
        .expect("the progress panel");
    assert!(!progress.is_complete);
    assert!(completed(&hand_in(&mut realm, uzzek, THE_CHAMPIONS_TRIAL)).is_none());
    assert_eq!(realm.current_calling().unwrap().key, "warrior");
}

// ── Character select ────────────────────────────────────────────────────────────────────────────

#[test]
fn the_select_screen_names_each_characters_calling() {
    let (mut realm, me, events) = capped_warrior();
    assert_eq!(realm.calling_name(me), Some("Basic Warrior"));
    advance(&mut realm, trainer(&events));
    realm.handle(RealmCommand::Logout);
    assert_eq!(realm.calling_name(me), Some("Warrior"));
    assert_eq!(realm.calling_name(me + 1), None, "nobody");
}
