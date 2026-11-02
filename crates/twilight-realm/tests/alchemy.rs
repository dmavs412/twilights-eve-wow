//! Alchemy tables in the realm (#542, ADR 0025): the three alchemists open them, makes resolve and
//! roll their Toxic, Herb Notes fill, grades are learned or bought as Rumours, sludge is sold and
//! every make goes in the Brew Log.

mod common;
mod gear_common;

use benilla_protocol::messages::PeriodicTick;
use benilla_protocol::{AlchemyGrade, AlchemyGradePage, SessionEvent, START_ELWYNN};
use common::*;
use gear_common::*;
use twilight_realm::{AlchemyAction, Audience, Realm, RealmCommand};

const SISTER_MARLA: u32 = 190_075;
const ALWEN_CARROW: u32 = 190_076;
const TABLE_LINE: &str = "Let me use your alchemy table.";

const PEACEBLOOM: u32 = 2447;
const BRIARTHORN: u32 = 2450;
const SLUDGE: u32 = 963_000;
const MINOR_HEALING: u32 = 963_100;
/// Restorative 10 on the Tincture ladder: Minor x2, clean (963500 + 4 x 8).
const MINOR_RESTORATIVE_TINCTURE: u32 = 963_532;
const PLAIN_FORTIFYING_TINCTURE_TOXIC_7: u32 = 963_747;

/// A new Elwynn character's save, its guid.
fn new_save() -> (u64, String) {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Mara", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    assert_eq!(create_result(&mut maker), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    (guid, maker.export_character(guid).unwrap())
}

/// `blob` with its `position` line set to `at`.
fn placed(blob: &str, at: [f32; 3]) -> String {
    blob.lines()
        .map(|l| {
            if l.starts_with("position = ") {
                format!("position = [{:.3}, {:.3}, {:.3}]", at[0], at[1], at[2])
            } else {
                l.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A character beside the alchemist `npc`, holding `satchel` in the Satchel, with `top` (top-level
/// save keys) and `money` copper, in a realm seeded `seed`.
fn beside(npc: u32, satchel: &[(u32, u32)], top: &str, money: u32, seed: u64) -> (Realm, u64) {
    let (guid, blob) = new_save();
    let [x, y, z] = hub_home(hub_guid(npc));
    let blob = placed(&blob, [x + 1.5, y, z]);
    let mut blob = blob
        .lines()
        .map(|l| {
            if l.starts_with("money = ") {
                format!("money = {money}")
            } else {
                l.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    blob = format!("{top}\n{blob}");
    for (entry, count) in satchel {
        blob.push_str(&format!(
            "\n[[satchel]]\nentry = {entry}\ncount = {count}\n"
        ));
    }
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    drain(&mut realm);
    (realm, guid)
}

/// Gossip at `npc`: its menu's lines.
fn hello(realm: &mut Realm, npc: u32) -> Vec<(u32, String)> {
    realm.handle(RealmCommand::GossipHello {
        guid: hub_guid(npc),
    });
    drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { options, .. } => {
                Some(options.into_iter().map(|o| (o.index, o.message)).collect())
            }
            _ => None,
        })
        .expect("the alchemist has a menu")
}

/// Opens `npc`'s table: its events.
fn open(realm: &mut Realm, npc: u32) -> Vec<SessionEvent> {
    let (option, _) = hello(realm, npc)
        .into_iter()
        .find(|(_, m)| m == TABLE_LINE)
        .expect("the table line");
    realm.handle(RealmCommand::GossipSelectOption {
        guid: hub_guid(npc),
        option,
    });
    drain(realm)
}

fn act(realm: &mut Realm, action: AlchemyAction) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::Alchemy(action));
    drain(realm)
}

fn make(realm: &mut Realm, make: &str, items: Vec<u32>, target: &str) -> Vec<SessionEvent> {
    act(
        realm,
        AlchemyAction::Make {
            make: make.to_owned(),
            items,
            target: target.to_owned(),
        },
    )
}

/// The chat lines in `events`.
fn lines(events: &[SessionEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Chat(m) => Some(m.text.clone()),
            _ => None,
        })
        .collect()
}

/// The saved character.
fn saved(realm: &Realm, guid: u64) -> toml::Table {
    realm.export_character(guid).unwrap().parse().unwrap()
}

/// How many of `entry` the saved character holds in the Satchel, and in the bags.
fn held(realm: &Realm, guid: u64, entry: u32) -> (i64, i64) {
    let save = saved(realm, guid);
    let count = |key: &str| {
        save.get(key)
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter(|r| r.get("entry").and_then(|e| e.as_integer()) == Some(entry.into()))
            .map(|r| r.get("count").and_then(|c| c.as_integer()).unwrap_or(1))
            .sum::<i64>()
    };
    (count("satchel"), count("items"))
}

/// The last bench's targets in `events`.
fn targets(events: &[SessionEvent]) -> Vec<AlchemyGrade> {
    events
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::AlchemyBench { targets, .. } => Some(targets.clone()),
            _ => None,
        })
        .expect("an AlchemyBench")
}

#[test]
fn sister_marla_opens_a_bench_with_only_the_potion_open_and_says_her_line_once() {
    let (mut realm, _) = beside(SISTER_MARLA, &[], "", 0, 1);
    let events = open(&mut realm, SISTER_MARLA);
    let open_makes: Vec<(String, bool)> = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::AlchemyBench { makes, .. } => {
                Some(makes.iter().map(|m| (m.key.clone(), m.open)).collect())
            }
            _ => None,
        })
        .expect("an AlchemyBench");
    assert_eq!(open_makes.len(), 5);
    assert!(
        open_makes.iter().all(|(k, o)| *o == (k == "potion")),
        "{open_makes:?}"
    );
    let said = |events: &[SessionEvent]| {
        events
            .iter()
            .filter(|e| matches!(e, SessionEvent::Chat(m) if m.text.contains("Two to four herbs in the Mortar")))
            .count()
    };
    assert_eq!(said(&events), 1);
    assert_eq!(said(&open(&mut realm, SISTER_MARLA)), 0);
}

