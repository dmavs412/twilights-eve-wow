//! The Zealot (#686), the Vanguard Line's holy Tier 2 tank: Brother-Knight Aldren Vale waits
//! unmarked among the graves of Tranquil Gardens until he is met, Commander Wray and Commander
//! Althea Ebonlocke point the way, his three-quest Trial gates the Advancement, and the sheet gives
//! the Calling a Judgement that spends its Seal of Iron to mend, a Holy Wrath that stuns only the
//! dead, a rage bar Fervent widens and a Hallowed Ground Sanctified makes heal more.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::messages::{GossipOption, PeriodicTick};
use benilla_protocol::SessionEvent;
use common::fight::{cast, refusal, View};
use common::party::unit_creates;
use common::vanguard::{learned, set_row};
use common::*;
use gear_common::*;
use quest_common::*;

/// Brother-Knight Aldren Vale, Commander Aldous Wray and Commander Althea Ebonlocke, and where
/// Althea stands in Darkshire (`content/duskwood.toml`).
const VALE: u32 = 190111;
const WRAY: u32 = 190071;
const ALTHEA: u32 = 264;
const ALTHEA_AT: [f32; 3] = [-10559.5, -1152.25, 28.076];
/// Duskwood's zone, and Stormwind's.
const DUSKWOOD: u32 = 10;
const STORMWIND: u32 = 1519;

/// The Advancement Quest and its Trial in order.
const VOW: u32 = 64003;
const BONES: u32 = 64416;
const GHOUL: u32 = 64417;
const STAND: u32 = 64418;
/// The Hallowed Bone 64416 collects.
const HALLOWED_BONE: u32 = 980821;

/// `SPELL_FAILED_CASTER_AURASTATE`: the cast needs a state the caster is not in.
const CASTER_AURASTATE: u8 = 0x12;

/// A save of `calling` at `level` standing 1.5 yd east of `at` in `zone`, carrying `items`, with
/// `quests` under way and `done` handed in, entered in the world.
fn standing(
    at: [f32; 3],
    zone: u32,
    calling: &str,
    level: u8,
    items: &[Carried],
    quests: &[Underway],
    done: &[u32],
) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(level, items, 0, quests);
    let [x, y, z] = at;
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

