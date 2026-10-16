//! Tiers and Advancement, played through the realm's seam: the Tier Cap that stops XP, the
//! Advancement Quest at the Warrior trainer, what Advancement replaces and what it keeps, the
//! Tier Bonus, Zealot's quest from Vanguard, and Ironclad's and Templar's quests that cannot be
//! finished.

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
    walk_up_to(&mut realm, zunjin);
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
    walk_up_to(&mut realm, zunjin);
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
fn a_vanguard_levels_past_ten_and_stops_at_thirty() {
    let quest = &[(CROCOLISK_SKINS, [0; 4])];
    // Level 10 a Vanguard keeps: 4000 of its 7600 XP, and the quest's 3800 on top.
    let (guid, blob) = save_with_quests(10, EIGHT_SKINS, 0, quest);
    let blob = blob
        .replace("calling = \"basic_warrior\"", "calling = \"vanguard\"")
        .replace(
            "xp = 0
",
            "xp = 4000
",
        );
    let (mut realm, me, mut events) = enter_saved(guid, &blob);
    events.extend(hand_in(&mut realm, giver(&events, ZUNJIN), CROCOLISK_SKINS));
    assert_eq!(descriptor(&events, me).unit_level(), Some(11));

    let (mut realm, me, mut events) = questing_as("vanguard", 30, EIGHT_SKINS, 0, quest);
    let turned_in = hand_in(&mut realm, giver(&events, ZUNJIN), CROCOLISK_SKINS);
    assert_eq!(completed(&turned_in).unwrap().xp, 0, "Tier 1's cap is 30");
    events.extend(turned_in);
    assert_eq!(descriptor(&events, me).unit_level(), Some(30));
}

// ── The Tier Bonus ──────────────────────────────────────────────────────────────────────────────

