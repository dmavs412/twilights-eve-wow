//! The Nightblade (#688), the Cutthroat Line's second Tier 2 Calling: the Calling Choice offers it
//! beside the Bloodletter at the Cutthroat's cap, Jorik Vane, Innkeeper Farley and Warden Thelwater
//! point the way, Silas Dunmere waits in the Riot's round room, his two-quest Trial gates the
//! Advancement, and the sheet gives the Calling its ten Talent Rows. Ambush needs Stealth and the
//! target's back, Vanish ends a fight in Stealth, Shadow Speed's run speed goes with the Stealth it
//! came with, and Shadow Dance's spellmod shortens Vanish's cooldown in the realm too.

mod common;

use benilla_protocol::messages::{dialog_status, GossipOption};
use benilla_protocol::{EntityKind, SessionEvent, START_ELWYNN};
use common::cutthroat::{cutthroat_save, set_rows};
use common::fight::{cast, flat, pull, refusal, View, CAMP};
use common::party::*;
use common::vanguard::enter;
use common::*;
use twilight_realm::{Realm, RealmCommand};

/// Silas Dunmere's template and spawn point (`stockade_riot.toml`).
const SILAS: u32 = 97290;
const SILAS_AT: [f32; 3] = [83.5, -147.0, -33.9396];
/// Beside Silas, facing him.
const BY_SILAS: [f32; 3] = [85.0, -147.0, -33.9396];
/// Jorik Vane (the Northshire Cutthroat Trainer), Innkeeper Farley and Warden Thelwater.
const JORIK: u32 = 190051;
const FARLEY: u32 = 295;
const THELWATER: u32 = 1719;

/// The jail's door, the Stockade's entrance trigger, the crossing at the foot of the corridor
/// and the head of the east stairs (`tests/stockade.rs`).
const DOOR: [f32; 3] = [-8761.85, 848.557, 87.805];
const ENTRANCE: u32 = 101;
const CROSSING: [f32; 3] = [131.0, 3.0, -25.5229];
const EAST_STAIRS: [f32; 3] = [131.0, -12.0, -27.0];

/// The Advancement Quest and its Trial, in order, and the Bloodletter's beside it.
const UNLOCKED_CELL: u32 = 64402;
const OUT_THE_WAY: u32 = 64426;
const RINGLEADERS: u32 = 64427;
const BLOODLETTERS_SALT: u32 = 64401;

/// `SPELL_FAILED_NOT_BEHIND`.
const NOT_BEHIND: u8 = 0x33;
/// `UNIT_FLAG_IN_COMBAT`.
const UNIT_FLAG_IN_COMBAT: u32 = 0x0008_0000;

/// What the client has been sent since its `from`th event.
fn since(party: &Party, from: usize) -> Vec<SessionEvent> {
    party.seen(0)[from..].to_vec()
}

/// A level-30 Cutthroat at the jail's door, having handed in `done` and with `quests` under way
/// (each quest's kills by objective).
fn cutthroat_party(done: &[u32], quests: &[(u32, [u8; 4])]) -> Party {
    let done = done.to_vec();
    let quests = quests.to_vec();
    let mut party = Party::edited(
        Realm::with_seed(1),
        &[30],
        START_ELWYNN,
        Box::new(NoGround),
        move |_, blob| {
            let mut blob = blob.replace("calling = \"basic_warrior\"", "calling = \"cutthroat\"");
            if !done.is_empty() {
                blob = blob.replace("xp = 0\n", &format!("xp = 0\ncompleted = {done:?}\n"));
            }
            for (slot, (id, kills)) in quests.iter().enumerate() {
                blob.push_str(&format!(
                    "\n[[quests]]\nid = {id}\nslot = {slot}\nkills = {kills:?}\n"
                ));
            }
            blob
        },
    );
    party.go(0, DOOR, 0.66);
    party.run(0.5);
    party
}