#[test]
fn two_peacebloom_brew_a_minor_healing_potion_and_open_the_still() {
    let (mut realm, guid) = beside(SISTER_MARLA, &[(PEACEBLOOM, 4)], "", 0, 1);
    open(&mut realm, SISTER_MARLA);
    let events = make(&mut realm, "potion", vec![PEACEBLOOM; 2], "");
    assert_eq!(held(&realm, guid, MINOR_HEALING), (0, 1));
    let save = saved(&realm, guid);
    let notes = save["alchemy_notes"].as_array().unwrap();
    assert_eq!(notes[0].as_array().unwrap()[1].as_integer(), Some(2));
    assert!(save["alchemy_makes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m.as_str() == Some("tincture")));
    let grades = save["alchemy_grades"].as_array().unwrap();
    assert!(grades.iter().any(|g| g["family"].as_str() == Some("potion")
        && g["key"].as_str() == Some("Restorative")
        && g["grade"].as_integer() == Some(0)
        && g["learned"].as_bool() == Some(true)));
    let (pages, log): (Vec<AlchemyGradePage>, usize) = events
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::AlchemyJournal { grades, log, .. } => Some((grades.clone(), log.len())),
            _ => None,
        })
        .expect("an AlchemyJournal");
    let page = pages
        .iter()
        .find(|p| p.family == "potion" && p.key == "Restorative" && p.grade == 0)
        .unwrap();
    assert!(page.learned);
    assert_eq!(page.threshold, 4);
    assert_eq!(log, 1);
    assert!(lines(&events)
        .iter()
        .any(|l| l == "Tinctures are open: 5 herbs at the Still."));
}

#[test]
fn a_tincture_goes_to_the_alchemy_satchel() {
    let top = "alchemy_makes = [\"tincture\"]";
    let (mut realm, guid) = beside(SISTER_MARLA, &[(PEACEBLOOM, 5)], top, 0, 1);
    open(&mut realm, SISTER_MARLA);
    make(&mut realm, "tincture", vec![PEACEBLOOM; 5], "");
    assert_eq!(held(&realm, guid, MINOR_RESTORATIVE_TINCTURE), (1, 0));
}

