//! The Bloodmender (#693), the Acolyte Line's Tier 2 main healer: the Calling Choice offers it at
//! the Acolyte's cap beside the Blightcaller, Sister Elowen Marr waits unmarked on the steps of
//! Cathedral Square until she is met, Brother Corwin and Brother Kristoff point the way, her
//! three-quest Trial gates the Advancement, and the sheet gives the Calling its ten Talent Rows.
//! It heals with its own blood: Transfusion pays in health for a friend's heal over time, Blood
//! Ward shields the friends around it, and Blood Feast drinks the foe's to fill up again.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::messages::{GossipOption, PeriodicTick};
use benilla_protocol::{EntityKind, SessionEvent, START_ELWYNN};
use common::acolyte::{dummy, YardGround};
use common::fight::{cast, refusal, View};
use common::party::Party;
use common::*;
use gear_common::SEED;
use quest_common::*;
use twilight_realm::{Realm, RealmCommand};

/// Sister Elowen Marr and Brother Corwin Ashgrave (the Northshire Acolyte Trainer), both Hub
/// NPCs, and Brother Kristoff (a friendly Stormwind spawn).
const MARR: u32 = 190119;
const CORWIN: u32 = 190054;
const KRISTOFF: u32 = 1444;
/// Brother Kristoff's spawn point (`content/stormwind.toml`).
const KRISTOFF_AT: [f32; 2] = [-8623.24, 766.655];
/// The three wounded of Darkshire and their spawn points (`content/duskwood.toml`).
const LADIMORE: u32 = 576;
const LADIMORE_AT: [f32; 2] = [-10581.6, -1172.89];
const DALTRY: u32 = 267;
const DALTRY_AT: [f32; 2] = [-10560.3, -1125.19];
const ELAINE: u32 = 633;
const ELAINE_AT: [f32; 2] = [-10594.1, -1200.87];
/// Stormwind City's zone.
const STORMWIND: u32 = 1519;

/// The Blightcaller's Advancement Quest, offered beside this one.
const GARDEN: u32 = 64407;
/// The Advancement Quest and its Trial, in order.
const GIFT: u32 = 64408;
const WATCH: u32 = 64456;
const BITTEN: u32 = 64457;
const CURSING: u32 = 64458;

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

/// An Acolyte of 30 who picked at the Tier 0 Calling Choice, entered in the world.
fn capped_acolyte() -> Vec<SessionEvent> {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Mender", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    assert_eq!(create_result(&mut maker), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("calling = \"basic_warrior\"", "calling = \"acolyte\"")
        .replacen("\nlevel = 1\n", "\nlevel = 30\nchosen_tiers = [0]\n", 1);
    let mut realm = Realm::with_seed(SEED);
    realm
        .import_character(&blob)
        .unwrap_or_else(|e| panic!("{e:?}: {blob}"));
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    drain(&mut realm)
}

