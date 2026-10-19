//! One mob, one kill: a hostile mob near the Crossroads, fought with vanilla melee both ways.

mod common;

use benilla_protocol::messages::ObjectType;
use benilla_protocol::{EntityKind, SessionEvent};
use common::*;
use twilight_realm::CROSSROADS;

/// The Monster faction template, hostile to every player race.
const MONSTER: u32 = 14;

#[test]
fn a_hostile_mob_stands_near_the_crossroads() {
    let mut realm = realm();
    let (_, events) = enter_new(&mut realm, "Cairne", TAUREN);

    let mob = the_mob(&events);
    let f = &mob.fields;
    assert_eq!(f.object_type(), Some(ObjectType::Unit));
    assert_eq!(f.unit_faction_template(), Some(MONSTER));
    assert_eq!(f.unit_level(), Some(3));
    assert!(f.unit_health().unwrap() > 0);
    assert_eq!(f.unit_health(), f.unit_max_health());
    // A creature guid (`HIGHGUID_UNIT`) carrying its entry in bits 24-47.
    assert_eq!(mob.guid >> 48, 0xF130);
    let entry = ((mob.guid >> 24) & 0xFF_FFFF) as u32;
    assert_eq!(f.object_entry(), Some(entry));
    let [x, y, _] = mob.position;
    let from_hub = ((x - CROSSROADS[0]).powi(2) + (y - CROSSROADS[1]).powi(2)).sqrt();
    assert!(
        (12.0..60.0).contains(&from_hub),
        "near the Crossroads but out of the way: {from_hub} yd"
    );
}

#[test]
fn the_mob_answers_to_its_name() {
    let mut realm = realm();
    let (_, events) = enter_new(&mut realm, "Cairne", TAUREN);
    let entry = the_mob(&events).fields.object_entry().unwrap();

    realm.handle(RealmCommand::CreatureQuery { entry });
    let events = drain(&mut realm);
    let name = events.iter().find_map(|e| match e {
        SessionEvent::CreatureName { entry: e, name, .. } if *e == entry => name.clone(),
        _ => None,
    });
    assert_eq!(name.as_deref(), Some("Vile Familiar"));
}

#[test]
fn the_character_sheet_shows_the_realms_stats() {
    let mut realm = realm();
    let (guid, events) = enter_new(&mut realm, "Anduin", HUMAN);
    let f = own_create(&events, guid);
    // A level-1 human warrior (§5 of the combat reference) in the starting outfit: the Worn
    // Shortsword in hand, the Recruit's Mail Vest (12 armour) and the Worn Wooden Shield (5 armour).
    assert_eq!(f.unit_max_health(), Some(60));
    assert_eq!(f.unit_health(), Some(60));
    assert_eq!(f.unit_attack_power(), Some(29));
    assert_eq!(
        f.unit_resistance(0),
        Some(40 + 12 + 5),
        "twice agility, plus the vest and the shield"
    );
    assert_eq!(f.player_crit_percentage(), Some(5.0));
    assert_eq!(f.player_dodge_percentage(), Some(5.0));
    assert_eq!(f.player_parry_percentage(), Some(0.0), "no Parry at Tier 0");
    assert_eq!(f.player_block_percentage(), Some(5.0), "the shield blocks");
    assert_eq!(
        (0..5).map(|i| f.unit_stat(i).unwrap()).collect::<Vec<_>>(),
        [23, 20, 22, 20, 20]
    );
    // 1-3 damage plus 29 attack power over a 1.9 s swing.
    assert!((f.unit_min_damage().unwrap() - 4.936).abs() < 1e-3);
    assert!((f.unit_max_damage().unwrap() - 6.936).abs() < 1e-3);
    assert_eq!(f.unit_base_attack_time(0), Some(1900));
    // The Attack and Defense rows: Unarmed and Defense at 5 of 5.
    let skills: Vec<_> = (0..2)
        .map(|s| f.player_skill(s).map(|k| (k.skill_id, k.value, k.max)))
        .collect();
    assert_eq!(skills, [Some((162, 5, 5)), Some((95, 5, 5))]);
    assert_eq!(f.player_xp(), Some(0));
    assert_eq!(f.player_next_level_xp(), Some(400));
}

