//! A shipped Cutthroat for the Talent Row scripts (#505): a save of any level, fists only when
//! `unarmed`, its rows set through the realm's seam as `talent_rows.rs` sets them, and the
//! Energy and combo points its client shows.

use benilla_protocol::{EntityKind, SessionEvent};
use twilight_realm::{Realm, RealmCommand};

use super::fight::*;
use super::vanguard::{enter, learned, scene_of, set_row, Scene};
use super::*;

/// Every Cutthroat row is open at 24, the level the row scripts play at.
pub const ROWS_OPEN: u8 = 24;

/// The main and off hand slots, empty for a fight with fists.
const MAIN_HAND: u8 = 15;
const OFF_HAND: u8 = 16;
/// `Powers` of Energy.
const ENERGY: u8 = 3;

/// A new human's save as a Cutthroat of `level`, fists only when `unarmed`: its guid and the blob.
pub fn cutthroat_save(level: u8, unarmed: bool) -> (u64, String) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Garrote", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"cutthroat\"");
    let blob = if unarmed {
        drop_slot(&drop_slot(&blob, MAIN_HAND), OFF_HAND)
    } else {
        blob
    };
    (guid, blob)
}

/// Sets each `(row, choice)` through the seam, each learned.
pub fn set_rows(realm: &mut Realm, view: &mut View, rows: &[(&str, u32)]) {
    for (row, choice) in rows {
        let events = set_row(realm, view, row, *choice);
        assert!(
            learned(&events, *choice),
            "{row} takes {choice}: {events:?}"
        );
    }
}

/// A Cutthroat of `level` on `seed`'s dice with each `(row, choice)` set, 2 yd from the mob at
/// `spot` and facing it.
pub fn cutthroat_scene(
    level: u8,
    unarmed: bool,
    seed: u64,
    rows: &[(&str, u32)],
    spot: [f32; 3],
) -> Scene {
    let (me, blob) = cutthroat_save(level, unarmed);
    let mut s = scene_of(me, &blob, seed, rows, spot);
    let mob = s.mob.clone();
    stand_at(&mut s.realm, &mob, 2.0);
    s
}

/// The scorpid camp's mobs the client has been told of in `events`.
pub fn camp_mobs(events: &[SessionEvent]) -> Vec<Mob> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                ..
            } if flat(*position, CAMP) < 15.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .collect()
}

/// The scorpid camp pulled onto a Cutthroat west of it, facing east.
pub struct Pack {
    pub realm: Realm,
    pub view: View,
    pub me: u64,
    pub mobs: Vec<Mob>,
}

/// Melee reach, with room for a mob that stopped a step short.
const REACH: f32 = 5.0;
const WEST: [f32; 3] = [CAMP[0] - 25.0, CAMP[1], 0.0];

/// A Cutthroat of `level` on `seed`'s dice, fists only when `unarmed`, with each `(row, choice)`
/// set, and the camp pulled onto them: every mob has stopped within reach.
pub fn pulled(seed: u64, level: u8, unarmed: bool, rows: &[(&str, u32)]) -> Pack {
    let (me, blob) = cutthroat_save(level, unarmed);
    let (mut realm, mut view, _) = enter(me, &blob, seed);
    set_rows(&mut realm, &mut view, rows);
    view.run(&mut realm, 1.0);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: WEST,
        orientation: 0.0,
    });
    let streamed = view.run(&mut realm, 1.0);
    let mut mobs = camp_mobs(&streamed);
    assert_eq!(mobs.len(), 4, "the scorpid camp streams in");
    for mob in &mobs {
        pull(&mut realm, &mut view, mob);
    }
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: WEST,
        orientation: 0.0,
    });
    for _ in 0..600 {
        for e in view.run(&mut realm, 0.1) {
            if let SessionEvent::MonsterMove { guid, path, .. } = e {
                if let (Some(mob), Some(end)) =
                    (mobs.iter_mut().find(|m| m.guid == guid), path.last())
                {
                    mob.position = *end;
                }
            }
        }
        assert!(view.health(me) > 0, "died waiting for the camp");
        if mobs.iter().all(|m| flat(m.position, WEST) <= REACH - 1.0) {
            // The paths end in reach; the mobs have still to walk them.
            view.run(&mut realm, 10.0);
            assert!(view.health(me) > 0, "died waiting for the camp");
            return Pack {
                realm,
                view,
                me,
                mobs,
            };
        }
    }
    panic!("the camp never came to hand");
}

pub fn energy(view: &View, me: u64) -> u32 {
    view.fields(me).unit_power(ENERGY).unwrap()
}

/// The combo points the client shows, and the guid they are on.
pub fn combo(view: &View, me: u64) -> (u8, u64) {
    let f = view.fields(me);
    (
        f.player_combo_points().unwrap_or(0),
        f.player_combo_target(),
    )
}

/// Ticks until the bar holds `points` Energy, at most 10 s: the events meanwhile.
pub fn wait_for_energy(
    realm: &mut Realm,
    view: &mut View,
    me: u64,
    points: u32,
) -> Vec<SessionEvent> {
    let mut events = Vec::new();
    for _ in 0..100 {
        if energy(view, me) >= points {
            return events;
        }
        events.extend(view.run(realm, 0.1));
    }
    panic!("the bar never held {points} Energy");
}

/// Casts `spell_id` at `target` and stops the swing it starts: what it sent, or `None` when the
/// cast was refused.
pub fn try_cast(
    realm: &mut Realm,
    view: &mut View,
    spell_id: u32,
    target: u64,
) -> Option<Vec<SessionEvent>> {
    realm.handle(cast(spell_id, Some(target)));
    let events = view.drain(realm);
    realm.handle(RealmCommand::AttackStop);
    refusal(&events, spell_id).is_none().then_some(events)
}

/// The power each `SpellEnergizeLog` of `spell_id` in `events` gave.
pub fn energized(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellEnergizeLog(l) if l.spell_id == spell_id => Some(l.amount),
            _ => None,
        })
        .collect()
}

/// The health each `SpellHealLog` of `spell_id` in `events` gave.
pub fn healed(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellHealLog(l) if l.spell_id == spell_id => Some(l.amount),
            _ => None,
        })
        .collect()
}

/// The damage `spell_id` logged on each target in `events`, and whether each crit.
pub fn damage_of(events: &[SessionEvent], spell_id: u32) -> Vec<(u64, u32, bool)> {
    /// `SpellDamageLog::hit_info` of a crit.
    const CRIT: u32 = 0x2;
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id => {
                Some((l.target, l.damage, l.hit_info & CRIT != 0))
            }
            _ => None,
        })
        .collect()
}
