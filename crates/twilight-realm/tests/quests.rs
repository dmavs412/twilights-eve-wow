//! Hunting Quests: the Hub's givers, accepting, progress and hand-in, once per character, through the
//! stock quest dialogs and the `PLAYER_QUEST_LOG` slots.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::messages::dialog_status;
use benilla_protocol::SessionEvent;
use common::*;
use gear_common::*;
use quest_common::*;
use twilight_realm::CROSSROADS;

#[test]
fn quest_givers_stand_at_the_hub_with_names_and_marks() {
    let (mut realm, _, events) = warrior_with(1, 0, &[]);
    // The Warrior trainer (5810) gives quests too: his Advancement Quests.
    // One giver per Hunting Ground, each at a gate or the watchtower of the town.
    for entry in [GREZZ, TAMAHA, ZUNJIN, 90104, MAHU] {
        let at = hub_home(giver(&events, entry));
        let d = ((at[0] - CROSSROADS[0]).powi(2) + (at[1] - CROSSROADS[1]).powi(2)).sqrt();
        assert!(d < 160.0, "giver {entry} stands {d} yd from the inn");
    }

    realm.handle(RealmCommand::CreatureQuery { entry: GREZZ });
    let name = drain(&mut realm).into_iter().find_map(|e| match e {
        SessionEvent::CreatureName { entry, name, .. } if entry == GREZZ => name,
        _ => None,
    });
    assert_eq!(name.as_deref(), Some("Grezz Plainswatcher"));

    // A new warrior sees a yellow `!` over the plains giver, and nothing over the one far south.
    assert_eq!(
        status(&mut realm, giver(&events, GREZZ)),
        dialog_status::AVAILABLE
    );
    assert_eq!(
        status(&mut realm, giver(&events, MAHU)),
        dialog_status::NONE
    );
}

#[test]
fn a_giver_lists_its_quests_and_the_accepted_one_goes_in_the_log() {
    let (mut realm, me, mut events) = warrior_with(1, 0, &[]);
    let grezz = giver(&events, GREZZ);
    walk_up_to(&mut realm, grezz);

    // Two quests on offer: the greeting list, both under "Available".
    let greeting = talk(&mut realm, grezz)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestGreeting(list) => Some(list),
            _ => None,
        })
        .expect("the greeting list");
    assert_eq!(greeting.npc, grezz);
    let rows: Vec<(u32, u32)> = greeting
        .quests
        .iter()
        .map(|q| (q.quest_id, q.level))
        .collect();
    // Every Tier 0 quest of the giver, whatever its level.
    assert_eq!(
        rows,
        [
            (THINNING_THE_HERDS, 2),
            (SCORPID_VENOM, 3),
            (91003, 4),
            (91004, 5),
            (91005, 6),
            (91052, 3),
            (91053, 4),
            (91051, 4)
        ]
    );
    assert!(greeting
        .quests
        .iter()
        .all(|q| q.icon == dialog_status::AVAILABLE));

    // Picking one opens its accept panel.
    realm.handle(RealmCommand::QuestGiverQueryQuest {
        npc: grezz,
        quest: THINNING_THE_HERDS,
    });
    let detail = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestDetail(d) => Some(d),
            _ => None,
        })
        .expect("the accept panel");
    assert_eq!(detail.title, "Thinning the Herds");
    // 60 copper and the old Token's worth at level 2, 68.
    assert_eq!(detail.money, 128);

    // Accepting closes the dialog and fills the first log slot.
    let accepted = accept(&mut realm, grezz, THINNING_THE_HERDS);
    assert!(accepted
        .iter()
        .any(|e| matches!(e, SessionEvent::GossipComplete)));
    events.extend(accepted);
    assert_eq!(quest_log(&events, me), [(0, THINNING_THE_HERDS, [0; 4], 0)]);
    // A yellow `!` for the other quest outranks the grey `?` of the one under way.
    assert_eq!(status(&mut realm, grezz), dialog_status::AVAILABLE);
    accept(&mut realm, grezz, SCORPID_VENOM);
    // Grezz's other quests are still on offer.
    assert_eq!(status(&mut realm, grezz), dialog_status::AVAILABLE);

    // The log asks for the template: kill five of each, in The Barrens.
    realm.handle(RealmCommand::QuestQuery {
        quest: THINNING_THE_HERDS,
    });
    let t = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestTemplate(t) => Some(t),
            _ => None,
        })
        .expect("the quest template");
    assert_eq!(
        (t.quest_id, t.level, t.zone_or_sort),
        (THINNING_THE_HERDS, 2, 17)
    );
    let kills: Vec<(u32, u32)> = t
        .objectives
        .iter()
        .map(|o| (o.creature_or_go, o.required_count))
        .collect();
    assert_eq!(kills, [(2955, 5), (3098, 5), (0, 0), (0, 0)]);
}

