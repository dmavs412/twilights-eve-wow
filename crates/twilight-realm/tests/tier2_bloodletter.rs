//! The Bloodletter (#687), the Cutthroat Line's Tier 2 Calling: the Calling Choice offers it at the
//! Cutthroat's cap, Sabine Redmark waits unmarked at the poachers' fire until she is met, Jorik
//! Vane and Wiley the Black point the way, her three-quest Trial gates the Advancement, and the
//! sheet gives the Calling its ten Talent Rows. Its Garrote needs no Stealth, its bleed ticks spill
//! its own blood droplets, and Red Frenzy spends the Bloodlust they grant.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::messages::GossipOption;
use benilla_protocol::{EntityKind, SessionEvent, START_ELWYNN};
use common::cutthroat::cutthroat_save;
use common::fight::{cast, flat, refusal, stand_at, View, CAMP};
use common::vanguard::enter;
use common::*;
use gear_common::*;
use quest_common::*;
use twilight_realm::{Realm, RealmCommand};

/// Sabine Redmark, Jorik Vane (the Northshire Cutthroat Trainer) and Wiley the Black in Lakeshire
/// (a friendly spawn, not a Hub NPC).
const SABINE: u32 = 190112;
const JORIK: u32 = 190051;
const WILEY: u32 = 266;
/// Wiley the Black's spawn point (`redridge_spawns.json`).
const WILEY_AT: [f32; 2] = [-9231.59, -2145.62];
/// Redridge Mountains' zone.
const REDRIDGE: u32 = 44;

/// The Advancement Quest and its Trial, in order.
const SALT: u32 = 64401;
const FIRST_BLOOD: u32 = 64421;
const HIDES: u32 = 64422;
const SPIDER_BLOOD: u32 = 64423;
/// The Torn Mongrel Hide 64422 collects.
const TORN_HIDE: u32 = 980831;

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

/// A Cutthroat of 30 who picked at the Tier 0 Calling Choice, entered in the world.
fn capped_cutthroat() -> Vec<SessionEvent> {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Leech", HUMAN, 0, WARRIOR) else {
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
    let mut realm = Realm::with_seed(SEED);
    realm
        .import_character(&blob)
        .unwrap_or_else(|e| panic!("{e:?}: {blob}"));
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    drain(&mut realm)
}

/// A Cutthroat of 30 standing beside Sabine at the fire, carrying `items`, with `quests` under way
/// and `done` handed in, entered in the world.
fn at_the_fire(
    items: &[Carried],
    quests: &[Underway],
    done: &[u32],
) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(30, items, 0, quests);
    let [x, y, z] = hub_home(hub_guid(SABINE));
    let (head, tail) = blob.split_once("\n[").unwrap_or((&blob, ""));
    let head: Vec<String> = head
        .lines()
        .map(|line| {
            if line.starts_with("map = ") {
                "map = 0".to_string()
            } else if line.starts_with("zone = ") {
                format!("zone = {REDRIDGE}")
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
        .replace("calling = \"basic_warrior\"", "calling = \"cutthroat\"");
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
fn a_level_30_cutthroat_is_offered_the_bloodletter() {
    let offers = offered(&capped_cutthroat());
    assert_eq!(offers.len(), 1, "{offers:?}");
    assert!(offers[0].contains(&SALT), "{offers:?}");
}

#[test]
fn sabine_is_unmarked_until_met_and_jorik_and_wiley_give_the_hints() {
    let (mut realm, _, events) = at_the_fire(&[], &[(SALT, [0; 4])], &[]);
    let sabine = hub_guid(SABINE);
    assert!(!marked(&events, sabine), "not met yet");
    walk_up_to(&mut realm, sabine);
    menu(&mut realm, sabine);
    realm.tick(0.0, &FlatGround);
    assert!(marked(&drain(&mut realm), sabine), "met at her gossip menu");

    let jorik = hub_guid(JORIK);
    walk_up_to(&mut realm, jorik);
    assert_eq!(
        hint(&mut realm, jorik, "Who sells blood for a living?"),
        "Wiley the Black, in Lakeshire. He sells salt and hooks to a woman who skins what she \
         bleeds. Tell him I sent you. Or don't. He'll guess."
    );

    let [x, y] = WILEY_AT;
    let streamed = walk_to(&mut realm, [x + 1.5, y, 0.0]);
    let wiley = streamed
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                fields,
                ..
            } if fields.object_entry() == Some(WILEY) => Some(*guid),
            _ => None,
        })
        .expect("Wiley the Black streams in");
    assert_eq!(
        hint(&mut realm, wiley, "Who buys your salt?"),
        "Sabine. She sits at the poachers' fire on Lakeridge Highway, south and west of the lake. \
         They think she's one of them. They're wrong about a lot of things."
    );
}