#[test]
fn a_vanguard_has_the_tier_bonus_on_its_sheet() {
    let (_, me, basic) = questing(1, &[], 0, &[]);
    let (_, _, tier1) = questing_as("vanguard", 1, &[], 0, &[]);
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
/// Into Vanguard, and Ironclad's, which cannot be finished.
const THE_WARRIORS_PATH: u32 = 64001;
const THE_IRONCLADS_OATH: u32 = 64002;
/// Into Zealot, from Vanguard; and Templar's, from Zealot, which cannot be finished.
const THE_ZEALOTS_VOW: u32 = 64003;
const THE_TEMPLARS_CHARGE: u32 = 64004;
/// Plate, required level 40.
const JOUSTERS_CHESTPLATE: u32 = 8157;
/// QuestSort.dbc 81, "Warrior": the quest log's header, as a negative `ZoneOrSort`.
const SORT_WARRIOR: i32 = -81;

// The two Callings' skills (`content/callings.toml`).
const BASIC_SKILLS: [u32; 2] = [78, 100];
/// Vanguard's: Mortal Strike, Overpower, Charge, Rend and Battle Shout.
const TIER_1_SKILLS: [u32; 5] = [12294, 7887, 6178, 6546, 6192];
/// Zealot's: Seal of Command, Seal of the Crusader, Judgement, Hammer of Justice, Mortal Strike and
/// Charge.
const ZEALOT_SKILLS: [u32; 6] = [20375, 20305, 20271, 10308, 12294, 6178];
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
    walk_up_to(realm, npc);
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
fn a_vanguard_is_not_offered_the_warriors_path() {
    let (mut realm, _, events) = questing_as("vanguard", 10, &[], 0, &[]);
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
fn advancement_makes_a_level_1_vanguard_with_the_new_skills_and_tree() {
    let (mut realm, me, mut events) = capped_warrior();
    let before = descriptor(&events, me);
    assert_eq!(realm.current_calling().unwrap().key, "basic_warrior");
    events.extend(advance(&mut realm, trainer(&events)));

    assert_eq!(realm.current_calling().unwrap().key, "vanguard");
    let now = descriptor(&events, me);
    assert_eq!(now.unit_level(), Some(1));
    assert_eq!(now.player_xp(), Some(0));
    assert_eq!(now.player_next_level_xp(), Some(400), "level 1's XP to go");
    assert_eq!(now.player_talent_points(), Some(0), "no points at level 1");
    assert_eq!(now.unit_health(), now.unit_max_health(), "at full health");

    // The kits swapped, and Cruelty's ranks gone with the old tree.
    let known = spellbook(&events);
    assert_eq!(
        known,
        TIER_1_SKILLS.into_iter().collect(),
        "exactly the kit"
    );
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
    assert_eq!(tabs, ["Weaponmaster", "Warlord"]);
    // The new Calling's default bar.
    let bar = events
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::ActionButtons { buttons } => Some(buttons.clone()),
            _ => None,
        })
        .expect("the bar is sent again");
    assert_eq!(bar.len(), 5);
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
fn advancement_keeps_gear_bags_gold_and_tokens() {
    let (mut realm, me, mut events) = capped_warrior();
    let uzzek = trainer(&events);
    let gear_before = descriptor(&events, me).player_inv_slot(15);
    let bag_before = descriptor(&events, me).player_inv_slot(23);
    events.extend(advance(&mut realm, uzzek));
    let now = descriptor(&events, me);
    assert_eq!(now.player_inv_slot(15), gear_before, "the worn sword");
    assert_eq!(now.player_inv_slot(23), bag_before, "the Hearthstone");
    assert_eq!(now.player_money(), Some(12345));
    let blob = realm.export_character(me).unwrap();
    assert_eq!(carried(&events, me, TOKEN), 7, "the Tokens in the bags");
    assert!(
        blob.contains(
            "money = 12345
"
        ),
        "{blob}"
    );
    assert!(
        blob.contains(&format!(
            "entry = {CROCOLISK_SKIN}
count = 3"
        )),
        "{blob}"
    );
}

#[test]
fn advancement_drops_the_hunting_quests_under_way() {
    let (mut realm, me, mut events) = capped_warrior();
    assert_eq!(
        quest_log(&events, me),
        [(0, THINNING_THE_HERDS, [1, 2, 0, 0], 0)]
    );
    events.extend(advance(&mut realm, trainer(&events)));
    assert!(
        quest_log(&events, me).is_empty(),
        "{:?}",
        quest_log(&events, me)
    );
    // Offered afresh to the level 1 Vanguard.
    assert!(on_offer(&mut realm, giver(&events, GREZZ)).contains(&THINNING_THE_HERDS));
}

#[test]
fn after_advancement_every_hunting_quest_is_offered_again() {
    let herds_done: &[Underway] = &[(THINNING_THE_HERDS, [5, 5, 0, 0])];
    let (mut realm, me, events) = questing(10, &[(23, VENOM_SAC, 6)], 0, herds_done);
    let grezz = giver(&events, GREZZ);
    accept(&mut realm, grezz, SCORPID_VENOM);
    hand_in(&mut realm, grezz, THINNING_THE_HERDS);
    hand_in(&mut realm, grezz, SCORPID_VENOM);
    assert!(on_offer(&mut realm, grezz).is_empty(), "done this Tier");

    advance(&mut realm, trainer(&events));
    assert_eq!(status(&mut realm, grezz), dialog_status::AVAILABLE);
    assert_eq!(
        on_offer(&mut realm, grezz),
        [THINNING_THE_HERDS, SCORPID_VENOM]
    );
    // The Advancement Quest stays done.
    let blob = realm.export_character(me).unwrap();
    assert!(
        blob.contains(&format!("completed = [{THE_WARRIORS_PATH}]")),
        "{blob}"
    );
}

#[test]
fn a_talent_respec_leaves_the_quests_alone() {
    let quests: &[Underway] = &[
        (CROCOLISK_SKINS, [0; 4]),
        (THINNING_THE_HERDS, [1, 2, 0, 0]),
    ];
    let (guid, blob) = save_with_quests(10, EIGHT_SKINS, 0, quests);
    let blob = blob
        .replace(
            "money = 0
",
            "money = 12345
",
        )
        .replace(
            "xp = 0
",
            "xp = 0
talents = [[157, 3]]
",
        );
    let (mut realm, me, mut events) = enter_saved(guid, &blob);
    let zunjin = giver(&events, ZUNJIN);
    events.extend(hand_in(&mut realm, zunjin, CROCOLISK_SKINS));
    let log_before = quest_log(&events, me);
    assert_eq!(log_before, [(1, THINNING_THE_HERDS, [1, 2, 0, 0], 0)]);

    let uzzek = trainer(&events);
    walk_up_to(&mut realm, uzzek);
    realm.handle(RealmCommand::TalentWipeConfirm { trainer: uzzek });
    events.extend(drain(&mut realm));
    assert_eq!(
        descriptor(&events, me).player_money(),
        Some(12345 + 400 - 10000),
        "the respec went through"
    );
    assert_eq!(quest_log(&events, me), log_before, "the quest under way");
    assert!(
        !on_offer(&mut realm, zunjin).contains(&CROCOLISK_SKINS),
        "still done this Tier"
    );
}

#[test]
fn advancement_resends_proficiency_with_plate() {
    let (mut realm, me, mut events) = capped_warrior();
    let armour = |events: &[SessionEvent]| {
        events
            .iter()
            .rev()
            .find_map(|e| match e {
                SessionEvent::Proficiency {
                    item_class: 4,
                    subclass_mask,
                } => Some(*subclass_mask),
                _ => None,
            })
            .expect("an armour proficiency")
    };
    assert_eq!(armour(&events) & 1 << 4, 0, "no plate as a Basic Warrior");
    let advanced = advance(&mut realm, trainer(&events));
    assert_ne!(armour(&advanced) & 1 << 4, 0, "plate as a Vanguard");
    events.extend(advanced);
    assert_eq!(descriptor(&events, me).unit_level(), Some(1));
}

#[test]
fn gear_worn_before_advancement_stays_worn_and_keeps_its_stats() {
    // Required level 25 mail, worn by a level 10 Basic Warrior who becomes a level 1 Vanguard.
    const LARGE_OGRE_CHAIN_ARMOR: u32 = 914;
    let (mut realm, me, mut events) = questing(10, &[(4, LARGE_OGRE_CHAIN_ARMOR, 1)], 0, &[]);
    let chest = descriptor(&events, me).player_inv_slot(4);
    let green = descriptor(&events, me).player_posstat(0);
    assert_eq!(green, Some(10), "its +10 strength");
    events.extend(advance(&mut realm, trainer(&events)));
    let now = descriptor(&events, me);
    assert_eq!(now.unit_level(), Some(1));
    assert_eq!(now.player_inv_slot(4), chest);
    assert_eq!(now.player_posstat(0), green, "still counting");
}

#[test]
fn the_calling_survives_the_save() {
    let (mut realm, me, events) = capped_warrior();
    advance(&mut realm, trainer(&events));
    let blob = realm.export_character(me).unwrap();
    assert!(blob.contains("calling = \"vanguard\"\n"), "{blob}");
    assert!(blob.contains("level = 1\n"), "{blob}");
    assert!(!blob.contains("talents"), "no talents left: {blob}");

    let mut reloaded = Realm::with_seed(SEED);
    reloaded.import_character(&blob).unwrap();
    reloaded.handle(RealmCommand::EnterWorld { guid: me });
    reloaded.tick(0.0, &FlatGround);
    let events = drain(&mut reloaded);
    assert_eq!(reloaded.current_calling().unwrap().key, "vanguard");
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
fn a_save_from_before_the_rename_loads_as_a_vanguard() {
    // Tier 1 was saved as "warrior" before it was Vanguard; Cruelty sits in the new tree too.
    let (guid, blob) = save_with(12, 0, &[]);
    let blob = blob
        .replace("calling = \"basic_warrior\"", "calling = \"warrior\"")
        .replace("money = 0\n", "money = 0\ntalents = [[157, 3]]\n");
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    assert_eq!(realm.current_calling().unwrap().key, "vanguard");
    assert_eq!(realm.calling_name(guid), Some("Vanguard"));
    let known = spellbook(&events);
    assert!(TIER_1_SKILLS.iter().all(|s| known.contains(s)));
    assert!(
        CRUELTY_RANKS.iter().all(|s| known.contains(s)),
        "the talent kept"
    );
    assert_eq!(descriptor(&events, guid).unit_level(), Some(12));
    let saved = realm.export_character(guid).unwrap();
    assert!(saved.contains("calling = \"vanguard\"\n"), "{saved}");
}

#[test]
fn advancement_saves_the_character() {
    let (mut realm, me, events) = capped_warrior();
    realm.drain_storage();
    advance(&mut realm, trainer(&events));
    let saved = realm.drain_storage();
    assert!(saved.iter().any(|s| matches!(
        s,
        twilight_realm::Storage::Write { guid, blob } if *guid == me && blob.contains("calling = \"vanguard\"")
    )));
}

// ── Ironclad ────────────────────────────────────────────────────────────────────────────────────

#[test]
fn at_thirty_ironclads_quest_is_offered_but_cannot_be_finished() {
    let (mut realm, _, events) = questing_as("vanguard", 29, &[], 0, &[]);
    assert!(
        gossip_quests(&mut realm, trainer(&events)).is_empty(),
        "not at 29"
    );

    let (mut realm, me, mut events) = questing_as("vanguard", 30, &[], 0, &[]);
    let uzzek = trainer(&events);
    let rows = gossip_quests(&mut realm, uzzek);
    assert_eq!(
        rows.iter()
            .map(|r| (r.0, r.1, r.3.as_str()))
            .collect::<Vec<_>>(),
        [
            (
                THE_IRONCLADS_OATH,
                dialog_status::AVAILABLE,
                "The Ironclad's Oath"
            ),
            (
                THE_ZEALOTS_VOW,
                dialog_status::AVAILABLE,
                "The Zealot's Vow"
            ),
        ],
        "both of Vanguard's branches"
    );
    events.extend(take(&mut realm, uzzek, THE_IRONCLADS_OATH));
    let log = quest_log(&events, me);
    assert_eq!(
        (log[0].1, log[0].3),
        (THE_IRONCLADS_OATH, 0),
        "never complete"
    );
    assert_eq!(
        status(&mut realm, uzzek),
        dialog_status::AVAILABLE,
        "the Zealot's Vow still on offer"
    );

    realm.handle(RealmCommand::QuestGiverCompleteQuest {
        npc: uzzek,
        quest: THE_IRONCLADS_OATH,
    });
    let progress = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestProgress(p) => Some(p),
            _ => None,
        })
        .expect("the progress panel");
    assert!(!progress.is_complete);
    assert!(completed(&hand_in(&mut realm, uzzek, THE_IRONCLADS_OATH)).is_none());
    assert_eq!(realm.current_calling().unwrap().key, "vanguard");
}

