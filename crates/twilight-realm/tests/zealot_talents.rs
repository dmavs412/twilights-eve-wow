//! The Zealot's three-tab tree of Paladin talents, played through the realm's seam: 49 points at
//! the cap, Holy Power behind 30 in Crusade, Benediction's cheaper Seals and Judgement with the
//! spell modifiers that tell the client, what the modelled talents do to the sheet and the
//! swings, and the respec that wipes it all.

mod common;
mod gear_common;

use benilla_protocol::SessionEvent;
use common::fight::*;
use common::*;
use gear_common::{descriptor, save_with, SEED};

// `Talent.dbc` ids in the Zealot tree (`content/callings.toml`).
// Crusade.
const BENEDICTION: u32 = 1407;
const DEFLECTION: u32 = 1403;
const CONVICTION: u32 = 1411;
const VINDICATION: u32 = 1633;
const PURSUIT_OF_JUSTICE: u32 = 1634;
const EYE_FOR_AN_EYE: u32 = 1632;
const TWO_HANDED_WEAPON_SPECIALIZATION: u32 = 1410;
const IMPROVED_HAMMER_OF_JUSTICE: u32 = 1521;
const VENGEANCE: u32 = 1402;
const HOLY_POWER: u32 = 1627;
const HOLY_POWER_RANKS: [u32; 5] = [5923, 5924, 5925, 5926, 25829];

/// A human Zealot of `level` in the world, with the entry burst.
fn zealot(level: u8) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with(level, 0, &[]);
    let blob = blob.replace("calling = \"basic_warrior\"", "calling = \"zealot\"");
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

fn learn(realm: &mut Realm, events: &mut Vec<SessionEvent>, talent_id: u32, ranks: u32) {
    for rank in 0..ranks {
        realm.handle(RealmCommand::LearnTalent { talent_id, rank });
    }
    events.extend(drain(realm));
}

fn points(events: &[SessionEvent], me: u64) -> Option<u32> {
    descriptor(events, me).player_talent_points()
}

/// Whether the spellbook ends up holding any of `ranks`.
fn knows_any(events: &[SessionEvent], ranks: &[u32]) -> bool {
    let mut known = false;
    for e in events {
        match e {
            SessionEvent::SpellLearned { spell_id } if ranks.contains(spell_id) => known = true,
            SessionEvent::SpellRemoved { spell_id } if ranks.contains(spell_id) => known = false,
            _ => {}
        }
    }
    known
}

/// Twenty-nine points down Crusade's first six rows: one short of Holy Power's.
fn twenty_nine_in_crusade(realm: &mut Realm, events: &mut Vec<SessionEvent>) {
    learn(realm, events, BENEDICTION, 5);
    learn(realm, events, DEFLECTION, 5);
    learn(realm, events, CONVICTION, 5);
    learn(realm, events, VINDICATION, 3);
    learn(realm, events, PURSUIT_OF_JUSTICE, 2);
    learn(realm, events, EYE_FOR_AN_EYE, 2);
    learn(realm, events, TWO_HANDED_WEAPON_SPECIALIZATION, 3);
    learn(realm, events, IMPROVED_HAMMER_OF_JUSTICE, 3);
    learn(realm, events, VENGEANCE, 1);
}

// ── Benediction ──────────────────────────────────────────────────────────────────────────────

/// vmangos `SpellModOp` COST, in tenths of rage on a flat cell.
const SPELLMOD_COST: u8 = 14;
/// The family bits the client gives the borrowed spells (`borrowed` in `content/callings.toml`).
const SEAL_OF_COMMAND_BIT: u8 = 33;
const SEAL_OF_THE_CRUSADER_BIT: u8 = 34;
const JUDGEMENT_BIT: u8 = 35;
const HAMMER_OF_JUSTICE_BIT: u8 = 36;

