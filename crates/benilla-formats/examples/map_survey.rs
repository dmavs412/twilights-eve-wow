//! A world-coordinate survey from the installed client's terrain minimap bakes and ADT areas.
//! No illustrated zone maps, painted fill, geographic warps, or fabricated terrain.
use anyhow::{Context, Result};
use image::{Rgba, RgbaImage};
use serde_json::json;
use std::{collections::BTreeMap, io::Cursor, path::PathBuf};

const TILE: f64 = 533.333_3_f32 as f64;
const CENTER: f64 = 32.0 * TILE;

fn noise(x: f64, y: f64) -> f64 {
    let (ix, iy) = (x.floor() as u32, y.floor() as u32);
    let hash = |x: u32, y: u32| {
        let mut h = x
            .wrapping_mul(374761393)
            .wrapping_add(y.wrapping_mul(668265263));
        h = (h ^ (h >> 13)).wrapping_mul(1274126177);
        (h ^ (h >> 16)) as f64 / u32::MAX as f64
    };
    let smooth = |t: f64| t * t * (3.0 - 2.0 * t);
    let (u, v) = (smooth(x.fract()), smooth(y.fract()));
    let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;
    lerp(
        lerp(hash(ix, iy), hash(ix + 1, iy), u),
        lerp(hash(ix, iy + 1), hash(ix + 1, iy + 1), u),
        v,
    )
}

struct Tile {
    image: Option<RgbaImage>,
    areas: [u32; 256],
}

fn project(rect: [f64; 4], size: (u32, u32), x: f64, y: f64) -> [f64; 2] {
    [
        (rect[0] - y) / (rect[0] - rect[1]) * size.0 as f64,
        (rect[2] - x) / (rect[2] - rect[3]) * size.1 as f64,
    ]
}

fn unproject(rect: [f64; 4], size: (u32, u32), px: f64, py: f64) -> [f64; 2] {
    [
        rect[2] - py / size.1 as f64 * (rect[2] - rect[3]),
        rect[0] - px / size.0 as f64 * (rect[0] - rect[1]),
    ]
}