// ── Zealot ──────────────────────────────────────────────────────────────────────────────────────

/// A Vanguard at the cap in Jouster's Chestplate, with Cruelty 3/3.
fn capped_vanguard() -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(30, &[(4, JOUSTERS_CHESTPLATE, 1)], 0, &[]);
    let blob = blob
        .replace("calling = \"basic_warrior\"", "calling = \"vanguard\"")
        .replace("xp = 0\n", "xp = 0\ntalents = [[157, 3]]\n");
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

/// Takes the Zealot's Vow and hands it in.
fn become_zealot(realm: &mut Realm, npc: u64) -> Vec<SessionEvent> {
    let mut events = take(realm, npc, THE_ZEALOTS_VOW);
    events.extend(hand_in(realm, npc, THE_ZEALOTS_VOW));
    events
}

#[test]
fn the_zealots_vow_makes_a_level_1_zealot_with_six_skills_in_the_same_plate() {
    let (mut realm, me, mut events) = capped_vanguard();
    let chest = descriptor(&events, me).player_inv_slot(4);
    assert!(chest.is_some(), "the plate is worn");
    let advanced = become_zealot(&mut realm, trainer(&events));
    assert!(completed(&advanced).is_some(), "it completes");
    events.extend(advanced);

    assert_eq!(realm.current_calling().unwrap().key, "zealot");
    let now = descriptor(&events, me);
    assert_eq!(now.unit_level(), Some(1));
    assert_eq!(now.player_xp(), Some(0));
    assert_eq!(
        spellbook(&events),
        ZEALOT_SKILLS.into_iter().collect(),
        "exactly the six, Cruelty and the Vanguard kit forgotten"
    );
    assert_eq!(now.player_inv_slot(4), chest, "the plate stays worn");
    assert_eq!(now.unit_shapeshift_form(), 17, "still in Battle Stance");
    assert_eq!(now.unit_power(1), Some(0), "a rage bar");
}