/// The last value the flat COST cell on `bit` was set to.
fn cost_mod(events: &[SessionEvent], bit: u8) -> Option<i32> {
    events
        .iter()
        .filter_map(|e| match *e {
            SessionEvent::SpellModifier {
                flat: true,
                mask_bit,
                op: SPELLMOD_COST,
                value,
            } if mask_bit == bit => Some(value),
            _ => None,
        })
        .next_back()
}

#[test]
fn benediction_tells_the_client_the_seals_and_judgement_cost_less() {
    let (mut realm, _, mut events) = zealot(50);
    learn(&mut realm, &mut events, BENEDICTION, 1);
    assert_eq!(cost_mod(&events, SEAL_OF_COMMAND_BIT), Some(-10));
    assert_eq!(cost_mod(&events, SEAL_OF_THE_CRUSADER_BIT), Some(-10));
    assert_eq!(
        cost_mod(&events, JUDGEMENT_BIT),
        Some(-10),
        "every rank lowers Judgement"
    );
    learn(&mut realm, &mut events, BENEDICTION, 5);
    // 3 rage off the Seals' 10, 2 off Judgement's 5, in tenths.
    assert_eq!(cost_mod(&events, SEAL_OF_COMMAND_BIT), Some(-30));
    assert_eq!(cost_mod(&events, SEAL_OF_THE_CRUSADER_BIT), Some(-30));
    assert_eq!(cost_mod(&events, JUDGEMENT_BIT), Some(-20));
    assert_eq!(cost_mod(&events, HAMMER_OF_JUSTICE_BIT), None, "not a Seal");
}

#[test]
fn benediction_takes_rage_off_the_seals_and_judgement() {
    let (mut realm, me, mob, mut view) = tanking(6, &[(BENEDICTION, 5)], 20);
    stand_at(&mut realm, &mob, 2.0);
    let rage = view.rage(me);
    realm.handle(cast(SEAL_OF_THE_CRUSADER, None));
    assert_eq!(refusal(&view.drain(&mut realm), SEAL_OF_THE_CRUSADER), None);
    assert_eq!(view.rage(me), rage - 7, "10 rage, less 3");
    // The global cooldown.
    view.run(&mut realm, 1.6);
    let rage = view.rage(me);
    realm.handle(cast(SEAL_OF_COMMAND, None));
    assert_eq!(refusal(&view.drain(&mut realm), SEAL_OF_COMMAND), None);
    assert_eq!(view.rage(me), rage - 7, "10 rage, less 3");
    view.run(&mut realm, 1.6);
    let rage = view.rage(me);
    realm.handle(cast(JUDGEMENT, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(&mut realm), JUDGEMENT), None);
    assert_eq!(view.rage(me), rage - 3, "5 rage, less 2");
}

#[test]
fn one_rank_of_benediction_takes_rage_off_judgement() {
    let (mut realm, me, mob, mut view) = tanking(6, &[(BENEDICTION, 1)], 20);
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(cast(SEAL_OF_THE_CRUSADER, None));
    assert_eq!(refusal(&view.drain(&mut realm), SEAL_OF_THE_CRUSADER), None);
    view.run(&mut realm, 1.6);
    let rage = view.rage(me);
    realm.handle(cast(JUDGEMENT, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(&mut realm), JUDGEMENT), None);
    assert_eq!(view.rage(me), rage - 4, "5 rage, less 1");
}

#[test]
fn a_benediction_from_the_save_reaches_the_client_at_login() {
    let (_, _, _, view) = zealot_at(10, SEED, &[(BENEDICTION, 2)], &[]);
    assert_eq!(cost_mod(&view.entry, SEAL_OF_COMMAND_BIT), Some(-10));
    assert_eq!(cost_mod(&view.entry, JUDGEMENT_BIT), Some(-10));
}

// Blade.
const IMPROVED_CHARGE: u32 = 126;
const IMPALE: u32 = 662;
const DEEP_WOUNDS: u32 = 121;
const SWORD_SPECIALIZATION: u32 = 123;
const FLURRY: u32 = 156;