#[test]
fn sabine_sets_the_trial_in_order_and_its_end_makes_a_level_1_bloodletter() {
    let sabine = hub_guid(SABINE);
    let (mut realm, _, _) = at_the_fire(&[], &[], &[]);
    assert!(
        !on_offer(&mut realm, sabine).contains(&FIRST_BLOOD),
        "64401 not taken"
    );

    let (mut realm, _, _) = at_the_fire(&[], &[(SALT, [0; 4])], &[]);
    assert_eq!(on_offer(&mut realm, sabine), [FIRST_BLOOD]);
    assert!(completed(&hand_in(&mut realm, sabine, SALT)).is_none());

    let (mut realm, _, _) = at_the_fire(&[], &[(SALT, [0; 4]), (FIRST_BLOOD, [10, 0, 0, 0])], &[]);
    assert!(completed(&hand_in(&mut realm, sabine, FIRST_BLOOD)).is_some());
    assert_eq!(on_offer(&mut realm, sabine), [HIDES]);

    let (mut realm, _, _) = at_the_fire(
        &[(23, TORN_HIDE, 8)],
        &[(SALT, [0; 4]), (HIDES, [0; 4])],
        &[FIRST_BLOOD],
    );
    assert!(completed(&hand_in(&mut realm, sabine, HIDES)).is_some());
    assert_eq!(on_offer(&mut realm, sabine), [SPIDER_BLOOD]);
    assert!(completed(&hand_in(&mut realm, sabine, SALT)).is_none());

    let (mut realm, me, entered) = at_the_fire(
        &[],
        &[(SALT, [0; 4]), (SPIDER_BLOOD, [8, 0, 0, 0])],
        &[FIRST_BLOOD, HIDES],
    );
    assert!(completed(&hand_in(&mut realm, sabine, SALT)).is_none());
    assert_eq!(realm.current_calling().unwrap().key, "cutthroat");
    assert!(completed(&hand_in(&mut realm, sabine, SPIDER_BLOOD)).is_some());
    let advanced = hand_in(&mut realm, sabine, SALT);
    assert!(completed(&advanced).is_some(), "{advanced:?}");
    assert_eq!(realm.current_calling().unwrap().key, "bloodletter");
    let saved = realm.export_character(me).unwrap();
    assert!(saved.contains("\nlevel = 1\n"), "{saved}");

    let mut events = entered;
    events.extend(advanced);
    let known = spellbook(&events);
    for key in ["rupture", "hemorrhage", "garrote", "open_veins", "kick"] {
        assert!(
            known.contains(&spell("bloodletter", key)),
            "{key}: {known:?}"
        );
    }
    for key in ["blood_fan", "red_frenzy", "blood_pool"] {
        assert!(!known.contains(&spell("bloodletter", key)), "{key} at 1");
    }

    // At level 10 the login's spell book has Blood Fan.
    let blob = saved.replacen("\nlevel = 1\n", "\nlevel = 10\n", 1);
    let mut later = Realm::with_seed(SEED);
    later.import_character(&blob).unwrap();
    later.handle(RealmCommand::EnterWorld { guid: me });
    later.tick(0.0, &FlatGround);
    let known = spellbook(&drain(&mut later));
    assert!(
        known.contains(&spell("bloodletter", "blood_fan")),
        "{known:?}"
    );
    assert!(!known.contains(&spell("bloodletter", "red_frenzy")));
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

/// The skill `key` of `calling`'s sheet.
fn skill(calling: &str, key: &str) -> serde_json::Value {
    sheet(calling)["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["key"] == key)
        .unwrap_or_else(|| panic!("{calling} has a {key}"))
        .clone()
}

/// The tick of `calling`'s Rupture at level `l`, read from its formula.
fn rupture_tick(calling: &str, l: f64) -> f64 {
    let tick = &skill(calling, "rupture")["effects"][0]["params"]["tick"];
    tick["base"].as_f64().unwrap() + tick["per_level"].as_f64().unwrap() * l
}

#[test]
fn the_sheet_gives_ten_rows_a_stronger_rupture_and_a_garrote_without_stealth() {
    let c = sheet("bloodletter");
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
    assert!(rupture_tick("bloodletter", 1.0) > rupture_tick("cutthroat", 30.0));
    assert_eq!(skill("cutthroat", "garrote")["requires_stealth"], true);
    assert_ne!(skill("bloodletter", "garrote")["requires_stealth"], true);
}

// ── The blood ──────────────────────────────────────────────────────────────────────────────────

static GARROTE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("bloodletter", "garrote"));
static GARROTE_BLEED: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("bloodletter", "aura_garrote"));
static DROPLET: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("bloodletter", "aura_blood_droplet"));
static BLOODLUST: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("bloodletter", "aura_bloodlust"));
static RED_FRENZY: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("bloodletter", "red_frenzy"));
static FRENZY_BUFF: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("bloodletter", "aura_red_frenzy"));

