//! A shipped Vanguard for the Talent Row scripts (#504): a save of any level, with or without the
//! starting outfit's shield, its rows set through the realm's seam, and a scene 25 yd west of a
//! mob south of the Crossroads, facing it.

use benilla_protocol::{EntityKind, SessionEvent, TalentRowView};
use twilight_realm::{Realm, RealmCommand};

use super::fight::*;
use super::*;

/// The level 8 or 9 mob at the edge of the scorpid camp south of the Crossroads, sturdy enough to
/// outlast a fight (`content/barrens.toml`, 9000034). `CAMP` is the camp's level 3 first mob.
pub const STURDY: [f32; 3] = [-452.0, -3014.0, 0.0];

/// The off-hand slot, where the starting outfit's shield sits.
pub const OFF_HAND: u8 = 16;

/// The `victim_state` of a swing the victim blocked whole, and of one it parried.
pub const BLOCKS: u32 = 5;
pub const PARRY: u32 = 3;

/// A new human's save as a Vanguard of `level`, without the starting shield unless `shield`: its
/// guid and the blob.
pub fn vanguard_save(level: u8, shield: bool) -> (u64, String) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Bulwark", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"vanguard\"");
    (
        guid,
        if shield {
            blob
        } else {
            drop_slot(&blob, OFF_HAND)
        },
    )
}

/// A realm on `seed`'s dice holding `blob`, entered as `guid`: the realm, the view and the entry
/// burst.
pub fn enter(guid: u64, blob: &str, seed: u64) -> (Realm, View, Vec<SessionEvent>) {
    let mut realm = Realm::with_seed(seed);
    realm.import_character(blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let entry = view.drain(&mut realm);
    (realm, view, entry)
}

/// Sets `row` to `spell_id`: the events it sent.
pub fn set_row(realm: &mut Realm, view: &mut View, row: &str, spell_id: u32) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::SetTalentRow {
        row: row.into(),
        spell_id,
    });
    view.drain(realm)
}

/// The rows of the last `TalentRows` in `events`.
pub fn rows_in(events: &[SessionEvent]) -> Option<&[TalentRowView]> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::TalentRows { rows, .. } => Some(rows.as_slice()),
        _ => None,
    })
}

pub fn learned(events: &[SessionEvent], spell: u32) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::SpellLearned { spell_id } if *spell_id == spell))
}

pub fn removed(events: &[SessionEvent], spell: u32) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::SpellRemoved { spell_id } if *spell_id == spell))
}

/// A character in the world west of a mob, its client's view and its guid.
pub struct Scene {
    pub realm: Realm,
    pub view: View,
    pub me: u64,
    pub mob: Mob,
}

/// A Vanguard of `level` on `seed`'s dice, the shield worn when `shield`, with each `(row,
/// choice)` set, 25 yd west of the mob at `spot` and facing it.
pub fn scene(level: u8, shield: bool, seed: u64, rows: &[(&str, u32)], spot: [f32; 3]) -> Scene {
    let (me, blob) = vanguard_save(level, shield);
    scene_of(me, &blob, seed, rows, spot)
}

/// The character of the save `blob`, entered as `me` on `seed`'s dice with each `(row, choice)`
/// set, 25 yd west of the mob at `spot` and facing it (a Cutthroat's scene too, #505).
pub fn scene_of(me: u64, blob: &str, seed: u64, rows: &[(&str, u32)], spot: [f32; 3]) -> Scene {
    let (mut realm, mut view, _) = enter(me, blob, seed);
    for (row, choice) in rows {
        let events = set_row(&mut realm, &mut view, row, *choice);
        assert!(
            learned(&events, *choice),
            "{row} takes {choice}: {events:?}"
        );
    }
    view.run(&mut realm, 1.0);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [spot[0] - 25.0, spot[1], 0.0],
        orientation: 0.0,
    });
    let mob = view
        .run(&mut realm, 1.0)
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                ..
            } if flat(*position, spot) < 1.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .expect("the mob at the spot streams in");
    Scene {
        realm,
        view,
        me,
        mob,
    }
}

impl Scene {
    /// Casts `spell_id` at the scene's mob and stops the swing the cast starts: what it sent.
    pub fn cast_at_mob(&mut self, spell_id: u32) -> Vec<SessionEvent> {
        self.realm.handle(cast(spell_id, Some(self.mob.guid)));
        let events = self.view.drain(&mut self.realm);
        assert_eq!(
            refusal(&events, spell_id),
            None,
            "spell {spell_id} goes off"
        );
        self.realm.handle(RealmCommand::AttackStop);
        events
    }

    /// Ticks until `done` holds for one tick's events, at most `secs`: every event seen, the
    /// last tick's among them. Panics if the character dies or time runs out.
    pub fn run_until(
        &mut self,
        secs: f32,
        what: &str,
        mut done: impl FnMut(&View, &[SessionEvent]) -> bool,
    ) -> Vec<SessionEvent> {
        let mut seen = Vec::new();
        for _ in 0..(secs / 0.1).round() as u32 {
            let events = self.view.run(&mut self.realm, 0.1);
            let stop = done(&self.view, &events);
            seen.extend(events);
            if stop {
                return seen;
            }
            assert!(self.view.health(self.me) > 0, "died waiting for {what}");
        }
        panic!("{secs} s passed with no {what}");
    }
}
