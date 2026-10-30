//! The painted map's draft (#422, spec #421): the canvas image, one fog per misted zone and the
//! exploration covers, cut from the stock zone maps for Davis to edit.
//! `cargo run -q -p benilla-formats --example map_draft -- --data <Data> --out target/map_draft`
//!
//! Each zone's art is cut along its `<file>Highlight` silhouette, which drops the parchment frame.
//! The tool writes only under `--out`; the committed PNGs are the source of truth and it never
//! touches them. Remaking the canvas, the fill file and opening a zone: `docs/map_pipeline.md`.

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
/// A city map's backdrop flood: from the frame inward, a step to a neighbour takes it when their
/// colours differ by under `FLOOD_STEP` and it is within `FLOOD_REACH` of the backdrop's colour
/// (RGB distances, 0 to 1). The ink outlines round the walls, districts and crest stop it.
const FLOOD_STEP: f32 = 0.06;
const FLOOD_REACH: f32 = 0.16;
/// The cut is blurred by `CUTOUT_BLUR` art pixels and sharpened across `CUTOUT_EDGE`, so its rim
/// is soft but not hazy.
const CUTOUT_BLUR: usize = 3;
const CUTOUT_EDGE: (f32, f32) = (0.35, 0.65);
/// How far inside the frame a cut-out city's clearing of the zone under it reaches full, in art
/// pixels, so the clearing has no hard rim.
const CLEAR_FADE: f32 = 150.0;

