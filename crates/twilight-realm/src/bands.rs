//! Level Bands, one file per zone (`content/elwynn_bands.toml`, `redridge_bands.toml`,
//! `duskwood_bands.toml`): each sub-zone gives the vanilla creatures that spawn in it a level from
//! its band, in place of the template's own levels.

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
    #[serde(default)]
    band: Vec<Band>,
}

/// Each zone's band file, by the name of its spawn file without `.toml`, in the order a runtime
/// spawn tries them.
const FILES: [(&str, &str); 3] = [
    ("duskwood", include_str!("../content/duskwood_bands.toml")),
    ("redridge", include_str!("../content/redridge_bands.toml")),
    ("elwynn", include_str!("../content/elwynn_bands.toml")),
];

fn parse(zone: &str, text: &str) -> BandFile {
    toml::from_str(text).unwrap_or_else(|e| panic!("content/{zone}_bands.toml parses: {e}"))
}

/// The band file for `zone`'s spawns, if it has one.
fn zone_file(zone: &str) -> Option<&'static BandFile> {
    static PARSED: [OnceLock<BandFile>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let i = FILES.iter().position(|(z, _)| *z == zone)?;
    Some(PARSED[i].get_or_init(|| parse(FILES[i].0, FILES[i].1)))
}

#[cfg(test)]
fn file() -> &'static BandFile {
    zone_file("elwynn").expect("Elwynn has a band file")
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

/// The level range for a vanilla Elwynn creature standing at (`x`, `y`) on Eastern Kingdoms:
/// `range_in` for Elwynn's band file.
pub(crate) fn range_at(x: f32, y: f32) -> Option<[u8; 2]> {
    range_in("elwynn", x, y)
}

/// The level range for a creature of `zone`'s spawn file standing at (`x`, `y`) on Eastern
/// Kingdoms: the band holding the point, or the band with the nearest box. `None` for a zone with
/// no band file, outside its bounds and inside an excluded box.
pub(crate) fn range_in(zone: &str, x: f32, y: f32) -> Option<[u8; 2]> {
    band_for(zone_file(zone)?, x, y).map(range)
}

/// The name of the band `range_in` would use for the point.
pub(crate) fn band_in(zone: &str, x: f32, y: f32) -> Option<&'static str> {
    band_for(zone_file(zone)?, x, y).map(|b| b.name.as_str())
}

/// The first zone, in `FILES` order, whose band file covers the point: inside its bounds and
/// outside its excluded boxes. That file decides a runtime spawn's band and a point's band name.
pub(crate) fn zone_at(x: f32, y: f32) -> Option<&'static str> {
    FILES
        .iter()
        .map(|(z, _)| *z)
        .find(|z| zone_file(z).is_some_and(|f| covers(f, x, y)))
}

fn range(b: &Band) -> [u8; 2] {
    [b.level_min, b.level_max.max(b.level_min)]
}

fn covers(f: &BandFile, x: f32, y: f32) -> bool {
    f.bounds.iter().any(|b| holds(b, x, y)) && !f.exclude.iter().any(|b| holds(b, x, y))
}

