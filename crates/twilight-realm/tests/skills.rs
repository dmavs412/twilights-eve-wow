//! The Basic Warrior's skills, played through the realm's seam: the spellbook at login, casts
//! from the action bar, what they cost, what they refuse, and what they do to a mob. The skill
//! mechanics its kit cannot show (a buff, a bleed, the global cooldown, Overpower's window) are
//! played on the Vanguard.
//!
//! A queued strike that finds its rage spent by the swing went with Cleave and Thunder Clap: no
//! Calling has both an on-next-swing strike and another rage spender now.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::*;
use common::*;

const HEROIC_STRIKE: u32 = 78;
const CHARGE: u32 = 100;
const GOUGE: u32 = 1776;
const FIRE_BLAST: u32 = 2136;

/// The Basic Warrior's kit.
const KIT: [u32; 4] = [HEROIC_STRIKE, CHARGE, GOUGE, FIRE_BLAST];

#[test]
fn a_new_basic_warriors_spellbook_holds_exactly_its_kit() {
    let mut realm = realm();
    let (_, events) = enter_new(&mut realm, "Garrosh", ORC);

    let books: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellBook {
                spell_ids,
                cooldowns,
            } => Some((spell_ids.clone(), cooldowns.len())),
            _ => None,
        })
        .collect();
    assert_eq!(books.len(), 1, "one spellbook at login");
    let (mut ids, cooldowns) = books[0].clone();
    ids.sort_unstable();
    let mut want = KIT.to_vec();
    want.sort_unstable();
    assert_eq!(ids, want);
    assert_eq!(cooldowns, 0, "a fresh character has nothing cooling down");
}

#[test]
fn a_warrior_stands_in_battle_stance_so_the_client_allows_its_stance_skills() {
    let mut realm = realm();
    let (guid, events) = enter_new(&mut realm, "Garrosh", ORC);
    assert_eq!(
        own_create(&events, guid).unit_shapeshift_form(),
        BATTLE_STANCE
    );
}

#[test]
fn a_new_character_finds_its_kit_on_its_battle_stance_bar() {
    let mut realm = realm();
    let (_, events) = enter_new(&mut realm, "Garrosh", ORC);
    let buttons = action_buttons(&events);
    // Battle Stance pages the main bar to slots 73-84 (0-based 72-83).
    let on_bar: Vec<(u8, u32)> = buttons.iter().map(|b| (b.slot, b.action)).collect();
    let want: Vec<(u8, u32)> = (72..).zip(KIT).collect();
    assert_eq!(on_bar, want);
    assert!(buttons.iter().all(|b| b.kind == 0), "spell buttons");
}

#[test]
fn the_bar_the_player_arranges_comes_back_at_the_next_login() {
    let mut realm = realm();
    let (guid, _) = enter_new(&mut realm, "Garrosh", ORC);
    // Charge moved to button 1 of the stance bar; Heroic Strike's old button cleared.
    realm.handle(RealmCommand::SetActionButton {
        slot: 72,
        packed: CHARGE,
    });
    for slot in [73, 74, 75] {
        realm.handle(RealmCommand::SetActionButton { slot, packed: 0 });
    }
    realm.handle(RealmCommand::Logout);
    realm.handle(RealmCommand::EnterWorld { guid });
    let events = drain(&mut realm);
    let on_bar: Vec<(u8, u32)> = action_buttons(&events)
        .iter()
        .map(|b| (b.slot, b.action))
        .collect();
    assert_eq!(on_bar, [(72, CHARGE)]);

    // And it survives the save.
    let blob = realm.export_character(guid).unwrap();
    let mut other = Realm::new();
    other.import_character(&blob).unwrap();
    other.handle(RealmCommand::EnterWorld { guid });
    let events = drain(&mut other);
    assert_eq!(action_buttons(&events)[0].action, CHARGE);
}

#[test]
fn a_skill_grows_with_level_without_being_relearned() {
    let charge = |level| {
        let (mut realm, _, mob, mut view) = warrior_at(level);
        stand_at(&mut realm, &mob, 15.0);
        realm.handle(cast(CHARGE, Some(mob.guid)));
        energized(&view.drain(&mut realm), CHARGE)
    };
    // 8.5 + 0.14 rage a level.
    assert_eq!(charge(1), vec![9]);
    assert_eq!(charge(10), vec![10]);
}

#[test]
fn a_skill_at_an_enemy_needs_a_target_in_front() {
    let (mut realm, _, mob, mut view) = warrior_at(10);
    realm.handle(cast(CHARGE, None));
    assert_eq!(refusal(&view.drain(&mut realm), CHARGE), Some(BAD_TARGETS));

    turn_around(&mut realm, &mob, 15.0);
    realm.handle(cast(CHARGE, Some(mob.guid)));
    assert_eq!(
        refusal(&view.drain(&mut realm), CHARGE),
        Some(UNIT_NOT_INFRONT)
    );

    // Fireball is not a Basic Warrior's.
    realm.handle(cast(133, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(&mut realm), 133), Some(NOT_KNOWN));
}

