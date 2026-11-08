Title: The mist shader: per-layer noise that wraps, billows, faster drift and distance fog (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 1, 2 and 5. Blocked by 01-mist-shots (so the "before" build exists).
Blocked by SO:07-client-opening, which adds a fade factor to `MistExt` and multiplies it into the
alpha in `mist.wgsl`; keep that fade working exactly as it lands. Client only.

## Why

Davis called the #554 curtain a flat painted wall that is dead, with no motion (decision 1). In
the code: the solid band is one flat colour with a vertical gradient, because noise touches only
the alpha above the band; every piece reads the same noise, so the wisps are the backdrop's
pattern moved 4 yd; the drift is 0.4 yd/s, so a 23-yd noise cell takes almost a minute to pass;
the dark foot ignores distance fog; triangles draw in segment order, so one segment's backdrop
covers the last segment's wisps; and the drift jumps once an hour when `elapsed_secs_wrapped`
wraps. This ticket fixes all of that on today's four pieces. Ticket 03 then replaces the pieces
with shells, which use everything here.

Lines are as of `6da00cb7`; SO:07 moves some. Files: `crates/benilla-world/src/mist.rs` and
`crates/benilla-world/src/shaders/mist.wgsl`.

## What to change

### A layer number on every vertex

`curtain_mesh` (mist.rs line 354) adds `Mesh::ATTRIBUTE_COLOR`, one `[f32; 4]` per vertex:
`x` is the piece's layer number, the same on all its vertices; `y`, `z` and `w` are 0 for now.
Today's pieces are layers 0 (the backdrop), 1 and 2 (the wisp sheets on rows 1 and 2) and 3 (the
skirt). Bevy defines `VERTEX_COLORS` for the shader when a mesh has a colour attribute, so read
`in.color` under `#ifdef VERTEX_COLORS` and take layer 0 without it. The material is unlit and the
fragment shader is ours, so the colour never tints anything.

### Draw order by layer

Build the index buffer layer by layer across every segment: the backdrop of every segment, then
every row-1 wisp, every row-2 wisp, every skirt. The camera stands on the open side, so that is
back to front. `CurtainBuilder::grid` (line 323) indexes the vertices just pushed; keep one index
list per layer and join them at the end.

### Noise

Replace `value_noise` and `fractal` (mist.wgsl lines 39 to 57) with gradient noise:

- One octave is 2D gradient noise in about [0, 1] (rescale the usual [-1, 1]).
- The lattice wraps: octave `k` (cell `23 / 2^k` yd) hashes its lattice point modulo `256 * 2^k`
  cells, so every octave repeats every 5,888 yd (256 cells of 23 yd), along and up. Add
  `pub const NOISE_PERIOD_YD: f32 = 5888.0;` in mist.rs with a comment that the shader's lattice
  must match it.
- The main noise `n` is 4 octaves (cells 23, 11.5, 5.75 and 2.875 yd), weights 0.5, 0.25, 0.125,
  0.0625, normalised. The two domain-warp samples are 2 octaves each.
- Each layer reads its own noise: add `layer * (137.0, 71.0)` yd to the sample point before the
  warp. Keep `NOISE_SCALE` (1/23).

### Billows and a thinner solid band

