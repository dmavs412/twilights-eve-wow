//! The fight scripts' harness: what the client has seen, a mob as its create shows it, and the
//! steps a fight repeats (stand by a mob, cast at it, read a refusal).

use benilla_protocol::messages::UnitAuraSlot;
use benilla_protocol::{ObjectFields, SessionEvent};
use twilight_realm::{Realm, RealmCommand};

use super::{drain, FlatGround};

/// The Crossroads, where a script's character enters the world, at ground level.
pub const CROSSROADS_XY: [f32; 3] = [-441.8, -2648.6, 0.0];
/// A camp of four level 3 mobs south of the Crossroads (`content/barrens.toml`, 9000011).
pub const CAMP: [f32; 3] = [-376.0, -2837.0, 0.0];

/// A mob, as its create shows it.
#[derive(Clone)]
pub struct Mob {
    pub guid: u64,
    pub position: [f32; 3],
}

/// What the client knows: every object's fields, merged as the packets arrive, and the entry
/// burst when a script keeps it.
#[derive(Default)]
pub struct View {
    pub objects: std::collections::HashMap<u64, ObjectFields>,
    pub entry: Vec<SessionEvent>,
}

impl View {
    pub fn see(&mut self, events: &[SessionEvent]) {
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

    pub fn drain(&mut self, realm: &mut Realm) -> Vec<SessionEvent> {
        let events = drain(realm);
        self.see(&events);
        events
    }

    /// Ticks `secs` in 0.1 s steps, collecting the events.
    pub fn run(&mut self, realm: &mut Realm, secs: f32) -> Vec<SessionEvent> {
        let mut events = Vec::new();
        for _ in 0..(secs / 0.1).round() as u32 {
            realm.tick(0.1, &FlatGround);
            events.extend(self.drain(realm));
        }
        events
    }

    pub fn fields(&self, guid: u64) -> &ObjectFields {
        self.objects
            .get(&guid)
            .expect("an object the client has seen")
    }

    /// Whole rage points.
    pub fn rage(&self, guid: u64) -> u32 {
        self.fields(guid).unit_power(1).unwrap_or(0) / 10
    }

    pub fn health(&self, guid: u64) -> u32 {
        self.fields(guid).unit_health().unwrap()
    }

    pub fn aura(&self, guid: u64, spell_id: u32) -> Option<UnitAuraSlot> {
        self.fields(guid)
            .unit_auras()
            .find(|a| a.spell_id == spell_id)
    }
}

pub fn cast(spell_id: u32, target: Option<u64>) -> RealmCommand {
    RealmCommand::CastSpell { spell_id, target }
}

/// Moves the player to `yards` from `mob`, on the Crossroads side, facing it.
pub fn stand_at(realm: &mut Realm, mob: &Mob, yards: f32) {
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
        orientation: (mob.position[1] - at[1]).atan2(mob.position[0] - at[0]),
    });
}

/// Pulls `mob` with a swing: stands 2 yd from it, starts auto-attack, ticks until the mob turns
/// on the player (a swing that resolves, hit or miss, pulls an idle mob; the click alone pulls
/// nothing since #185), then stops attacking. Returns what the client saw meanwhile.
pub fn pull(realm: &mut Realm, view: &mut View, mob: &Mob) -> Vec<SessionEvent> {
    stand_at(realm, mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let mut events = Vec::new();
    for _ in 0..100 {
        events.extend(view.run(realm, 0.1));
        let engaged = events.iter().any(
            |e| matches!(e, SessionEvent::AttackStart { attacker, .. } if *attacker == mob.guid),
        );
        if engaged {
            break;
        }
    }
    realm.handle(RealmCommand::AttackStop);
    events.extend(view.drain(realm));
    events
}

/// The distance from `a` to `b` on the ground, heights aside.
pub fn flat(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// The reason a cast of `spell_id` was refused in `events`, if it was.
pub fn refusal(events: &[SessionEvent], spell_id: u32) -> Option<u8> {
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

/// The save without its `[[items]]` row for `slot`.
pub fn drop_slot(blob: &str, slot: u8) -> String {
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

/// How long a step keeps the caster moving, which holds an auto-repeat shot (`MOVING_GRACE_SECS`).
pub const MOVING_GRACE_SECS: f32 = 0.75;

/// Each gap between `shots` is the shot timer, or the shot came due while the caster moved and
/// went as the hold lifted, a grace after the last step (within a tick).
pub fn assert_on_time(shots: &[f32], steps: &[f32], every: f32) {
    for pair in shots.windows(2) {
        let gap = pair[1] - pair[0];
        let last_step = steps.iter().rev().find(|&&t| t < pair[1]).copied();
        let held = last_step.is_some_and(|t| (pair[1] - t - MOVING_GRACE_SECS).abs() < 0.11);
        assert!(
            (gap - every).abs() < 0.1 || (gap > every && held),
            "a shot every {every} s or as the hold lifts: shots {shots:?}, steps {steps:?}"
        );
    }
}
