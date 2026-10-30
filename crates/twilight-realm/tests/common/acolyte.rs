//! A shipped Acolyte for the Talent Row scripts (#508): a save of any level, its rows set through
//! the realm's seam as `talent_rows.rs` sets them, scenes by a lone mob, a pulled camp and a
//! Test Grounds dummy, and readers for the heals, ticks and shields the scripts weigh.

use benilla_protocol::messages::PeriodicTick;
use benilla_protocol::SessionEvent;
use twilight_realm::{Realm, RealmCommand};

use super::cutthroat::{pulled_of, set_rows, Pack};
use super::fight::*;
use super::vanguard::{enter, scene_of, Scene};
use super::*;

/// Every Acolyte row is open at 24, the level the row scripts play at.
pub const ROWS_OPEN: u8 = 24;

/// A new human's save as an Acolyte of `level`: its guid and the blob.
pub fn acolyte_save(level: u8) -> (u64, String) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Vellune", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"acolyte\"");
    (guid, blob)
}

/// An Acolyte of [`ROWS_OPEN`] on `seed`'s dice with each `(row, choice)` set, 25 yd west of the
/// mob at `spot` and facing it.
pub fn acolyte_scene(seed: u64, rows: &[(&str, u32)], spot: [f32; 3]) -> Scene {
    let (me, blob) = acolyte_save(ROWS_OPEN);
    scene_of(me, &blob, seed, rows, spot)
}

/// An Acolyte of [`ROWS_OPEN`] in the world on `seed`'s dice with each `(row, choice)` set: the
/// realm, the view and their guid.
pub fn acolyte_world(seed: u64, rows: &[(&str, u32)]) -> (Realm, View, u64) {
    let (me, blob) = acolyte_save(ROWS_OPEN);
    let (mut realm, mut view, _) = enter(me, &blob, seed);
    set_rows(&mut realm, &mut view, rows);
    view.run(&mut realm, 1.0);
    (realm, view, me)
}

/// An Acolyte of [`ROWS_OPEN`] with each `(row, choice)` set and the scorpid camp south of the
/// Crossroads pulled onto them.
pub fn acolyte_pack(seed: u64, rows: &[(&str, u32)]) -> Pack {
    let (me, blob) = acolyte_save(ROWS_OPEN);
    pulled_of(me, &blob, seed, rows)
}

/// The spell id of the Acolyte's skill or aura row `key` on the sheet.
pub fn acolyte(key: &str) -> u32 {
    spell("acolyte", key)
}

/// The Acolyte's choice `key` of the row for `row`.
pub fn row_choice(row: &str, key: &str) -> u32 {
    choice("acolyte", row, key)
}

/// Casts `spell_id` at `target` and ticks until it goes off (a cast bar's length, at most 5 s):
/// every event meanwhile. Panics when the cast is refused.
pub fn cast_through(
    realm: &mut Realm,
    view: &mut View,
    spell_id: u32,
    target: u64,
) -> Vec<SessionEvent> {
    super::initiate::cast_through(realm, view, spell_id, target)
}

/// The Creeping Blight stacks `unit` shows, 0 without it.
pub fn blight_stacks(view: &View, unit: u64) -> u32 {
    view.aura(unit, acolyte("aura_creeping_blight"))
        .map_or(0, |a| u32::from(a.stacks))
}

/// The damage each periodic tick of `spell_id` dealt `unit` in `events`.
pub fn ticks_on(events: &[SessionEvent], unit: u64, spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == spell_id && l.target == unit => {
                match l.ticks[0] {
                    PeriodicTick::Damage { amount, .. } => Some(amount),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect()
}

/// The health each periodic heal tick of `spell_id` gave in `events`.
pub fn heal_ticks(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == spell_id => match l.ticks[0] {
                PeriodicTick::Heal { amount } => Some(amount),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// What the player's shields soaked over `secs`, one 0.1 s tick at a time, while `watch` holds
/// before the tick: every blow, spell and tick at them, less the health they lost, plus the health
/// they gained. No health comes back on its own in combat.
pub fn soaked(
    realm: &mut Realm,
    view: &mut View,
    me: u64,
    secs: f32,
    watch: impl Fn(&View) -> bool,
) -> u32 {
    let mut total = 0i64;
    for _ in 0..(secs / 0.1).round() as u32 {
        if !watch(view) {
            break;
        }
        let health = i64::from(view.health(me));
        let events = view.run(realm, 0.1);
        let mut harm = 0i64;
        let mut gained = 0i64;
        for e in &events {
            match e {
                SessionEvent::AttackerState(s) if s.victim == me => harm += i64::from(s.damage),
                SessionEvent::SpellDamageLog(l) if l.target == me => harm += i64::from(l.damage),
                SessionEvent::PeriodicAuraLog(l) if l.target == me => match l.ticks[0] {
                    PeriodicTick::Damage { amount, .. } => harm += i64::from(amount),
                    PeriodicTick::Heal { amount } => gained += i64::from(amount),
                    _ => {}
                },
                SessionEvent::SpellHealLog(l) if l.target == me => gained += i64::from(l.amount),
                _ => {}
            }
        }
        let lost = health - i64::from(view.health(me));
        total += harm - lost - gained;
        assert!(view.health(me) > 0, "the Acolyte died");
    }
    total.max(0) as u32
}

/// Walks the player to `to`, facing east, and lets a tick pass.
pub fn walk_to(realm: &mut Realm, view: &mut View, to: [f32; 3]) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: to,
        orientation: 0.0,
    });
    view.run(realm, 0.1)
}

/// Walks the player to 25 yd west of the mob at `spot`, facing it: the mob as it streams in.
pub fn walk_west_of(realm: &mut Realm, view: &mut View, spot: [f32; 3]) -> Mob {
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [spot[0] - 25.0, spot[1], 0.0],
        orientation: 0.0,
    });
    view.run(realm, 1.0)
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: benilla_protocol::EntityKind::Unit,
                position,
                ..
            } if flat(*position, spot) < 1.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .expect("the mob at the spot streams in")
}

