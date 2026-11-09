//! Level Bands, one file per zone (`content/elwynn_bands.toml`, `redridge_bands.toml`,
//! `duskwood_bands.toml`, `burning_steppes_bands.toml`): each sub-zone gives the vanilla creatures that spawn in it a level from
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
/// spawn tries them. The Burning Steppes' bounds reach over Northshire and Redridge's north edge,
/// so it comes last and the other zones keep their own land.
const FILES: [(&str, &str); 4] = [
    ("duskwood", include_str!("../content/duskwood_bands.toml")),
    ("redridge", include_str!("../content/redridge_bands.toml")),
    ("elwynn", include_str!("../content/elwynn_bands.toml")),
    (
        "burning_steppes",
        include_str!("../content/burning_steppes_bands.toml"),
    ),
];

fn parse(zone: &str, text: &str) -> BandFile {
    toml::from_str(text).unwrap_or_else(|e| panic!("content/{zone}_bands.toml parses: {e}"))
}

/// The band file for `zone`'s spawns, if it has one.
fn zone_file(zone: &str) -> Option<&'static BandFile> {
    static PARSED: [OnceLock<BandFile>; 4] = [const { OnceLock::new() }; 4];
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

    const ZONES: [(&str, &str); 2] = [
        ("redridge", include_str!("../content/redridge.toml")),
        ("duskwood", include_str!("../content/duskwood.toml")),
    ];

    /// The loaded spawn of a generated row, when its creature is hostile and not a critter.
    fn hostile_spawn(guid: u32, entry: u32) -> Option<&'static mobs::Spawn> {
        let hostile =
            mobs::template(entry).is_some_and(|t| t.hostile && !t.friendly && t.creature_type != 8);
        let spawn = mobs::content()
            .spawn
            .iter()
            .find(|s| s.map == EASTERN_KINGDOMS && s.guid == guid && s.entry == entry);
        spawn.filter(|_| hostile)
    }

    #[test]
    fn every_hostile_redridge_and_duskwood_spawn_takes_a_band() {
        // The bounds cover both zones whole (#616), so no spawn keeps a stretched level that no
        // band gives.
        for (zone, text) in ZONES {
            let (rows, _, _) = zone_rows(zone, text);
            let bands: Vec<[u8; 2]> = zone_file(zone).unwrap().band.iter().map(range).collect();
            let mut checked = 0;
            for [guid, entry, _, _, x, y] in rows {
                let Some(s) = hostile_spawn(guid as u32, entry as u32) else {
                    continue;
                };
                assert!(
                    range_in(zone, x as f32, y as f32).is_some(),
                    "{zone} spawn {guid} at {x}, {y} is outside the bands"
                );
                let inside = |[lo, hi]: &[u8; 2]| {
                    (*lo..=*hi).contains(&s.level_min) && (*lo..=*hi).contains(&s.level_max)
                };
                assert!(
                    bands.iter().any(inside),
                    "{zone} spawn {guid} ({entry}): {}-{}",
                    s.level_min,
                    s.level_max
                );
                checked += 1;
            }
            assert!(checked > 300, "only {checked} {zone} spawns checked");
        }
    }

    #[test]
    fn a_redridge_vanilla_entry_takes_a_band_only_from_a_band() {
        let (rows, _, _) = zone_rows("redridge", include_str!("../content/redridge.toml"));
        let [_, entry, _, _, x, _] = rows
            .into_iter()
            .find(|r| {
                let e = r[1] as u32;
                !mobs::is_vanilla(e) && mobs::template(e).is_some_and(|t| t.hostile)
            })
            .unwrap();
        // East of Redridge's bounds, where no band file reaches.
        let (entry, x, y) = (entry as u32, x as f32, -4300.0);
        assert_eq!(range_in("redridge", x, y), None);
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

    #[test]
    fn every_hostile_spawn_in_a_band_box_takes_that_band() {
        let want = [
            ("redridge", "Three Corners", [30, 32]),
            ("redridge", "Lakeshire", [31, 33]),
            ("redridge", "Lake Everstill", [32, 35]),
            ("redridge", "Lakeridge Highway", [33, 36]),
            ("redridge", "Ilgalar's Approach", [35, 38]),
            ("redridge", "Rethban Caverns", [38, 40]),
            ("redridge", "Redridge Canyons", [36, 39]),
            ("redridge", "Alther's Mill", [35, 37]),
            ("redridge", "Render's Camp", [41, 44]),
            ("redridge", "Render's Valley", [40, 43]),
            ("redridge", "Stonewatch", [43, 46]),
            ("redridge", "Stonewatch Falls", [44, 47]),
            ("redridge", "Galardell Valley", [45, 48]),
            ("duskwood", "The Darkened Bank", [30, 32]),
            ("duskwood", "Darkshire and Beggar's Haunt", [31, 33]),
            ("duskwood", "Manor Mistmantle", [32, 35]),
            ("duskwood", "Tranquil Gardens Cemetery", [33, 36]),
            ("duskwood", "Twilight Grove", [35, 38]),
            ("duskwood", "Raven Hill and the Hushed Bank", [36, 38]),
            ("duskwood", "The Yorgen Farmstead", [36, 39]),
            ("duskwood", "Addle's Stead", [37, 40]),
            ("duskwood", "Vul'Gol Ogre Mound", [38, 41]),
            ("duskwood", "Raven Hill Cemetery", [38, 42]),
            ("duskwood", "Brightwood Grove", [40, 43]),
            ("duskwood", "The Rotting Orchard", [45, 48]),
        ];
        for (zone, text) in ZONES {
            let f = zone_file(zone).unwrap();
            let names: Vec<&str> = f.band.iter().map(|b| b.name.as_str()).collect();
            let listed: Vec<&str> = want.iter().filter(|w| w.0 == zone).map(|w| w.1).collect();
            assert_eq!(names, listed, "{zone}");
            let (rows, _, _) = zone_rows(zone, text);
            for band in &f.band {
                let [lo, hi] = want.iter().find(|w| w.1 == band.name).unwrap().2;
                assert_eq!(range(band), [lo, hi], "{}", band.name);
                let mut checked = 0;
                for [guid, entry, _, _, x, y] in &rows {
                    let (x, y) = (*x as f32, *y as f32);
                    if !band.boxes.iter().any(|r| holds(r, x, y)) || !covers(f, x, y) {
                        continue;
                    }
                    let Some(s) = hostile_spawn(*guid as u32, *entry as u32) else {
                        continue;
                    };
                    // A level_overrides.toml area beats the band (Mor'Ladim, Morbent Fel).
                    if mobs::level_area(EASTERN_KINGDOMS, x, y) != Some(band.name.as_str()) {
                        continue;
                    }
                    let levels = s.level_min..=s.level_max;
                    assert!(
                        (lo..=hi).contains(levels.start()) && (lo..=hi).contains(levels.end()),
                        "{} spawn {} ({}) at {x}, {y}: {levels:?}",
                        band.name,
                        s.guid,
                        s.entry
                    );
                    checked += 1;
                }
                assert!(checked > 0, "no hostile spawn in {}", band.name);
            }
        }
    }

    /// The loaded spawns of `entry` on Eastern Kingdoms inside `area` ([x0, y0, x1, y1]).
    fn spawns_of(entry: u32, area: Rect) -> Vec<&'static mobs::Spawn> {
        mobs::content()
            .spawn
            .iter()
            .filter(|s| s.map == EASTERN_KINGDOMS && s.entry == entry)
            .filter(|s| holds(&area, s.position[0], s.position[1]))
            .collect()
    }

    #[test]
    fn the_lake_thresher_fetid_corpses_and_tranquil_gardens_skeletons_take_their_bands() {
        let cases = [
            (
                "Lake Thresher",
                14357,
                [-9700.0, -3000.0, -9100.0, -2300.0],
                32..=35,
            ),
            (
                "Fetid Corpse",
                1270,
                [-10450.0, -1450.0, -10200.0, -1150.0],
                32..=35,
            ),
            (
                "Skeletal Warrior",
                48,
                [-11150.0, -1450.0, -10850.0, -1200.0],
                33..=36,
            ),
        ];
        for (name, entry, area, levels) in cases {
            let spawns = spawns_of(entry, area);
            assert!(spawns.len() >= 2, "only {} {name} spawns", spawns.len());
            for s in spawns {
                assert!(
                    levels.contains(&s.level_min) && levels.contains(&s.level_max),
                    "{name} {}: {}-{}",
                    s.guid,
                    s.level_min,
                    s.level_max
                );
            }
        }
    }

    #[test]
    fn no_two_bands_of_a_zone_share_a_point() {
        let overlap =
            |a: &Rect, b: &Rect| a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3];
        // Elwynn's bands overlap on purpose, the band listed first winning.
        for (zone, _) in ZONES {
            let f = zone_file(zone).unwrap();
            for (i, a) in f.band.iter().enumerate() {
                for b in &f.band[i + 1..] {
                    for ra in &a.boxes {
                        for rb in &b.boxes {
                            assert!(!overlap(ra, rb), "{zone}: {} and {}", a.name, b.name);
                        }
                    }
                }
            }
        }
    }

    /// Ilgalar's door, the Tower of Ilgalar's portal (ilgalar.toml).
    const ILGALAR_DOOR: [f32; 2] = [-9282.0, -3328.0];

    #[test]
    fn a_shadowhide_gnoll_near_ilgalars_door_takes_the_approach() {
        // No generated Shadowhide Gnoll stands within 60 yd of the door, so a runtime one does,
        // on a ring round it; and the generated ones nearest the door, all within 120 yd.
        let [dx, dy] = ILGALAR_DOOR;
        for k in 0..16 {
            let a = k as f32 * std::f32::consts::TAU / 16.0;
            for r in [10.0, 30.0, 60.0] {
                let p = [dx + r * a.cos(), dy + r * a.sin(), 0.0];
                let [lo, hi] = mobs::banded(433, EASTERN_KINGDOMS, p).unwrap();
                assert!(lo >= 35 && hi <= 38, "at {p:?}: {lo}-{hi}");
            }
        }
        let near = spawns_of(433, [dx - 120.0, dy - 120.0, dx + 120.0, dy + 120.0]);
        let near: Vec<_> = near
            .into_iter()
            .filter(|s| (s.position[0] - dx).hypot(s.position[1] - dy) <= 120.0)
            .collect();
        assert!(near.len() >= 5, "only {} gnolls near the door", near.len());
        for s in near {
            assert!(s.level_min >= 35 && s.level_max <= 38, "gnoll {}", s.guid);
        }
    }

    #[test]
    fn rohh_the_silent_in_galardell_valley_is_45_to_48() {
        let (rows, _, _) = zone_rows("redridge", include_str!("../content/redridge.toml"));
        let [guid, _, _, _, x, y] = *rows.iter().find(|r| r[1] as u32 == 947).unwrap();
        assert!((x + 9010.0).abs() < 5.0 && (y + 3209.0).abs() < 5.0);
        let (x, y) = (x as f32, y as f32);
        assert_eq!(band_in("redridge", x, y), Some("Galardell Valley"));
        let s = hostile_spawn(guid as u32, 947).unwrap();
        assert!((45..=48).contains(&s.level_min) && (45..=48).contains(&s.level_max));
    }

    #[test]
    fn morladim_is_48_and_morbent_fel_40() {
        for (entry, level) in [(522, 48), (1200, 40)] {
            let s = spawns_of(entry, [-10583.0, 101.0, -10250.0, 483.0]);
            assert_eq!(s.len(), 1, "{entry}");
            assert_eq!((s[0].level_min, s[0].level_max), (level, level), "{entry}");
        }
    }

    #[test]
    fn tharilzun_is_44_and_corporal_keeshan_keeps_25() {
        for (entry, level) in [(486, 44), (349, 25)] {
            let s: Vec<_> = mobs::content()
                .spawn
                .iter()
                .filter(|s| s.map == EASTERN_KINGDOMS && s.entry == entry)
                .collect();
            assert_eq!(s.len(), 1, "{entry}");
            assert_eq!((s[0].level_min, s[0].level_max), (level, level), "{entry}");
        }
    }

    #[test]
    fn a_shadowhide_brute_in_galardell_valley_away_from_the_tower_takes_the_valley() {
        let p = [-9083.0, -3166.0, 0.0];
        assert_eq!(band_in("redridge", p[0], p[1]), Some("Galardell Valley"));
        assert_eq!(mobs::banded(432, EASTERN_KINGDOMS, p), Some([45, 48]));
        let (rows, _, _) = zone_rows("redridge", include_str!("../content/redridge.toml"));
        let near: Vec<_> = rows
            .iter()
            .filter(|r| r[1] as u32 == 432)
            .filter(|r| (r[4] as f32 - p[0]).hypot(r[5] as f32 - p[1]) <= 80.0)
            .collect();
        assert!(near.len() >= 2, "only {} brutes near {p:?}", near.len());
        for [guid, _, _, _, _, _] in near {
            let s = hostile_spawn(*guid as u32, 432).unwrap();
            let levels = [s.level_min, s.level_max];
            assert!(levels.iter().all(|l| (45..=48).contains(l)), "brute {guid}");
        }
    }

    /// The Lakeshire Watch's camp at Stonewatch Falls (#616, spec #595 decision 16).
    const WATCH_CAMP: [f32; 2] = [-9515.0, -3270.0];

    #[test]
    fn the_blackrock_outpost_at_stonewatch_falls_is_gone() {
        let removed = [6193, 6194, 6195, 6196, 6197, 6737, 6738];
        let [cx, cy] = WATCH_CAMP;
        for s in &mobs::content().spawn {
            if s.map != EASTERN_KINGDOMS {
                continue;
            }
            assert!(!removed.contains(&s.guid), "spawn {} remains", s.guid);
            let hostile = mobs::template(s.entry).is_some_and(|t| t.hostile && !t.friendly);
            let d = (s.position[0] - cx).hypot(s.position[1] - cy);
            assert!(
                !hostile || d > 30.0,
                "{} ({}) {d} yd from the camp",
                s.guid,
                s.entry
            );
        }
    }

    #[test]
    fn the_grey_captain_and_the_courier_stand_where_they_are_put() {
        // (entry, level, count, centre, nearest yd, farthest yd, band at the centre)
        let camps = [
            (96053, 47, 1, [-10460.0, -470.0], 0.0, 1.0, "Twilight Grove"),
            (
                96051,
                46,
                8,
                [-10460.0, -470.0],
                10.0,
                25.0,
                "Twilight Grove",
            ),
            (
                96052,
                47,
                4,
                [-10460.0, -470.0],
                10.0,
                25.0,
                "Twilight Grove",
            ),
            (
                96054,
                48,
                1,
                [-9120.0, -3240.0],
                0.0,
                1.0,
                "Galardell Valley",
            ),
            (
                96055,
                47,
                2,
                [-9120.0, -3240.0],
                0.0,
                5.0,
                "Galardell Valley",
            ),
        ];
        for (entry, level, count, [cx, cy], near, far, band) in camps {
            let zone = zone_at(cx, cy).unwrap();
            assert_eq!(band_in(zone, cx, cy), Some(band), "{entry}");
            let spawns: Vec<_> = mobs::content()
                .spawn
                .iter()
                .filter(|s| s.map == EASTERN_KINGDOMS && s.entry == entry)
                .collect();
            assert_eq!(spawns.len(), count, "{entry}");
            for s in spawns {
                let d = (s.position[0] - cx).hypot(s.position[1] - cy);
                assert!((near..=far + 0.01).contains(&d), "{} is {d} yd out", s.guid);
                assert_eq!((s.level_min, s.level_max), (level, level), "{}", s.guid);
            }
        }
    }

    #[test]
    fn the_grey_robes_dig_in_raven_hill_cemetery_outside_the_deeps() {
        let f = zone_file("duskwood").unwrap();
        let cemetery = f
            .band
            .iter()
            .find(|b| b.name == "Raven Hill Cemetery")
            .unwrap();
        let deeps = crate::phased::dungeon(11).expect("Raven Hill Deeps");
        let mut count = 0;
        // The Diggers and Lantern-bearers (#606), and the Surveyors (#611).
        for s in &mobs::content().spawn {
            if ![96048, 96049, 96050].contains(&s.entry) {
                continue;
            }
            count += 1;
            let [x, y, _] = s.position;
            assert_eq!(s.map, EASTERN_KINGDOMS);
            assert!(cemetery.boxes.iter().any(|r| holds(r, x, y)), "{}", s.guid);
            assert!(!deeps.holds(s.map, s.position), "{} in the Deeps", s.guid);
        }
        assert_eq!(count, 18);
    }

    /// `burning_steppes.toml`'s rows (#634).
    fn steppes_rows() -> Vec<[f64; 6]> {
        zone_rows(
            "burning_steppes",
            include_str!("../content/burning_steppes.toml"),
        )
        .0
    }

    #[test]
    fn a_burning_steppes_spawn_takes_its_sub_zones_band() {
        let rows = steppes_rows();
        for (band, [lo, hi]) in [
            ("Blackrock Pass", [50, 51]),
            ("Draco'dar", [57, 59]),
            ("Blackrock Mountain", [58, 60]),
        ] {
            let mut checked = 0;
            for [guid, entry, _, _, x, y] in &rows {
                let (x, y) = (*x as f32, *y as f32);
                let inside = zone_file("burning_steppes")
                    .unwrap()
                    .band
                    .iter()
                    .find(|b| b.name == band)
                    .is_some_and(|b| b.boxes.iter().any(|r| holds(r, x, y)));
                if !inside || band_in("burning_steppes", x, y) != Some(band) {
                    continue;
                }
                let Some(s) = hostile_spawn(*guid as u32, *entry as u32) else {
                    continue;
                };
                let levels = [s.level_min, s.level_max];
                assert!(
                    levels.iter().all(|l| (lo..=hi).contains(l)),
                    "{band} spawn {guid}: {levels:?}"
                );
                checked += 1;
            }
            assert!(checked > 0, "no hostile spawn in {band}");
        }
    }

    #[test]
    fn every_hostile_burning_steppes_spawn_is_50_to_60() {
        let mut checked = 0;
        for [guid, entry, _, _, x, y] in steppes_rows() {
            let Some(s) = hostile_spawn(guid as u32, entry as u32) else {
                continue;
            };
            assert!(range_in("burning_steppes", x as f32, y as f32).is_some());
            let levels = [s.level_min, s.level_max];
            assert!(
                levels.iter().all(|l| (50..=60).contains(l)),
                "spawn {guid} ({entry}): {levels:?}"
            );
            checked += 1;
        }
        assert!(checked > 300, "only {checked} Steppes spawns checked");
    }

    #[test]
    fn ragged_john_keeps_his_own_level_at_flame_crest() {
        let john = mobs::content()
            .spawn
            .iter()
            .find(|s| s.map == EASTERN_KINGDOMS && s.entry == 9563)
            .unwrap();
        let [x, y, _] = john.position;
        assert_eq!(band_in("burning_steppes", x, y), Some("Flame Crest"));
        assert!(mobs::template(9563).unwrap().friendly);
        assert_eq!((john.level_min, john.level_max), (57, 57));
    }

    #[test]
    fn a_runtime_spawn_at_morgans_vigil_takes_the_vigils_band() {
        // A Black Drake, a vanilla Steppes entry, at the Vigil; Redridge's bounds stop short.
        let p = [-8300.0, -2750.0, 0.0];
        assert_eq!(zone_at(p[0], p[1]), Some("burning_steppes"));
        assert_eq!(mobs::banded(7044, EASTERN_KINGDOMS, p), Some([50, 52]));
    }

    #[test]
    fn magistrate_solomon_and_lord_ebonlocke_keep_their_levels() {
        for (entry, level) in [(344, 36), (263, 30)] {
            let s = mobs::content()
                .spawn
                .iter()
                .find(|s| s.map == EASTERN_KINGDOMS && s.entry == entry)
                .unwrap();
            assert_eq!((s.level_min, s.level_max), (level, level), "{entry}");
        }
    }
}
