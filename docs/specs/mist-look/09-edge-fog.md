Title: The scene fog pulls in and greys near the edge of the mist (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 6. Blocked by 01-mist-shots (so the "before" build exists). Blocked by
SO:07-client-opening, which gives the closed-zone readers a reached set; this ticket adds one more
reader. Client only: `crates/benilla-formats/src/closed_zones.rs`, a new
`crates/benilla-world/src/lighting/edge.rs`, `crates/benilla-world/src/lighting/mod.rs` and
`crates/benilla-world/src/lighting/resolve.rs`. No mist file changes, so this runs beside tickets 02
to 08.

## Why

Davis wants the approach to the mist to feel different from the rest of the world: within about
60 yd of the edge, the scene fog closes in and goes grey (decision 6), so the whole screen feels
the Hush, not just the wall. The mist's colour is the scene fog's (decision 5), so it greys with
the air near the edge and stays the normal fog colour seen from far away. Ticket 10 fades the sound
by the same curve.

## What to change

### How far the edge is (`closed_zones.rs`)

`pub fn closed_distance(map, x, y, radius_yd, <reached set as SO:07 shaped it>) -> Option<f32>`:
the distance in yards from `(x, y)` to the nearest closed chunk's square among those
`closed_chunks_near` returns for the same radius, 0 when the point is on a closed chunk, `None`
when no closed chunk is within `radius_yd`. A gated chunk whose key is reached is open.

### The curve and the pull (`lighting/edge.rs`, new)

`pub mod edge;` in `lighting/mod.rs` (beside the private modules at lines 9 to 14), so ticket 10 can
call it from `benilla-app`.

| Constant | Value | Meaning |
|---|---|---|
| `EDGE_NEAR_YD` | 60.0 | the pull starts this far from the nearest closed chunk |
| `PULL_FOG_START_YD` | 20.0 | the scene fog's start at the edge |
| `PULL_FOG_END_YD` | 200.0 | the scene fog's end at the edge (Elwynn by day is 125 to 500) |
| `PULL_GREY` | 0.6 | how far the fog colour moves toward its own grey at the edge |
| `PULL_EASE_PER_S` | 0.25 | the most the applied pull changes in a second |

- `pub fn edge_pull(distance: Option<f32>) -> f32`: 0 for `None`, else
  `1 - smoothstep(0, EDGE_NEAR_YD, d)`, so 1 at the edge and 0 at 60 yd.
- `pub fn ease(current: f32, target: f32, dt: f32) -> f32`: moves toward `target` by at most
  `PULL_EASE_PER_S * dt`, never past it. A zone that opens (SO:07) or a teleport then eases the fog
  out or in over about 4 s instead of snapping.
- `pub fn pulled(pull, fog_start, fog_end, fog_color, sky) -> (start, end, color, sky)`:
  - `end = min(fog_end, lerp(fog_end, PULL_FOG_END_YD, pull))`
  - `start = min(fog_start, lerp(fog_start, PULL_FOG_START_YD, pull))`, and below `end`
  - `color = mix(fog_color, luma(fog_color), PULL_GREY * pull)`, luma weights 0.299, 0.587, 0.114
  - each of the five sky stops greyed the same way, so the mist's top, which fades to the fog
    colour, still meets the sky.

### Applying it (`resolve.rs`)

In `update_time_lighting` (line 171), after `scene_fog` (line 253) and before the WMO crossfade
(line 258), which then blends from the pulled values:

- `target = 0` when submerged (`submerged.any()`; the underwater fog owns the screen), else
  `edge_pull(closed_distance(map, wow_pos[0], wow_pos[1], EDGE_NEAR_YD, reached))`, where `wow_pos`
  is the camera eye the light sample already uses (line 218) and `reached` is the set SO:07 gives
  `benilla-world` (read it the way `build_mist_curtains` does after #633).
- Keep the applied pull in a `Local<f32>`, eased toward `target` with `time.delta_secs()`.
- `resolved.fog_color`, `fog_start`, `fog_end` and `sky` (lines 283 to 285 and the `sky` line)
  take `pulled`'s values. The WMO triple, the farclip wall, the water colours and everything else
  stay as they are.

Placed mist walls (ADR 0033) do not pull the fog; only closed chunks do. The minimap paints closed
chunks in the fog colour (`crates/benilla-app/src/minimap/mod.rs` line 618), so near the edge its
closed squares grey a little too. That is accepted; the maps are out of scope.

## Failing tests first

- `closed_zones.rs` tests: `closed_distance(0, -9700.0, 856.67, 60.0, <none reached>)` is 10.0
  within 0.01 (the edge is y 866.67 there); `(-9700.0, 880.0)` is 0.0 (on Westfall);
  `(-9700.0, 760.0)` is `None` (106.7 yd off).
- `lighting/edge.rs` tests:
  - `edge_pull`: 1 at 0, 0 at 60 and beyond, 0 for `None`, never rising with distance.
  - `ease`: from 0 toward 1 with `dt` 1.0 gives 0.25; from 0.9 toward 1 with `dt` 1.0 gives 1.0;
    toward a lower target it falls the same way.
  - `pulled`: Elwynn's clear fog (125, 500) at pull 1 gives (20, 200), at pull 0 is unchanged; a
    storm's (-139, 278) at pull 1 gives (-139, 200); a colour at pull 1 keeps its luma and its
    distance from its grey shrinks to 40%.

## Acceptance

- `cargo test -q -p benilla-formats closed_zones 2>&1 | grep -E "FAILED|panicked|test result"` and
  `cargo test -q -p benilla-world --lib lighting 2>&1 | grep -E "FAILED|panicked|test result"` pass.
- `cargo check -q -p benilla-world` and `cargo check -q -p benilla-app` are clean. CLAUDE.md and
  `docs/agents/unattended.md` say how to run client builds.
- What it looks like is ticket 11's (the `close` viewpoint stands 15 yd from the wall) and the
  spec's `unplayed` label's.

Assumptions allowed: the exact signature SO:07's reached set gives `closed_distance`, how the
system reaches the set. Record them as `Assumed:` lines.