/// A Vanguard of 30 among the graves beside Vale, carrying `items`, with `quests` under way and
/// `done` handed in.
fn in_the_gardens(
    items: &[Carried],
    quests: &[Underway],
    done: &[u32],
) -> (Realm, u64, Vec<SessionEvent>) {
    let at = hub_home(hub_guid(VALE));
    standing(at, DUSKWOOD, "vanguard", 30, items, quests, done)
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

/// Asks `npc` the gossip line `option`, standing beside it already: the text the reply shows.
fn ask(realm: &mut Realm, npc: u64, option: &str) -> String {
    let lines = menu(realm, npc);
    let line = lines
        .iter()
        .find(|o| o.message == option)
        .unwrap_or_else(|| panic!("{option:?} in {lines:?}"));
    realm.handle(RealmCommand::GossipSelectOption {
        guid: npc,
        option: line.index,
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
fn vale_is_unmarked_until_met_and_wray_and_althea_give_the_hints() {
    let (mut realm, _, events) = in_the_gardens(&[], &[(VOW, [0; 4])], &[]);
    let vale = hub_guid(VALE);
    assert!(!marked(&events, vale), "not met yet");
    walk_up_to(&mut realm, vale);
    menu(&mut realm, vale);
    realm.tick(0.0, &FlatGround);
    assert!(marked(&drain(&mut realm), vale), "met at his gossip menu");

    let (mut realm, _, _) = standing(
        hub_home(hub_guid(WRAY)),
        STORMWIND,
        "vanguard",
        30,
        &[],
        &[(VOW, [0; 4])],
        &[],
    );
    assert_eq!(
        ask(
            &mut realm,
            hub_guid(WRAY),
            "Where does the last Zealot keep vigil?"
        ),
        "Duskwood. Commander Althea Ebonlocke runs the Night Watch in Darkshire. She knows every \
         soul that walks her graves at night. Ask her."
    );

    let (mut realm, _, events) = standing(
        ALTHEA_AT,
        DUSKWOOD,
        "vanguard",
        30,
        &[],
        &[(VOW, [0; 4])],
        &[],
    );
    let althea = unit_creates(&events)
        .into_iter()
        .find(|(_, entry, _)| *entry == ALTHEA)
        .expect("Althea streams in")
        .0;
    assert_eq!(
        ask(&mut realm, althea, "Who keeps vigil among your dead?"),
        "A knight in white walks the Tranquil Gardens south and east of town, among the graves \
         the dead keep climbing out of. My watchers leave him be. He prays louder than they scream."
    );
}

#[test]
fn vale_sets_the_trial_in_order_and_its_end_makes_a_level_1_zealot() {
    let vale = hub_guid(VALE);
    let (mut realm, _, _) = in_the_gardens(&[], &[], &[]);
    assert!(
        !on_offer(&mut realm, vale).contains(&BONES),
        "64003 not taken"
    );

    let (mut realm, _, _) = in_the_gardens(&[], &[(VOW, [0; 4])], &[]);
    assert_eq!(on_offer(&mut realm, vale), [BONES]);
    assert!(completed(&hand_in(&mut realm, vale, VOW)).is_none());

    let (mut realm, _, _) = in_the_gardens(
        &[(23, HALLOWED_BONE, 8)],
        &[(VOW, [0; 4]), (BONES, [0; 4])],
        &[],
    );
    assert!(completed(&hand_in(&mut realm, vale, BONES)).is_some());
    assert_eq!(on_offer(&mut realm, vale), [GHOUL]);

    let (mut realm, _, _) = in_the_gardens(&[], &[(VOW, [0; 4]), (GHOUL, [1, 0, 0, 0])], &[BONES]);
    assert!(completed(&hand_in(&mut realm, vale, GHOUL)).is_some());
    assert_eq!(on_offer(&mut realm, vale), [STAND]);
    assert!(completed(&hand_in(&mut realm, vale, VOW)).is_none());

    let (mut realm, me, entered) = in_the_gardens(
        &[],
        &[(VOW, [0; 4]), (STAND, [12, 0, 0, 0])],
        &[BONES, GHOUL],
    );
    assert!(completed(&hand_in(&mut realm, vale, VOW)).is_none());
    assert_eq!(realm.current_calling().unwrap().key, "vanguard");
    assert!(completed(&hand_in(&mut realm, vale, STAND)).is_some());
    let advanced = hand_in(&mut realm, vale, VOW);
    assert!(completed(&advanced).is_some(), "{advanced:?}");
    assert_eq!(realm.current_calling().unwrap().key, "zealot");
    let saved = realm.export_character(me).unwrap();
    assert!(saved.contains("\nlevel = 1\n"), "{saved}");

    let mut events = entered;
    events.extend(advanced);
    let known = spellbook(&events);
    for key in [
        "bulwark_charge",
        "judgement",
        "seal_of_iron",
        "hallowed_ground",
        "rebuke",
    ] {
        assert!(known.contains(&spell("zealot", key)), "{key}: {known:?}");
    }
    for key in ["righteous_fury", "holy_wrath", "hammer_of_justice"] {
        assert!(!known.contains(&spell("zealot", key)), "{key} at 1");
    }
    let now = descriptor(&events, me);
    assert_eq!(now.unit_shapeshift_form(), 17, "still in Battle Stance");
    assert_eq!(now.unit_power(1), Some(0), "a rage bar");

    // At level 10 the login's spell book has Righteous Fury.
    let blob = saved.replacen("\nlevel = 1\n", "\nlevel = 10\n", 1);
    let mut later = Realm::with_seed(SEED);
    later.import_character(&blob).unwrap();
    later.handle(RealmCommand::EnterWorld { guid: me });
    later.tick(0.0, &FlatGround);
    let known = spellbook(&drain(&mut later));
    assert!(
        known.contains(&spell("zealot", "righteous_fury")),
        "{known:?}"
    );
    assert!(!known.contains(&spell("zealot", "holy_wrath")));
}

#[test]
fn wray_offers_a_capped_vanguard_no_tier_2_quest_and_still_offers_the_templars_charge() {
    let wray = hub_guid(WRAY);
    let home = hub_home(wray);
    let (mut realm, _, _) = standing(home, STORMWIND, "vanguard", 30, &[], &[], &[]);
    let offer = on_offer(&mut realm, wray);
    assert!(!offer.contains(&VOW), "{offer:?}");
    let (mut realm, _, _) = standing(home, STORMWIND, "zealot", 50, &[], &[], &[]);
    assert!(on_offer(&mut realm, wray).contains(&64004));
}

// ── The kit ─────────────────────────────────────────────────────────────────────────────────────

/// An undead creature (`creature_type = 6`), and a Bristleback Hunter, a humanoid (7), of
/// `content/barrens.toml`, as tests/holy_tank_kinds.rs picks them.
const UNDEAD: u32 = 7067;
const HUMANOID: u32 = 3258;

/// Where the Zealot stands: east of the scorpid camp, out of its notice, facing west.
const STAND_AT: [f32; 3] = [common::fight::CAMP[0] + 40.0, common::fight::CAMP[1], 0.0];

/// A level 30 Zealot east of the camp with each `(row, choice)` set and a full rage bar: the
/// realm, their guid and the view.
fn zealot(rows: &[(&str, u32)]) -> (Realm, u64, View) {
    zealot_at(30, rows)
}

/// [`zealot`] at `level`.
fn zealot_at(level: u8, rows: &[(&str, u32)]) -> (Realm, u64, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Aldren", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"zealot\"");
    let mut realm = Realm::with_seed(4);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    for (row, choice) in rows {
        let events = set_row(&mut realm, &mut view, row, *choice);
        assert!(learned(&events, *choice), "{row} takes {choice}");
    }
    view.run(&mut realm, 1.0);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: STAND_AT,
        orientation: std::f32::consts::PI,
    });
    view.run(&mut realm, 1.0);
    realm.set_player_power(1000);
    view.drain(&mut realm);
    (realm, guid, view)
}

/// A level 3 creature of `entry` `west` yards west and `north` yards north of the Zealot, seen.
fn spawn(realm: &mut Realm, view: &mut View, me: u64, entry: u32, west: f32, north: f32) -> u64 {
    spawn_at_level(realm, view, me, entry, 3, [west, north])
}

/// [`spawn`] at `level`, `[west, north]` of the Zealot.
fn spawn_at_level(
    realm: &mut Realm,
    view: &mut View,
    me: u64,
    entry: u32,
    level: u8,
    [west, north]: [f32; 2],
) -> u64 {
    let at = [STAND_AT[0] - west, STAND_AT[1] + north, 0.0];
    let guid = realm
        .spawn_for(
            &FlatGround,
            entry,
            level,
            at,
            1,
            twilight_realm::Audience::of(&[me]),
        )
        .expect("a creature of the entry");
    view.run(realm, 0.5);
    guid
}

/// Casts `spell_id` at `target`, asserting it went; what the client saw, with a tick after.
fn cast_at(
    realm: &mut Realm,
    view: &mut View,
    spell_id: u32,
    target: Option<u64>,
) -> Vec<SessionEvent> {
    if let Some(t) = target {
        realm.set_creature_health_pct(t, 100);
    }
    realm.handle(cast(spell_id, target));
    let mut events = view.drain(realm);
    assert_eq!(refusal(&events, spell_id), None, "{spell_id} is cast");
    events.extend(view.run(realm, 0.1));
    events
}

/// The damage and school `spell_id` logged on `target` in `events`.
fn hit(events: &[SessionEvent], spell_id: u32, target: u64) -> Option<(u32, u8)> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id && l.target == target => {
            Some((l.damage, l.school))
        }
        _ => None,
    })
}

