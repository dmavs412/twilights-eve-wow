//! Drifting puffs along the foot of the mist curtain (#662, decision 4): soft round quads where
//! the shells meet the ground, so the cloud bank churns. Camera-facing quads on the world effect
//! stream with the stock `snowmist01.blp`, as the weather's mist puffs
//! (`weather/precip/mist.rs`). Each closed-zone segment near the camera keeps
//! [`PUFFS_PER_SEGMENT`] alive; placed walls get none, since a puff is wider than the tram tunnel.
//! Every draw comes from a hash of the segment's `from` point, the slot and its respawn count, so
//! a capture repeats.

use std::collections::HashMap;
use std::f32::consts::TAU;
use std::ops::Range;

use benilla_assets::coords::{bevy_to_wow, wow_to_bevy};
use bevy::prelude::*;

use crate::lighting::WowLighting;
use crate::mist::{
    open_fade, shell_ground, FadingMist, MistSegments, Segment, COLOUR_RAMP_YD, FOOT_DESAT,
    FOOT_VALUE,
};
use crate::particles::buffer::{
    EffectBlend, EffectDrawSpec, EffectFog, EffectLighting, EffectQuads, EffectVertex,
};
use crate::view::WorldCamera;

/// Yards from the camera within which a segment's middle keeps its puffs.
pub const PUFF_RANGE_YD: f32 = 160.0;
/// Puffs alive on each 33-yd segment.
pub const PUFFS_PER_SEGMENT: usize = 3;
/// The most puffs alive at once; the segments nearest the camera win.
pub const PUFF_CAP: usize = 256;
/// A puff's quad side in yards, a uniform draw.
pub const PUFF_SIZE_YD: Range<f32> = 14.0..28.0;
/// Yards a puff's centre stands past the edge line along `inward`, a uniform draw; negative is
/// over open ground.
pub const PUFF_DEPTH_YD: Range<f32> = -3.0..10.0;
/// A puff's centre height above the ground at birth, as a share of its size.
pub const PUFF_LIFT: f32 = 0.35;
/// A puff's speed along its segment in yards per second, a uniform draw, either way.
pub const PUFF_DRIFT_YD_S: Range<f32> = 0.4..1.0;
/// A puff's upward speed in yards per second.
pub const PUFF_RISE_YD_S: f32 = 0.15;
/// A puff's turn in the screen plane in degrees per second, a uniform draw.
pub const PUFF_SPIN_DEG_S: Range<f32> = -6.0..6.0;
/// A puff's life in seconds, a uniform draw.
pub const PUFF_LIFE_S: Range<f32> = 25.0..45.0;
/// Seconds a puff's alpha takes to rise after its birth.
pub const PUFF_FADE_IN_S: f32 = 6.0;
/// Seconds a puff's alpha takes to fall before its death.
pub const PUFF_FADE_OUT_S: f32 = 8.0;
/// A puff's peak alpha.
pub const PUFF_ALPHA: f32 = 0.45;
/// Yards from the camera over which a puff's alpha rises from 0 to full.
pub const PUFF_NEAR_YD: Range<f32> = 3.0..10.0;
/// The puffs' sort rung, anchored at the camera. The curtain sorts at its mesh's AABB centre,
/// which can stand behind the camera while the shells in front show; one far plane's worth of
/// rung puts the puffs after it at any view-z, and still under the ground-fx decals.
const PUFF_SORT_BIAS: f32 = -crate::sky_order::WORLD_VIEW_Z_FLOOR;

/// One puff, in WoW coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Puff {
    /// The centre.
    pub(crate) pos: Vec3,
    /// The ground under the birth spot, which the colour ramp counts up from.
    pub(crate) ground: f32,
    /// The unit vector along the segment, from `from` to `to`.
    pub(crate) along: Vec3,
    pub(crate) size: f32,
    /// Yards per second along `along`, either sign.
    pub(crate) drift: f32,
    /// Radians per second in the screen plane.
    pub(crate) spin: f32,
    /// The turn at age 0, in radians.
    pub(crate) angle: f32,
    pub(crate) age: f32,
    pub(crate) life: f32,
}

