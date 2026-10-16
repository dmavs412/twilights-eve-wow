//! The Hearthstone: every character carries one, and using it casts the stock hearth spell, which
//! brings the player back to their bind point and then rests for five minutes. The innkeeper
//! binds it through the stock binder gossip and confirm; a character starts bound to the
//! Crossroads inn.

mod common;
mod gear_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::{descriptor, stand_by, the_mob};
use twilight_realm::CROSSROADS;

const HEARTHSTONE: u32 = 6948;
/// `Spell.dbc` 8690, "Hearthstone": a 10 s cast.
const HEARTH_SPELL: u32 = 8690;
const CAST_MS: u32 = 10_000;
const COOLDOWN_MS: u32 = 300_000;

// `SpellCastResult`.
const SPELL_FAILED_INTERRUPTED: u8 = 0x23;
const SPELL_FAILED_NOT_READY: u8 = 0x3c;
const SPELL_FAILED_SPELL_IN_PROGRESS: u8 = 0x61;

/// The backpack's first slot, where a new character's Hearthstone sits.
const FIRST_PACK_SLOT: u8 = 23;
/// The player's own slots, as `CMSG_USE_ITEM` names them.
const PLAYER_BAG: u8 = 255;

/// Far out past The Barrens' camps: nothing streams in to bother the cast.
const FAR_AWAY: [f32; 3] = [CROSSROADS[0] + 3000.0, CROSSROADS[1], 0.0];

fn use_hearthstone() -> RealmCommand {
    RealmCommand::UseItem {
        bag: PLAYER_BAG,
        slot: FIRST_PACK_SLOT,
    }
}

/// A new warrior, walked out far from the Crossroads; its guid and its Hearthstone's guid.
fn far_from_home(realm: &mut Realm) -> (u64, u64) {
    let (me, events) = enter_new(realm, "Thrall", ORC);
    let stone = descriptor(&events, me)
        .player_pack_slot(0)
        .expect("something in the first backpack slot");
    realm.handle(RealmCommand::Move {
        position: FAR_AWAY,
        orientation: 0.0,
    });
    realm.tick(0.1, &FlatGround);
    drain(realm);
    (me, stone)
}

/// Ticks `secs` in 0.1 s steps, collecting the events.
fn run(realm: &mut Realm, secs: f32) -> Vec<SessionEvent> {
    let mut events = Vec::new();
    for _ in 0..(secs * 10.0).round() as u32 {
        realm.tick(0.1, &FlatGround);
        events.extend(drain(realm));
    }
    events
}

fn teleport(events: &[SessionEvent]) -> Option<([f32; 3], f32)> {
    events.iter().find_map(|e| match e {
        SessionEvent::Teleport {
            position,
            orientation,
            ..
        } => Some((*position, *orientation)),
        _ => None,
    })
}

fn spell_start(events: &[SessionEvent]) -> Option<(u64, u32, u32)> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellStart {
            caster,
            spell_id,
            cast_time_ms,
            ..
        } => Some((*caster, *spell_id, *cast_time_ms)),
        _ => None,
    })
}

/// The failure reason of the first failed cast result.
fn cast_failure(events: &[SessionEvent]) -> Option<(u32, Option<u8>)> {
    events.iter().find_map(|e| match e {
        SessionEvent::CastResult {
            spell_id,
            success: false,
            reason,
            ..
        } => Some((*spell_id, *reason)),
        _ => None,
    })
}

fn at_crossroads(position: [f32; 3]) -> bool {
    let [x, y, z, _] = CROSSROADS;
    (position[0] - x).abs() < 0.01
        && (position[1] - y).abs() < 0.01
        && (position[2] - z).abs() < 0.01
}

#[test]
fn a_new_character_carries_a_hearthstone_in_its_backpack() {
    let mut realm = realm();
    let (me, events) = enter_new(&mut realm, "Thrall", ORC);
    let stone = descriptor(&events, me)
        .player_pack_slot(0)
        .expect("an item in the first backpack slot");
    assert_eq!(descriptor(&events, stone).object_entry(), Some(HEARTHSTONE));
}

