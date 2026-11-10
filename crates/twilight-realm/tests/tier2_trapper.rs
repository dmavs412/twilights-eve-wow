//! The Trapper (#691), the Ranger Line's mobile Tier 2 Calling: the Calling Choice offers it at the
//! Ranger's cap beside the Sharpshooter, Hale Brannock waits unmarked on the tree line south of the
//! Eastvale Logging Camp until he is met, Thalyssa Windbough points the way, his four-quest Trial
//! gates the Advancement, and the sheet's traps stand side by side and spring on a group. Trap Vault
//! lays its trap with no target, and Trap Mastery's percent spellmod shortens the realm's cooldowns.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::messages::GossipOption;
use benilla_protocol::{EntityKind, SessionEvent, START_ELWYNN};
use common::acolyte::{dummy, YardGround, DUMMY_SPOTS};
use common::cutthroat::set_rows;
use common::fight::*;
use common::*;
use gear_common::*;
use quest_common::*;

/// Hale Brannock and Thalyssa Windbough (the Northshire Ranger Trainer).
const HALE: u32 = 190117;
const THALYSSA: u32 = 190053;
/// Elwynn Forest's zone.
const ELWYNN: u32 = 12;

/// The Sharpshooter's Advancement Quest, offered beside this one.
const STILLNESS: u32 = 64405;
/// The Advancement Quest and its Trial, in order.
const LINE: u32 = 64406;
const RUN_THEM_DOWN: u32 = 64446;
const SILK: u32 = 64447;
const GORETUSKS: u32 = 64448;
const BELLYGRUB: u32 = 64449;

/// Large Ogre Chain Armor, a mail chest.
const OGRE_CHAIN: u32 = 914;
const BACKPACK: u8 = 255;
const PACK_1: u8 = 23;

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

/// A Ranger of 30 who picked at the Tier 0 Calling Choice, entered in the world.
fn capped_ranger() -> Vec<SessionEvent> {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Pilgrim", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    assert_eq!(create_result(&mut maker), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("calling = \"basic_warrior\"", "calling = \"ranger\"")
        .replacen("\nlevel = 1\n", "\nlevel = 30\nchosen_tiers = [0]\n", 1);
    let mut realm = Realm::with_seed(SEED);
    realm
        .import_character(&blob)
        .unwrap_or_else(|e| panic!("{e:?}: {blob}"));
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    drain(&mut realm)
}