/// The splitmix64 finaliser.
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The seed of a puff: the segment's `from` point, the slot and how many times it respawned.
fn seed(s: &Segment, slot: usize, respawns: u32) -> u64 {
    let from = s.from[0].to_bits() as u64 | (s.from[1].to_bits() as u64) << 32;
    mix(mix(mix(from) ^ slot as u64) ^ respawns as u64)
}

/// Draw `i` of `seed`, uniform in `range`.
fn draw(seed: u64, i: u64, range: Range<f32>) -> f32 {
    let bits = mix(seed ^ i.wrapping_mul(0xD6E8_FEB8_6659_FD93)) >> 40;
    range.start + (range.end - range.start) * (bits as f32 / (1u64 << 24) as f32)
}

/// Slot `slot`'s puff on `s` after `respawns` respawns. A `settled` puff starts at a random age
/// in its life, so an edge that just came into range already looks settled.
pub(crate) fn spawn(s: &Segment, slot: usize, respawns: u32, settled: bool) -> Puff {
    let k = seed(s, slot, respawns);
    let (from, to) = (Vec2::from(s.from), Vec2::from(s.to));
    let span = to - from;
    let inward = Vec2::from(s.inward);
    let size = draw(k, 0, PUFF_SIZE_YD);
    let spot = from + span * draw(k, 1, 0.0..1.0) + inward * draw(k, 2, PUFF_DEPTH_YD);
    let ground = shell_ground(s, spot);
    let speed = draw(k, 3, PUFF_DRIFT_YD_S);
    let life = draw(k, 5, PUFF_LIFE_S);
    Puff {
        pos: spot.extend(ground + PUFF_LIFT * size),
        ground,
        along: span.normalize_or_zero().extend(0.0),
        size,
        drift: if draw(k, 4, 0.0..1.0) < 0.5 {
            -speed
        } else {
            speed
        },
        spin: draw(k, 7, PUFF_SPIN_DEG_S).to_radians(),
        angle: draw(k, 8, 0.0..TAU),
        age: if settled {
            draw(k, 6, 0.0..1.0) * life
        } else {
            0.0
        },
        life,
    }
}

/// Moves `p` on by `dt` seconds: along its segment, past the end if it gets there, and up.
pub(crate) fn step(p: &mut Puff, dt: f32) {
    p.pos += dt * (p.along * p.drift + Vec3::Z * PUFF_RISE_YD_S);
    p.age += dt;
}

/// `p`'s alpha `cam_dist` yards from the camera on a curtain at `fade`: [`PUFF_ALPHA`] times the
/// birth and death ramps, the camera ramp over [`PUFF_NEAR_YD`] and the curtain's fade.
pub(crate) fn alpha(p: &Puff, cam_dist: f32, fade: f32) -> f32 {
    let born = (p.age / PUFF_FADE_IN_S).clamp(0.0, 1.0);
    let dying = ((p.life - p.age) / PUFF_FADE_OUT_S).clamp(0.0, 1.0);
    let near = PUFF_NEAR_YD;
    let camera = ((cam_dist - near.start) / (near.end - near.start)).clamp(0.0, 1.0);
    PUFF_ALPHA * born * dying * camera * fade
}

/// The mist's colour `above_ground` yards up, as `mist.wgsl` works it: the fog colour (raw gamma)
/// desaturated by [`FOOT_DESAT`] and darkened by [`FOOT_VALUE`] at the foot, back to the fog
/// colour over [`COLOUR_RAMP_YD`].
pub(crate) fn colour(fog: [f32; 3], above_ground: f32) -> [f32; 3] {
    let fog = Vec3::from(fog);
    let luma = fog.dot(Vec3::new(0.299, 0.587, 0.114));
    let dark = fog.lerp(Vec3::splat(luma), FOOT_DESAT) * FOOT_VALUE;
    let t = (above_ground / COLOUR_RAMP_YD).clamp(0.0, 1.0);
    dark.lerp(fog, t * t * (3.0 - 2.0 * t)).to_array()
}

/// One segment the puffs see this frame, with its curtain's fade factor; a `fading` curtain
/// spawns no puff.
pub(crate) struct Seen<'a> {
    pub(crate) segment: &'a Segment,
    pub(crate) fade: f32,
    pub(crate) fading: bool,
}

/// A segment's identity across curtains: its two ends. The copy a zone opening fades keeps the
/// key, so its puffs carry over.
type Key = [u32; 4];