#[test]
fn the_hearthstone_is_used_for_the_hearth_spell_on_a_five_minute_cooldown() {
    let mut realm = realm();
    realm.handle(RealmCommand::ItemQuery { entry: HEARTHSTONE });
    let info = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::ItemTemplate { entry, info } if entry == HEARTHSTONE => info,
            _ => None,
        })
        .expect("the Hearthstone's template");
    assert_eq!(info.name, "Hearthstone");
    let use_spell = info.use_spell.expect("an on-use spell");
    assert_eq!(use_spell.spell_id, HEARTH_SPELL);
    assert_eq!(use_spell.cooldown_ms, COOLDOWN_MS as i32);
    // Not the spell's own hour-long category cooldown.
    assert_eq!(use_spell.category_cooldown_ms, 0);
    // The ON_USE block the client's `CMSG_USE_ITEM` names.
    assert_eq!(info.use_spell_index(), Some(0));
}

#[test]
fn using_it_casts_for_ten_seconds_then_returns_the_player_to_the_crossroads() {
    let mut realm = realm();
    let (me, stone) = far_from_home(&mut realm);

    realm.handle(use_hearthstone());
    let events = drain(&mut realm);
    assert_eq!(spell_start(&events), Some((me, HEARTH_SPELL, CAST_MS)));

    let events = run(&mut realm, 9.8);
    assert_eq!(teleport(&events), None, "still casting");

    let events = run(&mut realm, 0.3);
    assert!(events.iter().any(|e| matches!(
        e,
        SessionEvent::CastResult {
            spell_id: HEARTH_SPELL,
            success: true,
            ..
        }
    )));
    assert!(events.iter().any(|e| matches!(
        e,
        SessionEvent::SpellGo { caster, spell_id: HEARTH_SPELL, item_caster: Some(item), .. }
            if *caster == me && *item == stone
    )));
    let (position, orientation) = teleport(&events).expect("a teleport");
    assert!(at_crossroads(position), "{position:?}");
    assert_eq!(orientation, CROSSROADS[3]);

    // The realm has the player there too: logged out, the roster says so.
    realm.handle(RealmCommand::Logout);
    let c = roster(&mut realm)
        .into_iter()
        .find(|c| c.guid == me)
        .unwrap();
    assert!(at_crossroads([c.position.x, c.position.y, c.position.z]));
}

#[test]
fn a_move_sent_before_the_client_saw_the_teleport_does_not_undo_it() {
    let mut realm = realm();
    let (me, _) = far_from_home(&mut realm);
    realm.handle(use_hearthstone());
    let events = run(&mut realm, 10.1);
    let counter = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::Teleport { counter, .. } => Some(*counter),
            _ => None,
        })
        .expect("a teleport");

    // The client's movement from before the snap, then its ack.
    realm.handle(RealmCommand::Move {
        position: FAR_AWAY,
        orientation: 0.0,
    });
    realm.handle(RealmCommand::TeleportAck { counter });
    realm.handle(RealmCommand::Logout);
    let c = roster(&mut realm)
        .into_iter()
        .find(|c| c.guid == me)
        .unwrap();
    assert!(at_crossroads([c.position.x, c.position.y, c.position.z]));
}

#[test]
fn moving_interrupts_the_cast() {
    let mut realm = realm();
    let (me, _) = far_from_home(&mut realm);
    realm.handle(use_hearthstone());
    run(&mut realm, 3.0);

    realm.handle(RealmCommand::Move {
        position: [FAR_AWAY[0] + 1.0, FAR_AWAY[1], FAR_AWAY[2]],
        orientation: 0.0,
    });
    let events = run(&mut realm, 0.1);
    assert!(events.iter().any(|e| matches!(
        e,
        SessionEvent::SpellFailedOther { caster, spell_id: HEARTH_SPELL } if *caster == me
    )));
    assert_eq!(
        cast_failure(&events),
        Some((HEARTH_SPELL, Some(SPELL_FAILED_INTERRUPTED)))
    );

    assert_eq!(teleport(&run(&mut realm, 12.0)), None);
}

#[test]
fn turning_on_the_spot_does_not_interrupt_the_cast() {
    let mut realm = realm();
    let (_, _) = far_from_home(&mut realm);
    realm.handle(use_hearthstone());
    run(&mut realm, 3.0);
    realm.handle(RealmCommand::Move {
        position: FAR_AWAY,
        orientation: 2.0,
    });
    assert!(teleport(&run(&mut realm, 7.1)).is_some());
}

#[test]
fn an_interrupted_cast_puts_nothing_on_cooldown() {
    let mut realm = realm();
    far_from_home(&mut realm);
    realm.handle(use_hearthstone());
    run(&mut realm, 1.0);
    realm.handle(RealmCommand::Move {
        position: [FAR_AWAY[0] + 1.0, FAR_AWAY[1], FAR_AWAY[2]],
        orientation: 0.0,
    });
    run(&mut realm, 0.1);

    realm.handle(use_hearthstone());
    assert!(spell_start(&drain(&mut realm)).is_some());
}

