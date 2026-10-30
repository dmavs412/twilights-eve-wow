//! The Ranger's movement (#125): Vault's leap back and its cooldown. Played through the realm's
//! seam by a level 10 Ranger with a Cracked Shortbow near the Crossroads.

mod common;
mod gear_common;

use benilla_protocol::SessionEvent;
use common::fight::{cast, refusal, View};
use common::*;
use gear_common::save_shooting;
use twilight_realm::{Realm, RealmCommand};

const RANGED: u8 = 17;
const CRACKED_SHORTBOW: u32 = 2773;

static VAULT: std::sync::LazyLock<u32> = std::sync::LazyLock::new(|| spell("ranger", "vault"));

const SPELL_FAILED_NOT_READY: u8 = 0x3c;

/// A level 10 Ranger with a bow, in the world at `seed`: the realm and the client's view.
fn ranger(seed: u64) -> (Realm, View) {
    let (guid, blob) = save_shooting(10, &[(RANGED, CRACKED_SHORTBOW, 1)]);
    let blob = blob.replace("calling = \"basic_warrior\"", "calling = \"ranger\"");
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    (realm, view)
}

/// Where the player was teleported to in `events`.
fn teleported(events: &[SessionEvent]) -> Option<[f32; 3]> {
    events.iter().find_map(|e| match e {
        SessionEvent::Teleport { position, .. } => Some(*position),
        _ => None,
    })
}

#[test]
fn vault_leaps_15_yards_back_and_waits_15_seconds() {
    let (mut realm, mut view) = ranger(0xB0_0001);
    let start = [-441.8, -2648.6, 0.0];
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: start,
        orientation: 0.0,
    });
    view.run(&mut realm, 1.0);
    realm.handle(cast(*VAULT, None));
    let events = view.run(&mut realm, 0.1);
    assert_eq!(refusal(&events, *VAULT), None);
    let to = teleported(&events).expect("Vault moves the Ranger");
    // Facing east (orientation 0), so straight back is west.
    assert!((to[0] - (start[0] - 15.0)).abs() < 0.01, "to {to:?}");
    assert!((to[1] - start[1]).abs() < 0.01, "to {to:?}");

    view.run(&mut realm, 14.0);
    realm.handle(cast(*VAULT, None));
    assert_eq!(
        refusal(&view.drain(&mut realm), *VAULT),
        Some(SPELL_FAILED_NOT_READY)
    );
    view.run(&mut realm, 1.0);
    realm.handle(cast(*VAULT, None));
    assert_eq!(refusal(&view.drain(&mut realm), *VAULT), None);
}

// Ranger's Talent Rows (#507): a shipped Ranger of 24, its rows set, 25 yd west of the sturdy
// scorpid south of the Crossroads (`common::vanguard::STURDY`) and facing it.

use common::fight::{flat, stand_at, Mob};
use common::initiate::lasts;
use common::ranger::{
    face_from, gone_off, leap, mana, ranger_choice, ranger_save, ranger_scene, shots, ROWS_OPEN,
};
use common::vanguard::{scene_of, Scene, STURDY};

/// The sturdy scorpid's entry (`content/barrens.toml`, 9000034).
const STURDY_ENTRY: u32 = 3113;