#[test]
fn a_zealot_has_the_tier_2_bonus_on_its_sheet() {
    let (_, me, basic) = questing(1, &[], 0, &[]);
    let (_, _, zealot) = questing_as("zealot", 1, &[], 0, &[]);
    let (basic, zealot) = (descriptor(&basic, me), descriptor(&zealot, me));
    let stats = |f: &benilla_protocol::ObjectFields| -> [u32; 5] {
        std::array::from_fn(|i| f.unit_stat(i as u8).unwrap())
    };
    let [s, a, st, i, sp] = stats(&basic);
    assert_eq!(stats(&zealot), [s + 28, a + 14, st + 28, i, sp + 7]);
}

#[test]
fn a_zealot_save_round_trips_and_names_the_calling() {
    let (mut realm, me, events) = capped_vanguard();
    become_zealot(&mut realm, trainer(&events));
    realm.handle(RealmCommand::Logout);
    assert_eq!(realm.calling_name(me), Some("Zealot"));
    let blob = realm.export_character(me).unwrap();
    assert!(blob.contains("calling = \"zealot\"\n"), "{blob}");

    let mut reloaded = Realm::with_seed(SEED);
    reloaded.import_character(&blob).unwrap();
    assert_eq!(reloaded.calling_name(me), Some("Zealot"));
    reloaded.handle(RealmCommand::EnterWorld { guid: me });
    reloaded.tick(0.0, &FlatGround);
    let events = drain(&mut reloaded);
    assert_eq!(spellbook(&events), ZEALOT_SKILLS.into_iter().collect());
    assert_eq!(
        reloaded.export_character(me).unwrap(),
        blob,
        "the save reads back as it was written"
    );
}

