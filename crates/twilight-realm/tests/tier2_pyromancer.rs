//! The Pyromancer (#689), the Initiate Line's Tier 2 Calling: the Calling Choice offers it at the
//! Initiate's cap, Magus Ilyra Vance waits unmarked in the Mage Quarter until she is met, Elsa
//! Brightwood points the way, her four-quest Trial gates the Advancement, and the sheet gives the
//! Calling its ten Talent Rows and Embers that Fireball grows, Kindle copies, Flamestrike spreads
//! and Fuel feeds on.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::messages::GossipOption;
use benilla_protocol::{EntityKind, SessionEvent, START_ELWYNN};
use common::fight::Mob;
use common::fight::*;
use common::initiate::{cast_through, went_off, EAST_CAMP};
use common::*;
use gear_common::{Carried, SEED};
use quest_common::*;
use twilight_realm::{Realm, RealmCommand};

/// Magus Ilyra Vance and Elsa Brightwood (the Northshire Initiate Trainer).
const VANCE: u32 = 190114;
const ELSA: u32 = 190052;
/// Stormwind City's zone.
const STORMWIND: u32 = 1519;

/// The Advancement Quest and its Trial, in order.
const FLAME: u32 = 64403;
const KINDLING: u32 = 64431;
const TALLOW_QUEST: u32 = 64432;
const HIGHWAY: u32 = 64433;
const ALPHAS: u32 = 64434;
/// The Gnoll Tallow 64432 asks for.
const TALLOW: u32 = 980851;

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

/// An Initiate of 30 who picked at the Tier 0 Calling Choice, entered in the world.
fn capped_initiate() -> Vec<SessionEvent> {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Kindler", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    assert_eq!(create_result(&mut maker), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("calling = \"basic_warrior\"", "calling = \"initiate\"")
        .replacen("\nlevel = 1\n", "\nlevel = 30\nchosen_tiers = [0]\n", 1);
    let mut realm = Realm::with_seed(SEED);
    realm
        .import_character(&blob)
        .unwrap_or_else(|e| panic!("{e:?}: {blob}"));
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    drain(&mut realm)
}

/// A save of `calling` at `level` standing beside Vance in the Mage Quarter, carrying `items`,
/// with `quests` under way and `done` handed in, entered in the world.
fn by_the_fire(
    calling: &str,
    level: u8,
    items: &[Carried],
    quests: &[Underway],
    done: &[u32],
) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(level, items, 0, quests);
    let [x, y, z] = hub_home(hub_guid(VANCE));
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