#[test]
fn attacking_echoes_attack_start_and_swings_on_the_weapon_timer() {
    let (mut realm, me, mob) = warrior_at(1, 0);
    stand_by(&mut realm, &mob);
    realm.handle(RealmCommand::SetSelection { guid: mob.guid });
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let events = run(&mut realm, 5.0);

    assert!(events.iter().any(
        |e| matches!(e, SessionEvent::AttackStart { attacker, victim } if *attacker == me && *victim == mob.guid)
    ));
    // Unarmed swings every 2 s, the first at once: 0, 2 and 4 s.
    assert_eq!(swings(&events, me).count(), 3);
}

#[test]
fn every_swing_is_reported_and_the_mob_loses_what_landed() {
    let (mut realm, me, mob) = warrior_at(1, 0);
    stand_by(&mut realm, &mob);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let events = run(&mut realm, 12.0);

    let dealt: u32 = swings(&events, me).map(|s| s.damage).sum();
    assert!(dealt > 0);
    let health = last_health(&events, mob.guid).expect("the mob's health moved");
    assert_eq!(health, mob.fields.unit_max_health().unwrap() - dealt);
    for s in swings(&events, me) {
        if s.damage > 0 {
            assert_ne!(s.hit_info & 0x2, 0, "a landed swing affects its victim");
        } else {
            assert!(
                s.hit_info & 0x10 != 0 || matches!(s.victim_state, 2 | 3 | 5),
                "a swing that dealt nothing missed or was avoided: {s:?}"
            );
        }
    }
}

#[test]
fn a_mob_without_a_shield_never_blocks() {
    // The Vile Familiar carries no shield (vmangos: no block without one); it still parries.
    let (mut blocked, mut parried) = (0, 0);
    for seed in 0..200 {
        let (mut realm, me, mob) = warrior_seeded(1, seed);
        stand_by(&mut realm, &mob);
        realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
        let events = run(&mut realm, 12.0);
        for s in swings(&events, me) {
            blocked += u32::from(s.blocked > 0 || s.victim_state == 5);
            parried += u32::from(s.victim_state == 3);
        }
    }
    assert_eq!(blocked, 0);
    assert!(parried > 0, "it parries from the front");
}

#[test]
fn the_mob_fights_back_and_its_hits_come_off_the_players_health() {
    let (mut realm, me, mob) = warrior_at(1, 0);
    stand_by(&mut realm, &mob);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let events = run(&mut realm, 12.0);

    assert!(events.iter().any(
        |e| matches!(e, SessionEvent::AttackStart { attacker, victim } if *attacker == mob.guid && *victim == me)
    ));
    let taken: u32 = swings(&events, mob.guid).map(|s| s.damage).sum();
    assert!(swings(&events, mob.guid).count() >= 5, "a swing every 2 s");
    assert!(taken > 0);
    assert_eq!(last_health(&events, me), Some(60 - taken));
}

#[test]
fn rage_fills_from_hits_given_and_taken() {
    let (mut realm, me, mob) = warrior_at(1, 0);
    stand_by(&mut realm, &mob);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let events = run(&mut realm, 12.0);

    // Rage only climbs in a fight: every report is above the one before.
    let rage: Vec<u32> = own_values(&events, me)
        .filter_map(|f| f.unit_power(1))
        .collect();
    assert!(rage.len() >= 3, "{rage:?}");
    assert!(rage.windows(2).all(|w| w[1] > w[0]), "{rage:?}");
    // A level-1 warrior earns 9.9 rage for 10 damage dealt: a few hits are a lot of rage.
    assert!(*rage.last().unwrap() >= 100, "{rage:?}");
}