#[test]
fn at_fifty_templars_quest_is_offered_but_cannot_be_finished() {
    let (mut realm, _, events) = questing_as("zealot", 49, &[], 0, &[]);
    assert!(
        gossip_quests(&mut realm, trainer(&events)).is_empty(),
        "not at 49"
    );

    let (mut realm, me, mut events) = questing_as("zealot", 50, &[], 0, &[]);
    let uzzek = trainer(&events);
    let rows = gossip_quests(&mut realm, uzzek);
    assert_eq!(
        rows.iter()
            .map(|r| (r.0, r.1, r.3.as_str()))
            .collect::<Vec<_>>(),
        [(
            THE_TEMPLARS_CHARGE,
            dialog_status::AVAILABLE,
            "The Templar's Charge"
        )]
    );
    events.extend(take(&mut realm, uzzek, THE_TEMPLARS_CHARGE));
    let log = quest_log(&events, me);
    assert_eq!((log[0].1, log[0].3), (THE_TEMPLARS_CHARGE, 0));
    assert_eq!(status(&mut realm, uzzek), dialog_status::INCOMPLETE);
    assert!(completed(&hand_in(&mut realm, uzzek, THE_TEMPLARS_CHARGE)).is_none());
    assert_eq!(realm.current_calling().unwrap().key, "zealot");
}

// ── Character select ────────────────────────────────────────────────────────────────────────────

#[test]
fn the_select_screen_names_each_characters_calling() {
    let (mut realm, me, events) = capped_warrior();
    assert_eq!(realm.calling_name(me), Some("Basic Warrior"));
    advance(&mut realm, trainer(&events));
    realm.handle(RealmCommand::Logout);
    assert_eq!(realm.calling_name(me), Some("Vanguard"));
    assert_eq!(realm.calling_name(me + 1), None, "nobody");
}