/// A level 30 Ranger's save standing beside Hale on the tree line, with `quests` under way and
/// `done` handed in, entered in the world.
fn on_the_tree_line(quests: &[Underway], done: &[u32]) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(30, &[], 0, quests);
    let [x, y, z] = hub_home(hub_guid(HALE));
    let (head, tail) = blob.split_once("\n[").unwrap_or((&blob, ""));
    let head: Vec<String> = head
        .lines()
        .map(|line| {
            if line.starts_with("map = ") {
                "map = 0".to_string()
            } else if line.starts_with("zone = ") {
                format!("zone = {ELWYNN}")
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
        .replace("calling = \"basic_warrior\"", "calling = \"ranger\"");
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
fn a_level_30_ranger_is_offered_the_sharpshooter_and_the_trapper() {
    let offers = offered(&capped_ranger());
    assert_eq!(offers.len(), 1, "{offers:?}");
    assert!(offers[0].contains(&STILLNESS), "{offers:?}");
    assert!(offers[0].contains(&LINE), "{offers:?}");
}

#[test]
fn hale_is_unmarked_until_met_and_thalyssa_gives_the_hint() {
    let (mut realm, _, events) = on_the_tree_line(&[(LINE, [0; 4])], &[]);
    let hale = hub_guid(HALE);
    assert!(!marked(&events, hale), "not met yet");
    walk_up_to(&mut realm, hale);
    menu(&mut realm, hale);
    realm.tick(0.0, &FlatGround);
    assert!(marked(&drain(&mut realm), hale), "met at his gossip menu");

    let thalyssa = hub_guid(THALYSSA);
    walk_up_to(&mut realm, thalyssa);
    let lines = menu(&mut realm, thalyssa);
    assert_eq!(
        hint(
            &mut realm,
            thalyssa,
            &lines,
            "Where does the Trapper set his snares?"
        ),
        "Hale Brannock works the tree line south of the Eastvale logging camp. Follow the cut \
         wire, and step where he stepped."
    );
}

#[test]
fn hale_sets_the_trial_in_order_and_its_end_makes_a_level_1_trapper() {
    let hale = hub_guid(HALE);
    let (mut realm, _, _) = on_the_tree_line(&[], &[]);
    assert!(
        !on_offer(&mut realm, hale).contains(&RUN_THEM_DOWN),
        "64406 not taken"
    );

    let (mut realm, _, _) = on_the_tree_line(&[(LINE, [0; 4])], &[]);
    assert_eq!(on_offer(&mut realm, hale), [RUN_THEM_DOWN]);
    assert!(completed(&hand_in(&mut realm, hale, LINE)).is_none());

    let (mut realm, _, _) =
        on_the_tree_line(&[(LINE, [0; 4]), (RUN_THEM_DOWN, [10, 0, 0, 0])], &[]);
    assert!(completed(&hand_in(&mut realm, hale, RUN_THEM_DOWN)).is_some());
    assert_eq!(on_offer(&mut realm, hale), [SILK]);

    let (mut realm, _, _) = on_the_tree_line(
        &[(LINE, [0; 4]), (GORETUSKS, [8, 0, 0, 0])],
        &[RUN_THEM_DOWN, SILK],
    );
    assert!(completed(&hand_in(&mut realm, hale, GORETUSKS)).is_some());
    assert_eq!(on_offer(&mut realm, hale), [BELLYGRUB]);
    assert!(completed(&hand_in(&mut realm, hale, LINE)).is_none());

    let (mut realm, me, entered) = on_the_tree_line(
        &[(LINE, [0; 4]), (BELLYGRUB, [1, 0, 0, 0])],
        &[RUN_THEM_DOWN, SILK, GORETUSKS],
    );
    assert!(completed(&hand_in(&mut realm, hale, LINE)).is_none());
    assert_eq!(realm.current_calling().unwrap().key, "ranger");
    assert!(completed(&hand_in(&mut realm, hale, BELLYGRUB)).is_some());
    let advanced = hand_in(&mut realm, hale, LINE);
    assert!(completed(&advanced).is_some(), "{advanced:?}");
    assert_eq!(realm.current_calling().unwrap().key, "trapper");
    let saved = realm.export_character(me).unwrap();
    assert!(saved.contains("\nlevel = 1\n"), "{saved}");

    let mut events = entered;
    events.extend(advanced);
    let known = spellbook(&events);
    for key in [
        "auto_shot",
        "snapshot",
        "immolation_trap",
        "vault",
        "frost_trap",
        "silencing_shot",
    ] {
        assert!(known.contains(&spell("trapper", key)), "{key}: {known:?}");
    }
    for key in ["caltrops", "spring_the_traps", "tripwire"] {
        assert!(!known.contains(&spell("trapper", key)), "{key} at 1");
    }

    // At level 10 the login's spell book has Caltrops.
    let blob = saved.replacen("\nlevel = 1\n", "\nlevel = 10\n", 1);
    let mut later = Realm::with_seed(SEED);
    later.import_character(&blob).unwrap();
    later.handle(RealmCommand::EnterWorld { guid: me });
    later.tick(0.0, &FlatGround);
    let known = spellbook(&drain(&mut later));
    assert!(known.contains(&spell("trapper", "caltrops")), "{known:?}");
    assert!(!known.contains(&spell("trapper", "spring_the_traps")));
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

/// The bonus of `calling`'s Snapshot at level `l`, read from its formula.
fn snapshot_bonus(calling: &str, l: f64) -> f64 {
    let c = sheet(calling);
    let shot = c["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["key"] == "snapshot")
        .expect("a Snapshot");
    let bonus = &shot["effects"][0]["params"]["bonus"];
    bonus["base"].as_f64().unwrap() + bonus["per_level"].as_f64().unwrap() * l
}

#[test]
fn the_sheet_gives_ten_rows_and_a_snapshot_that_starts_stronger() {
    let c = sheet("trapper");
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
    assert!(snapshot_bonus("trapper", 1.0) > snapshot_bonus("ranger", 30.0));
}

#[test]
fn the_trapper_cannot_wear_mail_and_the_ranger_can() {
    for (calling, refused) in [("trapper", true), ("ranger", false)] {
        let (guid, blob) = save_with(30, 0, &[(PACK_1, OGRE_CHAIN, 1)]);
        let blob = blob.replace(
            "calling = \"basic_warrior\"",
            &format!("calling = \"{calling}\""),
        );
        let (mut realm, _, _) = enter_saved(guid, &blob);
        realm.handle(RealmCommand::AutoEquipItem {
            bag: BACKPACK,
            slot: PACK_1,
        });
        let reason = drain(&mut realm).into_iter().find_map(|e| match e {
            SessionEvent::InventoryFailure { reason, .. } => Some(reason),
            _ => None,
        });
        // `EQUIP_ERR_NO_REQUIRED_PROFICIENCY`.
        assert_eq!(reason, refused.then_some(8), "{calling}");
    }
}

/// A level 30 Trapper in the Test Grounds with each `(row, choice)` set, standing at `at`.
struct Yard {
    realm: Realm,
    view: View,
}

impl Yard {
    fn new(rows: &[(&str, u32)], at: [f32; 2]) -> Self {
        let mut maker = realm();
        let RealmCommand::CreateCharacter(mut req) = create_cmd("Brannock", HUMAN, 0, WARRIOR)
        else {
            unreachable!()
        };
        req.start = START_ELWYNN;
        maker.handle(RealmCommand::CreateCharacter(req));
        let me = roster(&mut maker)[0].guid;
        let blob = maker
            .export_character(me)
            .unwrap()
            .replace("level = 1\n", "level = 30\n")
            .replace("calling = \"basic_warrior\"", "calling = \"trapper\"");
        let mut realm = Realm::with_seed(0x691);
        realm.import_character(&blob).unwrap();
        realm.handle(RealmCommand::EnterWorld { guid: me });
        realm.tick(0.0, &YardGround);
        let mut view = View::default();
        view.drain(&mut realm);
        set_rows(&mut realm, &mut view, rows);
        let mut y = Yard { realm, view };
        for (xy, facing) in [([-9720.0, 255.0], None), (at, Some(0.0))] {
            y.realm.handle(RealmCommand::DevTeleport {
                x: xy[0],
                y: xy[1],
                facing,
            });
            let landed = [y.view.drain(&mut y.realm), y.run(1.0)].concat();
            // The client acknowledges the teleport, or the realm ignores its steps.
            if let Some(counter) = landed.iter().find_map(|e| match e {
                SessionEvent::Teleport { counter, .. } => Some(*counter),
                _ => None,
            }) {
                y.realm.handle(RealmCommand::TeleportAck { counter });
            }
        }
        y
    }

    /// Ticks the Test Grounds for `secs`, 0.1 s at a time: every event.
    fn run(&mut self, secs: f32) -> Vec<SessionEvent> {
        let mut events = Vec::new();
        for _ in 0..(secs / 0.1).round().max(1.0) as u32 {
            self.realm.tick(0.1, &YardGround);
            events.extend(self.view.drain(&mut self.realm));
        }
        events
    }

    /// Casts the self skill `spell_id`: what the client saw, and whether it was refused.
    fn lay(&mut self, spell_id: u32) -> (Vec<SessionEvent>, Option<u8>) {
        self.realm.handle(cast(spell_id, None));
        let events = self.view.drain(&mut self.realm);
        let refused = refusal(&events, spell_id);
        (events, refused)
    }

    /// Walks to `xy`, facing the dummies.
    fn walk(&mut self, xy: [f32; 2]) {
        self.realm.handle(RealmCommand::Move {
            motion: Default::default(),
            position: [xy[0], xy[1], 47.2],
            orientation: 0.0,
        });
        self.view.drain(&mut self.realm);
    }

    /// Which of the four dummies wear `spell_id`.
    fn wearing(&self, spell_id: u32) -> [bool; 4] {
        [0, 1, 2, 3].map(|i| self.view.aura(dummy(i), spell_id).is_some())
    }
}

/// The DynamicObjects of `spell_id` that `events` create.
fn trap_objects(events: &[SessionEvent], spell_id: u32) -> Vec<u64> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::DynamicObject,
                fields,
                ..
            } if fields.dynamicobject_spell_id() == Some(spell_id) => Some(*guid),
            _ => None,
        })
        .collect()
}

fn destroyed(events: &[SessionEvent], object: u64) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::ObjectDestroyed(g) if *g == object))
}

