//! Elwynn Level Bands (`content/elwynn_bands.toml`): each sub-zone gives the vanilla creatures
//! that spawn in it a level from its band, in place of the template's own levels.

use std::sync::OnceLock;

use serde::Deserialize;

type Rect = [f32; 4];

#[derive(Deserialize)]
struct Band {
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
    let f = file();
    if !f.bounds.iter().any(|b| holds(b, x, y)) || f.exclude.iter().any(|b| holds(b, x, y)) {
        return None;
    }
    f.band
        .iter()
        .flat_map(|b| b.boxes.iter().map(move |r| (b, gap(r, x, y))))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(b, _)| [b.level_min, b.level_max.max(b.level_min)])
}

/// A level in `range` for creature `guid`, the same every time.
pub(crate) fn pick(guid: u32, [lo, hi]: [u8; 2]) -> u8 {
    lo + (guid % u32::from(hi - lo + 1)) as u8
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
    fn forest_edge_is_excluded() {
        assert!(range_at(-10000.0, 600.0).is_none());
    }
}