#[test]
fn a_mob_that_notices_the_player_runs_at_them_and_attacks() {
    let (mut realm, me, mob) = warrior_at(1, 0);
    // 10 yd away: inside a level-3 mob's 12 yd notice radius for a level-1 player.
    let at = toward(mob.position, CROSSROADS_XY, 10.0);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: at,
        orientation: 0.0,
    });
    let events = run(&mut realm, 0.1);
    assert!(events.iter().any(
        |e| matches!(e, SessionEvent::AttackStart { attacker, victim } if *attacker == mob.guid && *victim == me)
    ));
    let (path, duration_ms) = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::MonsterMove {
                guid,
                path,
                duration_ms,
                run_mode: true,
                ..
            } if *guid == mob.guid => Some((path.clone(), *duration_ms)),
            _ => None,
        })
        .expect("a chase leg");
    // A straight line to just short of the player.
    assert_eq!(path.len(), 2);
    assert_eq!(path[0], mob.position);
    let short = distance(path[1], at);
    assert!((1.0..5.0).contains(&short), "stops {short} yd short");
    // At the template's run speed: 7 × 1.143 yd/s.
    let expect_ms = distance(path[0], path[1]) / 8.001 * 1000.0;
    assert!((duration_ms as f32 - expect_ms).abs() < 5.0);

    let events = run(&mut realm, 3.0);
    assert!(
        swings(&events, mob.guid).count() >= 1,
        "it arrives and swings"
    );
}

#[test]
fn a_swing_out_of_reach_is_refused_once_until_the_mob_closes_in() {
    use benilla_protocol::messages::AttackSwingError;
    let (mut realm, me, mob) = warrior_at(1, 0);
    let at = toward(mob.position, CROSSROADS_XY, 8.0);
    let facing = (mob.position[1] - at[1]).atan2(mob.position[0] - at[0]);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: at,
        orientation: facing,
    });
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let events = run(&mut realm, 3.0);
    let refusals = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                SessionEvent::AttackSwingError(AttackSwingError::NotInRange)
            )
        })
        .count();
    assert_eq!(refusals, 1);
    // The mob runs in, and the waiting swing goes off.
    assert!(swings(&events, me).count() >= 1);
}

#[test]
fn a_swing_facing_away_is_refused_once() {
    use benilla_protocol::messages::AttackSwingError;
    let (mut realm, _, mob) = warrior_at(1, 0);
    // Right next to it, but facing away.
    stand_by(&mut realm, &mob);
    let away = toward(mob.position, CROSSROADS_XY, 2.0);
    let facing_away = (away[1] - mob.position[1]).atan2(away[0] - mob.position[0]);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: away,
        orientation: facing_away,
    });
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let events = run(&mut realm, 1.0);
    let refusals: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::AttackSwingError(r) => Some(*r),
            _ => None,
        })
        .collect();
    assert_eq!(refusals, [AttackSwingError::BadFacing]);
}

#[test]
fn killing_the_mob_pays_vanilla_kill_xp() {
    let (mut realm, me, mob) = warrior_at(5, 0);
    stand_by(&mut realm, &mob);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let events = run_until_dead(&mut realm, &mob);

    assert!(events.iter().any(
        |e| matches!(e, SessionEvent::AttackStop { attacker, victim } if *attacker == me && *victim == mob.guid)
    ));
    let gain = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::XpGain(x) => Some(*x),
            _ => None,
        })
        .expect("kill XP");
    // Level 5 kills level 3: 70 × 3/5.
    assert_eq!((gain.victim, gain.total, gain.kill), (mob.guid, 42, true));
    assert_eq!(
        own_values(&events, me).filter_map(|f| f.player_xp()).last(),
        Some(42)
    );
}

#[test]
fn a_gray_mob_pays_nothing() {
    let (mut realm, _, mob) = warrior_at(9, 0);
    stand_by(&mut realm, &mob);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let events = run_until_dead(&mut realm, &mob);
    assert!(!events.iter().any(|e| matches!(e, SessionEvent::XpGain(_))));
}