#[test]
fn a_giver_far_away_does_not_talk() {
    let (mut realm, _, events) = warrior_with(1, 0, &[]);
    let grezz = giver(&events, GREZZ);
    walk_up_to(&mut realm, grezz);
    let [x, y, _] = hub_home(grezz);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [x + 40.0, y, 0.0],
        orientation: 0.0,
    });
    realm.handle(RealmCommand::QuestGiverHello { npc: grezz });
    assert!(drain(&mut realm).is_empty());
    realm.handle(RealmCommand::QuestGiverAcceptQuest {
        npc: grezz,
        quest: THINNING_THE_HERDS,
    });
    assert!(drain(&mut realm).is_empty());
}

/// The Vile Familiar in front of the inn: the first unit to stream in.
const VILE_FAMILIAR: u32 = 3101;

#[test]
fn a_kill_counts_toward_the_quest_and_the_last_one_completes_it() {
    // Four familiars down, one to go.
    let (mut realm, me, mut events) = questing(5, &[], 0, &[(THE_FAMILIAR_THREAT, [4, 0, 0, 0])]);
    let grezz = giver(&events, GREZZ);
    walk_up_to(&mut realm, grezz);
    // Grezz's other quests' yellow `!` shows until this one is done.
    assert_eq!(status(&mut realm, grezz), dialog_status::AVAILABLE);
    let mob = the_mob(&events);
    assert_eq!(mob.fields.object_entry(), Some(VILE_FAMILIAR));

    let fight = kill(&mut realm, &mob);
    let ticks: Vec<(u32, u32, u32, u32)> = fight
        .iter()
        .filter_map(|e| match e {
            SessionEvent::QuestObjectiveKill {
                quest_id,
                entry,
                count,
                required,
            } => Some((*quest_id, *entry, *count, *required)),
            _ => None,
        })
        .collect();
    assert_eq!(ticks, [(THE_FAMILIAR_THREAT, VILE_FAMILIAR, 5, 5)]);
    assert!(fight.iter().any(|e| matches!(
        e,
        SessionEvent::QuestObjectivesComplete { quest_id } if *quest_id == THE_FAMILIAR_THREAT
    )));
    events.extend(fight);
    // `QUEST_STATE_COMPLETE` in the slot, and a yellow `?` over the giver.
    assert_eq!(
        quest_log(&events, me),
        [(0, THE_FAMILIAR_THREAT, [5, 0, 0, 0], 1)]
    );
    assert_eq!(status(&mut realm, grezz), dialog_status::REWARD2);
}

fn levels(events: &[SessionEvent]) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::LevelUp(l) => Some(l.level),
            _ => None,
        })
        .collect()
}