/// A city map's cut-out: 0 on its backdrop, the ground flooded in from the frame, and 1 on all it
/// encloses, smoothed.
fn cutout(art: &Picture) -> Vec<f32> {
    let bg = commonest_colour(art);
    let (w, h) = (ART_W as usize, ART_H as usize);
    let colour = |i: usize| art.texel((i % w) as u32, (i / w) as u32);
    let dist =
        |a: [f32; 4], b: [f32; 4]| ((0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f32>()).sqrt();
    let bg4 = [bg[0], bg[1], bg[2], 1.0];
    let mut flooded = vec![false; w * h];
    let mut queue = std::collections::VecDeque::new();
    for i in 0..w * h {
        let (x, y) = (i % w, i / w);
        let edge = x.min(w - 1 - x).min(y).min(h - 1 - y) as f32;
        if edge < FRAME_INSET {
            flooded[i] = true;
            queue.push_back(i);
        }
    }
    while let Some(i) = queue.pop_front() {
        let (x, y) = (i % w, i / w);
        let c = colour(i);
        let around = [
            (x > 0).then(|| i - 1),
            (x + 1 < w).then(|| i + 1),
            (y > 0).then(|| i - w),
            (y + 1 < h).then(|| i + w),
        ];
        for j in around.into_iter().flatten() {
            if flooded[j] {
                continue;
            }
            let n = colour(j);
            if dist(n, c) < FLOOD_STEP && dist(n, bg4) < FLOOD_REACH {
                flooded[j] = true;
                queue.push_back(j);
            }
        }
    }
    let mut key = Picture::blank(ART_W, ART_H);
    for (i, f) in flooded.iter().enumerate() {
        let a = if *f { 0.0 } else { 1.0 };
        key.put((i % w) as u32, (i / w) as u32, [0.0, 0.0, 0.0, a]);
    }
    soften_alpha(&mut key, CUTOUT_BLUR);
    key.rgba
        .chunks(4)
        .map(|p| {
            let a = f32::from(p[3]) / 255.0;
            let t = ((a - CUTOUT_EDGE.0) / (CUTOUT_EDGE.1 - CUTOUT_EDGE.0)).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        })
        .collect()
}

/// The commonest colour of the art's shown 1002 by 668, by 16-step bins, as the bin's mean.
fn commonest_colour(art: &Picture) -> [f32; 3] {
    let mut bins = std::collections::HashMap::<[u8; 3], ([f64; 3], u32)>::new();
    for y in 0..ART_H {
        for x in 0..ART_W {
            let c = art.texel(x, y);
            let key = [0, 1, 2].map(|i| (c[i] * 15.99) as u8);
            let e = bins.entry(key).or_insert(([0.0; 3], 0));
            for i in 0..3 {
                e.0[i] += f64::from(c[i]);
            }
            e.1 += 1;
        }
    }
    let (sum, n) = bins
        .into_values()
        .max_by_key(|(_, n)| *n)
        .unwrap_or(([0.0; 3], 1));
    sum.map(|s| (s / f64::from(n)) as f32)
}

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

    fn open(path: &Path) -> Result<Self> {
        let img = image::open(path)
            .with_context(|| format!("reading {}", path.display()))?
            .to_rgba8();
        let (w, h) = img.dimensions();
        Ok(Picture {
            w,
            h,
            rgba: img.into_raw(),
        })
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
    /// For a black Highlight: the art's cut-out, 1 on the city's walls, districts, labels and
    /// crest and 0 on its grey-purple backdrop, row by row over the 1002 by 668 it shows.
    cutout: Option<Vec<f32>>,
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
        let cutout = (peak == 0.0).then(|| cutout(&explored));
        Ok(Zone {
            file,
            rect,
            base,
            explored,
            highlight,
            peak,
            cutout,
            tex_pct_y: tex_pct_y(rect),
            overlays: overlays.to_vec(),
        })
    }

    /// The silhouette's alpha at zone UV `(u, v)`: the Highlight's brightness over its peak, its
    /// texture coords `(0, 1, 0, texPctY)` stretched over the zone's rect. A Highlight is a faint
    /// tinted glow drawn in ADD (Elwynn's peaks at 58 of 255), so the peak reads as opaque.
    /// `StormwindHighlight` is black, as the stock client draws no highlight for the city, so a
    /// black Highlight falls back to the art inset by its frame with a soft edge, times its
    /// cut-out, so the city lies on the land around it and not on its card.
    fn silhouette(&self, u: f32, v: f32) -> f32 {
        if self.peak == 0.0 {
            let frame = Self::frame(u, v);
            let Some(cut) = &self.cutout else {
                return frame;
            };
            let (cx, cy) = (
                ((u * ART_W as f32) as u32).min(ART_W - 1) as usize,
                ((v * ART_H as f32) as u32).min(ART_H - 1) as usize,
            );
            return frame * cut[cy * ART_W as usize + cx];
        }
        let c = self.highlight.sample(u, v * self.tex_pct_y);
        (c[0].max(c[1]).max(c[2]) * c[3] / self.peak).min(1.0)
    }

    /// The art inset by its frame with a soft edge, at zone UV `(u, v)`.
    fn frame(u: f32, v: f32) -> f32 {
        let (x, y) = (u * ART_W as f32, v * ART_H as f32);
        let edge = x.min(ART_W as f32 - x).min(y).min(ART_H as f32 - y);
        ((edge - FRAME_INSET) / FRAME_FADE).clamp(0.0, 1.0)
    }

    /// How much of the zone art already laid under this zone it clears from the fill's layer: a
    /// cut-out city clears its whole frame, so the zone it sits in (Elwynn draws its own
    /// Stormwind, label and all) does not show between its districts; the painting does.
    fn clears(&self, u: f32, v: f32) -> f32 {
        if self.cutout.is_some() {
            let (x, y) = (u * ART_W as f32, v * ART_H as f32);
            let edge = x.min(ART_W as f32 - x).min(y).min(ART_H as f32 - y);
            ((edge - FRAME_INSET) / CLEAR_FADE).clamp(0.0, 1.0)
        } else {
            0.0
        }
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

/// Draw `zone`'s explored art onto `canvas` over its placed rect, its silhouette as the alpha, and
/// onto `layer` too: the zone art alone, premultiplied RGBA, with no parchment under it.
fn draw(canvas: &mut Picture, layer: &mut [[f32; 4]], at: [f64; 4], zone: &Zone) {
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
            let k = (py * canvas.w + px) as usize;
            let clear = zone.clears(u, v);
            if clear > 0.0 {
                layer[k] = layer[k].map(|o| o * (1.0 - clear));
            }
            let a = zone.silhouette(u, v);
            if a <= 0.0 {
                continue;
            }
            let s = zone.art(u, v);
            let d = canvas.texel(px, py);
            let c = [0, 1, 2].map(|i| s[i] * a + d[i] * (1.0 - a));
            canvas.put(px, py, [c[0], c[1], c[2], 1.0]);
            let src = [s[0] * a, s[1] * a, s[2] * a, a];
            for (o, n) in layer[k].iter_mut().zip(src) {
                *o = n + *o * (1.0 - a);
            }
        }
    }
}

/// A painting to fill the canvas's bare parchment with, and the landmarks that pin it: each pair
/// is one place's canvas UV and its UV on the painting. Each erase patches a box of the painting
/// with the land at an offset from it, for a painted label the stock art does not fully cover.
struct FillSpec {
    painting: PathBuf,
    pairs: Vec<FillPair>,
    /// Each pair's name, in the same order.
    names: Vec<String>,
    erase: Vec<Erase>,
    /// Boxes `[u0, v0, u1, v1]` of the canvas where the zone art stays solid however faint its
    /// cover, for a stock label its own zone's silhouette clips.
    keep: Vec<[f64; 4]>,
}

/// The width in canvas pixels over which a keep box fades into the fill around it.
const KEEP_FEATHER: f64 = 4.0;

struct FillPair {
    canvas: [f64; 2],
    painting: [f64; 2],
}

/// A box `[u0, v0, u1, v1]` of the painting, refilled from the box moved by `from`.
struct Erase {
    rect: [f64; 4],
    from: [f64; 2],
}

/// The width in painting pixels over which an erase's patch fades into the painting around it.
const ERASE_FEATHER: f64 = 3.0;