#[test]
fn taking_damage_interrupts_the_cast() {
    let mut realm = realm();
    let (me, events) = enter_new(&mut realm, "Thrall", ORC);
    let full = descriptor(&events, me).unit_max_health().unwrap();
    let mob = the_mob(&events);
    stand_by(&mut realm, &mob);
    // Pull it, then stop swinging and hearth with it on us.
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    realm.handle(RealmCommand::AttackStop);
    drain(&mut realm);
    realm.handle(use_hearthstone());

    let mut events = drain(&mut realm);
    for _ in 0..100 {
        realm.tick(0.1, &FlatGround);
        events.extend(drain(&mut realm));
    }
    let hit = events.iter().position(|e| {
        matches!(e, SessionEvent::ObjectValues { guid, fields }
            if *guid == me && fields.unit_health().is_some_and(|h| h < full))
    });
    let failed = events.iter().position(|e| {
        matches!(
            e,
            SessionEvent::CastResult {
                success: false,
                reason: Some(SPELL_FAILED_INTERRUPTED),
                ..
            }
        )
    });
    let (hit, failed) = (
        hit.expect("the mob landed a hit"),
        failed.expect("an interrupt"),
    );
    assert!(hit < failed, "the hit came first");
    assert_eq!(teleport(&events), None);
}

#[test]
fn a_second_cast_while_casting_is_refused() {
    let mut realm = realm();
    far_from_home(&mut realm);
    realm.handle(use_hearthstone());
    drain(&mut realm);
    realm.handle(use_hearthstone());
    let events = drain(&mut realm);
    assert_eq!(spell_start(&events), None);
    assert_eq!(
        cast_failure(&events),
        Some((HEARTH_SPELL, Some(SPELL_FAILED_SPELL_IN_PROGRESS)))
    );
}

#[test]
fn after_hearthing_the_stone_rests_for_five_minutes() {
    let mut realm = realm();
    far_from_home(&mut realm);
    realm.handle(use_hearthstone());
    run(&mut realm, 10.1);

    realm.handle(use_hearthstone());
    let events = drain(&mut realm);
    assert_eq!(spell_start(&events), None);
    assert_eq!(
        cast_failure(&events),
        Some((HEARTH_SPELL, Some(SPELL_FAILED_NOT_READY)))
    );

    run(&mut realm, 299.0);
    realm.handle(use_hearthstone());
    assert_eq!(spell_start(&drain(&mut realm)), None, "not yet");

    run(&mut realm, 1.0);
    realm.handle(use_hearthstone());
    assert!(spell_start(&drain(&mut realm)).is_some(), "ready again");
}

/// A wall clock to start from: some afternoon in 2026, in Unix milliseconds.
const T0: u64 = 1_781_000_000_000;

/// The Hearthstone's cooldown in the spell book a login sends, in ms; `None` without one.
fn hearth_cooldown_at_login(events: &[SessionEvent]) -> Option<u32> {
    let cooldowns = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::SpellBook { cooldowns, .. } => Some(cooldowns.clone()),
            _ => None,
        })
        .expect("the spell book, with its cooldowns");
    let hearth = cooldowns
        .iter()
        .find(|c| u32::from(c.spell_id) == HEARTH_SPELL)?;
    assert_eq!(u32::from(hearth.item_id), HEARTHSTONE);
    Some(hearth.spell_cd_ms)
}

/// A character hearthed at `T0` (its cast over 10.2 s later) and then logged out, `waited`
/// seconds on: its save.
fn hearthed_and_saved(waited: f32) -> (u64, String) {
    let mut realm = realm();
    realm.set_wall_clock(T0);
    let (me, _) = far_from_home(&mut realm);
    realm.handle(use_hearthstone());
    run(&mut realm, 10.1 + waited);
    realm.handle(RealmCommand::Logout);
    drain(&mut realm);
    (me, realm.export_character(me).unwrap())
}

/// Logs the save in on a realm whose wall clock reads `at`.
fn log_in_at(at: u64, me: u64, blob: &str) -> (Realm, Vec<SessionEvent>) {
    let mut back = Realm::new();
    back.set_wall_clock(at);
    back.import_character(blob).unwrap();
    back.handle(RealmCommand::EnterWorld { guid: me });
    let events = drain(&mut back);
    (back, events)
}