#[test]
fn a_hunting_quest_at_its_own_level_gives_half_a_level_and_its_copper_and_is_not_offered_again() {
    let (mut realm, me, mut events) = questing(4, &[], 0, &[(THE_FAMILIAR_THREAT, [5, 0, 0, 0])]);
    let grezz = giver(&events, GREZZ);
    walk_up_to(&mut realm, grezz);

    // A finished kill quest skips the progress panel: its reward panel opens.
    realm.handle(RealmCommand::QuestGiverCompleteQuest {
        npc: grezz,
        quest: THE_FAMILIAR_THREAT,
    });
    let offer = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestOffer(o) => Some(o),
            _ => None,
        })
        .expect("the reward panel");
    // 100 copper and the old Token's worth at level 4, 75.
    assert_eq!((offer.quest_id, offer.money), (THE_FAMILIAR_THREAT, 175));
    assert!(offer.rewards.is_empty(), "no Token");

    let turned_in = hand_in(&mut realm, grezz, THE_FAMILIAR_THREAT);
    let done = completed(&turned_in).expect("the quest completes");
    // Half of level 4's 2100 XP.
    assert_eq!(
        (done.quest_id, done.xp, done.money),
        (THE_FAMILIAR_THREAT, 1050, 175)
    );
    assert!(done.items.is_empty(), "no Token: {:?}", done.items);
    assert!(levels(&turned_in).is_empty());
    events.extend(turned_in);
    let me_now = descriptor(&events, me);
    assert_eq!(me_now.unit_level(), Some(4));
    assert_eq!(me_now.player_xp(), Some(1050));
    assert_eq!(me_now.player_money(), Some(175));
    assert_eq!(carried(&events, me, TOKEN), 0);
    assert!(quest_log(&events, me).is_empty(), "the slot clears");

    // Done for this Tier: Grezz no longer offers it, and it cannot be taken.
    let offered = on_offer(&mut realm, grezz);
    assert!(!offered.contains(&THE_FAMILIAR_THREAT), "{offered:?}");
    assert!(!offered.is_empty(), "her other quests are still on offer");
    realm.handle(RealmCommand::QuestGiverQueryQuest {
        npc: grezz,
        quest: THE_FAMILIAR_THREAT,
    });
    assert!(!drain(&mut realm)
        .iter()
        .any(|e| matches!(e, SessionEvent::QuestDetail(_))));
    events.extend(accept(&mut realm, grezz, THE_FAMILIAR_THREAT));
    assert!(quest_log(&events, me).is_empty());
}

#[test]
fn a_giver_does_not_offer_the_hunting_quests_a_character_has_done() {
    let (mut realm, _, events) = questing(
        1,
        &[(23, VENOM_SAC, 6)],
        0,
        &[(THINNING_THE_HERDS, [5, 5, 0, 0]), (SCORPID_VENOM, [0; 4])],
    );
    let grezz = giver(&events, GREZZ);
    hand_in(&mut realm, grezz, THINNING_THE_HERDS);
    hand_in(&mut realm, grezz, SCORPID_VENOM);
    let offered = on_offer(&mut realm, grezz);
    assert!(!offered.contains(&THINNING_THE_HERDS) && !offered.contains(&SCORPID_VENOM));
    // Grezz's later quests are offered at any level of the Tier.
    assert_eq!(status(&mut realm, grezz), dialog_status::AVAILABLE);
}

#[test]
fn an_unfinished_quest_shows_its_progress_and_cannot_be_handed_in() {
    let (mut realm, _, events) = questing(4, &[], 0, &[(THE_FAMILIAR_THREAT, [2, 0, 0, 0])]);
    let grezz = giver(&events, GREZZ);
    walk_up_to(&mut realm, grezz);
    realm.handle(RealmCommand::QuestGiverCompleteQuest {
        npc: grezz,
        quest: THE_FAMILIAR_THREAT,
    });
    let progress = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestProgress(p) => Some(p),
            _ => None,
        })
        .expect("the progress panel");
    assert!(!progress.is_complete);
    assert!(completed(&hand_in(&mut realm, grezz, THE_FAMILIAR_THREAT)).is_none());
}