impl Erase {
    fn apply(&self, painting: &mut Picture) {
        let (w, h) = (f64::from(painting.w), f64::from(painting.h));
        let [x0, y0, x1, y1] = [
            self.rect[0] * w,
            self.rect[1] * h,
            self.rect[2] * w,
            self.rect[3] * h,
        ];
        let (dx, dy) = (self.from[0] * w, self.from[1] * h);
        let source = Picture {
            w: painting.w,
            h: painting.h,
            rgba: painting.rgba.clone(),
        };
        let lo = |t: f64| (t - ERASE_FEATHER).floor().max(0.0) as u32;
        for py in lo(y0)..((y1 + ERASE_FEATHER).ceil() as u32).min(painting.h) {
            for px in lo(x0)..((x1 + ERASE_FEATHER).ceil() as u32).min(painting.w) {
                let (x, y) = (f64::from(px) + 0.5, f64::from(py) + 0.5);
                let inside = (x - x0).min(x1 - x).min(y - y0).min(y1 - y);
                let a = ((inside + ERASE_FEATHER) / (2.0 * ERASE_FEATHER)).clamp(0.0, 1.0) as f32;
                if a <= 0.0 {
                    continue;
                }
                let s = source.bilinear((x + dx) as f32, (y + dy) as f32, source.w, source.h);
                let d = source.texel(px, py);
                let c = [0, 1, 2].map(|i| s[i] * a + d[i] * (1.0 - a));
                painting.put(px, py, [c[0], c[1], c[2], 1.0]);
            }
        }
    }
}