#[test]
fn enough_xp_levels_the_character_up() {
    // 52 XP for the kill takes a level-4 warrior from 2060 past the 2100 to level 5.
    let (mut realm, me, mob) = warrior_at(4, 2060);
    stand_by(&mut realm, &mob);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let events = run_until_dead(&mut realm, &mob);

    let up = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::LevelUp(l) => Some(*l),
            _ => None,
        })
        .expect("a level-up");
    assert_eq!(up.level, 5);
    // Human level 4 → 5: base health 47 → 56, stamina 25 → 26 (+10 health).
    assert_eq!(up.health, 19);
    assert_eq!(up.stats, [2, 1, 1, 0, 0]);

    let mut sheet = benilla_protocol::ObjectFields::default();
    for f in own_values(&events, me) {
        sheet.merge(f.clone());
    }
    assert_eq!(sheet.unit_level(), Some(5));
    assert_eq!(sheet.player_xp(), Some(12));
    assert_eq!(sheet.player_next_level_xp(), Some(2800));
    assert_eq!(sheet.unit_max_health(), Some(136));
    assert_eq!(sheet.unit_health(), Some(136), "a level-up heals");
    assert_eq!(sheet.unit_attack_power(), Some(51));

    // The level is the character's now: the roster shows it, and a save was asked for.
    assert_eq!(roster(&mut realm)[0].level, 5);
    assert!(realm.drain_storage().iter().any(
        |s| matches!(s, twilight_realm::Storage::Write { blob, .. } if blob.contains("level = 5"))
    ));
}

#[test]
fn rage_drains_once_the_fight_is_over() {
    let (mut realm, me, mob) = warrior_at(5, 0);
    stand_by(&mut realm, &mob);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let events = run_until_dead(&mut realm, &mob);
    let rage = own_values(&events, me)
        .filter_map(|f| f.unit_power(1))
        .last()
        .unwrap();
    assert!(rage > 20);

    // Two rage every two seconds.
    let events = run(&mut realm, 2.05);
    let after: Vec<u32> = own_values(&events, me)
        .filter_map(|f| f.unit_power(1))
        .collect();
    assert_eq!(after, [rage - 20]);
    let events = run(&mut realm, 120.0);
    assert_eq!(
        own_values(&events, me)
            .filter_map(|f| f.unit_power(1))
            .last(),
        Some(0)
    );
}

/// `UNIT_FLAG_PVP_ATTACKABLE`: the player is player-controlled. Without it the client's
/// `CanAttack` lets the player attack only hostile mobs, never neutral ones.
const UNIT_FLAG_PVP_ATTACKABLE: u32 = 0x8;
/// `UNIT_FLAG_IN_COMBAT`.
const UNIT_FLAG_IN_COMBAT: u32 = 0x0008_0000;
/// `UNIT_FIELD_FLAGS`.
const UNIT_FIELD_FLAGS: u16 = 46;

#[test]
fn the_player_is_player_controlled_from_entry_through_a_fight() {
    let mut fresh = realm();
    let (guid, events) = enter_new(&mut fresh, "Anduin", HUMAN);
    assert_ne!(
        own_create(&events, guid).unit_flags() & UNIT_FLAG_PVP_ATTACKABLE,
        0,
        "the create carries the player-controlled flag"
    );

    let (mut realm, me, mob) = warrior_at(5, 0);
    stand_by(&mut realm, &mob);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let mut events = run_until_dead(&mut realm, &mob);
    events.extend(run(&mut realm, 10.0));
    let flags: Vec<u32> = own_values(&events, me)
        .filter_map(|f| {
            f.raw_fields()
                .find_map(|(i, v)| (i == UNIT_FIELD_FLAGS).then_some(v))
        })
        .collect();
    assert!(
        flags.iter().any(|f| f & UNIT_FLAG_IN_COMBAT != 0),
        "the fight sets the combat flag: {flags:x?}"
    );
    assert!(
        flags.iter().any(|f| f & UNIT_FLAG_IN_COMBAT == 0),
        "the fight's end clears it: {flags:x?}"
    );
    assert!(
        flags.iter().all(|f| f & UNIT_FLAG_PVP_ATTACKABLE != 0),
        "no flags update drops the player-controlled flag: {flags:x?}"
    );
}

// ── Helpers ───────────────────────────────────────────────────────────────────────────────────

const CROSSROADS_XY: [f32; 3] = [CROSSROADS[0], CROSSROADS[1], 0.0];
const SEED: u64 = 4;