#[test]
fn briarthorn_at_the_still_sometimes_spoils_and_sometimes_carries_toxic_7() {
    let top = "alchemy_makes = [\"tincture\"]";
    let (mut sludge, mut tincture) = (0, 0);
    for seed in 1..=10 {
        let (mut realm, guid) = beside(SISTER_MARLA, &[(BRIARTHORN, 5)], top, 0, seed);
        open(&mut realm, SISTER_MARLA);
        make(&mut realm, "tincture", vec![BRIARTHORN; 5], "");
        sludge += held(&realm, guid, SLUDGE).0;
        tincture += held(&realm, guid, PLAIN_FORTIFYING_TINCTURE_TOXIC_7).0;
    }
    assert!(sludge >= 1, "no sludge in ten seeds");
    assert!(tincture >= 1, "no Tincture carrying Toxic 7 in ten seeds");
}

#[test]
fn rumours_are_sold_by_the_tables_whose_tiers_hold_them() {
    let buy = |realm: &mut Realm| {
        act(
            realm,
            AlchemyAction::BuyRumour {
                family: "potion".to_owned(),
                key: "Restorative".to_owned(),
                grade: 1,
            },
        )
    };
    let (mut realm, guid) = beside(SISTER_MARLA, &[], "", 10_000, 1);
    open(&mut realm, SISTER_MARLA);
    buy(&mut realm);
    assert_eq!(saved(&realm, guid)["money"].as_integer(), Some(10_000));
    let (mut realm, guid) = beside(ALWEN_CARROW, &[], "", 10_000, 1);
    open(&mut realm, ALWEN_CARROW);
    let events = buy(&mut realm);
    assert_eq!(saved(&realm, guid)["money"].as_integer(), Some(4_000));
    assert!(lines(&events)
        .iter()
        .any(|l| l == "Rumour: Lesser Healing Potion needs Restorative 12."));
    let t = targets(&events)
        .into_iter()
        .find(|t| t.family == "potion" && t.key == "Restorative" && t.grade == 1)
        .unwrap();
    assert!(t.rumoured && !t.learned);
    assert_eq!(t.threshold, 12);
}

#[test]
fn three_murky_sludge_sell_for_15_copper() {
    let (mut realm, guid) = beside(SISTER_MARLA, &[(SLUDGE, 3)], "", 0, 1);
    open(&mut realm, SISTER_MARLA);
    let events = act(&mut realm, AlchemyAction::SellSludge);
    assert_eq!(saved(&realm, guid)["money"].as_integer(), Some(15));
    assert_eq!(held(&realm, guid, SLUDGE), (0, 0));
    assert!(lines(&events)
        .iter()
        .any(|l| l == "Sold 3 Murky Sludge for 15c."));
}

#[test]
fn the_alchemy_fields_survive_a_save_round_trip() {
    let (mut realm, guid) = beside(SISTER_MARLA, &[(PEACEBLOOM, 2)], "", 0, 1);
    open(&mut realm, SISTER_MARLA);
    make(&mut realm, "potion", vec![PEACEBLOOM; 2], "");
    let blob = realm.export_character(guid).unwrap();
    let mut again = Realm::with_seed(1);
    again.import_character(&blob).unwrap();
    let (before, after) = (saved(&realm, guid), saved(&again, guid));
    for key in [
        "alchemy_notes",
        "alchemy_grades",
        "alchemy_makes",
        "alchemy_tables",
        "alchemy_last",
        "brew_log",
    ] {
        assert!(before.contains_key(key), "{key} is saved");
        assert_eq!(before.get(key), after.get(key), "{key}");
    }
}

// Using alchemy (#543): potions share a cooldown, oils coat a hand and proc, flasks give two stats
// through death.

const MINOR_HEALING_X125: u32 = 963_101;
const MINOR_MANA: u32 = 963_160;
const MINOR_IRONHIDE: u32 = 963_120;
const LESSER_IRONHIDE: u32 = 963_125;
const MINOR_BLASTING: u32 = 963_140;
const MINOR_BURNING_OIL: u32 = 963_240;
/// A Lesser Fortifying x1 and a Minor Volatile x1 Essence: Flask of Stone and Ember.
const STONE_AND_EMBER: u32 = 968_720;
const POTION_SPELL: u32 = 2598;
const NOT_READY: u8 = 0x3c;
const STUNNED: u8 = 0x64;
const BAD_TARGETS: u8 = 0x0a;
const FIRE: u8 = 2;
const WALL_MS: u64 = 1_000_000_000;