/// The Test Grounds' ground (#199): every point at the dummies' height.
pub struct YardGround;

impl Host for YardGround {
    fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
        Some(47.2)
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
}

/// The Test Grounds' dummies in their row along the north side, 6 yd apart (levels 5, 15, 25 and
/// the level 30 elite): where each stands (`content/test_grounds.toml`).
pub const DUMMY_SPOTS: [[f32; 3]; 4] = [
    [-9640.0, 264.0, 47.2],
    [-9640.0, 258.0, 47.2],
    [-9640.0, 252.0, 47.2],
    [-9640.0, 246.0, 47.2],
];

/// The guid of dummy `i` of [`DUMMY_SPOTS`].
pub fn dummy(i: usize) -> u64 {
    let (low, entry) = [
        (790001, 97001),
        (790002, 97001),
        (790003, 97001),
        (790004, 97002),
    ][i];
    0xF130 << 48 | entry << 24 | low
}

/// An Acolyte of [`ROWS_OPEN`] in the Test Grounds, whose dummies take hits but never fight back
/// or die.
pub struct Yard {
    pub realm: Realm,
    pub view: View,
    pub me: u64,
}

/// An Acolyte of [`ROWS_OPEN`] started in Elwynn on `seed`'s dice with each `(row, choice)` set,
/// standing at `at` in the Test Grounds facing the dummies.
pub fn yard(seed: u64, rows: &[(&str, u32)], at: [f32; 2]) -> Yard {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Vellune", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = benilla_protocol::START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("level = 1\n", &format!("level = {ROWS_OPEN}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"acolyte\"");
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &YardGround);
    let mut view = View::default();
    view.drain(&mut realm);
    set_rows(&mut realm, &mut view, rows);
    let mut y = Yard { realm, view, me };
    for (xy, facing, secs) in [([-9720.0, 255.0], None, 1.0), (at, Some(0.0), 1.0)] {
        y.realm.handle(RealmCommand::DevTeleport {
            x: xy[0],
            y: xy[1],
            facing,
        });
        let landed = [y.view.drain(&mut y.realm), y.run(secs)].concat();
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

impl Yard {
    /// Ticks the Test Grounds for `secs`, 0.1 s at a time: every event.
    pub fn run(&mut self, secs: f32) -> Vec<SessionEvent> {
        let mut events = Vec::new();
        for _ in 0..(secs / 0.1).round().max(1.0) as u32 {
            self.realm.tick(0.1, &YardGround);
            events.extend(self.view.drain(&mut self.realm));
        }
        events
    }

    /// Casts `spell_id` at `target` and ticks until it goes off (at most 5 s): every event
    /// meanwhile. Panics when the cast is refused.
    pub fn cast_through(&mut self, spell_id: u32, target: u64) -> Vec<SessionEvent> {
        self.realm.handle(cast(spell_id, Some(target)));
        let mut events = self.view.drain(&mut self.realm);
        assert_eq!(
            refusal(&events, spell_id),
            None,
            "spell {spell_id} goes off: {events:?}"
        );
        self.realm.handle(RealmCommand::AttackStop);
        for _ in 0..50 {
            if super::initiate::went_off(&events, spell_id) {
                return events;
            }
            events.extend(self.run(0.1));
        }
        panic!("spell {spell_id} never went off");
    }

    /// Walks to `xy` in the box, facing the dummies.
    pub fn walk(&mut self, xy: [f32; 2]) -> Vec<SessionEvent> {
        self.realm.handle(RealmCommand::Move {
            motion: Default::default(),
            position: [xy[0], xy[1], 47.2],
            orientation: 0.0,
        });
        self.view.drain(&mut self.realm)
    }
}

/// The player's maximum health, as their client shows it.
pub fn max_health(view: &View, me: u64) -> u32 {
    view.fields(me).unit_max_health().unwrap()
}