#[test]
fn a_collect_quest_takes_its_items_and_pays_money() {
    // A level 15 Vanguard on a level 10 quest: still half of level 10's 7600 XP, at the Tier XP
    // Rate's ×3.
    let skins: &[Carried] = &[(23, CROCOLISK_SKIN, 5), (24, CROCOLISK_SKIN, 4)];
    let (mut realm, me, mut events) =
        questing_as("vanguard", 15, skins, 2, &[(CROCOLISK_SKINS, [0; 4])]);
    let zunjin = giver(&events, ZUNJIN);
    walk_up_to(&mut realm, zunjin);
    assert_eq!(quest_log(&events, me)[0].3, 1, "complete from the bags");

    // The progress panel asks for the skins and may go on; the reward panel shows the copper.
    realm.handle(RealmCommand::QuestGiverCompleteQuest {
        npc: zunjin,
        quest: CROCOLISK_SKINS,
    });
    realm.handle(RealmCommand::QuestGiverRequestReward {
        npc: zunjin,
        quest: CROCOLISK_SKINS,
    });
    let panels = drain(&mut realm);
    let progress = panels
        .iter()
        .find_map(|e| match e {
            SessionEvent::QuestProgress(p) => Some(p.clone()),
            _ => None,
        })
        .expect("the progress panel");
    assert!(progress.is_complete);
    let wanted: Vec<(u32, u32)> = progress
        .required_items
        .iter()
        .map(|i| (i.item_id, i.count))
        .collect();
    assert_eq!(wanted, [(CROCOLISK_SKIN, 8)]);
    let offer = panels
        .iter()
        .find_map(|e| match e {
            SessionEvent::QuestOffer(o) => Some(o.clone()),
            _ => None,
        })
        .expect("the reward panel");
    // 400 copper and the old Token's worth at level 10, 100.
    assert_eq!(offer.money, 500);
    assert!(offer.rewards.is_empty(), "no Token");

    let turned_in = hand_in(&mut realm, zunjin, CROCOLISK_SKINS);
    let done = completed(&turned_in).expect("the quest completes");
    assert_eq!((done.xp, done.money), (3 * 3800, 500));
    assert!(done.items.is_empty(), "no Token: {:?}", done.items);
    events.extend(turned_in);
    // Eight of the nine skins are gone: the first stack whole, three of the second... or the
    // other way round; one skin is left either way.
    let left: u32 = item_creates(&events)
        .iter()
        .filter(|(guid, f)| {
            f.object_entry() == Some(CROCOLISK_SKIN)
                && !events
                    .iter()
                    .any(|e| matches!(e, SessionEvent::ObjectDestroyed(g) if g == guid))
        })
        .map(|(guid, _)| descriptor(&events, *guid).item_stack_count().unwrap_or(1))
        .sum();
    assert_eq!(left, 1);
    // The copper is paid, and the two old Tokens in the bags stay two.
    assert_eq!(descriptor(&events, me).player_money(), Some(500));
    assert_eq!(carried(&events, me, TOKEN), 2);
    assert!(!events.iter().any(|e| matches!(
        e,
        SessionEvent::ItemPushResult(p) if p.item_entry == TOKEN
    )));
}

/// A test-only copy of Crocolisk Skins that hands over a quiver's worth of arrows.
const SKINS_FOR_ARROWS: u32 = 99301;
const SKINS_FOR_ARROWS_TOML: &str = r#"
[[quest]]
id = 99301
kind = "hunting"
giver = 90103
level = 10
title = "Skins for Arrows"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0
collect = [{ item = 2925, count = 8, chance = 60, from = [3110, 3231] }]
rewards = [[2512, 200]]
"#;

#[test]
fn a_hand_in_whose_rewards_do_not_fit_in_the_bags_is_refused() {
    let _ = Realm::with_seed(SEED).with_quests(SKINS_FOR_ARROWS_TOML);
    // Fifteen skins, one per slot, and the sixteenth slot full: the eight skins leave seven
    // slots full, but the bags are checked before anything is taken, as vmangos does.
    let mut items: Vec<Carried> = (23..38).map(|s| (s, CROCOLISK_SKIN, 1)).collect();
    items.push((38, 4560, 1));
    let (mut realm, me, mut events) = questing(15, &items, 0, &[(SKINS_FOR_ARROWS, [0; 4])]);
    let zunjin = giver(&events, ZUNJIN);
    walk_up_to(&mut realm, zunjin);
    let turned_in = hand_in(&mut realm, zunjin, SKINS_FOR_ARROWS);
    assert!(completed(&turned_in).is_none());
    assert!(turned_in
        .iter()
        .any(|e| matches!(e, SessionEvent::InventoryFailure { reason: 50, .. })));
    events.extend(turned_in);
    assert_eq!(
        carried(&events, me, CROCOLISK_SKIN),
        15,
        "nothing was taken"
    );
    assert_eq!(
        quest_log(&events, me).len(),
        1,
        "the quest is still under way"
    );
}