/// A warrior of `level` holding `items`, the wall clock at [`WALL_MS`].
fn holding(level: u8, items: &[Carried]) -> (Realm, u64, Vec<SessionEvent>) {
    let (mut realm, me, events) = warrior_with(level, 0, items);
    realm.set_wall_clock(WALL_MS);
    (realm, me, events)
}

fn use_at(realm: &mut Realm, slot: u8) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::UseItem { bag: 255, slot });
    drain(realm)
}

/// The cast refusals in `events`, by reason.
fn refusals(events: &[SessionEvent]) -> Vec<u8> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::CastResult {
                success: false,
                reason,
                ..
            } => *reason,
            _ => None,
        })
        .collect()
}

#[test]
fn a_healing_potion_heals_and_starts_the_cooldown_every_potion_shares() {
    let (mut realm, me, _) = holding(
        20,
        &[
            (BACKPACK_1, MINOR_HEALING_X125, 1),
            (BACKPACK_1 + 1, MINOR_MANA, 1),
        ],
    );
    realm.set_player_health(50);
    let events = use_at(&mut realm, BACKPACK_1);
    let healed: Vec<u32> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellHealLog(l) if l.target == me && l.spell_id == POTION_SPELL => {
                Some(l.amount)
            }
            _ => None,
        })
        .collect();
    assert_eq!(healed.len(), 1, "{healed:?}");
    assert!((88..=113).contains(&healed[0]), "{healed:?}");
    let ready = saved(&realm, me)["potion_ready"].as_float().unwrap();
    assert_eq!(ready, WALL_MS as f64 / 1000.0 + 120.0);
    let events = use_at(&mut realm, BACKPACK_1 + 1);
    assert_eq!(refusals(&events), vec![NOT_READY]);
    assert_eq!(held(&realm, me, MINOR_MANA), (0, 1));
}

#[test]
fn an_ironhide_potion_shields_and_a_smaller_one_leaves_the_larger_standing() {
    let (mut realm, me, mut seen) = holding(
        20,
        &[
            (BACKPACK_1, LESSER_IRONHIDE, 1),
            (BACKPACK_1 + 1, MINOR_IRONHIDE, 1),
        ],
    );
    seen.extend(use_at(&mut realm, BACKPACK_1));
    assert!(
        descriptor(&seen, me)
            .unit_aura_ids()
            .any(|id| id == POTION_SPELL),
        "the shield's aura is up"
    );
    // Past the shared cooldown, though no time passes for the shield.
    realm.set_wall_clock(WALL_MS + 121_000);
    let events = use_at(&mut realm, BACKPACK_1 + 1);
    assert!(lines(&events)
        .iter()
        .any(|l| l == "Ironhide Potion: a 60 shield, but your 150 shield is larger and stays."));
    assert_eq!(held(&realm, me, MINOR_IRONHIDE), (0, 0), "used either way");
}

#[test]
fn a_blasting_potion_needs_a_hostile_target_and_burns_it() {
    let (mut realm, me, events) = holding(10, &[(BACKPACK_1, MINOR_BLASTING, 2)]);
    let refused = use_at(&mut realm, BACKPACK_1);
    assert_eq!(refusals(&refused), vec![BAD_TARGETS]);
    assert_eq!(held(&realm, me, MINOR_BLASTING), (0, 2));
    let mob = the_mob(&events);
    stand_by(&mut realm, &mob);
    realm.handle(RealmCommand::SetSelection { guid: mob.guid });
    let thrown = use_at(&mut realm, BACKPACK_1);
    let burns: Vec<(u32, u8)> = thrown
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l)
                if l.attacker == me && l.target == mob.guid && l.spell_id == POTION_SPELL =>
            {
                Some((l.damage, l.school))
            }
            _ => None,
        })
        .collect();
    assert_eq!(burns.len(), 1, "{burns:?}");
    assert!((30..=45).contains(&burns[0].0) && burns[0].1 == FIRE);
    let fight = run(&mut realm, 5.0);
    assert!(
        fight
            .iter()
            .any(|e| matches!(e, SessionEvent::AttackerState(s)
            if s.attacker == mob.guid && s.victim == me)),
        "the mob attacks"
    );
}