#[test]
fn charge_runs_the_warrior_to_the_mob_for_rage_and_a_stun() {
    let (mut realm, me, mob, mut view) = warrior_at(1);
    stand_at(&mut realm, &mob, 3.0);
    realm.handle(cast(CHARGE, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(&mut realm), CHARGE), Some(TOO_CLOSE));
    stand_at(&mut realm, &mob, 30.0);
    realm.handle(cast(CHARGE, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(&mut realm), CHARGE), Some(OUT_OF_RANGE));

    stand_at(&mut realm, &mob, 15.0);
    realm.handle(cast(CHARGE, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, CHARGE), None);
    let path = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::MonsterMove { guid, path, .. } if *guid == me => Some(path.clone()),
            _ => None,
        })
        .expect("the charge's run");
    let end = *path.last().unwrap();
    assert!(dist(end, mob.position) < 2.5, "ends at the mob");
    assert_eq!(energized(&events, CHARGE), vec![9]);
    assert_eq!(view.rage(me), 9);
    assert!(view.aura(mob.guid, 7922).is_some(), "stunned");

    // Stunned for a second: the mob does not swing until it is over.
    let first = first_swing_at(&mut realm, &mut view, mob.guid, 3.0).expect("it swings at last");
    assert!(first >= 1.0, "swung {first} s in");

    // In combat now, and on cooldown anyway.
    realm.handle(cast(CHARGE, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(&mut realm), CHARGE), Some(NOT_READY));
}

#[test]
fn charge_is_refused_in_combat() {
    let (mut realm, _, mob, mut view) = warrior_at(10);
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    view.run(&mut realm, 0.5);
    stand_at(&mut realm, &mob, 15.0);
    realm.handle(cast(CHARGE, Some(mob.guid)));
    assert_eq!(
        refusal(&view.drain(&mut realm), CHARGE),
        Some(AFFECTING_COMBAT)
    );
}

#[test]
fn heroic_strike_needs_its_rage() {
    let (mut realm, _, mob, mut view) = warrior_at(10);
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(cast(HEROIC_STRIKE, Some(mob.guid)));
    assert_eq!(
        refusal(&view.drain(&mut realm), HEROIC_STRIKE),
        Some(NO_POWER)
    );
}

#[test]
fn heroic_strike_waits_for_the_next_swing_and_replaces_it() {
    let (mut realm, me, mob, mut view) = warrior_at(1);
    rage_up(&mut realm, &mut view, me, &mob, CHARGE, 15);
    // The swing timer comes round again.
    view.run(&mut realm, 2.0);
    let rage = view.rage(me);

    realm.handle(cast(HEROIC_STRIKE, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert!(
        events.iter().all(|e| cast_packet(e).is_none()),
        "nothing until the swing"
    );
    assert_eq!(view.rage(me), rage, "paid at the swing");

    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let events = view.run(&mut realm, 0.1);
    assert_eq!(go_hits(&events, HEROIC_STRIKE), Some(vec![mob.guid]));
    let damage = spell_damage(&events, HEROIC_STRIKE);
    // The white roll plus 9.5 + 1.4, through a level-3 mob's armour: a white swing tops out at 7.
    assert!(damage[0] >= 11, "{damage:?}");
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, SessionEvent::AttackerState(s) if s.attacker == me)),
        "the strike took the white swing's place"
    );
    assert!(view.rage(me) <= rage - 15 + 1);

    // The swing after is white again.
    let events = view.run(&mut realm, 2.0);
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::AttackerState(s) if s.attacker == me)));
    assert_eq!(go_hits(&events, HEROIC_STRIKE), None);
}

// ── What the Basic Warrior's two skills cannot show, on the Vanguard ──────────────────────

// The Vanguard's skills (the master CSV).
const T1_BULWARK_CHARGE: u32 = 100;
const T1_ZEALOUS_CLEAVE: u32 = 845;

#[test]
fn a_melee_skill_needs_a_target_in_reach() {
    let (mut realm, me, mob, mut view) = tier1_seeded(1, SEED);
    rage_up(&mut realm, &mut view, me, &mob, T1_BULWARK_CHARGE, 20);
    realm.handle(cast(T1_ZEALOUS_CLEAVE, None));
    assert_eq!(
        refusal(&view.drain(&mut realm), T1_ZEALOUS_CLEAVE),
        Some(BAD_TARGETS)
    );
    stand_at(&mut realm, &mob, 7.0);
    realm.handle(cast(T1_ZEALOUS_CLEAVE, Some(mob.guid)));
    assert_eq!(
        refusal(&view.drain(&mut realm), T1_ZEALOUS_CLEAVE),
        Some(OUT_OF_RANGE)
    );
}

// ── Helpers ──────────────────────────────────────────────────────────────────────────────────

/// `UNIT_FIELD_BYTES_1` byte 2's Battle Stance.
const BATTLE_STANCE: u8 = 17;

fn own_create(events: &[SessionEvent], guid: u64) -> benilla_protocol::ObjectFields {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid: g, fields, ..
            } if *g == guid => Some(fields.clone()),
            _ => None,
        })
        .expect("the player's own create")
}