/// The reply to the hint line `line` in `options`, chosen at `npc`.
fn hint(realm: &mut Realm, npc: u64, options: &[GossipOption], line: &str) -> String {
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
fn a_level_30_initiate_is_offered_the_pyromancer() {
    let offers = offered(&capped_initiate());
    assert_eq!(offers.len(), 1, "{offers:?}");
    assert!(offers[0].contains(&FLAME), "{offers:?}");
}

#[test]
fn vance_is_unmarked_until_met_and_elsa_gives_the_hint() {
    let (mut realm, _, events) = by_the_fire("initiate", 30, &[], &[(FLAME, [0; 4])], &[]);
    let vance = hub_guid(VANCE);
    assert!(!marked(&events, vance), "not met yet");
    walk_up_to(&mut realm, vance);
    menu(&mut realm, vance);
    realm.tick(0.0, &FlatGround);
    assert!(marked(&drain(&mut realm), vance), "met at her gossip menu");

    let elsa = hub_guid(ELSA);
    walk_up_to(&mut realm, elsa);
    let lines = menu(&mut realm, elsa);
    assert_eq!(
        hint(
            &mut realm,
            elsa,
            &lines,
            "Who keeps the oldest fire in Stormwind?"
        ),
        "Magus Ilyra Vance, in the Mage Quarter's courtyard. She lit it the day the Second War \
         ended and hasn't let it go out since. You'll smell it before you see it."
    );
}

#[test]
fn vance_sets_the_trial_in_order_and_its_end_makes_a_level_1_pyromancer() {
    let vance = hub_guid(VANCE);
    let (mut realm, _, _) = by_the_fire("initiate", 30, &[], &[], &[]);
    assert!(
        !on_offer(&mut realm, vance).contains(&KINDLING),
        "64403 not taken"
    );

    let (mut realm, _, _) = by_the_fire("initiate", 30, &[], &[(FLAME, [0; 4])], &[]);
    assert_eq!(on_offer(&mut realm, vance), [KINDLING]);
    assert!(completed(&hand_in(&mut realm, vance, FLAME)).is_none());

    let (mut realm, _, _) = by_the_fire(
        "initiate",
        30,
        &[],
        &[(FLAME, [0; 4]), (KINDLING, [8, 6, 0, 0])],
        &[],
    );
    assert!(completed(&hand_in(&mut realm, vance, KINDLING)).is_some());
    assert_eq!(on_offer(&mut realm, vance), [TALLOW_QUEST]);

    let (mut realm, _, _) = by_the_fire(
        "initiate",
        30,
        &[(23, TALLOW, 8)],
        &[(FLAME, [0; 4]), (TALLOW_QUEST, [0; 4])],
        &[KINDLING],
    );
    assert!(completed(&hand_in(&mut realm, vance, TALLOW_QUEST)).is_some());
    assert_eq!(on_offer(&mut realm, vance), [HIGHWAY]);
    assert!(completed(&hand_in(&mut realm, vance, FLAME)).is_none());

    let (mut realm, _, _) = by_the_fire(
        "initiate",
        30,
        &[],
        &[(FLAME, [0; 4]), (HIGHWAY, [10, 0, 0, 0])],
        &[KINDLING, TALLOW_QUEST],
    );
    assert!(completed(&hand_in(&mut realm, vance, HIGHWAY)).is_some());
    assert_eq!(on_offer(&mut realm, vance), [ALPHAS]);

    let (mut realm, me, entered) = by_the_fire(
        "initiate",
        30,
        &[],
        &[(FLAME, [0; 4]), (ALPHAS, [6, 0, 0, 0])],
        &[KINDLING, TALLOW_QUEST, HIGHWAY],
    );
    assert!(completed(&hand_in(&mut realm, vance, FLAME)).is_none());
    assert_eq!(realm.current_calling().unwrap().key, "initiate");
    assert!(completed(&hand_in(&mut realm, vance, ALPHAS)).is_some());
    let advanced = hand_in(&mut realm, vance, FLAME);
    assert!(completed(&advanced).is_some(), "{advanced:?}");
    assert_eq!(realm.current_calling().unwrap().key, "pyromancer");
    let saved = realm.export_character(me).unwrap();
    assert!(saved.contains("\nlevel = 1\n"), "{saved}");

    let mut events = entered;
    events.extend(advanced);
    let known = spellbook(&events);
    for key in [
        "searing_ember",
        "fireball",
        "catalyst",
        "pyroblast",
        "counterspell",
    ] {
        assert!(
            known.contains(&spell("pyromancer", key)),
            "{key}: {known:?}"
        );
    }
    for key in ["kindle", "flamestrike", "blast_wave"] {
        assert!(!known.contains(&spell("pyromancer", key)), "{key} at 1");
    }

    // At level 10 the login's spell book has Kindle.
    let blob = saved.replacen("\nlevel = 1\n", "\nlevel = 10\n", 1);
    let mut later = Realm::with_seed(SEED);
    later.import_character(&blob).unwrap();
    later.handle(RealmCommand::EnterWorld { guid: me });
    later.tick(0.0, &FlatGround);
    let known = spellbook(&drain(&mut later));
    assert!(known.contains(&spell("pyromancer", "kindle")), "{known:?}");
    assert!(!known.contains(&spell("pyromancer", "flamestrike")));
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

/// The hit of `calling`'s Searing Ember at level `l`, read from its formula.
fn ember_hit(calling: &str, l: f64) -> f64 {
    let c = sheet(calling);
    let ember = c["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["key"] == "searing_ember")
        .expect("a Searing Ember");
    let amount = &ember["effects"][0]["params"]["amount"];
    amount["base"].as_f64().unwrap() + amount["per_level"].as_f64().unwrap() * l
}

#[test]
fn the_sheet_gives_ten_rows_and_an_ember_that_starts_stronger() {
    let c = sheet("pyromancer");
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
    assert!(ember_hit("pyromancer", 1.0) > ember_hit("initiate", 30.0));
}

/// The Ember every Pyromancer skill grows, copies and spreads, and the skills that do.
fn ember() -> u32 {
    spell("pyromancer", "aura_searing_ember")
}

/// Where the Pyromancer stands, west of the camp east of the Crossroads (mobs of 7 to 10, which
/// outlive a few casts), facing it, and where the first mob is put, 25 yd east.
const ME: [f32; 3] = [EAST_CAMP[0] - 40.0, EAST_CAMP[1], 0.0];
const A: [f32; 3] = [EAST_CAMP[0] - 15.0, EAST_CAMP[1], 0.0];

/// A Pyromancer of `level` west of the camp with `rows` set: the realm, their guid, the camp's
/// mobs (at least four) and the view.
fn world(level: u8, rows: &[(&str, u32)]) -> (Realm, u64, Vec<Mob>, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Cinder", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"pyromancer\"");
    let mut realm = Realm::with_seed(0x689);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let mut seen = view.drain(&mut realm);
    for (row, choice) in rows {
        realm.handle(RealmCommand::SetTalentRow {
            row: row.to_string(),
            spell_id: *choice,
        });
    }
    seen.extend(view.run(&mut realm, 1.0));
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: ME,
        orientation: 0.0,
    });
    seen.extend(view.run(&mut realm, 1.0));
    let mobs: Vec<Mob> = seen
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                ..
            } if flat(*position, EAST_CAMP) < 15.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .collect();
    assert!(
        mobs.len() >= 4,
        "four camp mobs in view, has {}",
        mobs.len()
    );
    (realm, guid, mobs, view)
}