#[test]
fn an_oil_coats_the_main_hand_then_the_off_hand_weapon_else_the_main_again() {
    let oil = |realm: &Realm, me: u64, key: &str| {
        saved(realm, me)
            .get(key)
            .map(|o| (o["item"].as_integer(), o["left_secs"].as_float()))
    };
    let coated = Some((Some(i64::from(MINOR_BURNING_OIL)), Some(1800.0)));
    let (mut realm, me, _) = holding(10, &[(BACKPACK_1, MINOR_BURNING_OIL, 2)]);
    let events = use_at(&mut realm, BACKPACK_1);
    assert!(lines(&events)
        .iter()
        .any(|l| l == "Burning Oil on your main hand for 30 min."));
    use_at(&mut realm, BACKPACK_1);
    assert_eq!(oil(&realm, me, "main_hand_oil"), coated);
    assert_eq!(oil(&realm, me, "off_hand_oil"), None, "no off-hand weapon");

    let (mut realm, me, _) = holding(
        10,
        &[
            (BACKPACK_1, MINOR_BURNING_OIL, 2),
            (MAIN_HAND_SLOT, THUGS_CUTLASS, 1),
            (MAIN_HAND_SLOT + 1, THUGS_CUTLASS, 1),
        ],
    );
    use_at(&mut realm, BACKPACK_1);
    use_at(&mut realm, BACKPACK_1);
    assert_eq!(oil(&realm, me, "main_hand_oil"), coated);
    assert_eq!(oil(&realm, me, "off_hand_oil"), coated);
}

#[test]
fn a_flask_adds_its_two_stats_survives_death_and_wears_off() {
    let (mut realm, me, mut seen) = holding(10, &[(BACKPACK_1, STONE_AND_EMBER, 2)]);
    let stats = |seen: &[SessionEvent]| {
        let f = descriptor(seen, me);
        (
            f.unit_resistance(0).unwrap(),
            f.unit_attack_power().unwrap(),
        )
    };
    let (armour, ap) = stats(&seen);
    seen.extend(use_at(&mut realm, BACKPACK_1));
    assert_eq!(stats(&seen), (armour + 100, ap + 10));
    let events = use_at(&mut realm, BACKPACK_1);
    assert!(lines(&events)
        .iter()
        .any(|l| l == "Flask of Stone and Ember is replaced; only one flask is active."));
    realm.set_player_health(0);
    let died = run(&mut realm, 0.2);
    assert_eq!(last_health(&died, me), Some(0));
    assert!(saved(&realm, me).contains_key("flask"), "it survives death");
    realm.tick(7200.0, &FlatGround);
    assert!(lines(&drain(&mut realm))
        .iter()
        .any(|l| l == "Flask of Stone and Ember has worn off."));
    assert!(!saved(&realm, me).contains_key("flask"));
}

#[test]
fn the_flask_the_oils_and_the_potion_cooldown_survive_a_save_round_trip() {
    let (mut realm, me, _) = holding(
        10,
        &[
            (BACKPACK_1, MINOR_HEALING_X125, 1),
            (BACKPACK_1 + 1, MINOR_BURNING_OIL, 1),
            (BACKPACK_1 + 2, STONE_AND_EMBER, 1),
        ],
    );
    for slot in BACKPACK_1..BACKPACK_1 + 3 {
        use_at(&mut realm, slot);
    }
    let blob = realm.export_character(me).unwrap();
    let mut again = Realm::with_seed(1);
    again.import_character(&blob).unwrap();
    let (before, after) = (saved(&realm, me), saved(&again, me));
    for key in ["potion_ready", "flask", "main_hand_oil"] {
        assert!(before.contains_key(key), "{key} is saved");
        assert_eq!(before.get(key), after.get(key), "{key}");
    }
}

// Hybrid potions (#544), each at Minor x1: heal over time, sleep, invisibility, burst and mana
// shields, area bombs and the rest.