fn main() -> Result<()> {
    let mut data = benilla_formats::wow_data();
    let mut out = PathBuf::from("target/map_survey");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data" => data = args.next().map(PathBuf::from),
            "--out" => {
                out = args
                    .next()
                    .map(PathBuf::from)
                    .context("--out needs a directory")?
            }
            _ => anyhow::bail!("usage: map_survey [--data Data] [--out directory]"),
        }
    }
    std::fs::create_dir_all(&out)?;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("data/twilights-eve/map/manifest.json"),
    )?)?;
    let rect: [f64; 4] = serde_json::from_value(manifest["canvas"]["loc_rect"].clone())?;
    let size = (
        (rect[0] - rect[1]).ceil() as u32,
        (rect[2] - rect[3]).ceil() as u32,
    );
    let (_, playable) = benilla_formats::map_canvas::canvas();
    let mut chain = benilla_formats::open_chain(&data.context("no WoW Data folder")?)?;
    let translate = benilla_formats::load_minimap_translate(&mut chain)?;
    let areas = benilla_formats::load_area_table_catalog(&mut chain)?;
    let pois = benilla_formats::load_area_poi_catalog(&mut chain)?;
    let (west, east) = (
        ((CENTER - rect[0]) / TILE).floor() as u32,
        ((CENTER - rect[1]) / TILE).floor() as u32,
    );
    let (north, south) = (
        ((CENTER - rect[2]) / TILE).floor() as u32,
        ((CENTER - rect[3]) / TILE).floor() as u32,
    );
    let mut tiles = BTreeMap::new();
    let mut records = Vec::new();
    let mut grove = (0.0f64, 0.0f64, 0u64);
    let mut resolutions = Vec::new();
    for ty in north..=south {
        for tx in west..=east {
            let path = format!("World\\Maps\\Azeroth\\Azeroth_{tx}_{ty}.adt");
            let mut ids = [0u32; 256];
            if let Ok(bytes) = chain.read_file(&path) {
                let benilla_adt::ParsedAdt::Root(adt) =
                    benilla_adt::parse_adt(&mut Cursor::new(&*bytes))?;
                for chunk in &adt.mcnk_chunks {
                    // Header positions are authoritative world X north / Y west.
                    let wx = chunk.header.position[0] as f64;
                    let wy = chunk.header.position[1] as f64;
                    let col = (((CENTER - tx as f64 * TILE - wy) / (TILE / 16.0)).round() as usize)
                        .min(15);
                    let row = (((CENTER - ty as f64 * TILE - wx) / (TILE / 16.0)).round() as usize)
                        .min(15);
                    ids[row * 16 + col] = chunk.header.area_id;
                    if areas
                        .name(chunk.header.area_id)
                        .is_some_and(|n| n.contains("Twilight Grove"))
                    {
                        grove.0 += wx - TILE / 32.0;
                        grove.1 += wy - TILE / 32.0;
                        grove.2 += 1;
                    }
                }
            }
            let logical = translate.tile("Azeroth", tx, ty);
            let image = match logical {
                Some(hash) => {
                    let (w, h, rgba) = benilla_formats::read_texture_rgba(
                        &mut chain,
                        &format!("textures\\Minimap\\{hash}"),
                    )?;
                    resolutions.push(TILE / w as f64);
                    records.push(json!({"tile": [tx,ty], "hash": hash, "size": [w,h], "yards_per_source_texel": [TILE / w as f64,TILE / h as f64]}));
                    RgbaImage::from_raw(w, h, rgba)
                }
                None => None,
            };
            tiles.insert((tx, ty), Tile { image, areas: ids });
        }
    }
    let mut survey = RgbaImage::new(size.0, size.1);
    let mut fog = RgbaImage::new(size.0, size.1);
    let mut preview = RgbaImage::new(size.0, size.1);
    let mut unknown_pixels = 0u64;
    let mut missing_playable_pixels = 0u64;
    for py in 0..size.1 {
        for px in 0..size.0 {
            let [wx, wy] = unproject(rect, size, px as f64 + 0.5, py as f64 + 0.5);
            let gx = (CENTER - wy) / TILE;
            let gy = (CENTER - wx) / TILE;
            let tile = &tiles[&(gx.floor() as u32, gy.floor() as u32)];
            let (u, v) = (gx.fract(), gy.fract());
            let id = tile.areas[(v * 16.0) as usize * 16 + (u * 16.0) as usize];
            let open = areas
                .top_zone(id)
                .is_some_and(|zone| playable.contains(&zone));
            let color = if let Some(img) = &tile.image {
                let x = ((u * img.width() as f64) as u32).min(img.width() - 1);
                let y = ((v * img.height() as f64) as u32).min(img.height() - 1);
                *img.get_pixel(x, y)
            } else {
                unknown_pixels += 1;
                if open {
                    missing_playable_pixels += 1;
                }
                // Missing source is explicit, never invented terrain or assumed ocean.
                Rgba([35, 39, 46, 255])
            };
            let light = (191.0
                + 28.0
                    * (noise(px as f64 / 220.0, py as f64 / 220.0) * 0.7
                        + noise(px as f64 / 65.0, py as f64 / 65.0) * 0.3))
                as u8;
            let cloud = Rgba([light, light + 4, light + 9, if open { 0 } else { 255 }]);
            survey.put_pixel(px, py, color);
            fog.put_pixel(px, py, cloud);
            preview.put_pixel(px, py, if open { color } else { cloud });
        }
    }
    // Feather only into open terrain: every closed-area pixel stays fully opaque.
    // This softens the ADT chunk steps without revealing any locked terrain.
    let soft = image::imageops::blur(&fog, 8.0);
    for py in 0..size.1 {
        for px in 0..size.0 {
            let mut cloud = *fog.get_pixel(px, py);
            cloud[3] = cloud[3].max(soft.get_pixel(px, py)[3]);
            fog.put_pixel(px, py, cloud);
            let terrain = survey.get_pixel(px, py);
            let alpha = cloud[3] as f64 / 255.0;
            let mut color = [0u8; 4];
            for channel in 0..3 {
                color[channel] = (cloud[channel] as f64 * alpha
                    + terrain[channel] as f64 * (1.0 - alpha))
                    .round() as u8;
            }
            color[3] = 255;
            preview.put_pixel(px, py, Rgba(color));
        }
    }
    survey.save(out.join("terrain-survey.png"))?;
    fog.save(out.join("terrain-survey-fog.png"))?;
    preview.save(out.join("terrain-survey-playable.png"))?;
    let mut markers = Vec::new();
    for (id, poi) in pois.rows() {
        if poi.continent_id != 0 {
            continue;
        }
        let pixel = project(rect, size, poi.pos[0] as f64, poi.pos[1] as f64);
        if pixel[0] < 0.0
            || pixel[1] < 0.0
            || pixel[0] >= size.0 as f64
            || pixel[1] >= size.1 as f64
        {
            continue;
        }
        if fog.get_pixel(pixel[0] as u32, pixel[1] as u32)[3] != 0 {
            continue;
        }
        markers.push(json!({"id": id, "name": poi.name, "world": poi.pos, "pixel": pixel, "source": "AreaPOI.dbc exact world position"}));
    }
    if grove.2 > 0 {
        let pos = [grove.0 / grove.2 as f64, grove.1 / grove.2 as f64];
        markers.push(json!({"name": "Necropolis", "world": pos, "pixel": project(rect,size,pos[0],pos[1]), "source": "Custom annotation at the ADT Twilight Grove area centroid; not a verified placed world model"}));
    }
    let yards = [
        (rect[0] - rect[1]) / size.0 as f64,
        (rect[2] - rect[3]) / size.1 as f64,
    ];
    let mut max_error = 0.0f64;
    for row in 0..=100 {
        for col in 0..=100 {
            let x = rect[2] - (rect[2] - rect[3]) * row as f64 / 100.0;
            let y = rect[0] - (rect[0] - rect[1]) * col as f64 / 100.0;
            let p = project(rect, size, x, y);
            let back = unproject(rect, size, p[0], p[1]);
            max_error = max_error.max((x - back[0]).hypot(y - back[1]));
        }
    }
    anyhow::ensure!(
        max_error < 0.000001,
        "projection round trip exceeds a millionth of a yard"
    );
    std::fs::write(
        out.join("survey.json"),
        serde_json::to_string_pretty(&json!({
            "size": [size.0,size.1], "loc_rect": rect, "yards_per_pixel": yards,
            "pixel_center_quantization_max_yards": yards[0].hypot(yards[1]) / 2.0,
            "projection_round_trip_max_yards": max_error,
            "projection_test_points": 10201,
            "source": "Installed client terrain minimap bakes, MD5Translate.trs, ADT MCNK area IDs, AreaTable.dbc and AreaPOI.dbc",
            "fog_source": "Actual ADT area ancestry, not the illustrated map's Highlights",
            "missing_source_pixels": unknown_pixels, "missing_playable_source_pixels": missing_playable_pixels,
            "source_resolution_yards_per_texel_range": [resolutions.iter().copied().fold(f64::INFINITY,f64::min),resolutions.iter().copied().fold(0.0,f64::max)],
            "accuracy_limit": "Coordinates are precise; source minimap texels have their recorded resolution. Raster upsampling does not create sub-texel geometric detail. Missing bakes remain explicitly uncharted.",
            "stormwind_reference": "Stormwind.png is the complete original stock city illustration, separate from the coordinate survey",
            "markers": markers, "tiles": records
        }))?,
    )?;
    println!("Survey {}x{}; {} source tiles; round-trip error {max_error:.12} yd; {missing_playable_pixels} playable pixels without source",size.0,size.1,resolutions.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn world_axes_and_bounds_are_not_swapped_or_flipped() {
        let r = [2000.0, -4000.0, -7000.0, -11000.0];
        assert_eq!(project(r, (6000, 4000), -7000.0, 2000.0), [0.0, 0.0]);
        assert_eq!(
            project(r, (6000, 4000), -11000.0, -4000.0),
            [6000.0, 4000.0]
        );
        assert_eq!(unproject(r, (6000, 4000), 1234.0, 2345.0), [-9345.0, 766.0]);
    }
}