impl FillSpec {
    /// `{"painting": "<png beside this file>", "pairs": [{"name", "canvas": [u, v],
    /// "painting": [u, v]}, ..], "erase": [{"name", "rect": [u0, v0, u1, v1],
    /// "from": [du, dv]}, ..], "keep": [{"name", "rect": [u0, v0, u1, v1]}, ..]}`; `erase` and
    /// `keep` may be absent.
    fn read(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let v: Value = serde_json::from_str(&text)?;
        let uv = |p: &Value, key: &str| -> Result<[f64; 2]> {
            let a = p[key]
                .as_array()
                .context("a pair needs canvas and painting")?;
            match (
                a.first().and_then(Value::as_f64),
                a.get(1).and_then(Value::as_f64),
            ) {
                (Some(u), Some(v)) => Ok([u, v]),
                _ => bail!("{key} of {} is not [u, v]", p["name"]),
            }
        };
        let rows = v["pairs"].as_array().context("the fill file has pairs")?;
        let names = rows
            .iter()
            .map(|p| p["name"].as_str().unwrap_or("?").to_string())
            .collect();
        let pairs = rows
            .iter()
            .map(|p| {
                Ok(FillPair {
                    canvas: uv(p, "canvas")?,
                    painting: uv(p, "painting")?,
                })
            })
            .collect::<Result<_>>()?;
        let nums = |p: &Value, key: &str, n: usize| -> Result<Vec<f64>> {
            let a: Vec<f64> = p[key]
                .as_array()
                .map(|a| a.iter().filter_map(Value::as_f64).collect())
                .unwrap_or_default();
            if a.len() != n {
                bail!("{key} of erase {} needs {n} numbers", p["name"]);
            }
            Ok(a)
        };
        let erase = v["erase"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .map(|e| {
                let (r, f) = (nums(e, "rect", 4)?, nums(e, "from", 2)?);
                Ok(Erase {
                    rect: [r[0], r[1], r[2], r[3]],
                    from: [f[0], f[1]],
                })
            })
            .collect::<Result<_>>()?;
        let keep = v["keep"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .map(|k| {
                let r = nums(k, "rect", 4)?;
                Ok([r[0], r[1], r[2], r[3]])
            })
            .collect::<Result<_>>()?;
        let painting = v["painting"]
            .as_str()
            .context("the fill file names a painting")?;
        Ok(FillSpec {
            painting: path.parent().unwrap_or(Path::new(".")).join(painting),
            pairs,
            names,
            erase,
            keep,
        })
    }
}

/// How far the thin-plate warp may bend away from an exact fit through every landmark, since
/// landmarks read off a repainted map disagree by a few pixels.
const WARP_SMOOTHING: f64 = 1e-4;
/// The cover values the fill's blend runs between: painting below the first, zone art above the
/// second.
/// The fog's edge: a Highlight's glow is not flat inside its zone, so a higher edge lets the
/// painting's own labels and icons show through the stock art.
const FILL_EDGE: (f32, f32) = FOG_EDGE;
/// Pins along each canvas edge for `Warp::fit_inside`, less one.
const BORDER_PINS: u32 = 12;

/// A thin-plate spline from canvas points to painting points, both in UV; canvas V is scaled by
/// the canvas's height over its width so distances are square.
struct Warp {
    points: Vec<[f64; 2]>,
    /// Per output axis: one weight per point, then the affine part `a0 + a1 x + a2 y`.
    coef: [Vec<f64>; 2],
    aspect: f64,
}

fn tps_kernel(r2: f64) -> f64 {
    if r2 <= 0.0 {
        0.0
    } else {
        0.5 * r2 * r2.ln()
    }
}

impl Warp {
    fn fit(pairs: &[FillPair], aspect: f64) -> Result<Self> {
        let n = pairs.len();
        if n < 3 {
            bail!("a fill needs at least 3 landmarks, got {n}");
        }
        let points: Vec<[f64; 2]> = pairs
            .iter()
            .map(|p| [p.canvas[0], p.canvas[1] * aspect])
            .collect();
        let m = n + 3;
        let mut a = vec![vec![0.0; m]; m];
        for i in 0..n {
            for j in 0..n {
                let (dx, dy) = (points[i][0] - points[j][0], points[i][1] - points[j][1]);
                a[i][j] = tps_kernel(dx * dx + dy * dy);
            }
            a[i][i] += WARP_SMOOTHING;
            let row = [1.0, points[i][0], points[i][1]];
            for (k, v) in row.into_iter().enumerate() {
                a[i][n + k] = v;
                a[n + k][i] = v;
            }
        }
        let solve_axis = |axis: usize| {
            let mut b = vec![0.0; m];
            for (i, p) in pairs.iter().enumerate() {
                b[i] = p.painting[axis];
            }
            solve(a.clone(), b)
        };
        Ok(Warp {
            coef: [solve_axis(0)?, solve_axis(1)?],
            points,
            aspect,
        })
    }

    /// `fit`, then refit with the canvas border pinned inside the painting: a repainted map often
    /// shows less land than the canvas, and a border point the warp sends past the painting's edge
    /// is pulled back onto it, so the margin stretches the painting's edge instead of running off.
    fn fit_inside(pairs: &[FillPair], aspect: f64) -> Result<Self> {
        let first = Warp::fit(pairs, aspect)?;
        let mut all: Vec<FillPair> = pairs
            .iter()
            .map(|p| FillPair {
                canvas: p.canvas,
                painting: p.painting,
            })
            .collect();
        for i in 0..=BORDER_PINS {
            let t = f64::from(i) / f64::from(BORDER_PINS);
            for (u, v) in [(t, 0.0), (t, 1.0), (0.0, t), (1.0, t)] {
                let (pu, pv) = first.at(u, v);
                all.push(FillPair {
                    canvas: [u, v],
                    painting: [pu.clamp(0.0, 1.0), pv.clamp(0.0, 1.0)],
                });
            }
        }
        Warp::fit(&all, aspect)
    }

    /// The painting UV under canvas UV `(u, v)`.
    fn at(&self, u: f64, v: f64) -> (f64, f64) {
        let (x, y) = (u, v * self.aspect);
        let n = self.points.len();
        let mut out = [0.0; 2];
        for (axis, c) in self.coef.iter().enumerate() {
            let mut s = c[n] + c[n + 1] * x + c[n + 2] * y;
            for (p, w) in self.points.iter().zip(c) {
                let (dx, dy) = (x - p[0], y - p[1]);
                s += w * tps_kernel(dx * dx + dy * dy);
            }
            out[axis] = s;
        }
        (out[0], out[1])
    }
}

/// A landmark whose leave-one-out error passes this many times the median is flagged: likely
/// misread. A repainted map drifts unevenly, so every landmark has some error; a misread one has
/// far more than its neighbours.
const LANDMARK_FLAG: f64 = 2.0;

/// The median of `xs`, 0 when empty.
fn median(xs: &[f64]) -> f64 {
    let mut s = xs.to_vec();
    s.sort_by(f64::total_cmp);
    s.get(s.len() / 2).copied().unwrap_or(0.0)
}

/// Each landmark's leave-one-out error in painting pixels: how far the warp fitted through every
/// other landmark lands from where the file says the place sits on the painting. A misread
/// landmark disagrees with its neighbours and stands out; a warp through all of them hides it.
fn landmark_errors(pairs: &[FillPair], aspect: f64, painting: &Picture) -> Vec<f64> {
    (0..pairs.len())
        .map(|i| {
            let rest: Vec<FillPair> = pairs
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, p)| FillPair {
                    canvas: p.canvas,
                    painting: p.painting,
                })
                .collect();
            let Ok(warp) = Warp::fit(&rest, aspect) else {
                return f64::INFINITY;
            };
            let (u, v) = warp.at(pairs[i].canvas[0], pairs[i].canvas[1]);
            let dx = (u - pairs[i].painting[0]) * f64::from(painting.w);
            let dy = (v - pairs[i].painting[1]) * f64::from(painting.h);
            (dx * dx + dy * dy).sqrt()
        })
        .collect()
}