/// Parting Gift: the trap left where the Ranger stood roots the scorpid that runs over it after
/// them for 4 s. Plain Vault leaves no trap.
#[test]
fn parting_gifts_trap_roots_the_mob_that_walks_onto_the_spot_for_4_seconds() {
    let gift = ranger_choice("vault", "parting_gift");
    let rooted = spell("ranger", "aura_parting_gift");
    let snapshot = spell("ranger", "snapshot");
    for (rows, leap_id, trap) in [
        (vec![("vault", gift)], gift, true),
        (vec![], spell("ranger", "vault"), false),
    ] {
        let mut s = ranger_scene(0x507_6000, &rows, STURDY);
        let mob = s.mob.guid;
        s.cast_at_mob(snapshot);
        let events = leap(&mut s.realm, &mut s.view, leap_id);
        let objects = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    SessionEvent::ObjectCreate {
                        kind: benilla_protocol::EntityKind::DynamicObject,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(objects, usize::from(trap), "{rows:?}: the trap is laid");
        let mut held = false;
        for _ in 0..60 {
            s.view.run(&mut s.realm, 0.1);
            if s.view.aura(mob, rooted).is_some() {
                held = true;
                break;
            }
        }
        assert_eq!(held, trap, "{rows:?}");
        if trap {
            let secs = lasts(&mut s.realm, &mut s.view, mob, rooted, 6.0);
            assert!((secs - 4.0).abs() <= 0.15, "rooted {secs} s");
        }
    }
}

/// Parting Shot: once the leap lands, the Ranger's current Snapshot goes off at the selected
/// scorpid, free: plain Snapshot, or the Snapshot choice set on its row. At a dead target nothing
/// goes off.
#[test]
fn parting_shot_fires_the_current_snapshot_after_the_leap_and_nothing_at_a_dead_target() {
    let parting = ranger_choice("vault", "parting_shot");
    let draw = ranger_choice("snapshot", "rhythmic_draw");
    for (rows, shot) in [
        (vec![("vault", parting)], spell("ranger", "snapshot")),
        (vec![("vault", parting), ("snapshot", draw)], draw),
    ] {
        let mut s = ranger_scene(0x507_7000, &rows, STURDY);
        let mob = s.mob.guid;
        // 15 yd off, so the leap lands 30 yd off, inside Snapshot's 35.
        let at = s.mob.clone();
        face_from(&mut s.realm, &mut s.view, &at, 15.0);
        s.realm.handle(RealmCommand::SetSelection { guid: mob });
        s.view.drain(&mut s.realm);
        let before = mana(&s.view, s.me);
        let events = leap(&mut s.realm, &mut s.view, parting);
        let teleport = events
            .iter()
            .position(|e| matches!(e, SessionEvent::Teleport { .. }))
            .expect("the leap lands");
        let go = events
            .iter()
            .position(|e| {
                matches!(e, SessionEvent::SpellGo { spell_id, target, .. }
                    if *spell_id == shot && *target == Some(mob))
            })
            .unwrap_or_else(|| panic!("{rows:?}: the shot goes off at the scorpid: {events:?}"));
        assert!(go > teleport, "after the leap lands");
        assert_eq!(gone_off(&events, shot), 1);
        assert_eq!(mana(&s.view, s.me), before, "the shot is free");
    }

    // The level 3 scorpid at the camp's middle dies to a few Snapshots; Point Blank keeps them
    // going once it reaches the Ranger.
    let point_blank = ranger_choice("talents", "point_blank");
    let snapshot = spell("ranger", "snapshot");
    let mut s = ranger_scene(
        0x507_7100,
        &[("vault", parting), ("talents", point_blank)],
        common::fight::CAMP,
    );
    let mob = s.mob.guid;
    for _ in 0..20 {
        if s.view.health(mob) == 0 {
            break;
        }
        s.realm.handle(cast(snapshot, Some(mob)));
        s.view.drain(&mut s.realm);
        s.view.run(&mut s.realm, 0.3);
    }
    assert_eq!(s.view.health(mob), 0, "the scorpid is dead");
    let at = s.mob.clone();
    face_from(&mut s.realm, &mut s.view, &at, 15.0);
    s.realm.handle(RealmCommand::SetSelection { guid: mob });
    s.view.drain(&mut s.realm);
    let events = leap(&mut s.realm, &mut s.view, parting);
    assert_eq!(gone_off(&events, snapshot), 0, "nothing at a dead target");
}

/// The run speeds the client was told of for `me` in `events`, in order.
fn speeds(events: &[SessionEvent], me: u64) -> Vec<f32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ForceSpeedChange { guid, speed, .. } if *guid == me => Some(*speed),
            _ => None,
        })
        .collect()
}