const TROLLBLOOD: u32 = 963_300;
const BLOODFIRE: u32 = 963_320;
const REJUVENATING: u32 = 963_340;
const DEEPSLEEP: u32 = 963_360;
const EMBER_SHIELD: u32 = 963_380;
const MANA_SHIELD: u32 = 963_400;
const STEADY_STANCE: u32 = 963_420;
const SPELLFIRE: u32 = 963_440;
const SMOKE_BOMB: u32 = 963_460;
const INVISIBILITY: u32 = 963_480;

/// A level 10 Initiate (a mana user) holding `items`, at no mana and 50 health, the wall clock at
/// [`WALL_MS`].
fn mage_holding(items: &[Carried]) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with(10, 0, items);
    let blob = blob.replace("calling = \"basic_warrior\"", "calling = \"initiate\"");
    let (mut realm, me, events) = enter_saved(guid, &blob);
    realm.set_wall_clock(WALL_MS);
    realm.set_player_health(50);
    realm.set_player_power(0);
    drain(&mut realm);
    (realm, me, events)
}

/// The potion spell's heals on `me`, at once and in ticks.
fn heals(events: &[SessionEvent], me: u64) -> (Vec<u32>, Vec<u32>) {
    let mut once = Vec::new();
    let mut ticks = Vec::new();
    for e in events {
        match e {
            SessionEvent::SpellHealLog(l) if l.target == me && l.spell_id == POTION_SPELL => {
                once.push(l.amount)
            }
            SessionEvent::PeriodicAuraLog(l) if l.target == me && l.spell_id == POTION_SPELL => {
                for t in &l.ticks {
                    if let PeriodicTick::Heal { amount } = t {
                        ticks.push(*amount);
                    }
                }
            }
            _ => {}
        }
    }
    (once, ticks)
}

/// The potion spell's damage from `me` at `target`: `(damage, school)`.
fn potion_damage(events: &[SessionEvent], me: u64, target: u64) -> Vec<(u32, u8)> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l)
                if l.attacker == me && l.target == target && l.spell_id == POTION_SPELL =>
            {
                Some((l.damage, l.school))
            }
            _ => None,
        })
        .collect()
}

/// The potion spell's power gains on `me`.
fn energized(events: &[SessionEvent], me: u64) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellEnergizeLog(l) if l.target == me && l.spell_id == POTION_SPELL => {
                Some(l.amount)
            }
            _ => None,
        })
        .collect()
}

/// The control changes for `me` in `events`.
fn control(events: &[SessionEvent], me: u64) -> Vec<bool> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ClientControl { mover, allow_move } if *mover == me => Some(*allow_move),
            _ => None,
        })
        .collect()
}

/// Whether `mob` swung at `me` in `events`.
fn swung_at(events: &[SessionEvent], mob: u64, me: u64) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::AttackerState(s) if s.attacker == mob && s.victim == me))
}

/// Stands beside the first mob and selects it: the mob.
fn at_the_mob(realm: &mut Realm, events: &[SessionEvent]) -> Mob {
    let mob = the_mob(events);
    stand_by(realm, &mob);
    realm.handle(RealmCommand::SetSelection { guid: mob.guid });
    mob
}

#[test]
fn trollblood_draught_heals_12_every_2_s_for_10_s() {
    let (mut realm, me, _) = holding(20, &[(BACKPACK_1, TROLLBLOOD, 1)]);
    realm.set_player_health(50);
    let mut events = use_at(&mut realm, BACKPACK_1);
    events.extend(run(&mut realm, 11.0));
    assert_eq!(heals(&events, me), (vec![], vec![12; 5]));
}

#[test]
fn bloodfire_bomb_burns_25_to_35_fire_and_heals_half_of_it() {
    let (mut realm, me, events) = holding(10, &[(BACKPACK_1, BLOODFIRE, 1)]);
    realm.set_player_health(50);
    let mob = at_the_mob(&mut realm, &events);
    let thrown = use_at(&mut realm, BACKPACK_1);
    let burns = potion_damage(&thrown, me, mob.guid);
    assert_eq!(burns.len(), 1, "{burns:?}");
    let (damage, school) = burns[0];
    assert!((25..=35).contains(&damage) && school == FIRE, "{burns:?}");
    assert_eq!(heals(&thrown, me), (vec![damage / 2], vec![]));
}