/// Mark each landmark's canvas point on `pic` with a red cross: on the warped painting, the
/// painted place should sit on its cross.
fn mark_landmarks(pic: &mut Picture, pairs: &[FillPair]) {
    const ARM: i64 = 14;
    const HALF_WIDTH: i64 = 1;
    for p in pairs {
        let cx = (p.canvas[0] * f64::from(pic.w)) as i64;
        let cy = (p.canvas[1] * f64::from(pic.h)) as i64;
        for d in -ARM..=ARM {
            for t in -HALF_WIDTH..=HALF_WIDTH {
                for (x, y) in [(cx + d, cy + t), (cx + t, cy + d)] {
                    if (0..i64::from(pic.w)).contains(&x) && (0..i64::from(pic.h)).contains(&y) {
                        pic.put(x as u32, y as u32, [1.0, 0.0, 0.0, 1.0]);
                    }
                }
            }
        }
    }
}

/// Gaussian elimination with partial pivoting.
fn solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Result<Vec<f64>> {
    let m = b.len();
    for col in 0..m {
        let pivot = (col..m)
            .max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))
            .unwrap_or(col);
        if a[pivot][col].abs() < 1e-12 {
            bail!("the landmarks are degenerate (three in a line?)");
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        for row in col + 1..m {
            let f = a[row][col] / a[col][col];
            for k in col..m {
                a[row][k] -= f * a[col][k];
            }
            b[row] -= f * b[col];
        }
    }
    let mut x = vec![0.0; m];
    for row in (0..m).rev() {
        let s: f64 = (row + 1..m).map(|k| a[row][k] * x[k]).sum();
        x[row] = (b[row] - s) / a[row][row];
    }
    Ok(x)
}

/// The `w` by `h` canvas filled from `painting`, warped onto it by `warp`, under the zone art of
/// `layer`: the art where its cover is high, the painting where it is low, a smoothstep across
/// `FILL_EDGE` between. The art is unpremultiplied first, so a zone's soft rim fades into the
/// painting, not into parchment. Inside a `keep` box the art stays solid.
fn fill(
    w: u32,
    h: u32,
    layer: &[[f32; 4]],
    painting: &Picture,
    warp: &Warp,
    keep: &[[f64; 4]],
) -> Picture {
    let boxes: Vec<[f64; 4]> = keep
        .iter()
        .map(|r| {
            [
                r[0] * f64::from(w),
                r[1] * f64::from(h),
                r[2] * f64::from(w),
                r[3] * f64::from(h),
            ]
        })
        .collect();
    let kept = |px: u32, py: u32| -> f32 {
        let (x, y) = (f64::from(px) + 0.5, f64::from(py) + 0.5);
        boxes
            .iter()
            .map(|b| {
                let inside = (x - b[0]).min(b[2] - x).min(y - b[1]).min(b[3] - y);
                ((inside + KEEP_FEATHER) / (2.0 * KEEP_FEATHER)).clamp(0.0, 1.0) as f32
            })
            .fold(0.0, f32::max)
    };
    // The warp is smooth, so it is evaluated on a coarse grid and interpolated between.
    const STEP: u32 = 8;
    let (gw, gh) = (w / STEP + 2, h / STEP + 2);
    let grid: Vec<(f64, f64)> = (0..gh)
        .flat_map(|gy| (0..gw).map(move |gx| (gx, gy)))
        .map(|(gx, gy)| {
            warp.at(
                f64::from(gx * STEP) / f64::from(w),
                f64::from(gy * STEP) / f64::from(h),
            )
        })
        .collect();
    let mut out = Picture::blank(w, h);
    for py in 0..h {
        for px in 0..w {
            let l = layer[(py * w + px) as usize];
            let t = ((l[3] - FILL_EDGE.0) / (FILL_EDGE.1 - FILL_EDGE.0)).clamp(0.0, 1.0);
            let keep = (t * t * (3.0 - 2.0 * t)).max(if l[3] > 0.0 { kept(px, py) } else { 0.0 });
            let d = if l[3] > 0.0 {
                [l[0] / l[3], l[1] / l[3], l[2] / l[3], 1.0]
            } else {
                [0.0; 4]
            };
            if keep >= 1.0 {
                out.put(px, py, d);
                continue;
            }
            let (gx, gy) = (px / STEP, py / STEP);
            let (fx, fy) = (
                f64::from(px % STEP) / f64::from(STEP),
                f64::from(py % STEP) / f64::from(STEP),
            );
            let g = |x: u32, y: u32| grid[(y * gw + x) as usize];
            let lerp = |a: (f64, f64), b: (f64, f64), t: f64| {
                (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
            };
            let top = lerp(g(gx, gy), g(gx + 1, gy), fx);
            let bottom = lerp(g(gx, gy + 1), g(gx + 1, gy + 1), fx);
            let (u, v) = lerp(top, bottom, fy);
            let s = painting.sample(u as f32, v as f32);
            let c = [0, 1, 2].map(|i| d[i] * keep + s[i] * (1.0 - keep));
            out.put(px, py, [c[0], c[1], c[2], 1.0]);
        }
    }
    out
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

/// Deterministic smooth value noise; sampled in canvas space so zone seams cannot appear.
fn cloud_noise(x: f32, y: f32) -> f32 {
    let (ix, iy) = (x.floor() as u32, y.floor() as u32);
    let smooth = |t: f32| t * t * (3.0 - 2.0 * t);
    let (tx, ty) = (smooth(x.fract()), smooth(y.fract()));
    let hash = |x: u32, y: u32| {
        let mut h = x
            .wrapping_mul(374761393)
            .wrapping_add(y.wrapping_mul(668265263));
        h = (h ^ (h >> 13)).wrapping_mul(1274126177);
        (h ^ (h >> 16)) as f32 / u32::MAX as f32
    };
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    lerp(
        lerp(hash(ix, iy), hash(ix + 1, iy), tx),
        lerp(hash(ix, iy + 1), hash(ix + 1, iy + 1), tx),
        ty,
    )
}

fn args() -> Result<(PathBuf, PathBuf, Option<PathBuf>, bool)> {
    let mut data = None;
    let mut out = None;
    let mut fill = None;
    let mut exact = false;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--data" => data = it.next().map(PathBuf::from),
            "--out" => out = it.next().map(PathBuf::from),
            "--fill" => fill = it.next().map(PathBuf::from),
            "--exact" => exact = true,
            _ => bail!("usage: map_draft --data <WoW Data> --out <dir> [--fill <landmarks.json>] [--exact]"),
        }
    }
    let data = match data {
        Some(d) => d,
        None => benilla_formats::wow_data().context("no --data and no WoW install found")?,
    };
    Ok((
        data,
        out.unwrap_or_else(|| PathBuf::from("target/map_draft")),
        fill,
        exact,
    ))
}