#[test]
fn the_cooldown_is_kept_across_a_save_and_reload() {
    let (me, blob) = hearthed_and_saved(60.0);
    // Back at once: five minutes less the minute spent waiting.
    let (mut back, events) = log_in_at(T0 + 70_200, me, &blob);
    let left = hearth_cooldown_at_login(&events).expect("the Hearthstone's cooldown");
    assert!((239_000..=240_100).contains(&left), "{left}");

    back.handle(use_hearthstone());
    assert_eq!(
        cast_failure(&drain(&mut back)),
        Some((HEARTH_SPELL, Some(SPELL_FAILED_NOT_READY)))
    );
}

#[test]
fn time_spent_offline_counts_toward_the_cooldown() {
    let (me, blob) = hearthed_and_saved(0.0);
    // Four minutes away: one is left.
    let (_, events) = log_in_at(T0 + 10_200 + 240_000, me, &blob);
    let left = hearth_cooldown_at_login(&events).expect("a minute left");
    assert!((59_000..=60_100).contains(&left), "{left}");

    // Six minutes away: the stone has rested.
    let (mut back, events) = log_in_at(T0 + 10_200 + 360_000, me, &blob);
    assert_eq!(hearth_cooldown_at_login(&events), None);
    back.handle(RealmCommand::Move {
        position: FAR_AWAY,
        orientation: 0.0,
    });
    back.handle(use_hearthstone());
    assert!(spell_start(&drain(&mut back)).is_some());
}

#[test]
fn a_rested_stone_sends_no_cooldown_at_login() {
    let mut realm = realm();
    let (_, events) = enter_new(&mut realm, "Thrall", ORC);
    let cooldowns = events.iter().find_map(|e| match e {
        SessionEvent::SpellBook { cooldowns, .. } => Some(cooldowns.clone()),
        _ => None,
    });
    assert_eq!(cooldowns, Some(Vec::new()));
}

/// Innkeeper Boorand Plainswind, in the Crossroads inn.
const INNKEEPER: u32 = 3934;
/// `AreaTable.dbc`'s The Crossroads.
const THE_CROSSROADS: u32 = 380;
/// The gossip line's text and its icon (`GOSSIP_ICON_INTERACT_2`, the binder's).
const BIND_LINE: &str = "Make this inn your home.";
const ICON_BINDER: u8 = 5;

/// The bind line in the innkeeper's gossip menu: its index and icon.
fn bind_line(events: &[SessionEvent], innkeeper: u64) -> Option<(u32, u8)> {
    events.iter().find_map(|e| match e {
        SessionEvent::GossipMenu { npc, options, .. } if *npc == innkeeper => options
            .iter()
            .find(|o| o.message == BIND_LINE)
            .map(|o| (o.index, o.icon)),
        _ => None,
    })
}

/// Walks up to the innkeeper and picks the bind line, as the stock gossip frame does: the
/// events from the pick on.
fn ask_to_bind(realm: &mut Realm, innkeeper: u64) -> Vec<SessionEvent> {
    walk_up_to(realm, innkeeper);
    realm.handle(RealmCommand::GossipHello { guid: innkeeper });
    let (option, _) = bind_line(&drain(realm), innkeeper).expect("the bind line");
    realm.handle(RealmCommand::GossipSelectOption {
        guid: innkeeper,
        option,
    });
    drain(realm)
}

/// Binds at the innkeeper, accepting the confirm: where the player stood, and the events.
fn bind(realm: &mut Realm) -> (([f32; 3], f32), Vec<SessionEvent>) {
    let innkeeper = hub_guid(INNKEEPER);
    ask_to_bind(realm, innkeeper);
    realm.handle(RealmCommand::BinderActivate { binder: innkeeper });
    let [x, y, _] = hub_home(innkeeper);
    // Where `walk_up_to` stands the player.
    (([x + 1.5, y, 0.0], std::f32::consts::PI), drain(realm))
}

fn bound(events: &[SessionEvent]) -> Option<(u64, u32)> {
    events.iter().find_map(|e| match e {
        SessionEvent::PlayerBound { binder, area } => Some((*binder, *area)),
        _ => None,
    })
}

fn bind_point(events: &[SessionEvent]) -> Option<u32> {
    events.iter().find_map(|e| match e {
        SessionEvent::BindPoint { area } => Some(*area),
        _ => None,
    })
}

/// Walks far off, hearths and waits out the cast: where the teleport put the player.
fn hearth_home(realm: &mut Realm) -> ([f32; 3], f32) {
    realm.handle(RealmCommand::Move {
        position: FAR_AWAY,
        orientation: 0.0,
    });
    realm.tick(0.1, &FlatGround);
    drain(realm);
    realm.handle(use_hearthstone());
    teleport(&run(realm, 10.1)).expect("a teleport")
}

