//! A creature's ground patch (#240): a scenery GameObject at the target's position, shown to the
//! caster's Phase, that hurts every character standing in it once a tick until its time is up. It
//! follows no one and outlives its caster.

use super::{Gaze, InWorld, Scenery};
use crate::clock;
use crate::phased::{gameobject_create, GameObject};
use crate::rules::school::School;

/// Seconds of float drift a patch's clock forgives.
const CLOCK_EPSILON: f32 = 1e-3;
/// `HIGHGUID_GAMEOBJECT`, in a GameObject guid's top 16 bits.
const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
/// The first entry a patch takes, clear of the dungeons' own GameObjects.
const FIRST_ENTRY: u32 = 0x00F0_0000;
/// `GameObjectDisplayInfo` of the stock campfire: the flames a fire patch shows.
const FIRE_DISPLAY: u32 = 4;
/// `GAMEOBJECT_TYPE_GENERIC`.
const GENERIC: u32 = 5;
/// `GO_FLAG_NOT_SELECTABLE`: scenery, never clicked.
const NOT_SELECTABLE: u32 = 0x10;

/// One patch burning.
pub(super) struct Patch {
    guid: u64,
    caster: u64,
    spell_id: u32,
    at: [f32; 3],
    radius: f32,
    damage: u32,
    school: School,
    tick_secs: f32,
    /// Seconds until the patch goes.
    left: f32,
    /// Seconds until the next tick.
    next: f32,
}

impl InWorld {
    /// A patch of `radius` yards goes down at `at`: it ticks `damage` of `school` every `tick_ms`
    /// for `duration_ms`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn put_patch(
        &mut self,
        caster: u64,
        spell_id: u32,
        at: [f32; 3],
        radius: f32,
        duration_ms: u32,
        tick_ms: u32,
        damage: u32,
        school: School,
    ) {
        self.patch_serial += 1;
        // Every session numbers its own patches, so the owner's guid keeps two apart.
        let entry = FIRST_ENTRY + (self.guid as u32 & 0xfff) * 0x100 + self.patch_serial % 0x100;
        let guid = HIGHGUID_GAMEOBJECT << 48 | u64::from(entry);
        self.scenery
            .push(Scenery::Show(gameobject_create(&GameObject {
                guid,
                entry,
                display_id: FIRE_DISPLAY,
                type_id: GENERIC,
                flags: NOT_SELECTABLE,
                faction: 0,
                position: at,
                orientation: 0.0,
            })));
        let tick_secs = clock::secs(tick_ms.max(1));
        self.patches.push(Patch {
            guid,
            caster,
            spell_id,
            at,
            radius,
            damage,
            school,
            tick_secs,
            left: clock::secs(duration_ms),
            next: tick_secs,
        });
    }

    /// Advances the patches by `dt`: each ticks as often as its time allows, and one whose time is
    /// up is taken off the ground. A tick goes out as a gaze through cover, which the realm lands
    /// on everyone in the Phase within the patch's radius.
    pub(super) fn patch_tick(&mut self, dt: f32) {
        let mut gone = Vec::new();
        for patch in &mut self.patches {
            patch.left -= dt;
            patch.next -= dt;
            while patch.next <= CLOCK_EPSILON {
                self.gazes.push(Gaze {
                    caster: patch.caster,
                    spell_id: patch.spell_id,
                    at: patch.at,
                    radius: patch.radius,
                    damage: patch.damage,
                    school: patch.school,
                    through_cover: true,
                    root_ms: 0,
                });
                patch.next += patch.tick_secs;
            }
            if patch.left <= CLOCK_EPSILON {
                gone.push(patch.guid);
            }
        }
        self.patches.retain(|p| !gone.contains(&p.guid));
        self.scenery.extend(gone.into_iter().map(Scenery::Hide));
    }
}