/// A human warrior of `level` with `xp`, in the world (on flat ground), and the mob near it.
fn warrior_at(level: u8, xp: u32) -> (Realm, u64, Mob) {
    warrior_rolling(level, xp, SEED)
}

/// A human warrior of `level` whose fights roll from `seed`, and the mob near it.
fn warrior_seeded(level: u8, seed: u64) -> (Realm, u64, Mob) {
    warrior_rolling(level, 0, seed)
}

fn warrior_rolling(level: u8, xp: u32, seed: u64) -> (Realm, u64, Mob) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace("xp = 0\n", &format!("xp = {xp}\n"));
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    let mob = the_mob(&events);
    (realm, guid, mob)
}

/// Moves the player to 2 yd from `mob`, on the Crossroads side, facing it.
fn stand_by(realm: &mut Realm, mob: &Mob) {
    let at = toward(mob.position, CROSSROADS_XY, 2.0);
    let facing = (mob.position[1] - at[1]).atan2(mob.position[0] - at[0]);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: at,
        orientation: facing,
    });
}

/// The point `yards` from `from` toward `to`, at `from`'s height.
fn toward(from: [f32; 3], to: [f32; 3], yards: f32) -> [f32; 3] {
    let (dx, dy) = (to[0] - from[0], to[1] - from[1]);
    let len = (dx * dx + dy * dy).sqrt();
    [
        from[0] + dx / len * yards,
        from[1] + dy / len * yards,
        from[2],
    ]
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Ticks `secs` in 0.1 s steps (the first tick at once), collecting the events.
fn run(realm: &mut Realm, secs: f32) -> Vec<SessionEvent> {
    let mut events = Vec::new();
    realm.tick(0.0, &FlatGround);
    events.extend(drain(realm));
    let mut t = 0.0;
    while t < secs - 1e-3 {
        realm.tick(0.1, &FlatGround);
        events.extend(drain(realm));
        t += 0.1;
    }
    events
}

/// Ticks until `mob`'s health reaches 0 (at most two minutes), collecting the events.
fn run_until_dead(realm: &mut Realm, mob: &Mob) -> Vec<SessionEvent> {
    let mut events = run(realm, 0.0);
    for _ in 0..1200 {
        if last_health(&events, mob.guid) == Some(0) {
            return events;
        }
        realm.tick(0.1, &FlatGround);
        events.extend(drain(realm));
    }
    panic!("the mob outlived two minutes of fighting");
}

/// The melee reports with `attacker` as the attacker.
fn swings(
    events: &[SessionEvent],
    attacker: u64,
) -> impl Iterator<Item = &benilla_protocol::messages::AttackerState> {
    events.iter().filter_map(move |e| match e {
        SessionEvent::AttackerState(s) if s.attacker == attacker => Some(s),
        _ => None,
    })
}

/// Every values update for `guid`.
fn own_values(
    events: &[SessionEvent],
    guid: u64,
) -> impl Iterator<Item = &benilla_protocol::ObjectFields> {
    events.iter().filter_map(move |e| match e {
        SessionEvent::ObjectValues { guid: g, fields } if *g == guid => Some(fields),
        _ => None,
    })
}

fn last_health(events: &[SessionEvent], guid: u64) -> Option<u32> {
    own_values(events, guid)
        .filter_map(|f| f.unit_health())
        .last()
}

fn own_create(events: &[SessionEvent], guid: u64) -> benilla_protocol::ObjectFields {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid: g, fields, ..
            } if *g == guid => Some(fields.clone()),
            _ => None,
        })
        .expect("our own create")
}

/// The mob's create, out of the entry burst.
struct Mob {
    guid: u64,
    position: [f32; 3],
    fields: benilla_protocol::ObjectFields,
}

fn the_mob(events: &[SessionEvent]) -> Mob {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                fields,
                ..
            } => Some(Mob {
                guid: *guid,
                position: *position,
                fields: fields.clone(),
            }),
            _ => None,
        })
        .expect("a mob streams in with the world")
}
