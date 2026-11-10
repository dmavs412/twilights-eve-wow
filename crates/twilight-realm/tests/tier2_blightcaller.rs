//! The Blightcaller (#692), the Acolyte Line's Tier 2 healer: the Calling Choice offers it at the
//! Acolyte's cap, Mother Ysolde Grey waits unmarked in her garden of rot at Beggar's Haunt until she
//! is met, Brother Corwin, Madame Eva and Blind Mary point the way, her two-quest Trial gates the
//! Advancement, and the sheet gives the Calling its ten Talent Rows. Through the Blight it heals a
//! friend: Plague Tithe's ticks mend the most hurt friend, and Harvest shares what it reaps.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::messages::{GossipOption, PeriodicTick};
use benilla_protocol::{EntityKind, SessionEvent, START_ELWYNN};
use common::acolyte::{dummy, YardGround};
use common::fight::{cast, refusal, View};
use common::party::Party;
use common::*;
use gear_common::{Carried, SEED};
use quest_common::*;
use twilight_realm::{Realm, RealmCommand};

/// The first backpack slot.
const PACK_1: u8 = 23;

/// Mother Ysolde Grey, Brother Corwin Ashgrave (the Northshire Acolyte Trainer), Madame Eva and
/// Blind Mary (friendly Duskwood spawns, not Hub NPCs).
const GREY: u32 = 190118;
const CORWIN: u32 = 190054;
const EVA: u32 = 265;
const MARY: u32 = 302;
/// Madame Eva's and Blind Mary's spawn points (`content/duskwood.toml`).
const EVA_AT: [f32; 2] = [-10532.4, -1213.12];
const MARY_AT: [f32; 2] = [-10780.2, -1380.28];
/// Duskwood's zone.
const DUSKWOOD: u32 = 10;

/// The Advancement Quest and its Trial, in order.
const GARDEN: u32 = 64407;
const COMPOST: u32 = 64451;
const HEARTS: u32 = 64452;
/// Rotting Heart, the Trial's quest item.
const HEART: u32 = 980891;

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
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Gardener", HUMAN, 0, WARRIOR) else {
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

/// An Acolyte of 30 standing beside Mother Grey in her garden, carrying `items`, with `quests`
/// under way and `done` handed in, entered in the world.
fn in_the_garden(
    items: &[Carried],
    quests: &[Underway],
    done: &[u32],
) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(30, items, 0, quests);
    let [x, y, z] = hub_home(hub_guid(GREY));
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

/// Walks to the friendly spawn of `entry` at `at`: its guid, as it streams in.
fn walk_to_spawn(realm: &mut Realm, entry: u32, at: [f32; 2]) -> u64 {
    walk_to(realm, [at[0] + 1.5, at[1], 0.0])
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                fields,
                ..
            } if fields.object_entry() == Some(entry) => Some(*guid),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{entry} streams in"))
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
fn a_level_30_acolyte_is_offered_the_blightcaller() {
    let offers = offered(&capped_acolyte());
    assert_eq!(offers.len(), 1, "{offers:?}");
    assert!(offers[0].contains(&GARDEN), "{offers:?}");
}

#[test]
fn mother_grey_is_unmarked_until_met_and_corwin_eva_and_mary_give_the_hints() {
    let (mut realm, _, events) = in_the_garden(&[], &[(GARDEN, [0; 4])], &[]);
    let grey = hub_guid(GREY);
    assert!(!marked(&events, grey), "not met yet");
    walk_up_to(&mut realm, grey);
    menu(&mut realm, grey);
    realm.tick(0.0, &FlatGround);
    assert!(marked(&drain(&mut realm), grey), "met at her gossip menu");

    let corwin = hub_guid(CORWIN);
    walk_up_to(&mut realm, corwin);
    assert_eq!(
        hint(&mut realm, corwin, "Who reads the rot?"),
        "Madame Eva, in Darkshire. She reads cards for the Night Watch. Ask her about the garden."
    );

    let eva = walk_to_spawn(&mut realm, EVA, EVA_AT);
    assert_eq!(
        hint(&mut realm, eva, "What do the cards say of the rot?"),
        "The Tower, reversed. Then the Mother. Blind Mary, in the house on the hill south-east of \
         town, sees what I only read. Go to her."
    );

    let mary = walk_to_spawn(&mut realm, MARY, MARY_AT);
    assert_eq!(
        hint(&mut realm, mary, "Who keeps the rot's garden?"),
        "Mother Grey. Up at Beggar's Haunt, past the dead men who make medicine. She feeds the \
         beds what the beds want. Don't eat anything she grows."
    );
}