/// Judgement is refused with no Seal of Iron up and costs nothing; with it up it takes the Seal
/// off, hits in holy and heals the Zealot 10% of their maximum health.
#[test]
fn judgement_spends_the_seal_of_iron_for_a_holy_hit_and_a_heal() {
    let judgement = spell("zealot", "judgement");
    let seal = spell("zealot", "aura_seal_of_iron");
    let (mut realm, me, mut view) = zealot(&[]);
    let mob = spawn(&mut realm, &mut view, me, HUMANOID, 5.0, 0.0);
    let rage = view.rage(me);
    realm.handle(cast(judgement, Some(mob)));
    let events = view.drain(&mut realm);
    assert_eq!(
        refusal(&events, judgement),
        Some(CASTER_AURASTATE),
        "no Seal"
    );
    assert_eq!(view.rage(me), rage, "costs nothing");

    cast_at(&mut realm, &mut view, spell("zealot", "seal_of_iron"), None);
    assert!(view.aura(me, seal).is_some(), "the Seal is worn");
    view.run(&mut realm, 1.6);
    realm.set_player_health(1);
    view.drain(&mut realm);
    let events = cast_at(&mut realm, &mut view, judgement, Some(mob));
    assert!(view.aura(me, seal).is_none(), "judging takes it off");
    let (damage, school) = hit(&events, judgement, mob).expect("the judgement hits");
    assert_eq!(school, 1, "holy");
    assert!(damage > 0);
    let max = view.fields(me).unit_max_health().unwrap();
    let healed = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::SpellHealLog(l) if l.spell_id == judgement && l.target == me => {
                Some(l.amount)
            }
            _ => None,
        })
        .expect("the Zealot heals");
    assert!(
        healed.abs_diff(max / 10) <= 1,
        "10% of the maximum health {max}, rounded: {healed}"
    );
}

