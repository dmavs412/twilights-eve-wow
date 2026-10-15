//! The Zealot's three-tab tree of Paladin talents, played through the realm's seam: 49 points at
//! the cap, Holy Power behind 30 in Crusade, Benediction's cheaper Seals and Judgement with the
//! spell modifiers that tell the client, what the modelled talents do to the sheet and the
//! swings, and the respec that wipes it all.

mod common;
mod gear_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::*;

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
    assert_eq!(cost_mod(&events, JUDGEMENT_BIT), Some(0));
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

// ── Helpers ──────────────────────────────────────────────────────────────────────────────────

const SEAL_OF_COMMAND: u32 = 20375;
const SEAL_OF_THE_CRUSADER: u32 = 20305;
const JUDGEMENT: u32 = 20271;

const CROSSROADS_XY: [f32; 3] = [-441.8, -2648.6, 0.0];
/// A camp of four level 3 mobs south of the Crossroads (`content/barrens.toml`, 9000011).
const CAMP: [f32; 3] = [-376.0, -2837.0, 0.0];
const OFF_HAND: u8 = 16;

#[derive(Clone)]
struct Mob {
    guid: u64,
    position: [f32; 3],
}

/// What the client knows: every object's fields, merged as the packets arrive, and the entry
/// burst.
#[derive(Default)]
struct View {
    objects: std::collections::HashMap<u64, benilla_protocol::ObjectFields>,
    entry: Vec<SessionEvent>,
}

impl View {
    fn see(&mut self, events: &[SessionEvent]) {
        for e in events {
            match e {
                SessionEvent::ObjectCreate { guid, fields, .. } => {
                    self.objects.insert(*guid, fields.clone());
                }
                SessionEvent::ObjectValues { guid, fields } => {
                    self.objects.entry(*guid).or_default().merge(fields.clone());
                }
                _ => {}
            }
        }
    }

    fn drain(&mut self, realm: &mut Realm) -> Vec<SessionEvent> {
        let events = drain(realm);
        self.see(&events);
        events
    }

    fn run(&mut self, realm: &mut Realm, secs: f32) -> Vec<SessionEvent> {
        let mut events = Vec::new();
        for _ in 0..(secs / 0.1).round() as u32 {
            realm.tick(0.1, &FlatGround);
            events.extend(self.drain(realm));
        }
        events
    }

    fn fields(&self, guid: u64) -> &benilla_protocol::ObjectFields {
        self.objects
            .get(&guid)
            .expect("an object the client has seen")
    }

    fn rage(&self, guid: u64) -> u32 {
        self.fields(guid).unit_power(1).unwrap_or(0) / 10
    }

    fn health(&self, guid: u64) -> u32 {
        self.fields(guid).unit_health().unwrap()
    }
}

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

/// The save without its `[[items]]` row for `slot`.
fn drop_slot(blob: &str, slot: u8) -> String {
    let mut out = String::new();
    for (i, part) in blob.split("[[items]]").enumerate() {
        if i == 0 {
            out.push_str(part);
        } else if !part.contains(&format!("slot = {slot}\n")) {
            out.push_str("[[items]]");
            out.push_str(part);
        }
    }
    out
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

fn cast(spell_id: u32, target: Option<u64>) -> RealmCommand {
    RealmCommand::CastSpell { spell_id, target }
}

/// Moves the player to `yards` from `mob`, on the Crossroads side, facing it.
fn stand_at(realm: &mut Realm, mob: &Mob, yards: f32) {
    let (dx, dy) = (
        CROSSROADS_XY[0] - mob.position[0],
        CROSSROADS_XY[1] - mob.position[1],
    );
    let len = (dx * dx + dy * dy).sqrt();
    let at = [
        mob.position[0] + dx / len * yards,
        mob.position[1] + dy / len * yards,
        mob.position[2],
    ];
    realm.handle(RealmCommand::Move {
        position: at,
        orientation: (mob.position[1] - at[1]).atan2(mob.position[0] - at[0]),
    });
}

fn flat(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// The reason `spell_id` was refused, if it was.
fn refusal(events: &[SessionEvent], spell_id: u32) -> Option<u8> {
    events.iter().find_map(|e| match e {
        SessionEvent::CastResult {
            spell_id: s,
            success: false,
            reason,
            ..
        } if *s == spell_id => *reason,
        _ => None,
    })
}