#[test]
fn mother_grey_sets_the_trial_in_order_and_its_end_makes_a_level_1_blightcaller() {
    let grey = hub_guid(GREY);
    let (mut realm, _, _) = in_the_garden(&[], &[], &[]);
    assert!(
        !on_offer(&mut realm, grey).contains(&COMPOST),
        "64407 not taken"
    );

    let (mut realm, _, _) = in_the_garden(&[], &[(GARDEN, [0; 4])], &[]);
    assert_eq!(on_offer(&mut realm, grey), [COMPOST]);
    assert!(completed(&hand_in(&mut realm, grey, GARDEN)).is_none());

    let (mut realm, _, _) = in_the_garden(&[], &[(GARDEN, [0; 4]), (COMPOST, [6, 0, 0, 0])], &[]);
    assert!(completed(&hand_in(&mut realm, grey, COMPOST)).is_some());
    assert_eq!(on_offer(&mut realm, grey), [HEARTS]);
    assert!(completed(&hand_in(&mut realm, grey, GARDEN)).is_none());

    let (mut realm, me, entered) = in_the_garden(
        &[(PACK_1, HEART, 5)],
        &[(GARDEN, [0; 4]), (HEARTS, [5, 0, 0, 0])],
        &[COMPOST],
    );
    assert!(completed(&hand_in(&mut realm, grey, GARDEN)).is_none());
    assert_eq!(realm.current_calling().unwrap().key, "acolyte");
    assert!(completed(&hand_in(&mut realm, grey, HEARTS)).is_some());
    let advanced = hand_in(&mut realm, grey, GARDEN);
    assert!(completed(&advanced).is_some(), "{advanced:?}");
    assert_eq!(realm.current_calling().unwrap().key, "blightcaller");
    let saved = realm.export_character(me).unwrap();
    assert!(saved.contains("\nlevel = 1\n"), "{saved}");

    let mut events = entered;
    events.extend(advanced);
    let known = spellbook(&events);
    for key in [
        "creeping_blight",
        "plague_tithe",
        "miasma",
        "harvest",
        "hush",
    ] {
        assert!(
            known.contains(&spell("blightcaller", key)),
            "{key}: {known:?}"
        );
    }
    for key in ["spread_blight", "wasting_communion", "rise_again"] {
        assert!(!known.contains(&spell("blightcaller", key)), "{key} at 1");
    }

    // At level 10 the login's spell book has Spread Blight.
    let blob = saved.replacen("\nlevel = 1\n", "\nlevel = 10\n", 1);
    let mut later = Realm::with_seed(SEED);
    later.import_character(&blob).unwrap();
    later.handle(RealmCommand::EnterWorld { guid: me });
    later.tick(0.0, &FlatGround);
    let known = spellbook(&drain(&mut later));
    assert!(
        known.contains(&spell("blightcaller", "spread_blight")),
        "{known:?}"
    );
    assert!(!known.contains(&spell("blightcaller", "wasting_communion")));
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

/// The Creeping Blight tick of `calling` at level `l`, read from its aura row's formula.
fn blight_tick(calling: &str, l: f64) -> f64 {
    let c = sheet(calling);
    let aura = c["auras"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["key"] == "aura_creeping_blight")
        .expect("a Blight");
    let tick = &aura["effects"][0]["params"]["tick"];
    tick["base"].as_f64().unwrap() + tick["per_level"].as_f64().unwrap() * l
}

#[test]
fn the_sheet_gives_ten_rows_and_a_blight_that_starts_stronger() {
    let c = sheet("blightcaller");
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
    assert!(blight_tick("blightcaller", 1.0) > blight_tick("acolyte", 30.0));
}

/// Where the Blightcaller (player 0) and the friend (player 1) stand in the Test Grounds, 25 yd
/// west of dummy 1 and 2 yd apart.
const SPOTS: [[f32; 2]; 2] = [[-9665.0, 258.0], [-9665.0, 260.0]];

/// A level 20 Blightcaller and a level 20 friend in the Test Grounds, each in their own session.
fn healer_and_friend() -> Party {
    let realm = Realm::with_seed(0x692);
    let mut party = Party::edited(
        realm,
        &[20, 20],
        START_ELWYNN,
        Box::new(YardGround),
        |i, blob| {
            if i == 0 {
                blob.replace("calling = \"basic_warrior\"", "calling = \"blightcaller\"")
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

/// Brings the friend down to 100 health.
fn hurt_friend(party: &mut Party) {
    let [x, y] = SPOTS[1];
    party.go(1, [x, y, 47.2], 0.0);
    party.realm.set_player_health(100);
    party.run(0.2);
}

/// What player `i`'s client knows.
fn view(party: &Party, i: usize) -> View {
    let mut v = View::default();
    v.see(party.seen(i));
    v
}

/// The Blightcaller casts `spell_id` at dummy 1: refused or not.
fn cast_at_dummy(party: &mut Party, spell_id: u32) {
    let before = party.seen(0).len();
    party.cmd(0, cast(spell_id, Some(dummy(1))));
    assert_eq!(
        refusal(&party.seen(0)[before..], spell_id),
        None,
        "spell {spell_id} goes off"
    );
}

/// Plague Tithe's channel ticks on dummy 1, and each tick's heal over time goes on the hurt friend
/// beside the Blightcaller, in the friend's own session.
#[test]
fn plague_tithe_on_a_dummy_heals_the_hurt_friend() {
    let tithe = spell("blightcaller", "plague_tithe");
    let mut party = healer_and_friend();
    hurt_friend(&mut party);
    let friend = party.guids[1];
    assert_eq!(view(&party, 1).health(friend), 100);
    cast_at_dummy(&mut party, tithe);
    let got = party.run(6.0);
    let hits: Vec<u32> = got[0]
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == tithe && l.target == dummy(1) => {
                Some(l.damage)
            }
            _ => None,
        })
        .collect();
    // Three ticks, each rolled afresh, so one may miss.
    assert!((1..=3).contains(&hits.len()), "{hits:?}");
    let heals: Vec<u32> = got[1]
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == tithe && l.target == friend => {
                match l.ticks[0] {
                    PeriodicTick::Heal { amount } => Some(amount),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect();
    assert!(!heals.is_empty(), "the friend's heal ticks: {:?}", got[1]);
    // A tick's heal over time is a quarter of what that tick dealt, never of the ticks before it.
    let most = hits.iter().max().unwrap() / 4;
    assert!(heals.iter().all(|&h| h <= most), "{heals:?} of {hits:?}");
    // Out of combat, the friend's own regeneration adds to it.
    assert!(view(&party, 1).health(friend) >= 100 + heals.iter().sum::<u32>());
}

/// Wasting Communion laid at the hurt friend's feet heals them in their own session: every friendly
/// player inside wears the area's ally aura, not the caster alone.
#[test]
fn wasting_communion_heals_a_friend_inside_it() {
    let communion = spell("blightcaller", "wasting_communion");
    let mending = spell("blightcaller", "aura_communion");
    let mut party = healer_and_friend();
    hurt_friend(&mut party);
    let [x, y] = SPOTS[1];
    let before = party.seen(0).len();
    party.cmd(
        0,
        RealmCommand::CastSpellAtDest {
            spell_id: communion,
            dest: [x, y, 47.2],
        },
    );
    assert_eq!(
        refusal(&party.seen(0)[before..], communion),
        None,
        "Wasting Communion is cast"
    );
    let got = party.run(5.0);
    let friend = party.guids[1];
    let ticks = got[1]
        .iter()
        .filter(|e| {
            matches!(e, SessionEvent::PeriodicAuraLog(l)
                if l.spell_id == mending && l.target == friend)
        })
        .count();
    assert!(ticks >= 2, "the friend heals every 2 s: {:?}", got[1]);
}

/// Harvest takes the Blight off dummy 1 and splits all it dealt as healing between the two players
/// within 30 yd of it: the hurt friend heals half.
#[test]
fn harvest_on_a_blighted_dummy_heals_the_friend_a_share_of_its_damage() {
    let blight = spell("blightcaller", "creeping_blight");
    let harvest = spell("blightcaller", "harvest");
    let mut party = healer_and_friend();
    cast_at_dummy(&mut party, blight);
    party.run(4.0);
    let stacks = view(&party, 0)
        .aura(dummy(1), blight)
        .map_or(0, |a| a.stacks);
    assert!(stacks >= 2, "the Blight grew: {stacks}");
    hurt_friend(&mut party);
    let before = party.seen(0).len();
    cast_at_dummy(&mut party, harvest);
    let dealt: u32 = party.seen(0)[before..]
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == harvest && l.target == dummy(1) => {
                Some(l.damage)
            }
            _ => None,
        })
        .sum();
    assert!(dealt > 0, "Harvest struck");
    assert!(
        view(&party, 0).aura(dummy(1), blight).is_none(),
        "the Blight is gone"
    );
    let got = party.run(0.3);
    let friend = party.guids[1];
    let healed: u32 = got[1]
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellHealLog(l) if l.spell_id == harvest && l.target == friend => {
                Some(l.amount)
            }
            _ => None,
        })
        .sum();
    assert_eq!(healed, dealt / 2, "half of {dealt}");
}