/// Puts each of `mobs` at `A` plus its north offset in yards, in that order, at full health, and
/// the Pyromancer back at `ME`.
fn line_up(realm: &mut Realm, mobs: &[&Mob], offsets: &[f32]) {
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: ME,
        orientation: 0.0,
    });
    for (mob, dy) in mobs.iter().zip(offsets) {
        realm.place_creature(mob.guid, [A[0], A[1] + dy, 0.0]);
        realm.set_creature_health_pct(mob.guid, 100);
    }
}

/// Casts `spell_id` at `target` at full health and waits out its cast bar and the global
/// cooldown: the events.
fn go(realm: &mut Realm, view: &mut View, spell_id: u32, target: u64) -> Vec<SessionEvent> {
    realm.set_creature_health_pct(target, 100);
    let mut events = cast_through(realm, view, spell_id, target);
    events.extend(view.run(realm, 1.6));
    events
}

/// Whether `spell_id`'s direct hit landed on `target` in `events`.
fn hit(events: &[SessionEvent], spell_id: u32, target: u64) -> bool {
    events.iter().any(|e| {
        matches!(e, SessionEvent::SpellDamageLog(l)
            if l.spell_id == spell_id && l.target == target && !l.periodic && l.damage > 0)
    })
}

/// The Ember's stacks on `mob`, 0 for none.
fn stacks(view: &View, mob: u64) -> u8 {
    view.aura(mob, ember()).map_or(0, |a| a.stacks)
}

/// Casts Searing Ember at `mob` until it wears `want` stacks, lined up as `line` and `offsets`
/// say before each cast (a miss puts none on).
fn ember_up(
    realm: &mut Realm,
    view: &mut View,
    mob: u64,
    want: u8,
    line: &[&Mob],
    offsets: &[f32],
) {
    for _ in 0..12 {
        if stacks(view, mob) >= want {
            return;
        }
        line_up(realm, line, offsets);
        go(realm, view, spell("pyromancer", "searing_ember"), mob);
    }
    panic!("the Ember never reached {want} stacks");
}

#[test]
fn fireball_grows_the_ember_on_its_target() {
    let (mut realm, _, mobs, mut view) = world(10, &[]);
    let fireball = spell("pyromancer", "fireball");
    let mut landed = 0;
    for mob in mobs.iter().take(4) {
        ember_up(&mut realm, &mut view, mob.guid, 1, &[mob], &[0.0]);
        let before = stacks(&view, mob.guid);
        line_up(&mut realm, &[mob], &[0.0]);
        // Read as it lands, before the burn can finish the mob off.
        let events = cast_through(&mut realm, &mut view, fireball, mob.guid);
        if hit(&events, fireball, mob.guid) && view.health(mob.guid) > 0 {
            landed += 1;
            assert_eq!(stacks(&view, mob.guid), before + 1, "a stack more");
        }
        view.run(&mut realm, 1.6);
    }
    assert!(landed > 0, "no Fireball of four landed on a living mob");
}