/// A Frost Trap laid west of the dummies stands while an Immolation Trap goes down half a yard from
/// dummy 1, toward dummy 0. Once armed, dummy 1 steps in and springs it: dummy 1 (0.5 yd) and dummy
/// 0 (5.5 yd) burn; dummy 2 (6.5 yd) and dummy 3 (11.5 yd) are outside its 6 yd. The Frost Trap,
/// which no one stepped in, still stands.
#[test]
fn immolation_and_frost_traps_stand_together_and_immolation_springs_on_every_mob_within_6_yd() {
    let immolation = spell("trapper", "immolation_trap");
    let frost = spell("trapper", "frost_trap");
    let burn = spell("trapper", "aura_immolation_trap");
    let [_, one, ..] = DUMMY_SPOTS;
    let mut y = Yard::new(&[], [-9665.0, 255.0]);
    let (laid, refused) = y.lay(frost);
    assert_eq!(refused, None, "the Frost Trap is laid");
    let [frost_object] = trap_objects(&laid, frost)[..] else {
        panic!("one Frost Trap");
    };
    y.run(1.6);
    y.walk([one[0], one[1] + 0.5]);
    let (laid, refused) = y.lay(immolation);
    assert_eq!(refused, None, "the Immolation Trap is laid");
    let [immolation_object] = trap_objects(&laid, immolation)[..] else {
        panic!("one Immolation Trap");
    };
    assert!(!destroyed(&laid, frost_object), "both traps stand");
    let early = y.run(0.5);
    assert_eq!(y.wearing(burn), [false; 4], "nothing while it arms");
    assert!(!destroyed(&early, immolation_object), "and it stands");
    let late = y.run(1.0);
    assert_eq!(y.wearing(burn), [true, true, false, false]);
    assert!(destroyed(&late, immolation_object), "the trap is spent");
    assert!(
        !destroyed(&[early, late].concat(), frost_object),
        "the Frost Trap still stands"
    );
}

