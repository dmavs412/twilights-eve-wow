//! Assemble the M-key navigation map: aligned overview and terrain detail, fog and exploration.
use anyhow::{Context, Result};
use image::imageops;
#[path = "map_region/cartography.rs"]
mod cartography;
use serde_json::json;
use std::path::PathBuf;

#[cfg(test)]
fn project(rect: [f64; 4], size: (u32, u32), x: f64, y: f64) -> [f64; 2] {
    [
        (rect[0] - y) / (rect[0] - rect[1]) * size.0 as f64,
        (rect[2] - x) / (rect[2] - rect[3]) * size.1 as f64,
    ]
}
fn main() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join("data/twilights-eve/map");
    let mut out = root.join("target/map_region");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => {
                out = args
                    .next()
                    .map(PathBuf::from)
                    .context("--out needs a directory")?
            }
            _ => anyhow::bail!("map_region [--out directory]"),
        }
    }
    std::fs::create_dir_all(out.join("covers"))?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(source.join("manifest.json"))?)?;
    let survey: serde_json::Value =
        serde_json::from_slice(&std::fs::read(source.join("survey/survey.json"))?)?;
    let rect: [f64; 4] = serde_json::from_value(manifest["canvas"]["loc_rect"].clone())?;
    anyhow::ensure!(
        serde_json::from_value::<[f64; 4]>(survey["loc_rect"].clone())? == rect,
        "survey and game world bounds differ"
    );
    let size: (u32, u32) = serde_json::from_value(manifest["canvas"]["size"].clone())?;
    let terrain = image::open(source.join("survey/terrain-survey.png"))?.to_rgba8();
    let mask = image::open(source.join("survey/terrain-survey-fog.png"))?.to_rgba8();
    let (geography, detail) = cartography::layers(&terrain, size);
    let fog = imageops::resize(&mask, size.0, size.1, imageops::FilterType::Nearest);
    detail.save(out.join("world-map-detail.png"))?;
    let mut preview = geography.clone();
    imageops::overlay(&mut preview, &fog, 0, 0);
    geography.save(out.join("world-map-geography.png"))?;
    fog.save(out.join("world-map-exact-fog.png"))?;
    preview.save(out.join("world-map-exact.png"))?;
    // Keep exploration behavior, replacing old illustrated cover colors with the same
    // continuous cloud material. The original overlay alpha and UV placement are unchanged.
    let mut cover_count = 0;
    for cover in manifest["covers"]
        .as_array()
        .context("no exploration covers")?
    {
        let file = cover["file"].as_str().context("cover filename")?;
        let mut image = image::open(source.join(file))?.to_rgba8();
        let uv: [f64; 4] = serde_json::from_value(cover["rect"].clone())?;
        let used: [u32; 2] = serde_json::from_value(cover["used"].clone())?;
        for y in 0..used[1] {
            for x in 0..used[0] {
                let u = uv[0] + (uv[2] - uv[0]) * (x as f64 + 0.5) / used[0] as f64;
                let v = uv[1] + (uv[3] - uv[1]) * (y as f64 + 0.5) / used[1] as f64;
                let color = fog.get_pixel(
                    (u * size.0 as f64).clamp(0.0, (size.0 - 1) as f64) as u32,
                    (v * size.1 as f64).clamp(0.0, (size.1 - 1) as f64) as u32,
                );
                let p = image.get_pixel_mut(x, y);
                p[0] = color[0];
                p[1] = color[1];
                p[2] = color[2];
            }
        }
        image.save(out.join(file))?;
        cover_count += 1;
    }
    // Burning Steppes must be fully inside the unchanged game map extent.
    let burning_held = benilla_formats::map_canvas::held().contains(&46);
    anyhow::ensure!(
        burning_held,
        "Burning Steppes must remain included in the canvas extent"
    );
    let burning = manifest["fog"]
        .as_array()
        .context("no zone fog extents")?
        .iter()
        .find(|f| f["file"] == "fog_burningsteppes.png")
        .context("missing Burning Steppes extent")?;
    let burning_rect: [f64; 4] = serde_json::from_value(burning["rect"].clone())?;
    anyhow::ensure!(
        burning_rect.iter().all(|&v| (0.0..=1.0).contains(&v)),
        "Burning Steppes is clipped by the regional extent"
    );
    let report = json!({
        "size": [size.0,size.1], "loc_rect": rect,
        "terrain": "coordinate-preserving parchment treatment of the terrain survey",
        "detail": "original survey terrain, same sampling grid, no generated geography or city card",
        "burning_steppes_included": burning_held,
        "burning_steppes_status": "held, fogged until opened in map_canvas.json",
        "exploration_covers": cover_count,
        "native_source_resolution_yards": survey["source_resolution_yards_per_texel_range"],
        "survey_projection_max_error_yards": survey["projection_round_trip_max_yards"],
        "note": "Zone labels and quest, service and dungeon markers are live UI overlays. Overview and close terrain share one world-coordinate projection."
    });
    std::fs::write(
        out.join("world-map-region.json"),
        serde_json::to_string_pretty(&report)?,
    )?;
    println!("Navigation map assembled: {}x{}, surveyed overview and detail, {cover_count} exploration covers", size.0, size.1);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn region_coordinates_preserve_cardinal_directions() {
        let rect = [2260.9375, -4467.1875, -7031.24951171875, -11516.666015625];
        let size = (5015, 3343);
        let p = project(rect, size, -9000.0, 500.0);
        let east = project(rect, size, -9000.0, 400.0);
        let south = project(rect, size, -9100.0, 500.0);
        assert!(east[0] > p[0] && east[1] == p[1]);
        assert!(south[1] > p[1] && south[0] == p[0]);
        assert_eq!(project(rect, size, rect[2], rect[0]), [0.0, 0.0]);
    }
}