fn main() -> Result<()> {
    let (data, out, fill_with, exact) = args()?;
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
    std::fs::create_dir_all(out.join("stock"))?;
    let elwynn = on_map
        .iter()
        .find(|(_, a)| a.name == "Elwynn")
        .context("no Elwynn row")?;
    let elwynn = Zone::load(&mut chain, elwynn.1, &[])?;
    let parchment = elwynn.parchment();
    let mut img = Picture::blank(size.0, size.1);
    let mut art_layer = vec![[0.0f32; 4]; (size.0 * size.1) as usize];
    let mut playable = Picture::blank(size.0, size.1);
    for y in 0..size.1 {
        for x in 0..size.0 {
            img.put(x, y, [parchment[0], parchment[1], parchment[2], 1.0]);
        }
    }

    let mut fogs = Vec::new();
    let mut placed_fogs = Vec::new();
    let mut covers = Vec::new();
    let mut stock = Vec::new();
    let mut written = Vec::new();
    let filler = match &fill_with {
        Some(path) => {
            let spec = FillSpec::read(path)?;
            let mut painting = Picture::open(&spec.painting)?;
            for e in &spec.erase {
                e.apply(&mut painting);
            }
            let warp = Warp::fit_inside(&spec.pairs, f64::from(size.1) / f64::from(size.0))?;
            Some((painting, warp, spec))
        }
        None => None,
    };
    let groups = [(&misted, true), (&listed_rows, false)];
    for (rows, is_misted) in groups {
        for &(id, area) in rows.iter() {
            let zone = Zone::load(&mut chain, area, wmo.for_area(id))?;
            let at = place(canvas, size, zone.rect);
            if !is_misted {
                // Export the complete original map, including the city's streets and backdrop.
                // No silhouette, cutout, painting, warp, interpolation, or resized landmarks.
                let mut original = Picture::blank(ART_W, ART_H);
                for y in 0..ART_H {
                    for x in 0..ART_W {
                        original.put(x, y, zone.explored.texel(x, y));
                    }
                }
                let name = format!("stock/{}.png", zone.file);
                original.save(&out.join(&name))?;
                stock.push(json!({
                    "file": name,
                    "area_id": area.area_id,
                    "size": [ART_W, ART_H],
                    "loc_rect": [zone.rect.left, zone.rect.right, zone.rect.top, zone.rect.bottom],
                    "canvas_rect_pixels": at,
                    "yards_per_source_pixel": [
                        (zone.rect.left - zone.rect.right) / ART_W as f32,
                        (zone.rect.top - zone.rect.bottom) / ART_H as f32
                    ],
                    "accuracy": "Original illustrated game artwork; DBC-aligned, not a yard-accurate terrain survey"
                }));
            }
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
            draw(&mut img, &mut art_layer, at, &zone);
            if !is_misted && exact {
                for y in 0..size.1 {
                    let v = ((f64::from(y) + 0.5 - at[1]) / (at[3] - at[1])) as f32;
                    if !(0.0..1.0).contains(&v) {
                        continue;
                    }
                    for x in 0..size.0 {
                        let u = ((f64::from(x) + 0.5 - at[0]) / (at[2] - at[0])) as f32;
                        if !(0.0..1.0).contains(&u) {
                            continue;
                        }
                        let a = zone.silhouette(u, v);
                        if a > 0.05 {
                            playable.put(x, y, [1.0; 4]);
                        }
                    }
                }
            }
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
    if exact {
        // Preserve the shipped canvas pixel-for-pixel: its landmarks are already stock art
        // seated at DBC coordinates. Never ask an image model to redraw the geography.
        let source =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/twilights-eve/map/canvas.png");
        let mut mapped = Picture::open(&source)?;
        anyhow::ensure!(
            (mapped.w, mapped.h) == size,
            "source canvas dimensions differ from DBC layout"
        );
        anyhow::ensure!(
            size == (5015, 3343),
            "necropolis crop needs re-anchoring for a changed canvas extent"
        );
        // Extend the coarse Highlight outlines enough to retain connecting rivers and passes,
        // then feather outward. The city uses its real silhouette rather than a square card.
        soften_alpha(&mut playable, 36);
        for p in playable.rgba.chunks_mut(4) {
            p[3] = if p[3] > 2 { 255 } else { 0 };
        }
        let mut closed = Picture::blank(size.0, size.1);
        for y in 0..size.1 {
            for x in 0..size.0 {
                closed.put(x, y, [1.0, 1.0, 1.0, 1.0 - playable.texel(x, y)[3]]);
            }
        }
        soften_alpha(&mut closed, 20);
        for y in 0..size.1 {
            for x in 0..size.0 {
                let a = closed.texel(x, y)[3];
                // Restrained continuous cloud texture in canvas coordinates, never per-zone panels.
                let (xf, yf) = (x as f32, y as f32);
                let cloud = cloud_noise(xf / 180.0, yf / 180.0) * 0.6
                    + cloud_noise(xf / 70.0, yf / 70.0) * 0.28
                    + cloud_noise(xf / 24.0, yf / 24.0) * 0.12;
                let light = 0.68 + cloud * 0.23;
                closed.put(x, y, [light + 0.035, light + 0.025, light, a]);
            }
        }
        closed.save(&out.join("exact_fog.png"))?;
        playable.save(&out.join("playable_mask.png"))?;
        let patch_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/twilights-eve/map/necropolis-patch.png");
        let patch = Picture::open(&patch_path)?;
        // Only this local clearing can change. Mountains and all other landmarks are untouched.
        for y in 2435..2620 {
            for x in 1925..2135 {
                let edge = ((x - 1925).min(2134 - x).min(y - 2435).min(2619 - y)) as f32;
                let t = (edge / 14.0).clamp(0.0, 1.0);
                let a = t * t * (3.0 - 2.0 * t);
                let s = patch.sample((x - 1850) as f32 / 360.0, (y - 2370) as f32 / 300.0);
                let d = mapped.texel(x, y);
                mapped.put(
                    x,
                    y,
                    [
                        s[0] * a + d[0] * (1.0 - a),
                        s[1] * a + d[1] * (1.0 - a),
                        s[2] * a + d[2] * (1.0 - a),
                        1.0,
                    ],
                );
            }
        }
        mapped.save(&out.join("exact_geography.png"))?;
        blend(
            &mut mapped,
            [0.0, 0.0, size.0 as f64, size.1 as f64],
            &closed,
        );
        mapped.save(&out.join("exact.png"))?;
        let original = Picture::open(&source)?;
        let mut unchanged_open_pixels = 0u64;
        for y in 0..size.1 {
            for x in 0..size.0 {
                let patch = (1925..2135).contains(&x) && (2435..2620).contains(&y);
                if !patch && closed.texel(x, y)[3] == 0.0 {
                    anyhow::ensure!(
                        mapped.texel(x, y) == original.texel(x, y),
                        "open landmark pixel changed at {x},{y}"
                    );
                    unchanged_open_pixels += 1;
                }
            }
        }
        std::fs::write(
            out.join("exact_validation.json"),
            serde_json::to_string_pretty(&json!({
                "size": [size.0, size.1],
                "loc_rect": [canvas.left, canvas.right, canvas.top, canvas.bottom],
                "playable_area_ids": listed,
                "unchanged_clear_pixels_outside_necropolis": unchanged_open_pixels,
                "necropolis_edit_rect_pixels": [1925, 2435, 2135, 2620],
                "fog": "baked for the current playable zones; rerun when zones open",
                "geography": "existing canvas pixels preserved; no generated redraw or coordinate warp"
            }))?,
        )?;
    }
    written.insert(0, "canvas.png".to_string());
    if let Some((painting, warp, spec)) = &filler {
        let mut filled = fill(size.0, size.1, &art_layer, painting, warp, &spec.keep);
        filled.save(&out.join("filled.png"))?;
        written.push("filled.png".to_string());

        let aspect = f64::from(size.1) / f64::from(size.0);
        let errors = landmark_errors(&spec.pairs, aspect, painting);
        let mut order: Vec<usize> = (0..errors.len()).collect();
        order.sort_by(|&a, &b| errors[b].total_cmp(&errors[a]));
        let limit = LANDMARK_FLAG * median(&errors);
        println!("landmark leave-one-out error, painting px (over {limit:.1} is flagged):");
        for i in order {
            let flag = if errors[i] > limit { "  FLAG" } else { "" };
            println!("  {:6.1}  {}{flag}", errors[i], spec.names[i]);
        }
        let bare = vec![[0.0f32; 4]; (size.0 * size.1) as usize];
        let mut marked = fill(size.0, size.1, &bare, painting, warp, &[]);
        mark_landmarks(&mut marked, &spec.pairs);
        marked.save(&out.join("landmarks.png"))?;
        written.push("landmarks.png".to_string());
        for (at, f) in &placed_fogs {
            blend(&mut filled, *at, f);
        }
        filled.save(&out.join("filled_preview.png"))?;
        written.push("filled_preview.png".to_string());
    }
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
        "stock": stock,
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
    fn the_warp_passes_through_its_landmarks_and_keeps_a_shift() {
        let shift = |u: f64, v: f64| FillPair {
            canvas: [u, v],
            painting: [u - 0.05, v - 0.1],
        };
        let pairs = vec![
            shift(0.1, 0.1),
            shift(0.9, 0.1),
            shift(0.5, 0.5),
            shift(0.1, 0.9),
            shift(0.9, 0.9),
        ];
        let warp = Warp::fit(&pairs, 2.0 / 3.0).unwrap();
        for (u, v) in [(0.5, 0.5), (0.3, 0.7), (0.0, 0.0)] {
            let (pu, pv) = warp.at(u, v);
            assert!((pu - (u - 0.05)).abs() < 1e-3, "u {u}: {pu}");
            assert!((pv - (v - 0.1)).abs() < 1e-3, "v {v}: {pv}");
        }
    }

    #[test]
    fn a_misread_landmark_has_the_largest_leave_one_out_error() {
        let shift = |u: f64, v: f64| FillPair {
            canvas: [u, v],
            painting: [u - 0.05, v - 0.1],
        };
        let mut pairs: Vec<FillPair> = [0.1, 0.4, 0.7, 0.9]
            .iter()
            .flat_map(|&u| [0.1, 0.5, 0.9].map(move |v| (u, v)))
            .map(|(u, v)| shift(u, v))
            .collect();
        pairs[4].painting[0] += 0.05;
        let painting = Picture::blank(1000, 1000);
        let errors = landmark_errors(&pairs, 1.0, &painting);
        let worst = (0..errors.len())
            .max_by(|&a, &b| errors[a].total_cmp(&errors[b]))
            .unwrap();
        assert_eq!(worst, 4, "{errors:?}");
        assert!(errors[4] > LANDMARK_FLAG * median(&errors), "{errors:?}");
    }

    #[test]
    fn a_pinned_warp_keeps_the_canvas_border_on_the_painting() {
        // The painting shows a tenth less land on every side: the canvas border falls outside.
        let zoom = |u: f64, v: f64| FillPair {
            canvas: [u, v],
            painting: [(u - 0.5) * 1.25 + 0.5, (v - 0.5) * 1.25 + 0.5],
        };
        let pairs = vec![
            zoom(0.2, 0.2),
            zoom(0.8, 0.2),
            zoom(0.5, 0.5),
            zoom(0.2, 0.8),
            zoom(0.8, 0.8),
        ];
        let warp = Warp::fit_inside(&pairs, 2.0 / 3.0).unwrap();
        for t in [0.0, 0.3, 0.5, 1.0] {
            for (u, v) in [(t, 0.0), (t, 1.0), (0.0, t), (1.0, t)] {
                let (pu, pv) = warp.at(u, v);
                assert!((-0.01..=1.01).contains(&pu), "({u}, {v}) -> u {pu}");
                assert!((-0.01..=1.01).contains(&pv), "({u}, {v}) -> v {pv}");
            }
        }
        let (pu, pv) = warp.at(0.5, 0.5);
        assert!(
            (pu - 0.5).abs() < 0.01 && (pv - 0.5).abs() < 0.01,
            "{pu} {pv}"
        );
    }

    #[test]
    fn an_erase_copies_the_land_at_its_offset_and_leaves_the_rest() {
        let mut pic = Picture::blank(40, 40);
        for y in 0..40 {
            for x in 0..40 {
                let red = if y < 20 { 1.0 } else { 0.0 };
                pic.put(x, y, [red, 0.0, 0.0, 1.0]);
            }
        }
        // A box in the lower, black half takes the red land 20 px above it.
        Erase {
            rect: [0.25, 0.625, 0.75, 0.875],
            from: [0.0, -0.5],
        }
        .apply(&mut pic);
        assert_eq!(pic.texel(20, 30)[0], 1.0, "the box's middle is patched");
        assert_eq!(pic.texel(2, 30)[0], 0.0, "outside the box is untouched");
        assert_eq!(pic.texel(20, 5)[0], 1.0, "the source is untouched");
    }

    #[test]
    fn the_canvas_rect_pads_the_union_to_the_art_aspect() {
        let r = canvas_rect(&[CANVAS]).unwrap();
        let aspect = (r.left - r.right) / (r.top - r.bottom);
        assert!((aspect - 1002.0 / 668.0).abs() < 1e-3, "{aspect}");
    }
}