/// A Ranger of 24 with `rows` set beside the sturdy scorpid, which casts the Snaring affix's
/// Hamstring: once it has snared them, the realm, the view, their guid and the snare's spell.
fn hamstrung(rows: &[(&str, u32)]) -> (Realm, View, u64, u32) {
    let hamstring = creature_spell("hunter_hamstring");
    let (me, blob) = ranger_save(ROWS_OPEN);
    let mut realm =
        Realm::with_seed(0x507_8000).with_creature_spells(STURDY_ENTRY, &["hunter_hamstring"]);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    for (row, choice) in rows {
        realm.handle(RealmCommand::SetTalentRow {
            row: row.to_string(),
            spell_id: *choice,
        });
    }
    view.run(&mut realm, 1.0);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [STURDY[0] - 25.0, STURDY[1], 0.0],
        orientation: 0.0,
    });
    let mob = view
        .run(&mut realm, 1.0)
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: benilla_protocol::EntityKind::Unit,
                position,
                ..
            } if flat(*position, STURDY) < 1.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .expect("the sturdy scorpid streams in");
    stand_at(&mut realm, &mob, 2.0);
    for _ in 0..150 {
        view.run(&mut realm, 0.1);
        if view.aura(me, hamstring).is_some() {
            return (realm, view, me, hamstring);
        }
    }
    panic!("the scorpid never hamstrung the Ranger");
}

/// Unbound Leap: the leap takes a snare off the Ranger and they run 40% faster for 3 s. Plain
/// Vault leaves the snare on.
#[test]
fn unbound_leap_sheds_a_snare_and_runs_40_pct_faster_for_3_seconds() {
    const RUN: f32 = 7.0;
    let unbound = ranger_choice("vault", "unbound_leap");
    let (mut realm, mut view, me, snare) = hamstrung(&[("vault", unbound)]);
    let events = leap(&mut realm, &mut view, unbound);
    assert!(view.aura(me, snare).is_none(), "the snare is shed");
    assert_eq!(
        speeds(&events, me).last().copied(),
        Some(RUN * 1.4),
        "{events:?}"
    );
    let after = view.run(&mut realm, 3.1);
    assert_eq!(speeds(&after, me).last().copied(), Some(RUN), "{after:?}");

    let (mut realm, mut view, me, snare) = hamstrung(&[]);
    leap(&mut realm, &mut view, spell("ranger", "vault"));
    assert!(
        view.aura(me, snare).is_some(),
        "plain Vault keeps the snare"
    );
}

/// Steps the Ranger north and back every 0.1 s for `secs`, facing east at the scorpid: the
/// events, which hold the Auto Shots that went off.
fn shuffle(s: &mut Scene, secs: f32) -> Vec<SessionEvent> {
    let at = [STURDY[0] - 25.0, STURDY[1], 0.0];
    let mut events = Vec::new();
    for i in 0..(secs / 0.1).round() as u32 {
        let north = if i % 2 == 0 { 0.5 } else { 0.0 };
        s.realm.handle(RealmCommand::Move {
            motion: Default::default(),
            position: [at[0], at[1] + north, 0.0],
            orientation: 0.0,
        });
        events.extend(s.view.run(&mut s.realm, 0.1));
    }
    events
}

