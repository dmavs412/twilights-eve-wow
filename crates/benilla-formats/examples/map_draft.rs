//! The painted map's draft (#422, spec #421): the canvas image, one fog per misted zone and the
//! exploration covers, cut from the stock zone maps for Davis to edit.
//! `cargo run -q -p benilla-formats --example map_draft -- --data <Data> --out target/map_draft`
//!
//! Each zone's art is cut along its `<file>Highlight` silhouette, which drops the parchment frame.
//! The tool writes only under `--out`; the committed PNGs are the source of truth and it never
//! touches them. Output is Blizzard data: commit only what Davis has edited (#423).

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use benilla_formats::{
    load_world_map_area_catalog, load_world_map_overlay_catalog, read_texture_rgba, Chain,
    WorldMapArea, WorldMapOverlay,
};
use image::RgbaImage;
use serde_json::{json, Value};

/// The canvas image's scale: Stormwind City at its stock size, its 1002 art pixels across its rect.
const SCALE_ZONE: u32 = 1519;
/// The part of a zone's 4 by 3 grid of 256 px tiles the stock map shows.
const ART_W: u32 = 1002;
const ART_H: u32 = 668;
const TILE: u32 = 256;
const TILES_WIDE: u32 = 4;
const TILES_TALL: u32 = 3;
/// The mist tint of `WorldMapMist.xml`, and the fog's alpha inside the zone.
const FOG_TINT: [f32; 3] = [0.78, 0.80, 0.82];
const FOG_ALPHA: f32 = 0.9;
/// The silhouette values the fog's edge runs between: clear below the first, solid above the
/// second, so the fog fills the zone flat with a soft rim instead of tracing the Highlight's glow.
const FOG_EDGE: (f32, f32) = (0.15, 0.45);
/// For a zone with a black Highlight: the torn parchment frame's width in art pixels, and the soft
/// edge inside it.
const FRAME_INSET: f32 = 30.0;
const FRAME_FADE: f32 = 40.0;

/// A loc rect: left and right are world Y, top and bottom world X, as `WorldMapArea` has them.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

impl Rect {
    fn of(a: &WorldMapArea) -> Self {
        Rect {
            left: a.loc_left,
            right: a.loc_right,
            top: a.loc_top,
            bottom: a.loc_bottom,
        }
    }
}

/// `benilla_world::map_proj::canvas_rect`: the union of `rects`, padded evenly on its short axis to
/// the art's 1002 / 668.
fn canvas_rect(rects: &[Rect]) -> Option<Rect> {
    let aspect = ART_W as f32 / ART_H as f32;
    let first = *rects.first()?;
    let mut r = rects.iter().fold(first, |a, b| Rect {
        left: a.left.max(b.left),
        right: a.right.min(b.right),
        top: a.top.max(b.top),
        bottom: a.bottom.min(b.bottom),
    });
    let (width, height) = (r.left - r.right, r.top - r.bottom);
    if width < height * aspect {
        let pad = (height * aspect - width) / 2.0;
        r.left += pad;
        r.right -= pad;
    } else {
        let pad = (width / aspect - height) / 2.0;
        r.top += pad;
        r.bottom -= pad;
    }
    Some(r)
}

/// `worldmap.rs`'s `rects_overlap`: whether two loc rects share area.
fn overlaps(a: Rect, b: Rect) -> bool {
    let span = |p: f32, q: f32| (p.min(q), p.max(q));
    let meet = |(alo, ahi): (f32, f32), (blo, bhi): (f32, f32)| alo < bhi && blo < ahi;
    meet(span(a.left, a.right), span(b.left, b.right))
        && meet(span(a.top, a.bottom), span(b.top, b.bottom))
}

/// The canvas image's pixel size: `canvas` at `scale`'s stock size, its art width across its rect.
fn canvas_size(canvas: Rect, scale: Rect) -> (u32, u32) {
    let per_yard = f64::from(ART_W) / f64::from(scale.left - scale.right);
    (
        (f64::from(canvas.left - canvas.right) * per_yard).round() as u32,
        (f64::from(canvas.top - canvas.bottom) * per_yard).round() as u32,
    )
}