#[test]
fn outlevelling_a_quest_does_not_shrink_its_xp() {
    // A level 16 Vanguard on a level 10 quest: half of level 10's 7600, at the Tier XP Rate's ×3.
    let skins: &[Carried] = &[(23, CROCOLISK_SKIN, 8)];
    let (mut realm, _, events) =
        questing_as("vanguard", 16, skins, 0, &[(CROCOLISK_SKINS, [0; 4])]);
    let zunjin = giver(&events, ZUNJIN);
    walk_up_to(&mut realm, zunjin);
    let done = completed(&hand_in(&mut realm, zunjin, CROCOLISK_SKINS)).unwrap();
    assert_eq!(done.xp, 3 * 3800);
}

#[test]
fn the_xp_multiplier_multiplies_quest_xp() {
    let (me, blob) = save_with_quests(4, &[], 0, &[(THE_FAMILIAR_THREAT, [5, 0, 0, 0])]);
    let mut realm = Realm::with_seed(SEED).with_dev(twilight_realm::DevMultipliers {
        xp: 2.0,
        ..Default::default()
    });
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    let done = completed(&hand_in(
        &mut realm,
        giver(&events, GREZZ),
        THE_FAMILIAR_THREAT,
    ));
    assert_eq!(done.unwrap().xp, 2100);
}

#[test]
fn a_kill_no_quest_wants_changes_no_quest() {
    let (mut realm, me, mut events) = questing(5, &[], 0, &[(THINNING_THE_HERDS, [1, 2, 0, 0])]);
    let mob = the_mob(&events);
    let fight = kill(&mut realm, &mob);
    assert!(!fight
        .iter()
        .any(|e| matches!(e, SessionEvent::QuestObjectiveKill { .. })));
    events.extend(fight);
    assert_eq!(
        quest_log(&events, me),
        [(0, THINNING_THE_HERDS, [1, 2, 0, 0], 0)]
    );
}

/// The Scorpid Workers' camp east of the Crossroads (`crossroads_low_camps.json`'s low-3).
const SCORPID_CAMP: [f32; 3] = [-376.0, -2837.0, 0.0];
const SCORPID_WORKER: u32 = 3124;

/// Where `mob` is headed: the end of its last move, or where it spawned.
fn where_now(events: &[SessionEvent], mob: &Mob) -> [f32; 3] {
    events
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::MonsterMove { guid, path, .. } if *guid == mob.guid => {
                path.last().copied()
            }
            _ => None,
        })
        .unwrap_or(mob.position)
}

/// A level 8 warrior, with Scorpid Venom or without, kills the Scorpid Workers' camp and takes
/// every quest item their corpses offer. The loot windows' item rows, and every event.
fn clear_the_scorpids(with_quest: bool) -> (Vec<u32>, Vec<SessionEvent>, u64) {
    // Level 14: the four workers have twice the health and hit half again as hard, and a level 8
    // warrior no longer lives through the camp.
    let (mut realm, me, events) = warrior_with(14, 0, &[]);
    let mut all = events.clone();
    if with_quest {
        all.extend(accept(&mut realm, giver(&events, GREZZ), SCORPID_VENOM));
    }
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: SCORPID_CAMP,
        orientation: 0.0,
    });
    let near = run(&mut realm, 1.5);
    let scorpids: Vec<Mob> = near
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                position,
                fields,
                ..
            } if fields.object_entry() == Some(SCORPID_WORKER) => Some(Mob {
                guid: *guid,
                position: *position,
                fields: fields.clone(),
            }),
            _ => None,
        })
        .collect();
    assert_eq!(scorpids.len(), 4, "the camp's four workers");
    all.extend(near);

    let mut rows = Vec::new();
    for mob in &scorpids {
        // Fight each where it now stands: an earlier fight may have moved it.
        let mob = Mob {
            guid: mob.guid,
            position: where_now(&all, mob),
            fields: mob.fields.clone(),
        };
        all.extend(kill(&mut realm, &mob));
        realm.handle(RealmCommand::Loot { guid: mob.guid });
        let window = drain(&mut realm);
        let items = window
            .iter()
            .find_map(|e| match e {
                SessionEvent::LootResponse { items, .. } => Some(items.clone()),
                _ => None,
            })
            .unwrap_or_default();
        all.extend(window);
        for row in items {
            rows.push(row.item_id);
            if row.item_id == VENOM_SAC {
                realm.handle(RealmCommand::AutostoreLootItem { slot: row.slot });
            }
        }
        realm.handle(RealmCommand::LootRelease { guid: mob.guid });
        all.extend(drain(&mut realm));
    }
    (rows, all, me)
}