#[test]
fn rejuvenating_draught_restores_60_health_and_60_mana() {
    let (mut realm, me, _) = mage_holding(&[(BACKPACK_1, REJUVENATING, 1)]);
    let events = use_at(&mut realm, BACKPACK_1);
    assert_eq!(heals(&events, me), (vec![60], vec![]));
    assert_eq!(energized(&events, me), vec![60]);
}

#[test]
fn deepsleep_takes_control_for_12_s_and_heals_150_over_it() {
    let (mut realm, me, _) = holding(20, &[(BACKPACK_1, DEEPSLEEP, 1)]);
    realm.set_player_health(50);
    let slept = use_at(&mut realm, BACKPACK_1);
    assert_eq!(control(&slept, me), vec![false]);
    realm.handle(RealmCommand::CastSpell {
        spell_id: POTION_SPELL,
        target: None,
    });
    assert_eq!(refusals(&drain(&mut realm)), vec![STUNNED]);
    let events = run(&mut realm, 11.5);
    assert!(control(&events, me).is_empty(), "still asleep at 11.5 s");
    let (_, ticks) = heals(&events, me);
    let events = run(&mut realm, 1.0);
    assert_eq!(control(&events, me), vec![true]);
    let (_, last) = heals(&events, me);
    assert_eq!(ticks.iter().chain(&last).sum::<u32>(), 150);
    assert_eq!(ticks.len() + last.len(), 6);
}

#[test]
fn a_blow_wakes_the_sleeper_gives_control_back_and_stops_the_heal() {
    let (mut realm, me, events) = holding(10, &[(BACKPACK_1, DEEPSLEEP, 1)]);
    realm.set_player_health(100);
    let mob = the_mob(&events);
    stand_by(&mut realm, &mob);
    assert_eq!(control(&use_at(&mut realm, BACKPACK_1), me), vec![false]);
    let events = run(&mut realm, 12.0);
    let woke = events
        .iter()
        .position(|e| matches!(e, SessionEvent::ClientControl { mover, allow_move: true } if *mover == me))
        .expect("the player wakes");
    let blow = events
        .iter()
        .position(|e| {
            matches!(e, SessionEvent::AttackerState(s)
            if s.attacker == mob.guid && s.victim == me && s.damage > 0)
        })
        .expect("the mob lands a blow");
    assert!(blow < woke, "woken by the blow");
    let (_, after) = heals(&events[woke..], me);
    assert!(after.is_empty(), "the heal stops: {after:?}");
}

#[test]
fn ember_shield_soaks_40_and_bursts_20_fire_on_each_mob_within_8_yd() {
    let (mut realm, me, mut seen) = holding(10, &[(BACKPACK_1, EMBER_SHIELD, 1)]);
    let mob = the_mob(&seen);
    stand_by(&mut realm, &mob);
    seen.extend(use_at(&mut realm, BACKPACK_1));
    assert!(descriptor(&seen, me)
        .unit_aura_ids()
        .any(|id| id == POTION_SPELL));
    let events = run(&mut realm, 30.0);
    let bursts = potion_damage(&events, me, mob.guid);
    assert_eq!(bursts, vec![(20, FIRE)]);
}

#[test]
fn mana_shield_gives_15_mana_for_each_blow_it_soaks() {
    let (mut realm, me, events) = mage_holding(&[(BACKPACK_1, MANA_SHIELD, 1)]);
    realm.set_player_health(300);
    let mob = the_mob(&events);
    stand_by(&mut realm, &mob);
    use_at(&mut realm, BACKPACK_1);
    let events = run(&mut realm, 6.0);
    let gains = energized(&events, me);
    assert!(!gains.is_empty(), "a blow was soaked");
    assert!(gains.iter().all(|&g| g == 15), "{gains:?}");
}