/// A Bloodletter of `level`, fists only, in the world on `seed`'s dice, 2 yd from a camp mob: the
/// realm, the view, its guid and the mob.
fn bloodletter_at_camp(seed: u64, level: u8) -> (Realm, View, u64, common::fight::Mob) {
    let (me, blob) = cutthroat_save(level, true);
    let blob = blob.replace("calling = \"cutthroat\"", "calling = \"bloodletter\"");
    let (mut realm, mut view, _) = enter(me, &blob, seed);
    view.run(&mut realm, 1.0);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [CAMP[0] + 40.0, CAMP[1], 0.0],
        orientation: std::f32::consts::PI,
    });
    let mob = view
        .run(&mut realm, 1.0)
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                ..
            } if flat(*position, CAMP) < 15.0 => Some(common::fight::Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .expect("a camp mob streams in");
    stand_at(&mut realm, &mob, 2.0);
    (realm, view, me, mob)
}

/// The Bloodletter's droplets `events` create: each one's guid and centre.
fn droplets(events: &[SessionEvent]) -> Vec<(u64, [f32; 3])> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::DynamicObject,
                position,
                fields,
                ..
            } if fields.dynamicobject_spell_id() == Some(*DROPLET) => Some((*guid, *position)),
            _ => None,
        })
        .collect()
}

/// Garrote, cast out of Stealth on a camp mob, goes off; its ticks spill the Bloodletter's own
/// droplets at the Calling's 50%.
#[test]
fn garrote_casts_outside_stealth_and_its_ticks_spill_the_bloodletters_droplets() {
    let (mut ticks, mut spilled) = (0, 0);
    for seed in 0..6 {
        let (mut realm, mut view, _, mob) = bloodletter_at_camp(seed, 10);
        realm.handle(cast(*GARROTE, Some(mob.guid)));
        let mut events = view.drain(&mut realm);
        assert_eq!(refusal(&events, *GARROTE), None, "seed {seed}");
        events.extend(view.run_topped_up(&mut realm, mob.guid, 18.5));
        ticks += events
            .iter()
            .filter(
                |e| matches!(e, SessionEvent::PeriodicAuraLog(l) if l.spell_id == *GARROTE_BLEED),
            )
            .count();
        spilled += droplets(&events).len();
    }
    assert!(ticks >= 18, "only {ticks} Garrote ticks");
    assert!(
        spilled > 0 && spilled < ticks,
        "{spilled} droplets in {ticks} ticks"
    );
}

/// A droplet picked up grants the Bloodletter's own Bloodlust, which Red Frenzy then spends.
#[test]
fn a_droplet_grants_the_bloodletters_bloodlust_and_red_frenzy_spends_it() {
    for seed in 0..20 {
        let (mut realm, mut view, me, mob) = bloodletter_at_camp(seed, 20);
        realm.handle(cast(*GARROTE, Some(mob.guid)));
        view.drain(&mut realm);
        let mut found = None;
        for _ in 0..120 {
            realm.tick(0.1, &FlatGround);
            if let Some(d) = droplets(&view.drain(&mut realm)).first().copied() {
                found = Some(d);
                break;
            }
        }
        let Some((_, at)) = found else {
            continue;
        };
        if view.aura(me, *BLOODLUST).is_some() {
            continue;
        }
        realm.handle(RealmCommand::Move {
            motion: Default::default(),
            position: at,
            orientation: 0.0,
        });
        view.run(&mut realm, 0.2);
        let lust = view
            .aura(me, *BLOODLUST)
            .expect("the Bloodletter's Bloodlust is on the player");
        assert_eq!(lust.stacks, 1, "seed {seed}");

        realm.handle(cast(*RED_FRENZY, None));
        let events = view.drain(&mut realm);
        assert_eq!(refusal(&events, *RED_FRENZY), None, "seed {seed}");
        view.run(&mut realm, 0.2);
        assert!(view.aura(me, *BLOODLUST).is_none(), "seed {seed}: spent");
        let frenzy = view.aura(me, *FRENZY_BUFF).expect("Red Frenzy is up");
        assert_eq!(frenzy.stacks, 1, "seed {seed}");
        return;
    }
    panic!("no seed spilled a droplet");
}