/// Where `zone` lands on the `size` canvas image that shows `canvas`, as pixel edges
/// `[x0, y0, x1, y1]`, unclipped: a rect past the canvas edge keeps its full size.
fn place(canvas: Rect, size: (u32, u32), zone: Rect) -> [f64; 4] {
    let (cl, ct) = (f64::from(canvas.left), f64::from(canvas.top));
    let sx = f64::from(size.0) / (cl - f64::from(canvas.right));
    let sy = f64::from(size.1) / (ct - f64::from(canvas.bottom));
    [
        (cl - f64::from(zone.left)) * sx,
        (ct - f64::from(zone.top)) * sy,
        (cl - f64::from(zone.right)) * sx,
        (ct - f64::from(zone.bottom)) * sy,
    ]
}

/// `[x0, y0, x1, y1]` in pixels of the `size` canvas as canvas UV.
fn uv(px: [f64; 4], size: (u32, u32)) -> [f64; 4] {
    let (w, h) = (f64::from(size.0), f64::from(size.1));
    [px[0] / w, px[1] / h, px[2] / w, px[3] / h]
}

/// The client's power-of-two round-up (`0x4a833c`).
fn next_pow2(n: i64) -> i64 {
    let mut p = 1i64;
    while p < n {
        p <<= 1;
    }
    p
}

/// `texPctY` of `worldmap.rs`'s `seat`: the share of the Highlight texture's height the zone's
/// silhouette fills; `texPctX` is 1.
fn tex_pct_y(zone: Rect) -> f32 {
    let (w, h) = (zone.left - zone.right, zone.top - zone.bottom);
    if w == 0.0 {
        return 0.0;
    }
    let dim = ((h * 128.0) / w) as i64;
    let potdim = next_pow2(dim);
    if potdim > 0 {
        dim as f32 / potdim as f32
    } else {
        0.0
    }
}

/// An RGBA8 picture with bilinear lookups in 0..1 floats.
struct Picture {
    w: u32,
    h: u32,
    rgba: Vec<u8>,
}

impl Picture {
    fn blank(w: u32, h: u32) -> Self {
        Picture {
            w,
            h,
            rgba: vec![0; (w * h * 4) as usize],
        }
    }

    fn read(chain: &mut Chain, path: &str) -> Result<Self> {
        let (w, h, rgba) = read_texture_rgba(chain, path)?;
        Ok(Picture { w, h, rgba })
    }

    fn texel(&self, x: u32, y: u32) -> [f32; 4] {
        let i = ((y.min(self.h - 1) * self.w + x.min(self.w - 1)) * 4) as usize;
        let p = &self.rgba[i..i + 4];
        [p[0], p[1], p[2], p[3]].map(|c| f32::from(c) / 255.0)
    }