fn action_buttons(events: &[SessionEvent]) -> Vec<benilla_protocol::messages::ActionButton> {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ActionButtons { buttons } => Some(buttons.clone()),
            _ => None,
        })
        .expect("the action bar at login")
}

// `SpellCastResult` (vmangos `SpellDefines.h`), as the client's red error line reads them.
const AFFECTING_COMBAT: u8 = 0x00;
const BAD_TARGETS: u8 = 0x0a;
const NOT_KNOWN: u8 = 0x38;
const NOT_READY: u8 = 0x3c;
const NO_POWER: u8 = 0x4d;
const OUT_OF_RANGE: u8 = 0x59;
const TOO_CLOSE: u8 = 0x76;
const UNIT_NOT_INFRONT: u8 = 0x7c;

const SEED: u64 = 0x5EED_0008;

/// A human warrior of `level` in the world (on flat ground) at the Crossroads, the mob near it,
/// and what the client has seen so far.
fn warrior_at(level: u8) -> (Realm, u64, Mob, View) {
    warrior_seeded(level, SEED)
}

/// [`warrior_at`] with its own dice.
fn warrior_seeded(level: u8, seed: u64) -> (Realm, u64, Mob, View) {
    calling_seeded("basic_warrior", level, seed)
}

/// A human Vanguard, placed as [`warrior_seeded`] places a Basic Warrior.
fn tier1_seeded(level: u8, seed: u64) -> (Realm, u64, Mob, View) {
    calling_seeded("vanguard", level, seed)
}

/// A human of `calling` at `level`, placed as [`warrior_at`] places one.
fn calling_seeded(calling: &str, level: u8, seed: u64) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace(
            "calling = \"basic_warrior\"",
            &format!("calling = \"{calling}\""),
        );
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let events = view.drain(&mut realm);
    let mob = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: benilla_protocol::EntityKind::Unit,
                position,
                ..
            } => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .expect("a mob near the Crossroads");
    (realm, guid, mob, view)
}

/// Charges `mob` with `charge` for its rage, then swings at it until there is `rage`, and stops.
fn rage_up(realm: &mut Realm, view: &mut View, me: u64, mob: &Mob, charge: u32, rage: u32) {
    stand_at(realm, mob, 15.0);
    realm.handle(cast(charge, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(realm), charge), None, "the charge");
    stand_at(realm, mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    for _ in 0..600 {
        if view.rage(me) >= rage {
            realm.handle(RealmCommand::AttackStop);
            view.drain(realm);
            return;
        }
        view.run(realm, 0.1);
        assert!(view.health(me) > 0, "died waiting for {rage} rage");
        assert!(view.health(mob.guid) > 0, "killed it before {rage} rage");
    }
    panic!("a minute of swings built no {rage} rage");
}

/// Stands `yards` from `mob` as [`stand_at`] does, but with the back to it.
fn turn_around(realm: &mut Realm, mob: &Mob, yards: f32) {
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
        motion: Default::default(),
        position: at,
        orientation: dy.atan2(dx),
    });
}

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// `SMSG_SPELL_GO`'s hit list for `spell_id`, if it went off.
fn go_hits(events: &[SessionEvent], spell_id: u32) -> Option<Vec<u64>> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellGo {
            spell_id: s, hits, ..
        } if *s == spell_id => Some(hits.clone()),
        _ => None,
    })
}

/// `SMSG_SPELLNONMELEEDAMAGELOG` amounts for `spell_id`.
fn spell_damage(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id => Some(l.damage),
            _ => None,
        })
        .collect()
}

/// The seconds into the next `secs` at which `guid` first swings.
fn first_swing_at(realm: &mut Realm, view: &mut View, guid: u64, secs: f32) -> Option<f32> {
    swing_times(realm, view, guid, secs).first().copied()
}

fn swing_times(realm: &mut Realm, view: &mut View, guid: u64, secs: f32) -> Vec<f32> {
    let mut times = Vec::new();
    for step in 0..(secs / 0.1).round() as u32 {
        realm.tick(0.1, &FlatGround);
        let events = view.drain(realm);
        if events
            .iter()
            .any(|e| matches!(e, SessionEvent::AttackerState(s) if s.attacker == guid))
        {
            times.push((step + 1) as f32 * 0.1);
        }
    }
    times
}

/// The cast's own packets, by kind.
fn cast_packet(e: &SessionEvent) -> Option<&'static str> {
    match e {
        SessionEvent::SpellStart { .. } => Some("start"),
        SessionEvent::CastResult { success: true, .. } => Some("result ok"),
        SessionEvent::CastResult { success: false, .. } => Some("result failed"),
        SessionEvent::SpellGo { .. } => Some("go"),
        _ => None,
    }
}

/// `SMSG_SPELLENERGIZELOG` amounts for `spell_id`, in rage points.
fn energized(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellEnergizeLog(l) if l.spell_id == spell_id => {
                assert_eq!(l.power, 1, "rage");
                Some(l.amount)
            }
            _ => None,
        })
        .collect()
}