#[test]
fn blade_opens_its_second_row_from_its_first_and_flurry_at_ten() {
    let (mut realm, me, mut events) = zealot(50);
    learn(&mut realm, &mut events, IMPROVED_CHARGE, 2);
    learn(&mut realm, &mut events, IMPALE, 2);
    learn(&mut realm, &mut events, SWORD_SPECIALIZATION, 1);
    assert_eq!(points(&events, me), Some(45), "the second row needs 5");
    learn(&mut realm, &mut events, DEEP_WOUNDS, 1);
    learn(&mut realm, &mut events, SWORD_SPECIALIZATION, 4);
    assert_eq!(points(&events, me), Some(40));
    learn(&mut realm, &mut events, FLURRY, 1);
    assert_eq!(points(&events, me), Some(40), "Flurry's row needs 10");
    learn(&mut realm, &mut events, SWORD_SPECIALIZATION, 5);
    learn(&mut realm, &mut events, FLURRY, 1);
    assert_eq!(points(&events, me), Some(38));
}

#[test]
fn a_zealot_at_the_cap_has_49_points_and_the_31st_in_crusade_reaches_holy_power() {
    let (mut realm, me, mut events) = zealot(50);
    assert_eq!(points(&events, me), Some(49));
    twenty_nine_in_crusade(&mut realm, &mut events);
    assert_eq!(points(&events, me), Some(20));
    learn(&mut realm, &mut events, HOLY_POWER, 1);
    assert_eq!(points(&events, me), Some(20), "Holy Power's row needs 30");
    learn(&mut realm, &mut events, VENGEANCE, 2);
    assert_eq!(points(&events, me), Some(19));
    learn(&mut realm, &mut events, HOLY_POWER, 5);
    assert_eq!(points(&events, me), Some(14));
    assert!(knows_any(&events, &HOLY_POWER_RANKS));
}

// ── The sheet and the swings ─────────────────────────────────────────────────────────────────

// Faith.
const TOUGHNESS: u32 = 1423;
const ANTICIPATION: u32 = 1629;
const ONE_HANDED_WEAPON_SPECIALIZATION: u32 = 1429;

/// Brawler Gloves: 62 armour, worn in the hands slot.
const BRAWLER_GLOVES: u32 = 720;
const HANDS: u8 = 9;
const MAIN_HAND: u8 = 15;
/// A two-handed axe, for levels 2 to 4.
const LARGE_AXE: u32 = 2491;

/// The Defense line's bonus on the sheet (`PLAYER_SKILL_INFO` slot 1, its third dword).
fn defense_bonus(view: &View, me: u64) -> i32 {
    view.fields(me)
        .player_skill(1)
        .map_or(0, |s| i32::from(s.temp_bonus) + i32::from(s.perm_bonus))
}

#[test]
fn deflection_gives_a_parry_chance_and_conviction_a_crit_chance() {
    let sheet = |talents: &[(u32, u8)]| {
        let (_, me, _, view) = zealot_at(10, SEED, talents, &[]);
        let f = view.fields(me);
        (
            f.player_parry_percentage().unwrap(),
            f.player_crit_percentage().unwrap(),
        )
    };
    let (parry, crit) = sheet(&[]);
    assert_eq!(parry, 0.0, "no Parry skill");
    let (parry, _) = sheet(&[(DEFLECTION, 5)]);
    assert!((parry - 5.0).abs() < 1e-4, "{parry}");
    let (_, convicted) = sheet(&[(CONVICTION, 5)]);
    assert!(
        (convicted - crit - 5.0).abs() < 1e-4,
        "{crit} -> {convicted}"
    );
}