/// Walks into the jail, down the east stairs into a Run of the Riot, and up to Silas: his guid.
fn to_silas(party: &mut Party) -> u64 {
    party.go(0, DOOR, 0.66);
    party.cmd(0, RealmCommand::AreaTrigger { id: ENTRANCE });
    party.cmd(0, RealmCommand::WorldportAck);
    party.run(0.5);
    party.go(0, CROSSING, 0.0);
    party.run(0.3);
    party.go(0, EAST_STAIRS, 0.0);
    party.run(0.3);
    assert!(
        party.realm.run_elapsed_as(party.sessions[0]).is_some(),
        "in the Riot's Run"
    );
    party.go_facing(0, BY_SILAS, SILAS_AT);
    party.run(0.3);
    party.unit(0, SILAS, SILAS_AT).0
}

/// The quests `npc` offers to take, as talking to it shows them.
fn on_offer(party: &mut Party, npc: u64) -> Vec<u32> {
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::QuestGiverHello { npc });
    since(party, from)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestGreeting(list) => Some(
                list.quests
                    .iter()
                    .filter(|q| q.icon == dialog_status::AVAILABLE)
                    .map(|q| q.quest_id)
                    .collect(),
            ),
            SessionEvent::QuestDetail(d) => Some(vec![d.quest_id]),
            _ => None,
        })
        .unwrap_or_default()
}

/// Hands `quest` in at `npc` the stock way; whether it completed.
fn hand_in(party: &mut Party, npc: u64, quest: u32) -> bool {
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::QuestGiverCompleteQuest { npc, quest });
    party.cmd(0, RealmCommand::QuestGiverRequestReward { npc, quest });
    party.cmd(
        0,
        RealmCommand::QuestGiverChooseReward {
            npc,
            quest,
            choice: 0,
        },
    );
    party.run(0.2);
    since(party, from)
        .iter()
        .any(|e| matches!(e, SessionEvent::QuestComplete(c) if c.quest_id == quest))
}

/// The reply to the hint line `line` of `npc`'s gossip menu, standing beside it already.
fn hint(party: &mut Party, npc: u64, line: &str) -> String {
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::GossipHello { guid: npc });
    let options: Vec<GossipOption> = since(party, from)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { options, .. } => Some(options),
            _ => None,
        })
        .expect("a gossip menu");
    let option = options
        .iter()
        .find(|o| o.message == line)
        .unwrap_or_else(|| panic!("the hint line {line:?} in {options:?}"));
    let from = party.seen(0).len();
    party.cmd(
        0,
        RealmCommand::GossipSelectOption {
            guid: npc,
            option: option.index,
        },
    );
    let text_id = since(party, from)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { text_id, .. } => Some(text_id),
            _ => None,
        })
        .expect("the reply");
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::NpcTextQuery { text_id });
    since(party, from)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::NpcGreeting { blocks, .. } => Some(blocks[0].male.clone()),
            _ => None,
        })
        .expect("the reply's text")
}

/// Stands beside the Hub NPC `npc` on the open world.
fn beside(party: &mut Party, npc: u64) {
    let [x, y, z] = hub_home(npc);
    party.go(0, [x + 1.5, y, z], std::f32::consts::PI);
    party.run(0.3);
}

/// The spells a client knows after `events`.
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

/// What a fresh login of `blob` (character `guid`) knows.
fn login_spellbook(guid: u64, blob: &str) -> std::collections::BTreeSet<u32> {
    let mut realm = Realm::with_seed(1);
    realm.import_character(blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    spellbook(&drain(&mut realm))
}

#[test]
fn a_level_30_cutthroat_is_offered_the_bloodletter_and_the_nightblade() {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Shade", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    assert_eq!(create_result(&mut maker), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("calling = \"basic_warrior\"", "calling = \"cutthroat\"")
        .replacen("\nlevel = 1\n", "\nlevel = 30\nchosen_tiers = [0]\n", 1);
    let mut realm = Realm::with_seed(1);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    let offers: Vec<Vec<u32>> = drain(&mut realm)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::CallingChoice { options } => {
                Some(options.iter().map(|o| o.quest).collect())
            }
            _ => None,
        })
        .collect();
    assert_eq!(offers.len(), 1, "{offers:?}");
    assert!(offers[0].contains(&BLOODLETTERS_SALT), "{offers:?}");
    assert!(offers[0].contains(&UNLOCKED_CELL), "{offers:?}");
}