#[test]
fn a_login_names_the_bind_point_the_crossroads_inn() {
    let mut realm = realm();
    let (_, events) = enter_new(&mut realm, "Thrall", ORC);
    assert_eq!(bind_point(&events), Some(THE_CROSSROADS));
}

#[test]
fn the_innkeepers_bind_line_asks_with_the_stock_confirm_and_binds_nothing_yet() {
    let mut realm = realm();
    enter_new(&mut realm, "Thrall", ORC);
    let innkeeper = hub_guid(INNKEEPER);
    walk_up_to(&mut realm, innkeeper);
    realm.handle(RealmCommand::GossipHello { guid: innkeeper });
    let (_, icon) = bind_line(&drain(&mut realm), innkeeper).expect("the bind line");
    assert_eq!(icon, ICON_BINDER);

    let events = ask_to_bind(&mut realm, innkeeper);
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::GossipComplete)));
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::BinderConfirm { binder } if *binder == innkeeper)));
    assert_eq!(bound(&events), None, "only asked");
}

#[test]
fn confirming_binds_with_the_stock_messages() {
    let mut realm = realm();
    enter_new(&mut realm, "Thrall", ORC);
    let (_, events) = bind(&mut realm);
    assert_eq!(bind_point(&events), Some(THE_CROSSROADS));
    assert_eq!(bound(&events), Some((hub_guid(INNKEEPER), THE_CROSSROADS)));
}

#[test]
fn the_hearthstone_takes_the_player_to_where_they_bound() {
    let mut realm = realm();
    enter_new(&mut realm, "Thrall", ORC);
    let (home, _) = bind(&mut realm);
    assert!(!at_crossroads(home.0), "a spot of its own");
    assert_eq!(hearth_home(&mut realm), home);
}

#[test]
fn an_innkeeper_out_of_reach_binds_nothing() {
    let mut realm = realm();
    enter_new(&mut realm, "Thrall", ORC);
    let innkeeper = hub_guid(INNKEEPER);
    ask_to_bind(&mut realm, innkeeper);
    realm.handle(RealmCommand::Move {
        position: FAR_AWAY,
        orientation: 0.0,
    });
    realm.handle(RealmCommand::BinderActivate { binder: innkeeper });
    assert_eq!(bound(&drain(&mut realm)), None);
    let (position, _) = hearth_home(&mut realm);
    assert!(at_crossroads(position), "{position:?}");
}

#[test]
fn only_an_innkeeper_binds() {
    let mut realm = realm();
    enter_new(&mut realm, "Thrall", ORC);
    let barg = hub_guid(190001);
    walk_up_to(&mut realm, barg);
    realm.handle(RealmCommand::BinderActivate { binder: barg });
    assert_eq!(bound(&drain(&mut realm)), None);
}

#[test]
fn the_bind_survives_a_save_and_reload() {
    let mut realm = realm();
    let (me, _) = enter_new(&mut realm, "Thrall", ORC);
    let (home, _) = bind(&mut realm);
    realm.handle(RealmCommand::Logout);
    let blob = realm.export_character(me).unwrap();

    let (mut back, events) = log_in_at(0, me, &blob);
    assert_eq!(bind_point(&events), Some(THE_CROSSROADS));
    assert_eq!(hearth_home(&mut back), home);
}

#[test]
fn a_save_without_a_bind_loads_bound_to_the_crossroads_inn() {
    let mut realm = realm();
    let (me, _) = enter_new(&mut realm, "Thrall", ORC);
    bind(&mut realm);
    realm.handle(RealmCommand::Logout);
    let blob = realm.export_character(me).unwrap();
    let old = without_table(&blob, "bind");
    assert_ne!(old, blob, "the save carries its bind");

    let (mut back, events) = log_in_at(0, me, &old);
    assert_eq!(bind_point(&events), Some(THE_CROSSROADS));
    let (position, orientation) = hearth_home(&mut back);
    assert!(at_crossroads(position), "{position:?}");
    assert_eq!(orientation, CROSSROADS[3]);
}

/// `blob` without its `[name]` table.
fn without_table(blob: &str, name: &str) -> String {
    let header = format!("[{name}]");
    let mut out = String::new();
    let mut skipping = false;
    for line in blob.lines() {
        if line.starts_with('[') {
            skipping = line == header;
        }
        if !skipping {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}