#[test]
fn anticipation_raises_defense_and_toughness_the_armour_of_items() {
    let gloves = [(HANDS, BRAWLER_GLOVES)];
    let (_, me, _, plain) = zealot_at(10, SEED, &[], &gloves);
    let (_, _, _, anticipating) = zealot_at(10, SEED, &[(ANTICIPATION, 5)], &gloves);
    assert_eq!(
        defense_bonus(&anticipating, me),
        defense_bonus(&plain, me) + 10
    );
    let (_, _, _, tough) = zealot_at(10, SEED, &[(TOUGHNESS, 5)], &gloves);
    let armor = |v: &View| v.fields(me).unit_resistance(0).unwrap();
    // 10% more of the gloves' 62, rounded: 6.
    assert_eq!(armor(&tough), armor(&plain) + 6);
}

#[test]
fn the_weapon_specializations_add_to_the_damage_of_their_weapons() {
    // The same fight with and without the talent: the dice fall the same.
    let first = |seed: u64, talents: &[(u32, u8)], items: &[(u8, u32)]| {
        let (mut realm, me, mob, mut view) = zealot_at(10, seed, talents, items);
        stand_at(&mut realm, &mob, 2.0);
        first_swing(&mut realm, &mut view, me, &mob)
    };
    let axe = [(MAIN_HAND, LARGE_AXE)];
    let one = [(ONE_HANDED_WEAPON_SPECIALIZATION, 5)];
    let two = [(TWO_HANDED_WEAPON_SPECIALIZATION, 3)];
    let mut checked = 0;
    for seed in 0..20 {
        let (sword, axe_swing) = (first(seed, &[], &[]), first(seed, &[], &axe));
        if sword == 0 || axe_swing == 0 {
            continue;
        }
        // The Worn Shortsword is one-handed: 10% more at 5/5, nothing from the two-handed one.
        assert_eq!(first(seed, &one, &[]), sword * 110 / 100);
        assert_eq!(first(seed, &two, &[]), sword);
        // A two-handed axe: 6% more at 3/3, nothing from the one-handed one.
        assert_eq!(first(seed, &two, &axe), axe_swing * 106 / 100);
        assert_eq!(first(seed, &one, &axe), axe_swing);
        checked += 1;
    }
    assert!(
        checked >= 3,
        "only {checked} fights landed their first swings"
    );
}

/// Swings at `mob` and returns the first white swing's damage, 0 for one that missed.
fn first_swing(realm: &mut Realm, view: &mut View, me: u64, mob: &Mob) -> u32 {
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    for _ in 0..30 {
        for e in view.run(realm, 0.1) {
            if let SessionEvent::AttackerState(s) = e {
                if s.attacker == me {
                    return s.damage;
                }
            }
        }
    }
    panic!("no swing in three seconds");
}

// ── The respec ───────────────────────────────────────────────────────────────────────────────

/// `UNIT_NPC_FLAGS` trainer bit.
const NPC_TRAINER: u32 = 0x10;