#[test]
fn quest_items_drop_only_while_a_quest_wants_them() {
    let (rows, _, _) = clear_the_scorpids(false);
    assert!(!rows.contains(&VENOM_SAC), "{rows:?}");

    let (rows, events, me) = clear_the_scorpids(true);
    let sacs = rows.iter().filter(|&&i| i == VENOM_SAC).count();
    assert!((1..=4).contains(&sacs), "{rows:?}");
    // Into the bags, where the client counts them; the quest wants six, so it is not done.
    let carried: u32 = item_creates(&events)
        .iter()
        .filter(|(_, f)| f.object_entry() == Some(VENOM_SAC))
        .map(|(guid, _)| descriptor(&events, *guid).item_stack_count().unwrap_or(1))
        .sum();
    assert_eq!(carried as usize, sacs);
    assert_eq!(quest_log(&events, me), [(0, SCORPID_VENOM, [0; 4], 0)]);
}

#[test]
fn the_last_quest_item_looted_completes_the_quest() {
    // Five sacs in the pack: the next one finishes Scorpid Venom.
    let sacs: &[Carried] = &[(23, VENOM_SAC, 5)];
    let (mut realm, me, mut events) = questing(8, sacs, 0, &[(SCORPID_VENOM, [0; 4])]);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: SCORPID_CAMP,
        orientation: 0.0,
    });
    let near = run(&mut realm, 1.5);
    let mut done = false;
    for (guid, position, fields) in near.iter().filter_map(|e| match e {
        SessionEvent::ObjectCreate {
            guid,
            position,
            fields,
            ..
        } if fields.object_entry() == Some(SCORPID_WORKER) => {
            Some((*guid, *position, fields.clone()))
        }
        _ => None,
    }) {
        let mob = Mob {
            guid,
            position: settled_at(&events, guid, settled_at(&near, guid, position)),
            fields,
        };
        events.extend(kill(&mut realm, &mob));
        realm.handle(RealmCommand::Loot { guid });
        let window = drain(&mut realm);
        let sac = window.iter().find_map(|e| match e {
            SessionEvent::LootResponse { items, .. } => items
                .iter()
                .find(|r| r.item_id == VENOM_SAC)
                .map(|r| r.slot),
            _ => None,
        });
        if let Some(slot) = sac {
            realm.handle(RealmCommand::AutostoreLootItem { slot });
            let took = drain(&mut realm);
            assert!(took.iter().any(|e| matches!(
                e,
                SessionEvent::QuestObjectivesComplete { quest_id } if *quest_id == SCORPID_VENOM
            )));
            events.extend(took);
            done = true;
            break;
        }
    }
    assert!(done, "one of the four workers drops a sac");
    assert_eq!(quest_log(&events, me), [(0, SCORPID_VENOM, [0; 4], 1)]);
}

#[test]
fn a_giver_offers_a_quest_by_tier_not_level() {
    // Zun'jin's Crocolisk Skins is level 10: a Tier 1 level 1 character is offered it, a Tier 0
    // level 9 one is not.
    let (mut realm, _, events) = questing_as("warrior", 1, &[], 0, &[]);
    let zunjin = giver(&events, ZUNJIN);
    assert!(on_offer(&mut realm, zunjin).contains(&CROCOLISK_SKINS));
    let (mut realm, _, events) = questing(9, &[], 0, &[]);
    let zunjin = giver(&events, ZUNJIN);
    assert!(!on_offer(&mut realm, zunjin).contains(&CROCOLISK_SKINS));
}

#[test]
fn a_level_25_character_handing_in_a_level_10_quest_earns_half_its_levels_xp() {
    // Half of level 10's 7600 XP at the Tier XP Rate's x3, with no falloff.
    let skins: &[Carried] = &[(23, CROCOLISK_SKIN, 8)];
    let (mut realm, _, events) = questing_as("warrior", 25, skins, 0, &[(CROCOLISK_SKINS, [0; 4])]);
    let zunjin = giver(&events, ZUNJIN);
    let done = completed(&hand_in(&mut realm, zunjin, CROCOLISK_SKINS)).unwrap();
    assert_eq!(done.xp, 3 * 3800);
}