/// The band of the box holding the point, or of the nearest box; on a tie the band listed first.
fn band_for(f: &BandFile, x: f32, y: f32) -> Option<&Band> {
    if !covers(f, x, y) {
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

    const TEST_FILE: &str = r#"
bounds = [[0.0, 0.0, 100.0, 100.0]]
exclude = [[80.0, 80.0, 100.0, 100.0]]

[[band]]
name = "West"
level_min = 30
level_max = 33
boxes = [[0.0, 0.0, 40.0, 40.0]]

[[band]]
name = "East"
level_min = 36
level_max = 38
boxes = [[20.0, 20.0, 60.0, 60.0]]
"#;

    #[test]
    fn a_band_file_looks_up_boxes_bounds_and_excludes() {
        let f = parse("test", TEST_FILE);
        let at = |x, y| band_for(&f, x, y).map(range);
        assert_eq!(at(10.0, 10.0), Some([30, 33]));
        assert_eq!(at(50.0, 50.0), Some([36, 38]));
        // In the bounds, in no box: the nearest box's band.
        assert_eq!(at(70.0, 60.0), Some([36, 38]));
        assert_eq!(at(5.0, 50.0), Some([30, 33]));
        assert_eq!(at(90.0, 90.0), None);
        assert_eq!(at(150.0, 10.0), None);
        // Both boxes hold (30, 30): the band listed first wins.
        assert_eq!(
            band_for(&f, 30.0, 30.0).map(|b| b.name.as_str()),
            Some("West")
        );
    }

    #[test]
    fn a_band_file_with_no_band_parses() {
        let f = parse("test", "bounds = [[0.0, 0.0, 1.0, 1.0]]\nexclude = []\n");
        assert!(band_for(&f, 0.5, 0.5).is_none());
        for zone in ["duskwood", "redridge"] {
            assert!(zone_file(zone).is_some_and(|f| f.band.is_empty()), "{zone}");
        }
        assert!(zone_file("stormwind").is_none());
    }

    /// `zone`'s generated rows (guid, entry, level_min, level_max, x, y) and its Level Stretch.
    fn zone_rows(zone: &str, text: &str) -> (Vec<[f64; 6]>, [u8; 2], [u8; 2]) {
        let v: toml::Value = toml::from_str(text).unwrap();
        let rows = v["spawns"].as_array().unwrap();
        let num = |c: &toml::Value| c.as_float().or(c.as_integer().map(|i| i as f64)).unwrap();
        let rows = rows
            .iter()
            .map(|r| std::array::from_fn(|i| num(&r[i])))
            .collect();
        let zones: toml::Value = toml::from_str(include_str!("../content/zones.toml")).unwrap();
        let z = zones["zone"]
            .as_array()
            .unwrap()
            .iter()
            .find(|z| z["file"].as_str() == Some(zone))
            .unwrap();
        let pair = |k: &str| -> [u8; 2] {
            let a = z["levels"][k].as_array().unwrap();
            [0, 1].map(|i| a[i].as_integer().unwrap() as u8)
        };
        (rows, pair("from"), pair("to"))
    }

    #[test]
    fn with_empty_band_files_redridge_and_duskwood_keep_their_stretch() {
        let zones = [
            ("redridge", include_str!("../content/redridge.toml")),
            ("duskwood", include_str!("../content/duskwood.toml")),
        ];
        for (zone, text) in zones {
            let (rows, from, to) = zone_rows(zone, text);
            let mut checked = 0;
            for [guid, entry, lmin, lmax, x, y] in rows.into_iter().step_by(3) {
                let (guid, entry) = (guid as u32, entry as u32);
                let hostile = mobs::template(entry)
                    .is_some_and(|t| t.hostile && !t.friendly && t.creature_type != 8);
                let spawn = mobs::content()
                    .spawn
                    .iter()
                    .find(|s| s.map == EASTERN_KINGDOMS && s.guid == guid && s.entry == entry);
                let Some(s) = spawn.filter(|_| hostile) else {
                    continue;
                };
                assert!(range_in(zone, x as f32, y as f32).is_none());
                let want = [lmin as u8, lmax as u8].map(|l| stretch(l, from, to));
                assert_eq!([s.level_min, s.level_max], want, "{zone} spawn {guid}");
                checked += 1;
            }
            assert!(checked > 20, "only {checked} {zone} spawns checked");
        }
    }

    #[test]
    fn a_redridge_vanilla_entry_takes_a_band_only_from_a_band() {
        let (rows, _, _) = zone_rows("redridge", include_str!("../content/redridge.toml"));
        let [_, entry, _, _, x, y] = rows
            .into_iter()
            .find(|r| {
                let e = r[1] as u32;
                !mobs::is_vanilla(e) && mobs::template(e).is_some_and(|t| t.hostile)
            })
            .unwrap();
        let (entry, x, y) = (entry as u32, x as f32, y as f32);
        assert_eq!(mobs::banded(entry, EASTERN_KINGDOMS, [x, y, 0.0]), None);
        assert_eq!(mobs::level_area(EASTERN_KINGDOMS, x, y), None);
        // The same parsing path, with one band around the point.
        let text = format!(
            "bounds = [[{x0}, {y0}, {x1}, {y1}]]\nexclude = []\n\n[[band]]\nname = \"Lakeshire\"\n\
             level_min = 31\nlevel_max = 34\nboxes = [[{x0}, {y0}, {x1}, {y1}]]\n",
            x0 = x - 50.0,
            y0 = y - 50.0,
            x1 = x + 50.0,
            y1 = y + 50.0,
        );
        let f = parse("redridge", &text);
        assert_eq!(band_for(&f, x, y).map(range), Some([31, 34]));
    }
}