fn key(s: &Segment) -> Key {
    [s.from[0], s.from[1], s.to[0], s.to[1]].map(f32::to_bits)
}

#[derive(Debug, Clone, Copy, Default)]
struct Slot {
    respawns: u32,
    puff: Option<Puff>,
}

/// A live puff and its curtain's fade, as the push draws it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Drawn {
    pub(crate) puff: Puff,
    pub(crate) fade: f32,
}

/// Every segment's puff slots, and the frame's live puffs.
#[derive(Resource, Default)]
pub(crate) struct MistPuffs {
    slots: HashMap<Key, Vec<Slot>>,
    pub(crate) alive: Vec<Drawn>,
}

/// The middle of `s`, `[x, y]`.
fn middle(s: &Segment) -> Vec2 {
    (Vec2::from(s.from) + Vec2::from(s.to)) / 2.0
}

impl MistPuffs {
    /// One frame of `dt` seconds with the camera at `cam` (WoW coordinates): the segments whose
    /// middle is within [`PUFF_RANGE_YD`], nearest first, keep up to [`PUFFS_PER_SEGMENT`] slots
    /// each until [`PUFF_CAP`]; each live puff steps, a dead one is replaced at a new spot unless
    /// its curtain is fading, and a new slot starts settled. Segments out of range or gone lose
    /// their puffs.
    pub(crate) fn update(&mut self, seen: &[Seen], cam: Vec3, dt: f32) {
        let mut near: Vec<(f32, &Seen)> = seen
            .iter()
            .filter(|s| !s.segment.wall)
            .map(|s| (middle(s.segment).distance(cam.truncate()), s))
            .filter(|&(d, _)| d <= PUFF_RANGE_YD)
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut old = std::mem::take(&mut self.slots);
        self.alive.clear();
        let mut used = 0;
        for (_, s) in near {
            let n = PUFFS_PER_SEGMENT.min(PUFF_CAP - used);
            if n == 0 {
                break;
            }
            used += n;
            let k = key(s.segment);
            let mut slots = old.remove(&k).unwrap_or_default();
            slots.truncate(n);
            let had = slots.len();
            slots.resize(n, Slot::default());
            for (i, slot) in slots.iter_mut().enumerate() {
                if let Some(p) = slot.puff.as_mut() {
                    step(p, dt);
                }
                let dead = slot.puff.is_some_and(|p| p.age >= p.life);
                if dead {
                    slot.puff = None;
                    slot.respawns += 1;
                }
                if !s.fading && slot.puff.is_none() {
                    slot.puff = Some(spawn(s.segment, i, slot.respawns, i >= had));
                }
                if let Some(puff) = slot.puff {
                    self.alive.push(Drawn { puff, fade: s.fade });
                }
            }
            self.slots.insert(k, slots);
        }
    }
}

/// The live puffs as camera-facing quads in Bevy space, back to front from `cam`, each turned by
/// its spin in the screen plane and coloured by [`colour`] for the fog colour `fog`. A puff with
/// no alpha pushes nothing.
pub(crate) fn push_puffs(
    out: &mut Vec<EffectVertex>,
    alive: &[Drawn],
    cam: Vec3,
    right: Vec3,
    up: Vec3,
    fog: [f32; 3],
) {
    let mut order: Vec<(f32, Vec3, &Drawn)> = alive
        .iter()
        .map(|d| {
            let c = wow_to_bevy(d.puff.pos.to_array());
            (c.distance(cam), c, d)
        })
        .collect();
    order.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (dist, c, d) in order {
        let p = &d.puff;
        let a = alpha(p, dist, d.fade);
        if a <= 0.0 {
            continue;
        }
        let [r, g, b] = colour(fog, p.pos.z - p.ground);
        let (sin, cos) = (p.angle + p.spin * p.age).sin_cos();
        let half = p.size / 2.0;
        let x = (right * cos + up * sin) * half;
        let y = (up * cos - right * sin) * half;
        // Perimeter order (bl, br, tr, tl), as the weather's puffs.
        for (corner, uv) in [
            (c - x - y, [0.0, 1.0]),
            (c + x - y, [1.0, 1.0]),
            (c + x + y, [1.0, 0.0]),
            (c - x + y, [0.0, 0.0]),
        ] {
            out.push(EffectVertex {
                pos: corner.to_array(),
                uv,
                color: [r, g, b, a],
            });
        }
    }
}