#[test]
fn jorik_farley_and_thelwater_give_the_hints_once_64402_is_taken() {
    let mut party = cutthroat_party(&[], &[(UNLOCKED_CELL, [0; 4])]);
    let jorik = hub_guid(JORIK);
    beside(&mut party, jorik);
    assert_eq!(
        hint(&mut party, jorik, "Who teaches what comes after the knife?"),
        "Not me. Ask Farley at the Lion's Pride in Goldshire who pours for the Watch on their \
         nights off."
    );
    let farley = hub_guid(FARLEY);
    beside(&mut party, farley);
    assert_eq!(
        hint(&mut party, farley, "Who drinks with the Stockade's Watch?"),
        "The warden himself, Thelwater, when he can get away. He says one cell in the riot wing \
         never locked. He says it like it's a joke."
    );

    party.go(0, DOOR, 0.66);
    party.cmd(0, RealmCommand::AreaTrigger { id: ENTRANCE });
    party.cmd(0, RealmCommand::WorldportAck);
    party.run(0.5);
    party.go(0, CROSSING, 0.0);
    party.run(0.3);
    assert_eq!(
        hint(&mut party, hub_guid(THELWATER), "Which cell never locked?"),
        "The last one. Past the riot, past Rook, in the round room at the end of the east wing. \
         Whoever sits in it never asked to be let out, and nobody I send in comes back with a \
         description."
    );
}

#[test]
fn silas_is_a_friendly_creature_of_the_round_room_who_sets_the_trial_once_64402_is_taken() {
    let template = mob_template(SILAS);
    assert_eq!(template["name"].as_str(), Some("Silas Dunmere"));
    assert_eq!(template["friendly"].as_bool(), Some(true));
    assert_eq!(template["hostile"].as_bool(), Some(false));

    let mut party = cutthroat_party(&[], &[]);
    let silas = to_silas(&mut party);
    assert!(
        !on_offer(&mut party, silas).contains(&OUT_THE_WAY),
        "64402 not taken"
    );

    let mut party = cutthroat_party(&[], &[(UNLOCKED_CELL, [0; 4])]);
    let silas = to_silas(&mut party);
    assert_eq!(on_offer(&mut party, silas), [OUT_THE_WAY]);
    assert!(
        !hand_in(&mut party, silas, UNLOCKED_CELL),
        "the Trial first"
    );
}