#[test]
fn kindle_copies_a_three_stack_ember_to_the_two_nearest_within_10_yards() {
    let (mut realm, _, mobs, mut view) = world(10, &[]);
    let [a, b, c, d] = [&mobs[0], &mobs[1], &mobs[2], &mobs[3]];
    // b and c within 10 yd of a, d out of reach.
    let (line, offsets) = ([a, b, c, d], [0.0, 3.0, 6.0, 12.0]);
    ember_up(&mut realm, &mut view, a.guid, 3, &line, &offsets);
    line_up(&mut realm, &line, &offsets);
    realm.handle(cast(spell("pyromancer", "kindle"), Some(a.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, spell("pyromancer", "kindle")), None);
    assert_eq!(stacks(&view, b.guid), 3, "the nearest");
    assert_eq!(stacks(&view, c.guid), 3, "the next nearest");
    assert_eq!(stacks(&view, d.guid), 0, "12 yd is out of reach");
    assert_eq!(stacks(&view, a.guid), 3, "the original is untouched");
}

#[test]
fn flamestrike_burns_every_enemy_within_8_yards_of_its_point() {
    let (mut realm, _, mobs, mut view) = world(20, &[]);
    let [a, b, c, far] = [&mobs[0], &mobs[1], &mobs[2], &mobs[3]];
    let flamestrike = spell("pyromancer", "flamestrike");
    let (line, offsets) = ([a, b, c, far], [0.0, 3.0, -6.0, 14.0]);
    line_up(&mut realm, &line, &offsets);
    realm.handle(RealmCommand::CastSpellAtDest {
        spell_id: flamestrike,
        dest: A,
    });
    let mut events = view.drain(&mut realm);
    assert_eq!(refusal(&events, flamestrike), None, "{events:?}");
    for _ in 0..40 {
        if went_off(&events, flamestrike) {
            break;
        }
        events.extend(view.run(&mut realm, 0.1));
    }
    assert!(went_off(&events, flamestrike), "Flamestrike went off");
    events.extend(view.run(&mut realm, 0.2));
    let burned: Vec<u64> = [a, b, c]
        .iter()
        .filter(|m| hit(&events, flamestrike, m.guid))
        .map(|m| m.guid)
        .collect();
    assert!(
        burned.len() >= 2,
        "most of the three within 8 yd: {burned:?}"
    );
    for guid in burned {
        assert_eq!(stacks(&view, guid), 1, "each one hit takes an Ember");
    }
    assert!(!hit(&events, flamestrike, far.guid), "14 yd is outside");
    assert_eq!(stacks(&view, far.guid), 0);
}

/// Whether Fuel gave mana in `events`.
fn fuelled(events: &[SessionEvent], fuel: u32) -> bool {
    events.iter().any(
        |e| matches!(e, SessionEvent::SpellEnergizeLog(l) if l.spell_id == fuel && l.amount > 0),
    )
}

/// Fireballs `mob`, left at 1% health, until it dies: the events.
fn finish(realm: &mut Realm, view: &mut View, mob: &Mob) -> Vec<SessionEvent> {
    let fireball = spell("pyromancer", "fireball");
    let mut events = Vec::new();
    for _ in 0..6 {
        line_up(realm, &[mob], &[0.0]);
        realm.set_creature_health_pct(mob.guid, 1);
        realm.handle(cast(fireball, Some(mob.guid)));
        events.extend(view.drain(realm));
        events.extend(view.run(realm, 4.0));
        if view.health(mob.guid) == 0 {
            return events;
        }
    }
    panic!("the mob never died");
}

#[test]
fn fuel_gives_mana_for_a_burning_kill_only() {
    let fuel = choice("pyromancer", "pyromancer_inferno", "fuel");
    let (mut realm, _, mobs, mut view) = world(50, &[("pyromancer_inferno", fuel)]);
    let [a, b] = [&mobs[0], &mobs[1]];
    let cold = finish(&mut realm, &mut view, b);
    assert!(!fuelled(&cold, fuel), "no Ember, no mana");
    ember_up(&mut realm, &mut view, a.guid, 1, &[a], &[0.0]);
    let burning = finish(&mut realm, &mut view, a);
    assert!(fuelled(&burning, fuel), "a burning kill feeds the fire");
}
