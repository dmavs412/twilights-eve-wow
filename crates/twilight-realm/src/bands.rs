//! Elwynn Level Bands (`content/elwynn_bands.toml`): each sub-zone gives the vanilla creatures
//! that spawn in it a level from its band, in place of the template's own levels.

use std::sync::OnceLock;

use serde::Deserialize;

type Rect = [f32; 4];

#[derive(Deserialize)]
struct Band {
    name: String,
    level_min: u8,
    level_max: u8,
    boxes: Vec<Rect>,
}

#[derive(Deserialize)]
struct BandFile {
    bounds: Vec<Rect>,
    exclude: Vec<Rect>,
    band: Vec<Band>,
}

fn file() -> &'static BandFile {
    static FILE: OnceLock<BandFile> = OnceLock::new();
    FILE.get_or_init(|| {
        toml::from_str(include_str!("../content/elwynn_bands.toml"))
            .expect("content/elwynn_bands.toml parses")
    })
}

fn holds(r: &Rect, x: f32, y: f32) -> bool {
    (r[0]..=r[2]).contains(&x) && (r[1]..=r[3]).contains(&y)
}

/// Squared yards from the point to the box; 0 inside it.
fn gap(r: &Rect, x: f32, y: f32) -> f32 {
    let dx = (r[0] - x).max(x - r[2]).max(0.0);
    let dy = (r[1] - y).max(y - r[3]).max(0.0);
    dx * dx + dy * dy
}

/// The level range for a vanilla creature standing at (`x`, `y`) on Eastern Kingdoms: the band
/// holding the point, or the band with the nearest box. `None` outside the bounds and inside an
/// excluded box.
pub(crate) fn range_at(x: f32, y: f32) -> Option<[u8; 2]> {
    band_for(x, y).map(|b| [b.level_min, b.level_max.max(b.level_min)])
}

/// The name of the band `range_at` would use for the point.
pub(crate) fn band_at(x: f32, y: f32) -> Option<&'static str> {
    band_for(x, y).map(|b| b.name.as_str())
}

fn band_for(x: f32, y: f32) -> Option<&'static Band> {
    let f = file();
    if !f.bounds.iter().any(|b| holds(b, x, y)) || f.exclude.iter().any(|b| holds(b, x, y)) {
        return None;
    }
    f.band
        .iter()
        .flat_map(|b| b.boxes.iter().map(move |r| (b, gap(r, x, y))))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(b, _)| b)
}

/// A level in `range` for creature `guid`, the same every time.
pub(crate) fn pick(guid: u32, [lo, hi]: [u8; 2]) -> u8 {
    lo + (guid % u32::from(hi - lo + 1)) as u8
}