#[test]
fn the_trial_gates_the_advancement_and_its_end_makes_a_level_1_nightblade() {
    let mut party = cutthroat_party(&[], &[(UNLOCKED_CELL, [0; 4]), (OUT_THE_WAY, [6, 0, 0, 0])]);
    let silas = to_silas(&mut party);
    assert!(hand_in(&mut party, silas, OUT_THE_WAY));
    assert_eq!(on_offer(&mut party, silas), [RINGLEADERS]);
    assert!(
        !hand_in(&mut party, silas, UNLOCKED_CELL),
        "64427 still to do"
    );

    let mut party = cutthroat_party(
        &[OUT_THE_WAY],
        &[(UNLOCKED_CELL, [0; 4]), (RINGLEADERS, [3, 0, 0, 0])],
    );
    let silas = to_silas(&mut party);
    assert!(hand_in(&mut party, silas, RINGLEADERS));
    assert!(hand_in(&mut party, silas, UNLOCKED_CELL), "advanced");
    let me = party.guids[0];
    let saved = party.realm.export_character(me).unwrap();
    assert!(saved.contains("calling = \"nightblade\""), "{saved}");
    assert!(saved.contains("\nlevel = 1\n"), "{saved}");

    let known = login_spellbook(me, &saved);
    for key in ["eviscerate", "backstab", "stealth", "ambush", "kick"] {
        assert!(
            known.contains(&spell("nightblade", key)),
            "{key}: {known:?}"
        );
    }
    for key in ["vanish", "cold_blood", "kidney_shot"] {
        assert!(!known.contains(&spell("nightblade", key)), "{key} at 1");
    }
    let known = login_spellbook(me, &saved.replacen("\nlevel = 1\n", "\nlevel = 10\n", 1));
    assert!(known.contains(&spell("nightblade", "vanish")), "{known:?}");
    assert!(!known.contains(&spell("nightblade", "cold_blood")));
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

#[test]
fn the_sheet_gives_ten_rows_and_ambush_and_backstab_need_the_target_s_back() {
    let c = sheet("nightblade");
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
    let skills = c["skills"].as_array().unwrap();
    let skill = |key: &str| skills.iter().find(|s| s["key"] == key).unwrap().clone();
    assert_eq!(skill("ambush")["requires_stealth"], true);
    assert_eq!(skill("ambush")["requires_behind"], true);
    assert_eq!(skill("backstab")["requires_behind"], true);
    assert_ne!(skill("eviscerate")["requires_behind"], true);
}

// ── In a fight ─────────────────────────────────────────────────────────────────────────────────

/// A camp mob: its guid, where it stands and which way it faces.
struct Camped {
    mob: common::fight::Mob,
    facing: f32,
}

/// A Nightblade of `level`, fists only, in the world on `seed`'s dice, east of the scorpid camp
/// with each `(row, choice)` set: the realm, the view, its guid and the camp's nearest mob.
fn nightblade_at_camp(seed: u64, level: u8, rows: &[(&str, u32)]) -> (Realm, View, u64, Camped) {
    let (me, blob) = cutthroat_save(level, true);
    let blob = blob.replace("calling = \"cutthroat\"", "calling = \"nightblade\"");
    let (mut realm, mut view, _) = enter(me, &blob, seed);
    set_rows(&mut realm, &mut view, rows);
    view.run(&mut realm, 1.0);
    let stand = [CAMP[0] + 40.0, CAMP[1], 0.0];
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: stand,
        orientation: std::f32::consts::PI,
    });
    let camped = view
        .run(&mut realm, 1.0)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                orientation,
                ..
            } if flat(*position, CAMP) < 15.0 => Some(Camped {
                mob: common::fight::Mob {
                    guid: *guid,
                    position: *position,
                },
                facing: *orientation,
            }),
            _ => None,
        })
        .min_by(|a, b| flat(a.mob.position, stand).total_cmp(&flat(b.mob.position, stand)))
        .expect("a camp mob streams in");
    (realm, view, me, camped)
}

/// Puts the player 2 yd from `c`'s mob, `ahead` of it or behind it, facing it.
fn stand(realm: &mut Realm, c: &Camped, ahead: bool) {
    let sign = if ahead { 1.0 } else { -1.0 };
    let [x, y, z] = c.mob.position;
    let at = [
        x + sign * 2.0 * c.facing.cos(),
        y + sign * 2.0 * c.facing.sin(),
        z,
    ];
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: at,
        orientation: (y - at[1]).atan2(x - at[0]),
    });
}

/// Casts `spell_id` at `target`: what the realm sent.
fn try_cast(
    realm: &mut Realm,
    view: &mut View,
    spell_id: u32,
    target: Option<u64>,
) -> Vec<SessionEvent> {
    realm.handle(cast(spell_id, target));
    view.drain(realm)
}

#[test]
fn ambush_is_refused_out_of_stealth_and_from_the_front_and_goes_from_behind_in_stealth() {
    let ambush = spell("nightblade", "ambush");
    let stealth = spell("nightblade", "stealth");
    let (mut realm, mut view, me, camped) = nightblade_at_camp(3, 10, &[]);

    stand(&mut realm, &camped, false);
    let events = try_cast(&mut realm, &mut view, ambush, Some(camped.mob.guid));
    assert!(refusal(&events, ambush).is_some(), "out of Stealth");

    let events = try_cast(&mut realm, &mut view, stealth, None);
    assert_eq!(refusal(&events, stealth), None, "Stealth goes up");
    assert!(view.aura(me, stealth).is_some(), "stealthed");

    stand(&mut realm, &camped, true);
    let events = try_cast(&mut realm, &mut view, ambush, Some(camped.mob.guid));
    assert_eq!(refusal(&events, ambush), Some(NOT_BEHIND), "from the front");

    stand(&mut realm, &camped, false);
    let events = try_cast(&mut realm, &mut view, ambush, Some(camped.mob.guid));
    assert_eq!(refusal(&events, ambush), None, "from behind, in Stealth");
}

