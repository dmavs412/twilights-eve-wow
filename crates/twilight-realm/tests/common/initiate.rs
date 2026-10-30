//! A shipped Initiate for the Talent Row scripts (#506): a save of any level, its rows set through
//! the realm's seam as `talent_rows.rs` sets them, the mana its client shows, and casts that wait
//! out their cast bars.

use benilla_protocol::SessionEvent;
use twilight_realm::{Realm, RealmCommand};

use super::cutthroat::set_rows;
use super::fight::*;
use super::vanguard::{enter, scene_of, Scene};
use super::*;

/// Every Initiate row is open at 24, the level the row scripts play at.
pub const ROWS_OPEN: u8 = 24;

/// `Powers` of mana.
const MANA: u8 = 0;

/// A new human's save as an Initiate of `level`: its guid and the blob.
pub fn initiate_save(level: u8) -> (u64, String) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Weaver", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"initiate\"");
    (guid, blob)
}

/// An Initiate of [`ROWS_OPEN`] on `seed`'s dice with each `(row, choice)` set, 25 yd west of the
/// mob at `spot` and facing it.
pub fn initiate_scene(seed: u64, rows: &[(&str, u32)], spot: [f32; 3]) -> Scene {
    let (me, blob) = initiate_save(ROWS_OPEN);
    scene_of(me, &blob, seed, rows, spot)
}

/// An Initiate of [`ROWS_OPEN`] in the world on `seed`'s dice with each `(row, choice)` set: the
/// realm, the view and their guid.
pub fn initiate_world(seed: u64, rows: &[(&str, u32)]) -> (Realm, View, u64) {
    let (me, blob) = initiate_save(ROWS_OPEN);
    let (mut realm, mut view, _) = enter(me, &blob, seed);
    set_rows(&mut realm, &mut view, rows);
    view.run(&mut realm, 1.0);
    (realm, view, me)
}

pub fn mana(view: &View, me: u64) -> u32 {
    view.fields(me).unit_power(MANA).unwrap()
}

pub fn max_mana(view: &View, me: u64) -> u32 {
    view.fields(me).unit_max_power(MANA).unwrap()
}

/// The spell id of the Initiate's skill or aura row `key` on the sheet.
pub fn initiate(key: &str) -> u32 {
    spell("initiate", key)
}

/// Casts `spell_id` at `target` and ticks until it goes off (a cast bar's length and its pushback,
/// at most 5 s): every event meanwhile. Panics when the cast is refused.
pub fn cast_through(
    realm: &mut Realm,
    view: &mut View,
    spell_id: u32,
    target: u64,
) -> Vec<SessionEvent> {
    realm.handle(cast(spell_id, Some(target)));
    let mut events = view.drain(realm);
    assert_eq!(
        refusal(&events, spell_id),
        None,
        "spell {spell_id} goes off: {events:?}"
    );
    realm.handle(RealmCommand::AttackStop);
    for _ in 0..50 {
        if went_off(&events, spell_id) {
            return events;
        }
        events.extend(view.run(realm, 0.1));
    }
    panic!("spell {spell_id} never went off");
}

/// Whether `spell_id` went off in `events`.
pub fn went_off(events: &[SessionEvent], spell_id: u32) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::SpellGo { spell_id: s, .. } if *s == spell_id))
}

/// The shipped Initiate's Skill Choice `key` of the row for `skill`, as the realm loaded it.
pub fn choice_skill(skill: &str, key: &str) -> &'static twilight_realm::callings::Skill {
    twilight_realm::callings::calling("initiate")
        .expect("the Initiate loads")
        .rows
        .iter()
        .filter(|r| r.key == skill)
        .flat_map(|r| &r.choices)
        .find_map(|c| c.skill().filter(|s| s.key == key))
        .unwrap_or_else(|| panic!("{skill}/{key} is a loaded Skill Choice"))
}

/// A blow that landed on the player: what its report said, the health it took, whether `watch`
/// held before the tick it landed on, and that tick's events.
#[derive(Debug)]
pub struct Blow {
    pub reported: u32,
    pub lost: u32,
    pub watched: bool,
    pub events: Vec<SessionEvent>,
}