#[test]
fn a_respec_at_the_trainer_wipes_the_zealot_tree() {
    let (guid, blob) = save_with(50, 200_000, &[]);
    let blob = blob.replace("calling = \"basic_warrior\"", "calling = \"zealot\"");
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut events = drain(&mut realm);
    let trainer = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate { guid, fields, .. }
                if fields.unit_npc_flags() & NPC_TRAINER != 0 =>
            {
                Some(*guid)
            }
            _ => None,
        })
        .expect("the trainer stands by");
    learn(&mut realm, &mut events, BENEDICTION, 5);
    learn(&mut realm, &mut events, DEFLECTION, 5);
    learn(&mut realm, &mut events, IMPROVED_CHARGE, 2);
    assert_eq!(points(&events, guid), Some(37));

    walk_up_to(&mut realm, trainer);
    realm.handle(RealmCommand::GossipHello { guid: trainer });
    let option = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { options, .. } => options
                .into_iter()
                .find(|o| o.message == "I wish to unlearn my talents.")
                .map(|o| o.index),
            _ => None,
        })
        .expect("the unlearn line");
    realm.handle(RealmCommand::GossipSelectOption {
        guid: trainer,
        option,
    });
    realm.handle(RealmCommand::TalentWipeConfirm { trainer });
    let wiped = drain(&mut realm);
    events.extend(wiped.iter().cloned());

    assert_eq!(points(&events, guid), Some(49), "every point back");
    let removed: Vec<u32> = wiped
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellRemoved { spell_id } => Some(*spell_id),
            _ => None,
        })
        .collect();
    // Benediction's, Deflection's and Improved Charge's top ranks among them.
    for spell in [20105, 20064, 12697] {
        assert!(removed.contains(&spell), "{spell}: {removed:?}");
    }
    for bit in [SEAL_OF_COMMAND_BIT, SEAL_OF_THE_CRUSADER_BIT, JUDGEMENT_BIT] {
        assert_eq!(cost_mod(&wiped, bit), Some(0), "bit {bit}");
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────────────────────

const SEAL_OF_COMMAND: u32 = 20375;
const SEAL_OF_THE_CRUSADER: u32 = 20305;
const JUDGEMENT: u32 = 20271;

const OFF_HAND: u8 = 16;

/// A human Zealot of `level` with `talents` (`(talent id, rank)`) and `items` (`(slot, entry)`)
/// in its save, in the world at the Crossroads on `seed`'s dice; the nearest hostile mob, and the
/// view.
fn zealot_at(
    level: u8,
    seed: u64,
    talents: &[(u32, u8)],
    items: &[(u8, u32)],
) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"zealot\"");
    // The human's starting Worn Wooden Shield is set aside: a test that wants a shield gives one.
    blob = drop_slot(&blob, OFF_HAND);
    if !talents.is_empty() {
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        blob = blob.replace(
            "money = 0\n",
            &format!("money = 0\ntalents = [{}]\n", rows.join(", ")),
        );
    }
    for (slot, entry) in items {
        // In place of whatever the slot held.
        blob = drop_slot(&blob, *slot);
        blob.push_str(&format!(
            "\n[[items]]\nslot = {slot}\nentry = {entry}\ncount = 1\n"
        ));
    }
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let mut events = view.drain(&mut realm);
    view.entry = events.clone();
    // The world streams in over the first ticks.
    events.extend(view.run(&mut realm, 1.0));
    let mob = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: benilla_protocol::EntityKind::Unit,
                position,
                fields,
                ..
            } if fields.unit_npc_flags() == 0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .min_by(|a, b| flat(a.position, CROSSROADS_XY).total_cmp(&flat(b.position, CROSSROADS_XY)))
        .expect("a mob near the Crossroads");
    (realm, guid, mob, view)
}

/// A Zealot of `level` with `talents` that pulled the camp south of the Crossroads without
/// swinging back, and took its blows until it had `rage`.
fn tanking(level: u8, talents: &[(u32, u8)], rage: u32) -> (Realm, u64, Mob, View) {
    let (mut realm, me, _, mut view) = zealot_at(level, SEED, talents, &[]);
    // Out of reach of the four neutral scorpids, then each of them in turn.
    realm.handle(RealmCommand::Move {
        position: [CAMP[0] + 40.0, CAMP[1], 0.0],
        orientation: std::f32::consts::PI,
    });
    let streamed = view.run(&mut realm, 1.0);
    let camp: Vec<Mob> = streamed
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: benilla_protocol::EntityKind::Unit,
                position,
                ..
            } if flat(*position, CAMP) < 15.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .collect();
    let mob = camp.first().cloned().expect("the scorpid camp streams in");
    stand_at(&mut realm, &mob, 2.0);
    for other in camp.iter().rev() {
        realm.handle(RealmCommand::AttackSwing { guid: other.guid });
    }
    realm.handle(RealmCommand::AttackStop);
    for _ in 0..1800 {
        view.run(&mut realm, 0.1);
        assert!(view.health(me) > 0, "died waiting for {rage} rage");
        if view.rage(me) >= rage {
            return (realm, me, mob, view);
        }
    }
    panic!("three minutes of blows built no {rage} rage");
}