    fn put(&mut self, x: u32, y: u32, c: [f32; 4]) {
        let i = ((y * self.w + x) * 4) as usize;
        for (d, s) in self.rgba[i..i + 4].iter_mut().zip(c) {
            *d = (s.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }

    /// Bilinear at pixel coordinates `(x, y)` (texel centres at `n + 0.5`), clamped to the
    /// `lim_w` by `lim_h` corner.
    fn bilinear(&self, x: f32, y: f32, lim_w: u32, lim_h: u32) -> [f32; 4] {
        let fx = (x - 0.5).clamp(0.0, (lim_w - 1) as f32);
        let fy = (y - 0.5).clamp(0.0, (lim_h - 1) as f32);
        let (x0, y0) = (fx.floor() as u32, fy.floor() as u32);
        let (x1, y1) = ((x0 + 1).min(lim_w - 1), (y0 + 1).min(lim_h - 1));
        let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
        let lerp =
            |a: [f32; 4], b: [f32; 4], t: f32| [0, 1, 2, 3].map(|i| a[i] + (b[i] - a[i]) * t);
        let top = lerp(self.texel(x0, y0), self.texel(x1, y0), tx);
        let bottom = lerp(self.texel(x0, y1), self.texel(x1, y1), tx);
        lerp(top, bottom, ty)
    }

    /// Bilinear at UV `(u, v)` over the whole picture.
    fn sample(&self, u: f32, v: f32) -> [f32; 4] {
        self.bilinear(u * self.w as f32, v * self.h as f32, self.w, self.h)
    }

    fn save(&self, path: &Path) -> Result<()> {
        RgbaImage::from_raw(self.w, self.h, self.rgba.clone())
            .context("picture buffer size")?
            .save(path)
            .with_context(|| format!("writing {}", path.display()))
    }
}

/// One stock overlay piece: its 1-based number, its seat in the zone's art pixels, its used size
/// and its file size, as `BenillaCanvas_Pieces` (`WorldMapCanvas.xml`) cuts it.
struct Piece {
    n: u32,
    x: u32,
    y: u32,
    used: (u32, u32),
    file: (u32, u32),
}

/// The 256 px pieces of an overlay, numbered row by row from 1.
fn pieces(o: &WorldMapOverlay) -> Vec<Piece> {
    let pow2 = |used: u32| {
        let mut f = 16;
        while f < used {
            f *= 2;
        }
        f
    };
    let last = |size: u32| match size % TILE {
        0 => TILE,
        m => m,
    };
    let wide = o.texture_width.div_ceil(TILE);
    let tall = o.texture_height.div_ceil(TILE);
    let mut out = Vec::new();
    for j in 0..tall {
        let (ph, fh) = if j + 1 == tall {
            let ph = last(o.texture_height);
            (ph, pow2(ph))
        } else {
            (TILE, TILE)
        };
        for k in 0..wide {
            let (pw, fw) = if k + 1 == wide {
                let pw = last(o.texture_width);
                (pw, pow2(pw))
            } else {
                (TILE, TILE)
            };
            out.push(Piece {
                n: j * wide + k + 1,
                x: o.offset_x + TILE * k,
                y: o.offset_y + TILE * j,
                used: (pw, ph),
                file: (fw, fh),
            });
        }
    }
    out
}

/// A zone's stock art: the base tiles, the same with every overlay drawn (fully explored), and
/// its Highlight silhouette.
struct Zone {
    file: String,
    rect: Rect,
    base: Picture,
    explored: Picture,
    highlight: Picture,
    /// The Highlight's brightest channel, 0 for a black one.
    peak: f32,
    tex_pct_y: f32,
    overlays: Vec<WorldMapOverlay>,
}

impl Zone {
    fn load(chain: &mut Chain, area: &WorldMapArea, overlays: &[WorldMapOverlay]) -> Result<Self> {
        let file = area.name.clone();
        let dir = format!("Interface\\WorldMap\\{file}\\");
        let mut base = Picture::blank(TILE * TILES_WIDE, TILE * TILES_TALL);
        for n in 0..TILES_WIDE * TILES_TALL {
            let tile = Picture::read(chain, &format!("{dir}{file}{}.blp", n + 1))?;
            let (ox, oy) = (TILE * (n % TILES_WIDE), TILE * (n / TILES_WIDE));
            for y in 0..TILE.min(tile.h) {
                for x in 0..TILE.min(tile.w) {
                    base.put(ox + x, oy + y, tile.texel(x, y));
                }
            }
        }
        let mut explored = Picture {
            w: base.w,
            h: base.h,
            rgba: base.rgba.clone(),
        };
        for o in overlays {
            for p in pieces(o) {
                let path = format!("{dir}{}{}.blp", o.texture_name, p.n);
                let piece = Picture::read(chain, &path)?;
                for y in 0..p.used.1 {
                    for x in 0..p.used.0 {
                        let (zx, zy) = (p.x + x, p.y + y);
                        if zx >= explored.w || zy >= explored.h {
                            continue;
                        }
                        let s = piece.texel(x * piece.w / p.file.0, y * piece.h / p.file.1);
                        let d = explored.texel(zx, zy);
                        let a = s[3];
                        let c = [0, 1, 2].map(|i| s[i] * a + d[i] * (1.0 - a));
                        explored.put(zx, zy, [c[0], c[1], c[2], 1.0]);
                    }
                }
            }
        }
        let highlight = Picture::read(chain, &format!("{dir}{file}Highlight.blp"))?;
        let peak = highlight
            .rgba
            .chunks(4)
            .map(|p| p[0].max(p[1]).max(p[2]))
            .max()
            .map_or(0.0, |b| f32::from(b) / 255.0);
        let rect = Rect::of(area);
        Ok(Zone {
            file,
            rect,
            base,
            explored,
            highlight,
            peak,
            tex_pct_y: tex_pct_y(rect),
            overlays: overlays.to_vec(),
        })
    }

    /// The silhouette's alpha at zone UV `(u, v)`: the Highlight's brightness over its peak, its
    /// texture coords `(0, 1, 0, texPctY)` stretched over the zone's rect. A Highlight is a faint
    /// tinted glow drawn in ADD (Elwynn's peaks at 58 of 255), so the peak reads as opaque.
    /// `StormwindHighlight` is black, as the stock client draws no highlight for the city, so a
    /// black Highlight falls back to the art inset by its frame with a soft edge.
    fn silhouette(&self, u: f32, v: f32) -> f32 {
        if self.peak == 0.0 {
            let (x, y) = (u * ART_W as f32, v * ART_H as f32);
            let edge = x.min(ART_W as f32 - x).min(y).min(ART_H as f32 - y);
            return ((edge - FRAME_INSET) / FRAME_FADE).clamp(0.0, 1.0);
        }
        let c = self.highlight.sample(u, v * self.tex_pct_y);
        (c[0].max(c[1]).max(c[2]) * c[3] / self.peak).min(1.0)
    }

    /// The explored art at zone UV `(u, v)`, bilinear inside the 1002 by 668 the map shows.
    fn art(&self, u: f32, v: f32) -> [f32; 4] {
        self.explored
            .bilinear(u * ART_W as f32, v * ART_H as f32, ART_W, ART_H)
    }

    /// The mean base colour where the silhouette is zero: the map's own parchment.
    fn parchment(&self) -> [f32; 3] {
        let (mut sum, mut count) = ([0.0f64; 3], 0u64);
        for y in 0..ART_H {
            for x in 0..ART_W {
                let (u, v) = (
                    (x as f32 + 0.5) / ART_W as f32,
                    (y as f32 + 0.5) / ART_H as f32,
                );
                if self.silhouette(u, v) < 0.5 / 255.0 {
                    let c = self.base.texel(x, y);
                    for i in 0..3 {
                        sum[i] += f64::from(c[i]);
                    }
                    count += 1;
                }
            }
        }
        let n = count.max(1) as f64;
        sum.map(|s| (s / n) as f32)
    }
}

/// Draw `zone`'s explored art onto `canvas` over its placed rect, its silhouette as the alpha.
fn draw(canvas: &mut Picture, at: [f64; 4], zone: &Zone) {
    let x_lo = at[0].floor().max(0.0) as u32;
    let y_lo = at[1].floor().max(0.0) as u32;
    let x_hi = (at[2].ceil().max(0.0) as u32).min(canvas.w);
    let y_hi = (at[3].ceil().max(0.0) as u32).min(canvas.h);
    for py in y_lo..y_hi {
        let v = ((f64::from(py) + 0.5 - at[1]) / (at[3] - at[1])) as f32;
        if !(0.0..1.0).contains(&v) {
            continue;
        }
        for px in x_lo..x_hi {
            let u = ((f64::from(px) + 0.5 - at[0]) / (at[2] - at[0])) as f32;
            if !(0.0..1.0).contains(&u) {
                continue;
            }
            let a = zone.silhouette(u, v);
            if a <= 0.0 {
                continue;
            }
            let s = zone.art(u, v);
            let d = canvas.texel(px, py);
            let c = [0, 1, 2].map(|i| s[i] * a + d[i] * (1.0 - a));
            canvas.put(px, py, [c[0], c[1], c[2], 1.0]);
        }
    }
}

/// The fog's alpha over a silhouette value: 0 outside the zone, `FOG_ALPHA` inside, a smoothstep
/// across `FOG_EDGE` between.
fn fog_alpha(silhouette: f32) -> f32 {
    let t = ((silhouette - FOG_EDGE.0) / (FOG_EDGE.1 - FOG_EDGE.0)).clamp(0.0, 1.0);
    FOG_ALPHA * t * t * (3.0 - 2.0 * t)
}

/// The zone's fog over its whole placed rect: the mist tint, flat inside the zone's silhouette.
fn fog(at: [f64; 4], zone: &Zone) -> Picture {
    let w = (at[2] - at[0]).round().max(1.0) as u32;
    let h = (at[3] - at[1]).round().max(1.0) as u32;
    let mut out = Picture::blank(w, h);
    for y in 0..h {
        for x in 0..w {
            let (u, v) = ((x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32);
            let a = fog_alpha(zone.silhouette(u, v));
            out.put(x, y, [FOG_TINT[0], FOG_TINT[1], FOG_TINT[2], a]);
        }
    }
    // The Highlight is coarse next to the canvas, so its texels show as steps along the edge;
    // a blur about one texel wide rounds them off.
    let texel = (w as f32 / zone.highlight.w as f32).round().max(1.0) as usize;
    soften_alpha(&mut out, texel);
    out
}

/// Three box-blur passes of `radius` over `pic`'s alpha, each along rows then columns.
fn soften_alpha(pic: &mut Picture, radius: usize) {
    let (w, h) = (pic.w as usize, pic.h as usize);
    let mut alpha: Vec<f32> = pic.rgba.chunks(4).map(|p| f32::from(p[3])).collect();
    let mut line = Vec::new();
    let mut blur =
        |alpha: &mut [f32], len: usize, count: usize, at: &dyn Fn(usize, usize) -> usize| {
            for k in 0..count {
                line.clear();
                line.extend((0..len).map(|i| alpha[at(k, i)]));
                let mut sum: f32 = line[..radius.min(len)].iter().sum();
                for i in 0..len {
                    if i + radius < len {
                        sum += line[i + radius];
                    }
                    if i > radius {
                        sum -= line[i - radius - 1];
                    }
                    let n = (i + radius).min(len - 1) + 1 - i.saturating_sub(radius);
                    alpha[at(k, i)] = sum / n as f32;
                }
            }
        };
    for _ in 0..3 {
        blur(&mut alpha, w, h, &|y, x| y * w + x);
        blur(&mut alpha, h, w, &|x, y| y * w + x);
    }
    for (p, a) in pic.rgba.chunks_mut(4).zip(alpha) {
        p[3] = a.round().clamp(0.0, 255.0) as u8;
    }
}

/// Blend `fog`, placed at `at`, over `canvas`: the preview of the map with every misted zone shut.
fn blend(canvas: &mut Picture, at: [f64; 4], fog: &Picture) {
    let (x0, y0) = (at[0].round() as i64, at[1].round() as i64);
    for fy in 0..fog.h {
        for fx in 0..fog.w {
            let (px, py) = (x0 + i64::from(fx), y0 + i64::from(fy));
            if px < 0 || py < 0 || px >= i64::from(canvas.w) || py >= i64::from(canvas.h) {
                continue;
            }
            let s = fog.texel(fx, fy);
            let d = canvas.texel(px as u32, py as u32);
            let c = [0, 1, 2].map(|i| s[i] * s[3] + d[i] * (1.0 - s[3]));
            canvas.put(px as u32, py as u32, [c[0], c[1], c[2], 1.0]);
        }
    }
}

/// One overlay piece's cover: the base pixels under the piece, the piece's alpha, at the piece's
/// file size; texels past its used size are clear.
fn cover(chain: &mut Chain, zone: &Zone, o: &WorldMapOverlay, p: &Piece) -> Result<Picture> {
    let path = format!(
        "Interface\\WorldMap\\{0}\\{1}{2}.blp",
        zone.file, o.texture_name, p.n
    );
    let piece = Picture::read(chain, &path)?;
    let mut out = Picture::blank(p.file.0, p.file.1);
    for y in 0..p.used.1 {
        for x in 0..p.used.0 {
            let a = piece.texel(x * piece.w / p.file.0, y * piece.h / p.file.1)[3];
            let c = zone.base.texel(p.x + x, p.y + y);
            out.put(x, y, [c[0], c[1], c[2], a]);
        }
    }
    Ok(out)
}

fn args() -> Result<(PathBuf, PathBuf)> {
    let mut data = None;
    let mut out = None;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--data" => data = it.next().map(PathBuf::from),
            "--out" => out = it.next().map(PathBuf::from),
            _ => bail!("usage: map_draft --data <WoW Data> --out <dir>"),
        }
    }
    let data = match data {
        Some(d) => d,
        None => benilla_formats::wow_data().context("no --data and no WoW install found")?,
    };
    Ok((
        data,
        out.unwrap_or_else(|| PathBuf::from("target/map_draft")),
    ))
}

fn main() -> Result<()> {
    let (data, out) = args()?;
    let mut chain = benilla_formats::open_chain(&data)?;
    let wma = load_world_map_area_catalog(&mut chain)?;
    let wmo = load_world_map_overlay_catalog(&mut chain)?;
    let (map, listed) = benilla_formats::map_canvas::canvas();
    let held = benilla_formats::map_canvas::held();

    let on_map: Vec<(u32, &WorldMapArea)> = wma
        .iter()
        .filter(|(_, a)| a.map_id == map && a.area_id != 0)
        .collect();
    let find = |area_id: u32| {
        on_map
            .iter()
            .find(|(_, a)| a.area_id == area_id)
            .copied()
            .with_context(|| format!("canvas area {area_id} has no WorldMapArea row"))
    };
    let listed_rows: Vec<(u32, &WorldMapArea)> =
        listed.iter().map(|&id| find(id)).collect::<Result<_>>()?;
    let mut held_rects = Vec::new();
    for &id in held {
        held_rects.push(Rect::of(find(id)?.1));
    }
    let canvas = canvas_rect(
        &listed_rows
            .iter()
            .map(|(_, a)| Rect::of(a))
            .chain(held_rects)
            .collect::<Vec<_>>(),
    )
    .context("map_canvas.json lists no zone")?;
    let size = canvas_size(canvas, Rect::of(find(SCALE_ZONE)?.1));
    let mut misted: Vec<(u32, &WorldMapArea)> = on_map
        .iter()
        .filter(|(_, a)| !listed.contains(&a.area_id) && overlaps(canvas, Rect::of(a)))
        .copied()
        .collect();
    misted.sort_by_key(|(_, a)| a.area_id);
    println!(
        "canvas rect: left {:.2} right {:.2} top {:.2} bottom {:.2}, {} by {} px",
        canvas.left, canvas.right, canvas.top, canvas.bottom, size.0, size.1
    );

    std::fs::create_dir_all(out.join("covers"))?;
    let elwynn = on_map
        .iter()
        .find(|(_, a)| a.name == "Elwynn")
        .context("no Elwynn row")?;
    let parchment = Zone::load(&mut chain, elwynn.1, &[])?.parchment();
    let mut img = Picture::blank(size.0, size.1);
    for y in 0..size.1 {
        for x in 0..size.0 {
            img.put(x, y, [parchment[0], parchment[1], parchment[2], 1.0]);
        }
    }

    let mut fogs = Vec::new();
    let mut placed_fogs = Vec::new();
    let mut covers = Vec::new();
    let mut written = Vec::new();
    let groups = [(&misted, true), (&listed_rows, false)];
    for (rows, is_misted) in groups {
        for &(id, area) in rows.iter() {
            let zone = Zone::load(&mut chain, area, wmo.for_area(id))?;
            let at = place(canvas, size, zone.rect);
            println!(
                "{:>4} {:<18} at {:8.1} {:8.1} {:8.1} {:8.1}{}",
                area.area_id,
                zone.file,
                at[0],
                at[1],
                at[2],
                at[3],
                if is_misted { "  misted" } else { "" }
            );
            draw(&mut img, at, &zone);
            if is_misted {
                let name = format!("fog_{}.png", zone.file.to_lowercase());
                let f = fog(at, &zone);
                f.save(&out.join(&name))?;
                fogs.push(json!({ "file": name, "size": [f.w, f.h], "rect": uv(at, size) }));
                placed_fogs.push((at, f));
                written.push(name);
                continue;
            }
            let (zw, zh) = (at[2] - at[0], at[3] - at[1]);
            for o in &zone.overlays {
                for p in pieces(o) {
                    let name = format!("covers/{}_{}.png", o.texture_name.to_lowercase(), p.n);
                    let c = cover(&mut chain, &zone, o, &p)?;
                    c.save(&out.join(&name))?;
                    let edge = |zx: u32, zy: u32| {
                        (
                            at[0] + f64::from(zx) / f64::from(ART_W) * zw,
                            at[1] + f64::from(zy) / f64::from(ART_H) * zh,
                        )
                    };
                    let (x0, y0) = edge(p.x, p.y);
                    let (x1, y1) = edge(p.x + p.used.0, p.y + p.used.1);
                    covers.push(json!({
                        "file": name,
                        "zone": zone.file,
                        "size": [c.w, c.h],
                        "used": [p.used.0, p.used.1],
                        "rect": uv([x0, y0, x1, y1], size),
                    }));
                    written.push(name);
                }
            }
        }
    }
    img.save(&out.join("canvas.png"))?;
    written.insert(0, "canvas.png".to_string());
    for (at, f) in &placed_fogs {
        blend(&mut img, *at, f);
    }
    img.save(&out.join("preview.png"))?;
    written.insert(1, "preview.png".to_string());

    let manifest: Value = json!({
        "canvas": {
            "file": "canvas.png",
            "size": [size.0, size.1],
            "rect": [0.0, 0.0, 1.0, 1.0],
            "loc_rect": [canvas.left, canvas.right, canvas.top, canvas.bottom],
        },
        "fog": fogs,
        "covers": covers,
    });
    std::fs::write(
        out.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    written.push("manifest.json".to_string());
    println!("wrote {} files under {}", written.len(), out.display());
    for name in &written {
        println!("  {name}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The canvas rect `ui_world_map.rs` computes today.
    const CANVAS: Rect = Rect {
        left: 1579.69,
        right: -3785.94,
        top: -7939.58,
        bottom: -11516.67,
    };

    /// That rect's image before Burning Steppes was held (#422).
    const SIZE: (u32, u32) = (4000, 2667);

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.05
    }

    #[test]
    fn a_zone_equal_to_the_canvas_fills_the_image() {
        let at = place(CANVAS, SIZE, CANVAS);
        assert!(near(at[0], 0.0) && near(at[1], 0.0), "{at:?}");
        assert!(near(at[2], 4000.0) && near(at[3], 2667.0), "{at:?}");
    }

    #[test]
    fn elwynn_lands_33_px_in_at_the_top_edge() {
        let elwynn = Rect {
            left: 1535.42,
            right: -1935.42,
            top: -7939.58,
            bottom: -10254.17,
        };
        let at = place(CANVAS, SIZE, elwynn);
        assert!(near(at[0], 33.0), "left edge {}", at[0]);
        assert!(near(at[1], 0.0), "top edge {}", at[1]);
    }

    #[test]
    fn a_fog_rect_past_the_canvas_edge_keeps_its_full_size() {
        let zone = Rect {
            left: CANVAS.right + 500.0,
            right: CANVAS.right - 500.0,
            top: CANVAS.bottom + 300.0,
            bottom: CANVAS.bottom - 300.0,
        };
        let at = place(CANVAS, SIZE, zone);
        let px_per_yard = 4000.0 / f64::from(CANVAS.left - CANVAS.right);
        assert!(near(at[2] - at[0], 1000.0 * px_per_yard), "{at:?}");
        assert!(
            at[2] > 4000.0 && at[3] > 2667.0,
            "runs past the edge: {at:?}"
        );
    }

    #[test]
    fn the_canvas_size_puts_the_scale_zone_at_its_art_width() {
        let scale = Rect {
            left: 0.0,
            right: -1002.0,
            top: 0.0,
            bottom: -668.0,
        };
        let canvas = Rect {
            left: 1000.0,
            right: -3008.0,
            top: 0.0,
            bottom: -2672.0,
        };
        assert_eq!(canvas_size(canvas, scale), (4008, 2672));
        let at = place(canvas, canvas_size(canvas, scale), scale);
        assert!(near(at[2] - at[0], 1002.0), "{at:?}");
    }

    #[test]
    fn the_fog_is_flat_inside_the_zone_and_clear_outside() {
        assert_eq!(fog_alpha(0.0), 0.0);
        assert_eq!(fog_alpha(FOG_EDGE.0), 0.0);
        assert_eq!(fog_alpha(FOG_EDGE.1), FOG_ALPHA);
        assert_eq!(fog_alpha(1.0), FOG_ALPHA);
        let mid = fog_alpha((FOG_EDGE.0 + FOG_EDGE.1) / 2.0);
        assert!((mid - FOG_ALPHA / 2.0).abs() < 1e-6, "{mid}");
    }

    #[test]
    fn softening_keeps_a_flat_fog_flat_and_rounds_a_step() {
        let mut pic = Picture::blank(40, 3);
        for y in 0..3 {
            for x in 0..40 {
                pic.put(x, y, [0.0, 0.0, 0.0, if x < 20 { 1.0 } else { 0.0 }]);
            }
        }
        soften_alpha(&mut pic, 4);
        let a = |x| pic.texel(x, 1)[3];
        assert_eq!(a(0), 1.0);
        assert_eq!(a(39), 0.0);
        assert!(
            a(18) < 1.0 && a(18) > a(21) && a(21) > 0.0,
            "{} {}",
            a(18),
            a(21)
        );
    }

    #[test]
    fn the_canvas_rect_pads_the_union_to_the_art_aspect() {
        let r = canvas_rect(&[CANVAS]).unwrap();
        let aspect = (r.left - r.right) / (r.top - r.bottom);
        assert!((aspect - 1002.0 / 668.0).abs() < 1e-3, "{aspect}");
    }
}