/// The puffs' texture, the weather mist's own.
#[derive(Resource)]
pub(crate) struct PuffTexture(Handle<Image>);

pub(crate) fn setup_mist_puffs(mut commands: Commands, asset_server: Res<AssetServer>) {
    let effect = |s: &mut benilla_assets::BlpLoaderSettings| {
        s.variant = benilla_assets::BlpVariant::Effect;
    };
    commands.insert_resource(PuffTexture(
        asset_server.load_with_settings("mpq://textures/weather/snowmist01.blp", effect),
    ));
    commands.init_resource::<MistPuffs>();
}

/// Steps every curtain's puffs for the frame, from the segments each curtain keeps; a fading
/// curtain's puffs take its [`open_fade`].
pub(crate) fn simulate_mist_puffs(
    time: Res<Time>,
    cam: Query<&GlobalTransform, With<WorldCamera>>,
    curtains: Query<(&MistSegments, Option<&FadingMist>)>,
    puffs: Option<ResMut<MistPuffs>>,
) {
    let (Some(mut puffs), Ok(cam)) = (puffs, cam.single()) else {
        return;
    };
    let seen: Vec<Seen> = curtains
        .iter()
        .flat_map(|(segments, fading)| {
            let (fade, fading) = fading.map_or((1.0, false), |f| (open_fade(f.secs), true));
            segments.0.iter().map(move |segment| Seen {
                segment,
                fade,
                fading,
            })
        })
        .collect();
    let cam = Vec3::from(bevy_to_wow(cam.translation()));
    puffs.update(&seen, cam, time.delta_secs());
}

