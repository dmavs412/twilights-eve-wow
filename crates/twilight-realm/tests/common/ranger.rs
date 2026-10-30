//! A shipped Ranger for the Talent Row scripts (#507): a save of any level with a bow and a stack
//! of arrows, its rows set through the realm's seam as `talent_rows.rs` sets them, its mana, and
//! the damage its shots log.

use benilla_protocol::SessionEvent;
use twilight_realm::{Realm, RealmCommand};

use super::cutthroat::set_rows;
use super::fight::*;
use super::vanguard::{enter, scene_of, Scene};
use super::*;

/// Every Ranger row is open at 24, the level the row scripts play at.
pub const ROWS_OPEN: u8 = 24;

/// The ranged slot, a Cracked Shortbow, and the Rough Arrows the Ranger shoots (#157).
const RANGED: u8 = 17;
const CRACKED_SHORTBOW: u32 = 2773;
const ROUGH_ARROW: u32 = 2512;
const BACKPACK_1: u8 = 23;

/// `Powers` of mana.
const MANA: u8 = 0;

/// A new human's save as a Ranger of `level` with only a Cracked Shortbow and 200 Rough Arrows,
/// set as the ammo: its guid and the blob.
pub fn ranger_save(level: u8) -> (u64, String) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Strider", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker.export_character(guid).unwrap();
    // The items come last: drop the starting ones.
    if let Some(at) = blob.find("[[items]]") {
        blob.truncate(at);
    }
    let mut blob = blob
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"ranger\"")
        .replace("money = 0\n", &format!("money = 0\nammo = {ROUGH_ARROW}\n"))
        .replace("items = []\n", "");
    for (slot, entry, count) in [
        (RANGED, CRACKED_SHORTBOW, 1),
        (BACKPACK_1, ROUGH_ARROW, 200),
    ] {
        blob.push_str(&format!(
            "\n[[items]]\nslot = {slot}\nentry = {entry}\ncount = {count}\n"
        ));
    }
    (guid, blob)
}

/// A Ranger of [`ROWS_OPEN`] on `seed`'s dice with each `(row, choice)` set, 25 yd west of the mob
/// at `spot` and facing it.
pub fn ranger_scene(seed: u64, rows: &[(&str, u32)], spot: [f32; 3]) -> Scene {
    let (me, blob) = ranger_save(ROWS_OPEN);
    scene_of(me, &blob, seed, rows, spot)
}

/// A Ranger of [`ROWS_OPEN`] in the world on `seed`'s dice with each `(row, choice)` set: the
/// realm, the view and their guid.
pub fn ranger_world(seed: u64, rows: &[(&str, u32)]) -> (Realm, View, u64) {
    let (me, blob) = ranger_save(ROWS_OPEN);
    let (mut realm, mut view, _) = enter(me, &blob, seed);
    set_rows(&mut realm, &mut view, rows);
    view.run(&mut realm, 1.0);
    (realm, view, me)
}

/// The spell id of the Ranger's skill or aura row `key` on the sheet.
pub fn ranger(key: &str) -> u32 {
    spell("ranger", key)
}

/// The spell id of the Ranger's choice `key` of the row `row`.
pub fn ranger_choice(row: &str, key: &str) -> u32 {
    choice("ranger", row, key)
}

/// The shipped Ranger's Skill Choice `key` of the row for `skill`, as the realm loaded it.
pub fn ranger_choice_skill(skill: &str, key: &str) -> &'static twilight_realm::callings::Skill {
    twilight_realm::callings::calling("ranger")
        .expect("the Ranger loads")
        .rows
        .iter()
        .filter(|r| r.key == skill)
        .flat_map(|r| &r.choices)
        .find_map(|c| c.skill().filter(|s| s.key == key))
        .unwrap_or_else(|| panic!("{skill}/{key} is a loaded Skill Choice"))
}

pub fn mana(view: &View, me: u64) -> u32 {
    view.fields(me).unit_power(MANA).unwrap()
}

/// Each blow `spell_id` dealt in `events`: `(target, damage, crit)`.
pub fn shots(events: &[SessionEvent], spell_id: u32) -> Vec<(u64, u32, bool)> {
    super::cutthroat::damage_of(events, spell_id)
}

/// How many times `spell_id` went off in `events`.
pub fn gone_off(events: &[SessionEvent], spell_id: u32) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, SessionEvent::SpellGo { spell_id: s, .. } if *s == spell_id))
        .count()
}

/// Stands `yards` west of `mob`, facing it.
pub fn face_from(realm: &mut Realm, view: &mut View, mob: &Mob, yards: f32) {
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [mob.position[0] - yards, mob.position[1], 0.0],
        orientation: 0.0,
    });
    view.run(realm, 0.1);
}

/// The client's gravity (yd/s²), which brings a leap down.
pub const GRAVITY: f32 = 19.291_105;

/// Casts the leap `spell_id` (Vault or a Vault choice) and flies it as the client would: the
/// realm runs while the Ranger is in the air, the client reports the spot the launch reaches on
/// flat ground in the tick it lands, and the realm runs that tick. Every event meanwhile. Panics
/// when it is refused or launches nothing.
pub fn leap(realm: &mut Realm, view: &mut View, spell_id: u32) -> Vec<SessionEvent> {
    let from = realm.player_positions()[0].1;
    realm.handle(cast(spell_id, None));
    let mut events = view.drain(realm);
    assert_eq!(
        refusal(&events, spell_id),
        None,
        "the leap goes off: {events:?}"
    );
    let launch = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::KnockBack { launch, .. } => Some(*launch),
            _ => None,
        })
        .expect("the leap launches the Ranger");
    let air = 2.0 * -launch.zspeed / GRAVITY;
    let reach = launch.xy_speed * air;
    events.extend(view.run(realm, (air * 10.0).floor() / 10.0));
    // Straight back, so they face the other way.
    let facing = launch.sin_angle.atan2(launch.cos_angle) + std::f32::consts::PI;
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [
            from[0] + launch.cos_angle * reach,
            from[1] + launch.sin_angle * reach,
            from[2],
        ],
        orientation: facing.rem_euclid(std::f32::consts::TAU),
    });
    events.extend(view.run(realm, 0.1));
    events
}