/// An Acolyte of 30 standing beside Sister Marr on the square, with `quests` under way and `done`
/// handed in, entered in the world.
fn on_the_square(quests: &[Underway], done: &[u32]) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(30, &[], 0, quests);
    let [x, y, z] = hub_home(hub_guid(MARR));
    let (head, tail) = blob.split_once("\n[").unwrap_or((&blob, ""));
    let head: Vec<String> = head
        .lines()
        .map(|line| {
            if line.starts_with("map = ") {
                "map = 0".to_string()
            } else if line.starts_with("zone = ") {
                format!("zone = {STORMWIND}")
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
        .replace("calling = \"basic_warrior\"", "calling = \"acolyte\"");
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

/// The reply to the line `line` of `npc`'s menu, standing beside it already.
fn say(realm: &mut Realm, npc: u64, line: &str) -> String {
    let options = menu(realm, npc);
    let option = options
        .iter()
        .find(|o| o.message == line)
        .unwrap_or_else(|| panic!("the line {line:?} in {options:?}"));
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

/// The guid of the unit of `entry` created in `events`, if one was.
fn created(events: &[SessionEvent], entry: u32) -> Option<u64> {
    events.iter().find_map(|e| match e {
        SessionEvent::ObjectCreate {
            guid,
            kind: EntityKind::Unit,
            fields,
            ..
        } if fields.object_entry() == Some(entry) => Some(*guid),
        _ => None,
    })
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
fn a_level_30_acolyte_is_offered_the_blightcaller_and_the_bloodmender() {
    let offers = offered(&capped_acolyte());
    assert_eq!(offers.len(), 1, "{offers:?}");
    assert!(offers[0].contains(&GARDEN), "{offers:?}");
    assert!(offers[0].contains(&GIFT), "{offers:?}");
}

#[test]
fn sister_marr_is_unmarked_until_met_and_corwin_and_kristoff_give_the_hints() {
    let (mut realm, _, events) = on_the_square(&[(GIFT, [0; 4])], &[]);
    let marr = hub_guid(MARR);
    assert!(!marked(&events, marr), "not met yet");
    walk_up_to(&mut realm, marr);
    menu(&mut realm, marr);
    realm.tick(0.0, &FlatGround);
    assert!(marked(&drain(&mut realm), marr), "met at her gossip menu");

    // Brother Kristoff stands 10 yd from her, so he came in with the square.
    let kristoff = created(&events, KRISTOFF).expect("Kristoff on the square");
    walk_to(
        &mut realm,
        [KRISTOFF_AT[0] + 1.5, KRISTOFF_AT[1], hub_home(marr)[2]],
    );
    assert_eq!(
        say(&mut realm, kristoff, "Where is Sister Marr?"),
        "On the steps by the orphans, where she always is. She bleeds for anyone who asks. Be \
         kind to her."
    );

    let corwin = hub_guid(CORWIN);
    walk_up_to(&mut realm, corwin);
    assert_eq!(
        say(&mut realm, corwin, "Who heals with her own blood?"),
        "Sister Elowen Marr. Brother Kristoff brings her bread in Cathedral Square. He'll know \
         where she sits today."
    );
}

#[test]
fn the_wounded_watch_is_met_by_tending_the_three_of_darkshire() {
    let (mut realm, _, mut seen) = on_the_square(&[(GIFT, [0; 4]), (WATCH, [0; 4])], &[]);
    for (entry, at, line, reply) in [
        (
            LADIMORE,
            LADIMORE_AT,
            "Let me see that wound.",
            "It's nothing. It's been nothing for a week. ...Thank you.",
        ),
        (
            DALTRY,
            DALTRY_AT,
            "You're bleeding through your sleeve.",
            "Am I? I hadn't looked up from the ledger in days. Thank you.",
        ),
        (
            ELAINE,
            ELAINE_AT,
            "Hold still. This will sting.",
            "It does. It's better. Who sent you?",
        ),
    ] {
        // The three stand within sight of each other, so one may have come in with the last.
        seen.extend(walk_to(&mut realm, [at[0] + 1.5, at[1], 0.0]));
        let npc = created(&seen, entry).unwrap_or_else(|| panic!("{entry} streams in"));
        assert_eq!(say(&mut realm, npc, line), reply);
    }
    let marr = hub_guid(MARR);
    assert!(completed(&hand_in(&mut realm, marr, WATCH)).is_some());
    assert_eq!(on_offer(&mut realm, marr), [BITTEN]);
}

#[test]
fn sister_marr_sets_the_trial_in_order_and_its_end_makes_a_level_1_bloodmender() {
    let marr = hub_guid(MARR);
    let (mut realm, _, _) = on_the_square(&[], &[]);
    assert!(
        !on_offer(&mut realm, marr).contains(&WATCH),
        "64408 not taken"
    );

    let (mut realm, _, _) = on_the_square(&[(GIFT, [0; 4])], &[]);
    assert_eq!(on_offer(&mut realm, marr), [WATCH]);
    assert!(completed(&hand_in(&mut realm, marr, GIFT)).is_none());

    let (mut realm, _, _) = on_the_square(&[(GIFT, [0; 4]), (BITTEN, [8, 0, 0, 0])], &[WATCH]);
    assert!(completed(&hand_in(&mut realm, marr, BITTEN)).is_some());
    assert_eq!(on_offer(&mut realm, marr), [CURSING]);
    assert!(completed(&hand_in(&mut realm, marr, GIFT)).is_none());

    let (mut realm, me, entered) =
        on_the_square(&[(GIFT, [0; 4]), (CURSING, [6, 0, 0, 0])], &[WATCH, BITTEN]);
    assert!(completed(&hand_in(&mut realm, marr, GIFT)).is_none());
    assert_eq!(realm.current_calling().unwrap().key, "acolyte");
    assert!(completed(&hand_in(&mut realm, marr, CURSING)).is_some());
    let advanced = hand_in(&mut realm, marr, GIFT);
    assert!(completed(&advanced).is_some(), "{advanced:?}");
    assert_eq!(realm.current_calling().unwrap().key, "bloodmender");
    let saved = realm.export_character(me).unwrap();
    assert!(saved.contains("\nlevel = 1\n"), "{saved}");

    let mut events = entered;
    events.extend(advanced);
    let known = spellbook(&events);
    for key in [
        "blood_lance",
        "blood_feast",
        "dark_mending",
        "transfusion",
        "hush",
    ] {
        assert!(
            known.contains(&spell("bloodmender", key)),
            "{key}: {known:?}"
        );
    }
    for key in ["blood_ward", "circle_of_blood", "last_rites"] {
        assert!(!known.contains(&spell("bloodmender", key)), "{key} at 1");
    }

    // At level 10 the login's spell book has Blood Ward.
    let blob = saved.replacen("\nlevel = 1\n", "\nlevel = 10\n", 1);
    let mut later = Realm::with_seed(SEED);
    later.import_character(&blob).unwrap();
    later.handle(RealmCommand::EnterWorld { guid: me });
    later.tick(0.0, &FlatGround);
    let known = spellbook(&drain(&mut later));
    assert!(
        known.contains(&spell("bloodmender", "blood_ward")),
        "{known:?}"
    );
    assert!(!known.contains(&spell("bloodmender", "circle_of_blood")));
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

/// Blood Lance's hit for `calling` at level `l`, read from its skill row's formula.
fn lance_hit(calling: &str, l: f64) -> f64 {
    let c = sheet(calling);
    let lance = c["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["key"] == "blood_lance")
        .expect("a Blood Lance");
    let amount = &lance["effects"][0]["params"]["amount"];
    amount["base"].as_f64().unwrap() + amount["per_level"].as_f64().unwrap() * l
}

#[test]
fn the_sheet_gives_ten_rows_and_a_lance_that_starts_stronger() {
    let c = sheet("bloodmender");
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
    assert!(lance_hit("bloodmender", 1.0) > lance_hit("acolyte", 30.0));
}

/// Where the Bloodmender (player 0) and the friend (player 1) stand in the Test Grounds, 25 yd
/// west of dummy 1 and 2 yd apart.
const SPOTS: [[f32; 2]; 2] = [[-9665.0, 258.0], [-9665.0, 260.0]];

/// A level 20 Bloodmender and a level 20 friend in the Test Grounds, each in their own session.
fn mender_and_friend() -> Party {
    mender_and_friend_at(20)
}

/// [`mender_and_friend`], both of `level`.
fn mender_and_friend_at(level: u8) -> Party {
    let realm = Realm::with_seed(0x693);
    let mut party = Party::edited(
        realm,
        &[level, level],
        START_ELWYNN,
        Box::new(YardGround),
        |i, blob| {
            if i == 0 {
                blob.replace("calling = \"basic_warrior\"", "calling = \"bloodmender\"")
            } else {
                blob
            }
        },
    );
    for (i, [x, y]) in SPOTS.into_iter().enumerate() {
        for (xy, facing) in [([-9720.0, 255.0], None), ([x, y], Some(0.0))] {
            party.cmd(
                i,
                RealmCommand::DevTeleport {
                    x: xy[0],
                    y: xy[1],
                    facing,
                },
            );
            let landed = party.run(1.0).swap_remove(i);
            // The client acknowledges the teleport, or the realm ignores its steps.
            if let Some(counter) = landed.iter().find_map(|e| match e {
                SessionEvent::Teleport { counter, .. } => Some(*counter),
                _ => None,
            }) {
                party.cmd(i, RealmCommand::TeleportAck { counter });
            }
        }
    }
    party.run(0.5);
    party
}

/// Brings player `i` down to 100 health.
fn hurt(party: &mut Party, i: usize) {
    let [x, y] = SPOTS[i];
    party.go(i, [x, y, 47.2], 0.0);
    party.realm.set_player_health(100);
    party.run(0.2);
}

/// What player `i`'s client knows.
fn view(party: &Party, i: usize) -> View {
    let mut v = View::default();
    v.see(party.seen(i));
    v
}

/// Player 0 casts `spell_id` at `target`: refused or not.
fn cast_at(party: &mut Party, spell_id: u32, target: Option<u64>) {
    let before = party.seen(0).len();
    party.cmd(0, cast(spell_id, target));
    assert_eq!(
        refusal(&party.seen(0)[before..], spell_id),
        None,
        "spell {spell_id} goes off"
    );
}

/// Transfusion on the hurt friend costs the Bloodmender 15% of their maximum health, and the
/// friend heals 150% of it over 6 s, a tick a second, in their own session.
#[test]
fn transfusion_pays_in_health_for_a_heal_over_time_on_a_hurt_friend() {
    let (cost, heals) = transfused(mender_and_friend());
    assert_eq!(heals.len(), 6, "a tick a second: {heals:?}");
    let per_tick = (f64::from(cost) * 1.5).round() as u32 / 6;
    assert!(heals.iter().all(|&h| h == per_tick), "{heals:?} of {cost}");
}

/// The Talent `key` of the Bloodmender's row `row`: its spell id.
fn talent(row: &str, key: &str) -> u32 {
    sheet("bloodmender")["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["key"] == row)
        .and_then(|r| {
            r["choices"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["key"] == key)
        })
        .and_then(|c| c["spell_id"].as_u64())
        .unwrap_or_else(|| panic!("{row}/{key}")) as u32
}

/// With Crimson Gift on the level 50 row, Transfusion's ticks are 15% larger: the realm raises a
/// heal over time paid in health by the skill's amount Talents.
#[test]
fn crimson_gift_raises_transfusions_heal_by_15_percent() {
    let mut party = mender_and_friend_at(50);
    let gift = talent("bloodmender_hearts_blood", "crimson_gift");
    party.cmd(
        0,
        RealmCommand::SetTalentRow {
            row: "bloodmender_hearts_blood".into(),
            spell_id: gift,
        },
    );
    let (cost, heals) = transfused(party);
    assert_eq!(heals.len(), 6, "a tick a second: {heals:?}");
    let plain = (f64::from(cost) * 1.5).round() as u32 / 6;
    let more = (f64::from(plain) * 1.15).round() as u32;
    assert!(heals.iter().all(|&h| h == more), "{heals:?} of {plain}");
}

/// The Bloodmender casts Transfusion on the friend at 100 health: what it cost them in health, and
/// the friend's heal ticks over the next 6.5 s.
fn transfused(mut party: Party) -> (u32, Vec<u32>) {
    let transfusion = spell("bloodmender", "transfusion");
    hurt(&mut party, 1);
    let (me, friend) = (party.guids[0], party.guids[1]);
    let max = view(&party, 0).fields(me).unit_max_health().unwrap();
    let before = view(&party, 0).health(me);
    assert_eq!(before, max, "full health");
    assert_eq!(view(&party, 1).health(friend), 100);

    cast_at(&mut party, transfusion, Some(friend));
    let cost = max * 15 / 100;
    assert_eq!(view(&party, 0).health(me), max - cost, "15% of max health");

    let got = party.run(6.5);
    let heals: Vec<u32> = got[1]
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == transfusion && l.target == friend => {
                match l.ticks[0] {
                    PeriodicTick::Heal { amount } => Some(amount),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect();
    (cost, heals)
}

/// Blood Feast on a dummy drains it a tick a second for 3 s and heals the hurt Bloodmender twice
/// what each tick drained.
#[test]
fn blood_feast_refills_the_bloodmender_twice_what_it_drains() {
    let feast = spell("bloodmender", "blood_feast");
    let mut party = mender_and_friend();
    hurt(&mut party, 0);
    let me = party.guids[0];
    assert_eq!(view(&party, 0).health(me), 100);
    let before = party.seen(0).len();
    cast_at(&mut party, feast, Some(dummy(1)));
    party.run(3.5);
    let (mut dealt, mut healed) = (Vec::new(), Vec::new());
    for e in &party.seen(0)[before..] {
        match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == feast && l.target == dummy(1) => {
                dealt.push(l.damage)
            }
            SessionEvent::SpellHealLog(l) if l.spell_id == feast && l.target == me => {
                healed.push(l.amount)
            }
            _ => {}
        }
    }
    let landed: Vec<u32> = dealt.iter().copied().filter(|&d| d > 0).collect();
    assert!((1..=3).contains(&landed.len()), "{dealt:?}");
    assert_eq!(
        healed,
        landed.iter().map(|d| d * 2).collect::<Vec<_>>(),
        "twice each tick"
    );
    assert!(view(&party, 0).health(me) >= 100 + healed.iter().sum::<u32>());
}

/// Blood Ward costs the Bloodmender 12% of their maximum health, and the friend beside them
/// wears a shield of 60% of it, in their own session.
#[test]
fn blood_ward_shields_the_friend_beside_the_bloodmender() {
    let ward = spell("bloodmender", "blood_ward");
    let mut party = mender_and_friend();
    let (me, friend) = (party.guids[0], party.guids[1]);
    let max = view(&party, 0).fields(me).unit_max_health().unwrap();
    cast_at(&mut party, ward, None);
    assert_eq!(view(&party, 0).health(me), max - max * 12 / 100);
    party.run(0.3);
    assert!(
        view(&party, 0).aura(me, ward).is_some(),
        "the caster's shield"
    );
    assert!(
        view(&party, 1).aura(friend, ward).is_some(),
        "the friend's shield: {:?}",
        party.seen(1)
    );
}