/// Trap Vault's leap lays an Immolation Trap with no target selected: a self skill's cast_skill
/// needs none.
#[test]
fn trap_vault_lays_an_immolation_trap_with_no_target() {
    let trap_vault = choice("trapper", "vault", "trap_vault");
    let immolation = spell("trapper", "immolation_trap");
    let mut y = Yard::new(&[("vault", trap_vault)], [-9665.0, 255.0]);
    let (mut events, refused) = y.lay(trap_vault);
    assert_eq!(refused, None, "Trap Vault goes");
    events.extend(y.run(1.5));
    assert_eq!(trap_objects(&events, immolation).len(), 1, "a trap is laid");
}

/// Trap Mastery's pct COOLDOWN -20 makes Immolation Trap's 15 s cooldown 12 s in the realm: ready
/// again at 12.5 s with it, still cooling without.
#[test]
fn trap_mastery_shortens_immolation_traps_cooldown_by_a_fifth() {
    let mastery = choice("trapper", "trapper_trapcraft", "trap_mastery");
    let immolation = spell("trapper", "immolation_trap");
    for (rows, ready) in [
        (vec![], false),
        (vec![("trapper_trapcraft", mastery)], true),
    ] {
        let mut y = Yard::new(&rows, [-9665.0, 255.0]);
        assert_eq!(y.lay(immolation).1, None, "the first trap is laid");
        y.run(12.5);
        assert_eq!(y.lay(immolation).1.is_none(), ready, "{rows:?}");
    }
}