/// Running Fire: while its buff is up, Auto Shot keeps firing as the Ranger moves; under plain
/// Rapid Fire it holds. Point Blank keeps the shots going once the scorpid reaches the Ranger.
#[test]
fn running_fires_auto_shot_fires_while_moving_where_rapid_fires_holds() {
    let running = ranger_choice("rapid_fire", "running_fire");
    let point_blank = ranger_choice("talents", "point_blank");
    let auto_shot = spell("ranger", "auto_shot");
    let cases = [
        (
            vec![("rapid_fire", running), ("talents", point_blank)],
            running,
            true,
        ),
        (
            vec![("talents", point_blank)],
            spell("ranger", "rapid_fire"),
            false,
        ),
    ];
    for (rows, buff, fires) in cases {
        let mut s = ranger_scene(0x507_9000, &rows, STURDY);
        let mob = s.mob.guid;
        s.realm.handle(cast(buff, None));
        s.view.drain(&mut s.realm);
        s.realm.handle(cast(auto_shot, Some(mob)));
        let events = shuffle(&mut s, 4.0);
        let fired = gone_off(&events, auto_shot);
        if fires {
            assert!(fired >= 2, "{fired} Auto Shots on the move");
        } else {
            assert_eq!(fired, 0, "plain Rapid Fire's Auto Shot holds");
        }
    }
}

/// Walks east from `from` for `secs`, a step every 0.1 s, as a moving client reports.
fn walk(realm: &mut Realm, view: &mut View, from: [f32; 3], secs: f32) -> [f32; 3] {
    let mut at = from;
    for _ in 0..(secs / 0.1).round() as u32 {
        at[0] += 0.7;
        realm.handle(RealmCommand::Move {
            motion: Default::default(),
            position: at,
            orientation: 0.0,
        });
        view.run(realm, 0.1);
    }
    at
}

/// Guerilla Tactics: each 2 s of moving puts a stack up, three at most, and the next Snapshot
/// deals 50% more a stack and spends them. The same seed with the row empty is the yardstick.
#[test]
fn guerilla_tactics_stacks_on_the_move_and_the_next_snapshot_spends_them() {
    let tactics = ranger_choice("talents", "guerilla_tactics");
    let buff = spell("ranger", "aura_guerilla_tactics_5");
    let snapshot = spell("ranger", "snapshot");
    let (me, blob) = ranger_save(ROWS_OPEN);
    let mut s = scene_of(me, &blob, 0x507_A000, &[("talents", tactics)], STURDY);
    let start = [STURDY[0] - 60.0, STURDY[1], 0.0];
    let at = walk(&mut s.realm, &mut s.view, start, 1.8);
    assert!(s.view.aura(me, buff).is_none(), "not before 2 s");
    let at = walk(&mut s.realm, &mut s.view, at, 0.4);
    assert_eq!(s.view.aura(me, buff).expect("on at 2 s").stacks, 1);
    walk(&mut s.realm, &mut s.view, at, 4.4);
    assert_eq!(s.view.aura(me, buff).unwrap().stacks, 3, "three at most");

    let mut compared = 0;
    for seed in 0..20 {
        let mut dealt = Vec::new();
        for rows in [vec![], vec![("talents", tactics)]] {
            let (me, blob) = ranger_save(ROWS_OPEN);
            let mut s = scene_of(me, &blob, 0x507_A100 + seed, &rows, STURDY);
            let start = [STURDY[0] - 60.0, STURDY[1], 0.0];
            walk(&mut s.realm, &mut s.view, start, 2.2);
            let mob = s.mob.clone();
            face_from(&mut s.realm, &mut s.view, &mob, 25.0);
            let events = s.cast_at_mob(snapshot);
            let hit = shots(&events, snapshot).first().copied();
            if !rows.is_empty() && hit.is_some() {
                assert!(s.view.aura(me, buff).is_none(), "the Snapshot spent it");
            }
            dealt.push(hit.filter(|&(_, _, crit)| !crit).map(|(_, d, _)| d));
        }
        if let [Some(plain), Some(more)] = dealt[..] {
            let want = plain as f32 * 1.5;
            assert!(
                (more as f32 - want).abs() <= 1.5,
                "seed {seed}: {plain} then {more}"
            );
            compared += 1;
        }
    }
    assert!(compared >= 5, "{compared} plain hits compared");
}