#[test]
fn steady_stance_halves_the_threat_the_players_damage_makes() {
    let (mut realm, me, events) = holding(10, &[(BACKPACK_1, STEADY_STANCE, 1)]);
    let mob = at_the_mob(&mut realm, &events);
    use_at(&mut realm, BACKPACK_1);
    // The mob notices them: its seed.
    run(&mut realm, 0.1);
    let before = realm.creature_threat(mob.guid, me);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let mut dealt = 0;
    for _ in 0..60 {
        realm.tick(0.1, &FlatGround);
        dealt += drain(&mut realm)
            .iter()
            .filter_map(|e| match e {
                SessionEvent::AttackerState(s) if s.attacker == me => Some(s.damage),
                _ => None,
            })
            .sum::<u32>();
        if dealt > 0 {
            break;
        }
    }
    assert!(dealt > 0, "a swing landed");
    let after = realm.creature_threat(mob.guid, me);
    assert_eq!(after - before, dealt as f32 / 2.0);
}

#[test]
fn spellfire_bomb_burns_25_to_35_fire() {
    let (mut realm, me, events) = holding(10, &[(BACKPACK_1, SPELLFIRE, 1)]);
    let mob = at_the_mob(&mut realm, &events);
    let burns = potion_damage(&use_at(&mut realm, BACKPACK_1), me, mob.guid);
    assert_eq!(burns.len(), 1, "{burns:?}");
    assert!((25..=35).contains(&burns[0].0) && burns[0].1 == FIRE);
}

#[test]
fn smoke_bomb_hits_a_second_mob_5_yd_off_and_drops_threat_on_each_by_100() {
    let (mut realm, me, events) = holding(10, &[(BACKPACK_1, SMOKE_BOMB, 1)]);
    let mob = at_the_mob(&mut realm, &events);
    let entry = mob.fields.object_entry().unwrap();
    let level = mob.fields.unit_level().unwrap() as u8;
    let [x, y, z] = mob.position;
    let other = realm
        .spawn_for(&FlatGround, entry, level, [x, y, z], 1, Audience::of(&[me]))
        .expect("a second mob of its kind");
    realm.place_creature(other, [x + 5.0, y, z]);
    for guid in [mob.guid, other] {
        realm.add_creature_threat(guid, me, 1000.0);
    }
    let thrown = use_at(&mut realm, BACKPACK_1);
    for guid in [mob.guid, other] {
        let burns = potion_damage(&thrown, me, guid);
        assert_eq!(burns.len(), 1, "{burns:?}");
        assert!((20..=30).contains(&burns[0].0));
        let threat = realm.creature_threat(guid, me);
        assert_eq!(threat, 900.0 + burns[0].0 as f32, "{guid}");
    }
}

#[test]
fn invisibility_clears_threat_stops_the_mob_returns_60_mana_and_a_swing_ends_it() {
    let (mut realm, me, events) = mage_holding(&[(BACKPACK_1, INVISIBILITY, 1)]);
    realm.set_player_health(300);
    let mob = at_the_mob(&mut realm, &events);
    let fight = run(&mut realm, 3.0);
    assert!(swung_at(&fight, mob.guid, me), "the mob attacks");
    let mut seen = events;
    seen.extend(fight);
    let drunk = use_at(&mut realm, BACKPACK_1);
    assert_eq!(energized(&drunk, me), vec![60]);
    assert_eq!(realm.creature_threat(mob.guid, me), 0.0);
    seen.extend(drunk);
    let quiet = run(&mut realm, 3.0);
    assert!(!swung_at(&quiet, mob.guid, me), "the mob stops attacking");
    seen.extend(quiet);
    assert!(descriptor(&seen, me)
        .unit_aura_ids()
        .any(|id| id == POTION_SPELL));
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    seen.extend(run(&mut realm, 1.0));
    assert!(!descriptor(&seen, me)
        .unit_aura_ids()
        .any(|id| id == POTION_SPELL));
}

#[test]
fn a_second_hybrid_inside_the_cooldown_is_refused() {
    let (mut realm, me, _) = holding(
        20,
        &[
            (BACKPACK_1, TROLLBLOOD, 1),
            (BACKPACK_1 + 1, REJUVENATING, 1),
        ],
    );
    use_at(&mut realm, BACKPACK_1);
    assert_eq!(
        refusals(&use_at(&mut realm, BACKPACK_1 + 1)),
        vec![NOT_READY]
    );
    assert_eq!(held(&realm, me, REJUVENATING), (0, 1));
}