/// Judgement judges a Seal of Iron Skill Choice's Seal too (Seal of Mending's buff).
#[test]
fn judgement_judges_the_seal_a_skill_choice_puts_up() {
    let mending = choice("zealot", "seal_of_iron", "seal_of_mending");
    let worn = spell("zealot", "aura_seal_of_mending");
    let (mut realm, me, mut view) = zealot(&[("seal_of_iron", mending)]);
    let mob = spawn(&mut realm, &mut view, me, HUMANOID, 5.0, 0.0);
    cast_at(&mut realm, &mut view, mending, None);
    assert!(
        view.aura(me, worn).is_some(),
        "Seal of Mending's buff is worn"
    );
    view.run(&mut realm, 1.6);
    let judgement = spell("zealot", "judgement");
    let events = cast_at(&mut realm, &mut view, judgement, Some(mob));
    assert!(view.aura(me, worn).is_none(), "judging takes it off");
    assert!(hit(&events, judgement, mob).is_some(), "the judgement hits");
}

/// Holy Wrath stuns the undead creature within 8 yd and not the humanoid beside it.
#[test]
fn holy_wrath_stuns_the_undead_and_not_the_humanoid() {
    let (mut realm, me, mut view) = zealot(&[]);
    // Sturdy enough to outlive the doubled hit and wear the stun.
    let undead = spawn_at_level(&mut realm, &mut view, me, UNDEAD, 40, [4.0, 2.0]);
    let humanoid = spawn_at_level(&mut realm, &mut view, me, HUMANOID, 40, [4.0, -2.0]);
    let wrath = spell("zealot", "holy_wrath");
    let events = cast_at(&mut realm, &mut view, wrath, None);
    // The stun shows as its row's `aura_spell_id`, Charge Stun.
    let stun = 7922;
    assert!(view.aura(undead, stun).is_some(), "the undead is stunned");
    assert!(view.aura(humanoid, stun).is_none(), "the humanoid is not");
    let (_, school) = hit(&events, wrath, undead).expect("the undead is hit");
    assert_eq!(school, 1, "holy");
    assert!(
        hit(&events, wrath, humanoid).is_some(),
        "the humanoid is hit"
    );
}

/// Fervent's 10 more rage widens the bar the client shows.
#[test]
fn fervent_raises_the_rage_cap() {
    let (_, me, view) = zealot(&[]);
    assert_eq!(view.fields(me).unit_max_power(1), Some(1000));
    let fervent = spell_of_talent("zealot_crusader", "fervent");
    let (_, me, view) = zealot(&[("zealot_crusader", fervent)]);
    assert_eq!(view.fields(me).unit_max_power(1), Some(1100), "in tenths");
}

/// The spell id of the Talent `key` on the Zealot's Talent row `row`.
fn spell_of_talent(row: &str, key: &str) -> u32 {
    choice("zealot", row, key)
}

/// The heal of the first Hallowed Ground tick on a level 50 Zealot, with `rows` set.
fn hallowed_tick(rows: &[(&str, u32)]) -> u32 {
    let (mut realm, me, mut view) = zealot_at(50, rows);
    realm.set_player_health(1);
    view.drain(&mut realm);
    cast_at(
        &mut realm,
        &mut view,
        spell("zealot", "hallowed_ground"),
        None,
    );
    let aura = spell("zealot", "aura_hallowed");
    assert!(view.aura(me, aura).is_some(), "worn standing in it");
    let events = view.run(&mut realm, 1.5);
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == aura && l.target == me => {
                l.ticks.iter().find_map(|t| match t {
                    PeriodicTick::Heal { amount } => Some(*amount),
                    _ => None,
                })
            }
            _ => None,
        })
        .expect("a heal tick")
}

/// Standing in Hallowed Ground heals each second; Sanctified heals 30% more.
#[test]
fn hallowed_ground_heals_the_zealot_standing_in_it_and_sanctified_more() {
    let plain = hallowed_tick(&[]);
    assert!(plain > 8, "8 + 1 a level: {plain}");
    let sanctified = spell_of_talent("zealot_light_eternal", "sanctified");
    assert_eq!(
        hallowed_tick(&[("zealot_light_eternal", sanctified)]),
        (plain * 13 + 5) / 10,
        "{plain}, 30% more"
    );
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
fn the_sheet_gives_the_zealot_ten_rows_of_three() {
    let c = sheet("zealot");
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
}

/// Every shipped Calling is the sheet's: `content/callings.toml` holds none.
#[test]
fn no_shipped_calling_is_read_from_callings_toml() {
    let tables: toml::Table = include_str!("../content/callings.toml").parse().unwrap();
    assert!(tables.get("calling").is_none(), "{tables:?}");
}