/// A Level Stretch: `level` mapped linearly from the `from` range onto the `to` range, rounded
/// half away from zero and clamped into `to`.
pub(crate) fn stretch(level: u8, from: [u8; 2], to: [u8; 2]) -> u8 {
    debug_assert!(
        from[0] < from[1],
        "a stretch needs a range of two levels or more"
    );
    let [f0, f1] = from.map(f32::from);
    let [t0, t1] = to.map(f32::from);
    let mapped = t0 + (f32::from(level) - f0) * (t1 - t0) / (f1 - f0);
    mapped.round().clamp(t0, t1) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::EASTERN_KINGDOMS;
    use crate::mobs;

    /// The guids of `elwynn.toml`'s rows.
    fn elwynn_guids() -> Vec<u32> {
        let v: toml::Value = toml::from_str(include_str!("../content/elwynn.toml")).unwrap();
        let rows = v["spawns"].as_array().unwrap();
        rows.iter()
            .map(|r| r[0].as_integer().unwrap() as u32)
            .collect()
    }

    #[test]
    fn every_elwynn_spawn_in_the_bounds_takes_a_band() {
        let guids = elwynn_guids();
        let bands: Vec<[u8; 2]> = file()
            .band
            .iter()
            .map(|b| [b.level_min, b.level_max])
            .collect();
        let mut checked = 0;
        for s in &mobs::content().spawn {
            let [x, y, _] = s.position;
            if s.map != EASTERN_KINGDOMS || !guids.contains(&s.guid) || !mobs::is_vanilla(s.entry) {
                continue;
            }
            let Some(range) = range_at(x, y) else {
                continue;
            };
            checked += 1;
            assert!(bands.contains(&range));
            assert_eq!([s.level_min, s.level_max], range, "spawn {}", s.guid);
        }
        assert!(checked > 1000, "only {checked} spawns checked");
    }

    #[test]
    fn northshire_keeps_its_template_levels() {
        // Young Wolf is level 1 in Northshire's levels; Marshal McBride is 20 and hostile to none.
        let wolf = mobs::content()
            .spawn
            .iter()
            .find(|s| s.entry == 299 && s.position[0] > -9160.0)
            .unwrap();
        assert!(range_at(wolf.position[0], wolf.position[1]).is_none());
        assert_eq!((wolf.level_min, wolf.level_max), (1, 2));
        let mcbride = mobs::content()
            .spawn
            .iter()
            .find(|s| s.entry == 197)
            .unwrap();
        assert_eq!((mcbride.level_min, mcbride.level_max), (20, 20));
    }

    #[test]
    fn a_hand_template_keeps_its_own_level_inside_a_band() {
        let t = mobs::content()
            .template
            .iter()
            .find(|t| !mobs::is_vanilla(t.entry))
            .unwrap();
        // Goldshire's Lion's Pride Inn.
        assert!(range_at(-9462.0, 16.0).is_some());
        assert_eq!(
            mobs::banded(t.entry, EASTERN_KINGDOMS, [-9462.0, 16.0, 0.0]),
            None
        );
    }

    #[test]
    fn a_runtime_spawn_in_goldshire_takes_goldshire_levels() {
        let vanilla = mobs::content()
            .template
            .iter()
            .find(|t| mobs::is_vanilla(t.entry))
            .unwrap();
        for guid in 0..40 {
            let [lo, hi] =
                mobs::banded(vanilla.entry, EASTERN_KINGDOMS, [-9462.0, 16.0, 0.0]).unwrap();
            assert!((10..=13).contains(&pick(guid, [lo, hi])));
        }
    }

    #[test]
    fn the_shore_north_of_stone_cairn_lake_takes_a_band() {
        // Fedfennel's point, and the strip's Prowlers and Murloc Foragers (#260).
        let [lo, _] = range_at(-8978.21, -829.324).unwrap();
        assert!(lo >= 15);
        let mut checked = 0;
        for s in &mobs::content().spawn {
            let [x, y, _] = s.position;
            if s.map != EASTERN_KINGDOMS || !(-8996.0..=-8804.0).contains(&x) {
                continue;
            }
            if !(-1252.0..=-732.0).contains(&y) || !mobs::is_vanilla(s.entry) {
                continue;
            }
            let hostile = mobs::template(s.entry).is_some_and(|t| t.hostile);
            if hostile {
                checked += 1;
                assert!(s.level_min >= 15, "spawn {} at {x}, {y}", s.guid);
            }
        }
        assert!(checked >= 50, "only {checked} strip spawns checked");
    }

    #[test]
    fn no_friendly_npc_changes_level_north_of_the_old_edge() {
        for s in &mobs::content().spawn {
            let [x, y, _] = s.position;
            let friendly = mobs::template(s.entry).is_some_and(|t| t.friendly);
            if s.map == EASTERN_KINGDOMS && friendly && x > -9000.0 && range_at(x, y).is_some() {
                panic!("friendly {} ({}) at {x}, {y} takes a band", s.entry, s.guid);
            }
        }
    }

    #[test]
    fn stretch_maps_the_from_range_onto_the_to_range_and_clamps() {
        let duskwood = [18, 35];
        for (level, want) in [(18, 30), (19, 31), (20, 32), (35, 50), (36, 50)] {
            assert_eq!(stretch(level, duskwood, [30, 50]), want, "Duskwood {level}");
        }
        let redridge = [14, 27];
        let cases = [
            (14, 30),
            (15, 32),
            (16, 33),
            (24, 45),
            (25, 47),
            (27, 50),
            (1, 30),
        ];
        for (level, want) in cases {
            assert_eq!(stretch(level, redridge, [30, 50]), want, "Redridge {level}");
        }
    }

    #[test]
    fn forest_edge_is_excluded() {
        assert!(range_at(-10000.0, 600.0).is_none());
    }
}