#[test]
fn vanish_in_a_fight_leaves_the_nightblade_in_stealth_with_no_mob_attacking() {
    let vanish = spell("nightblade", "vanish");
    let stealth = spell("nightblade", "aura_stealth");
    let (mut realm, mut view, me, camped) = nightblade_at_camp(2, 10, &[]);
    pull(&mut realm, &mut view, &camped.mob);
    let fight = view.run(&mut realm, 4.0);
    assert!(
        fight.iter().any(|e| matches!(e,
            SessionEvent::AttackerState(a) if a.attacker == camped.mob.guid && a.victim == me)),
        "the mob swings at the Nightblade"
    );
    assert_ne!(
        view.fields(me).unit_flags() & UNIT_FLAG_IN_COMBAT,
        0,
        "in combat"
    );

    let mut events = try_cast(&mut realm, &mut view, vanish, None);
    assert_eq!(refusal(&events, vanish), None, "Vanish goes");
    events.extend(view.run(&mut realm, 3.0));
    assert!(
        !events.iter().any(|e| matches!(e,
            SessionEvent::AttackerState(a) if a.attacker == camped.mob.guid && a.victim == me)),
        "the mob swings no more"
    );
    assert_eq!(
        view.fields(me).unit_flags() & UNIT_FLAG_IN_COMBAT,
        0,
        "out of combat"
    );
    assert!(view.aura(me, stealth).is_some(), "stealthed");
}

#[test]
fn shadow_speed_s_run_speed_goes_when_its_stealth_breaks() {
    let speedy = choice("nightblade", "stealth", "shadow_speed");
    let aura = spell("nightblade", "aura_shadow_speed");
    let ambush = spell("nightblade", "ambush");
    let (mut realm, mut view, me, camped) = nightblade_at_camp(4, 20, &[("stealth", speedy)]);

    let events = try_cast(&mut realm, &mut view, speedy, None);
    assert_eq!(
        refusal(&events, speedy),
        None,
        "Shadow Speed's Stealth goes up"
    );
    assert!(view.aura(me, aura).is_some(), "stealthed");
    let fast = run_speed(&events).expect("the run speed rises");

    stand(&mut realm, &camped, false);
    let events = try_cast(&mut realm, &mut view, ambush, Some(camped.mob.guid));
    assert_eq!(refusal(&events, ambush), None, "Ambush breaks the Stealth");
    assert!(view.aura(me, aura).is_none(), "both effects gone");
    let after = run_speed(&events).expect("the run speed falls back");
    assert!(
        (fast / after - 1.3).abs() < 0.01,
        "{fast} in Stealth, {after} after"
    );
}

/// The last run speed the realm forced on the player in `events`.
fn run_speed(events: &[SessionEvent]) -> Option<f32> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::ForceSpeedChange { speed, .. } => Some(*speed),
        _ => None,
    })
}

#[test]
fn shadow_dance_takes_40_s_off_vanish_s_cooldown_in_the_realm() {
    let vanish = spell("nightblade", "vanish");
    let dance = choice("nightblade", "nightblade_shadowcraft", "shadow_dance");
    for (rows, ready) in [
        (&[][..], false),
        (&[("nightblade_shadowcraft", dance)][..], true),
    ] {
        let (mut realm, mut view, _, _) = nightblade_at_camp(5, 25, rows);
        let events = try_cast(&mut realm, &mut view, vanish, None);
        assert_eq!(refusal(&events, vanish), None);
        view.run(&mut realm, 81.0);
        let events = try_cast(&mut realm, &mut view, vanish, None);
        assert_eq!(
            refusal(&events, vanish).is_none(),
            ready,
            "Vanish ready again after 81 s with {rows:?}"
        );
    }
}