/// The blows that land on `me` over `secs`, one 0.1 s tick at a time: only a tick with one blow
/// and no other harm to the player.
pub fn blows(
    realm: &mut Realm,
    view: &mut View,
    me: u64,
    secs: f32,
    watch: impl Fn(&View) -> bool,
) -> Vec<Blow> {
    let mut out = Vec::new();
    for _ in 0..(secs / 0.1).round() as u32 {
        let (health, watched) = (view.health(me), watch(view));
        let events = view.run(realm, 0.1);
        let landed: Vec<u32> = events
            .iter()
            .filter_map(|e| match e {
                SessionEvent::AttackerState(s) if s.victim == me && s.damage > 0 => Some(s.damage),
                _ => None,
            })
            .collect();
        let other_harm = events.iter().any(|e| {
            matches!(e, SessionEvent::SpellDamageLog(l) if l.target == me)
                || matches!(e, SessionEvent::PeriodicAuraLog(l) if l.target == me)
        });
        if let (&[reported], false) = (&landed[..], other_harm) {
            out.push(Blow {
                reported,
                lost: health - view.health(me),
                watched,
                events,
            });
        }
        assert!(view.health(me) > 0, "the Initiate died");
    }
    out
}

/// A camp of four level 7 or 8 mobs and a level 9 or 10 one east of the Crossroads
/// (`content/barrens.toml`, 9000036): sturdy enough for a fight of several casts.
pub const EAST_CAMP: [f32; 3] = [40.0, -2560.0, 0.0];

/// An Initiate of [`ROWS_OPEN`] 25 yd west of [`EAST_CAMP`]'s first mob with two of its mobs
/// pulled onto them, side by side.
pub struct Pair {
    pub realm: Realm,
    pub view: View,
    pub me: u64,
    pub mobs: [Mob; 2],
}

/// The [`Pair`] on `seed`'s dice with each `(row, choice)` set: the camp's first mob and the one
/// at its west edge, each pulled with a swing, standing within 4 yd of the Initiate.
pub fn pair(seed: u64, rows: &[(&str, u32)]) -> Pair {
    let (realm, view, me) = initiate_world(seed, rows);
    let (mut realm, mut view) = (realm, view);
    let west = [EAST_CAMP[0] - 25.0, EAST_CAMP[1], 0.0];
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: west,
        orientation: 0.0,
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
            } if flat(*position, EAST_CAMP) < 15.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .collect();
    let first = camp
        .iter()
        .find(|m| flat(m.position, EAST_CAMP) < 1.0)
        .expect("the camp's first mob streams in")
        .clone();
    let edge = camp
        .iter()
        .find(|m| flat(m.position, [31.0, -2551.0, 0.0]) < 1.0)
        .expect("the camp's west mob streams in")
        .clone();
    let mut mobs = [first, edge];
    for mob in &mobs {
        pull(&mut realm, &mut view, mob);
    }
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: west,
        orientation: 0.0,
    });
    for _ in 0..300 {
        for e in view.run(&mut realm, 0.1) {
            if let SessionEvent::MonsterMove { guid, path, .. } = e {
                if let (Some(mob), Some(end)) =
                    (mobs.iter_mut().find(|m| m.guid == guid), path.last())
                {
                    mob.position = *end;
                }
            }
        }
        if mobs.iter().all(|m| flat(m.position, west) <= 4.0) {
            // The paths end beside the Initiate; the mobs have still to walk them.
            view.run(&mut realm, 3.0);
            return Pair {
                realm,
                view,
                me,
                mobs,
            };
        }
    }
    panic!("the pair never came to hand");
}

/// Ticks until the aura `spell_id` is off `unit`, at most `secs`: the seconds it took.
pub fn lasts(realm: &mut Realm, view: &mut View, unit: u64, spell_id: u32, secs: f32) -> f32 {
    for i in 0..(secs / 0.1).round() as u32 {
        if view.aura(unit, spell_id).is_none() {
            return i as f32 * 0.1;
        }
        view.run(realm, 0.1);
    }
    panic!("aura {spell_id} still on after {secs} s");
}

/// The hits `spell_id` dealt `target` in `events` that did not crit.
pub fn plain_hits_on(events: &[SessionEvent], spell_id: u32, target: u64) -> Vec<u32> {
    super::cutthroat::damage_of(events, spell_id)
        .into_iter()
        .filter(|&(t, _, crit)| t == target && !crit)
        .map(|(_, dealt, _)| dealt)
        .collect()
}

/// The cast bar `spell_id` started with in `events`, in ms.
pub fn cast_bar(events: &[SessionEvent], spell_id: u32) -> Option<u32> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellStart {
            spell_id: s,
            cast_time_ms,
            ..
        } if *s == spell_id => Some(*cast_time_ms),
        _ => None,
    })
}