- A second noise `b` (2 octaves, offset `(311.0, 173.0)` yd from `n`'s point, same warp) scales the
  colour's brightness by `1 + (b - 0.5) * 2 * BILLOW_LIGHT`, `BILLOW_LIGHT` 0.15, so lumps are up to
  15% lighter or darker. Apply it after the foot colour and before the distance fog.
- Inside the solid band (below `start`), alpha is `mix(BAND_FLOOR, 1, n)` instead of 1, with
  `BAND_FLOOR` 0.85, so the band shows faint billows against the fog-coloured void behind it.

### Faster drift that never jumps

- `DRIFT_YD_S` 0.4 becomes 1.2 and `DRIFT_UP_YD_S` 0.3 becomes 0.6. The upward speed must stay half
  the sideways one; say so in their doc comments.
- `pub const LAYER_DRIFT: [f32; 5] = [1.0, -0.5, 1.5, 0.5, -1.0];` multiplies each layer's sideways
  drift by its own number; the upward drift takes its absolute value. Every entry is a multiple of
  0.5. Layers past the end of the table take the last entry.
- The clock wraps where every layer has moved a whole number of noise periods:
  `pub const DRIFT_WRAP_SECS: f64 = 4.0 * NOISE_PERIOD_YD as f64 / DRIFT_YD_S as f64;`, 19,627 s.
  `tint_mist` (line 443) writes `(time.elapsed_secs_f64() % DRIFT_WRAP_SECS) as f32` into `fog.w`
  instead of `elapsed_secs_wrapped()`.
- The shader moves layer `L`'s sample point by `(-shape.y * m, -shape.z * abs(m)) * t`, where `m`
  is `LAYER_DRIFT[L]`. Pass the five multipliers in a new uniform (below).

### Distance fog

Mix the final colour toward the fog colour by the scene's linear fog, as `terrain.wgsl` does at
lines 231 to 238: planar eye-Z from `view.view_from_world`, `factor = clamp((end - eye_z) /
max(end - start, 0.001), 0, 1)`, `colour = mix(fog.rgb, colour, factor)`. `tint_mist` writes
`WowLighting::fog_start` and `fog_end` (both `pub(crate)` in `lighting/mod.rs` lines 41 and 42,
same crate) each frame.

### Uniforms

Add to `MistExt` (line 74), after whatever SO:07 added:

- `#[uniform(104)] scene: Vec4`: `fog_start`, `fog_end`, 0, 0. `tint_mist` writes it.
- `#[uniform(105)] drift: [Vec4; 2]` or two `Vec4`s: `LAYER_DRIFT[0..4]` in the first, `LAYER_DRIFT[4]`
  in the second's `x`. `setup_mist` writes it once. Shape it as you see fit; the shader indexes it
  by layer.

`BILLOW_LIGHT` and `BAND_FLOOR` can go in a spare lane (`rag.w` if SO:07 left it free, else the
new uniform's spare lanes). Every new constant is a `pub const` in mist.rs with a doc comment, as
the existing ones are, and the shader's header comment says what each lane holds.

## Failing tests first

In mist.rs's `mod tests`:

- `every_piece_carries_its_layer_number`: a one-segment mesh's `ATTRIBUTE_COLOR` is `x = 0` on the
  backdrop's 18 vertices, 1 and 2 on the wisp sheets' 18 each, 3 on the skirt's 27.
- `indices_run_layer_by_layer_across_segments`: in the two-segment mesh of
  `neighbouring_segments_share_their_corners` (line 586), the first 96 indices (two backdrops of 16
  triangles) reference only backdrop vertices, the next 96 only row-1 wisps, the next 96 only row-2
  wisps, and the last 192 only skirt vertices.
- `the_drift_wraps_on_whole_noise_periods`: for every `m` in `LAYER_DRIFT`,
  `m * DRIFT_YD_S * DRIFT_WRAP_SECS / NOISE_PERIOD_YD` and
  `|m| * DRIFT_UP_YD_S * DRIFT_WRAP_SECS / NOISE_PERIOD_YD` are whole numbers within 1e-3, and
  `DRIFT_WRAP_SECS` is over an hour.

Keep the existing tests passing; update the counts only where this ticket changes them.

## Acceptance

- `cargo test -q -p benilla-world --lib mist 2>&1 | grep -E "FAILED|panicked|test result"` passes.
- `cargo check -q -p benilla-world` and `cargo check -q -p benilla-app` are clean. CLAUDE.md and
  `docs/agents/unattended.md` say how to run client builds.
- The gate cannot compile WGSL. Read `mist.wgsl` through once more after the edit for undeclared
  names, a missing `#ifdef` and mismatched types. Ticket 11 compiles it in a capture run and fixes
  what you could not see.
- What it looks like is ticket 11's and the spec's `unplayed` label's, not this ticket's.

Assumptions allowed: the gradient-noise formula and hash, the uniform layout, which spare lanes
carry `BILLOW_LIGHT` and `BAND_FLOOR`. Record them as `Assumed:` lines.