/// Pushes the frame's puffs onto the effect stream after its clear: one alpha draw under the
/// scene's distance fog, sorted after the curtain by [`PUFF_SORT_BIAS`].
pub(crate) fn push_mist_puffs(
    puffs: Option<Res<MistPuffs>>,
    texture: Option<Res<PuffTexture>>,
    lighting: Option<Res<WowLighting>>,
    cam: Query<(Entity, &GlobalTransform), With<WorldCamera>>,
    quads: Option<ResMut<EffectQuads>>,
) {
    let (Some(puffs), Some(texture), Some(lighting), Some(mut quads)) =
        (puffs, texture, lighting, quads)
    else {
        return;
    };
    if puffs.alive.is_empty() {
        return;
    }
    let Ok((cam_entity, cam_tf)) = cam.single() else {
        return;
    };
    let cam_pos = cam_tf.translation();
    let start = quads.begin();
    push_puffs(
        &mut quads.verts,
        &puffs.alive,
        cam_pos,
        cam_tf.right().as_vec3(),
        cam_tf.up().as_vec3(),
        lighting.fog_color,
    );
    quads.commit_quads(
        start,
        EffectDrawSpec {
            cam: cam_entity,
            texture: texture.0.id(),
            blend: EffectBlend::Alpha,
            fog: EffectFog::Scene,
            lighting: EffectLighting::None,
            anchor: cam_pos,
            bias: PUFF_SORT_BIAS,
            raster_bias: 0,
            raster_slope: 0.0,
            cam_relative: false,
            no_depth_test: false,
            main_entity: Entity::PLACEHOLDER,
            light: None,
            clip: None,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mist::{wall_segment, SHELL_DEPTHS_YD};
    use benilla_formats::mist_walls::MistWall;

    /// A placed wall from `from` to `to` at Z 0, open toward +x.
    fn wall(from: [f32; 2], to: [f32; 2]) -> MistWall {
        MistWall {
            key: "test".into(),
            map: 0,
            from,
            to,
            bottom: -10.0,
            top: 100.0,
            floor: 0.0,
            open_side: [from[0] + 100.0, from[1]],
        }
    }

    /// A flat closed-zone segment from `from` to `to`, its closed side toward -x.
    fn edge(from: [f32; 2], to: [f32; 2]) -> Segment {
        Segment {
            wall: false,
            depths: SHELL_DEPTHS_YD,
            ..wall_segment(&wall(from, to))
        }
    }

    fn seen(segments: &[Segment], fade: f32, fading: bool) -> Vec<Seen<'_>> {
        segments
            .iter()
            .map(|segment| Seen {
                segment,
                fade,
                fading,
            })
            .collect()
    }

    /// Far past [`PUFF_NEAR_YD`].
    const FAR: f32 = 1000.0;

    #[test]
    fn the_same_seed_gives_the_same_puffs() {
        let s = edge([0.0, 0.0], [0.0, 33.3]);
        assert_eq!(spawn(&s, 0, 0, false), spawn(&s, 0, 0, false));
        assert_eq!(spawn(&s, 2, 5, true), spawn(&s, 2, 5, true));
        assert_ne!(spawn(&s, 0, 0, false), spawn(&s, 1, 0, false));
        assert_ne!(spawn(&s, 0, 0, false), spawn(&s, 0, 1, false));
    }

    #[test]
    fn puffs_stay_on_their_segment() {
        let s = edge([10.0, 20.0], [10.0, 53.3]);
        let (from, to) = (Vec2::from(s.from), Vec2::from(s.to));
        let u = (to - from).normalize();
        let n = Vec2::from(s.inward);
        let length = from.distance(to);
        let slack = PUFF_SIZE_YD.end / 2.0;
        for respawns in 0..1000 {
            let p = spawn(&s, (respawns % 3) as usize, respawns, false);
            let d = p.pos.truncate() - from;
            let along = d.dot(u);
            assert!(
                (-slack..=length + slack).contains(&along),
                "{along} yd along"
            );
            let depth = d.dot(n);
            assert!(
                (PUFF_DEPTH_YD.start - 1e-3..=PUFF_DEPTH_YD.end + 1e-3).contains(&depth),
                "{depth} yd deep"
            );
            assert!(PUFF_SIZE_YD.contains(&p.size));
            assert!(PUFF_LIFE_S.contains(&p.life));
            assert!(PUFF_DRIFT_YD_S.contains(&p.drift.abs()));
            assert!((p.pos.z - p.ground - PUFF_LIFT * p.size).abs() < 1e-3);
        }
    }

    #[test]
    fn alpha_rises_holds_and_falls() {
        let s = edge([0.0, 0.0], [0.0, 33.3]);
        let mut p = spawn(&s, 0, 0, false);
        assert_eq!(alpha(&p, FAR, 1.0), 0.0, "at birth");
        p.age = p.life / 2.0;
        assert!((alpha(&p, FAR, 1.0) - PUFF_ALPHA).abs() < 1e-6, "mid-life");
        assert_eq!(alpha(&p, 0.0, 1.0), 0.0, "at the camera");
        assert!((alpha(&p, PUFF_NEAR_YD.end, 1.0) - PUFF_ALPHA).abs() < 1e-6);
        assert!(alpha(&p, PUFF_NEAR_YD.start + 1.0, 1.0) < PUFF_ALPHA);
        p.age = p.life;
        assert_eq!(alpha(&p, FAR, 1.0), 0.0, "at death");
    }

    #[test]
    fn a_new_segment_starts_settled() {
        let segments = [
            edge([0.0, 0.0], [0.0, 33.3]),
            edge([0.0, 33.3], [0.0, 66.6]),
        ];
        let mut puffs = MistPuffs::default();
        puffs.update(&seen(&segments, 1.0, false), Vec3::ZERO, 0.0);
        assert_eq!(puffs.alive.len(), 6);
        let shares: Vec<f32> = puffs
            .alive
            .iter()
            .map(|d| d.puff.age / d.puff.life)
            .collect();
        assert!(shares.iter().all(|s| (0.0..1.0).contains(s)));
        let (low, high) = shares
            .iter()
            .fold((1.0f32, 0.0f32), |(l, h), &s| (l.min(s), h.max(s)));
        assert!(
            high - low > 0.3,
            "ages spread across their lives: {shares:?}"
        );
        // A respawn starts at age 0.
        puffs.update(&seen(&segments, 1.0, false), Vec3::ZERO, PUFF_LIFE_S.end);
        assert!(puffs.alive.iter().all(|d| d.puff.age == 0.0));
    }

    #[test]
    fn the_cap_keeps_the_nearest() {
        let segments: Vec<Segment> = (0..200)
            .map(|i| {
                let x = i as f32 * 0.5;
                edge([x, 0.0], [x, 33.3])
            })
            .collect();
        let mut puffs = MistPuffs::default();
        puffs.update(&seen(&segments, 1.0, false), Vec3::ZERO, 0.0);
        assert_eq!(puffs.alive.len(), PUFF_CAP);
        let kept = PUFF_CAP.div_ceil(PUFFS_PER_SEGMENT);
        assert_eq!(puffs.slots.len(), kept);
        for s in &segments[..kept] {
            assert!(puffs.slots.contains_key(&key(s)));
        }
    }

    #[test]
    fn a_fading_curtain_fades_its_puffs_and_spawns_none() {
        let segments = [edge([0.0, 0.0], [0.0, 33.3])];
        let mut puffs = MistPuffs::default();
        puffs.update(&seen(&segments, 1.0, false), Vec3::ZERO, 0.0);
        let before: Vec<Puff> = puffs.alive.iter().map(|d| d.puff).collect();
        puffs.update(&seen(&segments, 0.5, true), Vec3::ZERO, 0.0);
        assert_eq!(puffs.alive.len(), before.len(), "the puffs carry over");
        for (d, p) in puffs.alive.iter().zip(&before) {
            assert_eq!(d.puff, *p);
            let full = alpha(&d.puff, FAR, 1.0);
            assert!((alpha(&d.puff, FAR, d.fade) - full / 2.0).abs() < 1e-6);
        }
        // Every puff dies, and none is replaced.
        puffs.update(&seen(&segments, 0.5, true), Vec3::ZERO, PUFF_LIFE_S.end);
        assert!(puffs.alive.is_empty());
        // A curtain that is not fading replaces them, at age 0.
        let mut steady = MistPuffs::default();
        steady.update(&seen(&segments, 1.0, false), Vec3::ZERO, 0.0);
        steady.update(&seen(&segments, 1.0, false), Vec3::ZERO, PUFF_LIFE_S.end);
        assert_eq!(steady.alive.len(), PUFFS_PER_SEGMENT);
        assert!(steady.alive.iter().all(|d| d.puff.age == 0.0));
    }

    #[test]
    fn placed_walls_have_no_puffs() {
        let segments = [wall_segment(&wall([0.0, 0.0], [0.0, 20.0]))];
        let mut puffs = MistPuffs::default();
        puffs.update(&seen(&segments, 1.0, false), Vec3::ZERO, 0.0);
        assert!(puffs.alive.is_empty());
        assert!(puffs.slots.is_empty());
    }

    #[test]
    fn segments_out_of_range_lose_their_puffs() {
        let segments = [edge([0.0, 0.0], [0.0, 33.3])];
        let mut puffs = MistPuffs::default();
        puffs.update(&seen(&segments, 1.0, false), Vec3::ZERO, 0.0);
        let far = Vec3::new(PUFF_RANGE_YD * 2.0, 0.0, 0.0);
        puffs.update(&seen(&segments, 1.0, false), far, 0.0);
        assert!(puffs.alive.is_empty());
    }

    #[test]
    fn the_push_draws_back_to_front() {
        let segments = [edge([0.0, 0.0], [0.0, 33.3])];
        let mut puffs = MistPuffs::default();
        puffs.update(&seen(&segments, 1.0, false), Vec3::ZERO, 0.0);
        for d in &mut puffs.alive {
            d.puff.age = d.puff.life / 2.0;
        }
        let cam = wow_to_bevy([100.0, 16.0, 5.0]);
        let mut out = Vec::new();
        push_puffs(&mut out, &puffs.alive, cam, Vec3::X, Vec3::Y, [0.5; 3]);
        assert_eq!(out.len(), 4 * puffs.alive.len());
        let dists: Vec<f32> = out
            .chunks(4)
            .map(|q| {
                let c = q.iter().map(|v| Vec3::from(v.pos)).sum::<Vec3>() / 4.0;
                c.distance(cam)
            })
            .collect();
        assert!(dists.windows(2).all(|w| w[0] >= w[1] - 1e-3), "{dists:?}");
    }

    #[test]
    fn the_foot_is_dark_and_the_top_is_the_fog() {
        let fog = [0.6, 0.5, 0.4];
        let top = colour(fog, COLOUR_RAMP_YD);
        for (a, b) in top.iter().zip(fog) {
            assert!((a - b).abs() < 1e-6);
        }
        let foot = colour(fog, 0.0);
        assert!(foot.iter().zip(fog).all(|(a, b)| *a < b));
    }
}
