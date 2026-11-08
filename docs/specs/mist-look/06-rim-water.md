Title: The rim row's water draws and fades into the mist (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 3 (Claude's reading: the rim row's land includes its water). Blocked by
05-rim-terrain (the rim mask on `ChunkShading`, `rim_depth`, `RIM_FOG_YD` and the ADR amendment,
whose water line this ticket makes true). Client only.

## Why

Ticket 05 draws the closed terrain of the rim row and fogs it into the mist, but keeps clearing its
water. The Elwynn and Westfall border runs down a river: water at z 21.2 to 21.4 from y 840 to 880
along x -9640 to -9900, with the edge at y 866.7 and 733.3. With the water cut, the river stops at a
straight line on the edge while its dry bed runs on into the grey, which shows the cut the rim row
exists to hide. This ticket keeps the rim's water and fogs it the same way as its land.

## What to change

### Keep it (`crates/benilla-assets/src/adt.rs`)

`cut_closed` (line 312) stops clearing `liquids` on a rim chunk. A closed chunk that is not a rim
chunk still loses its water. Update the comment and ticket 05's rim test (the rim chunk now keeps
its liquids).

### Carry the depth (`crates/benilla-world/src/liquid/surface.rs`)

- `spawn_liquids` (line 143) learns each liquid's chunk: take the chunk's north-west corner
  (`positions[0]`) and its `ChunkShading::rim_open` beside each `LiquidMesh`. Its caller in
  `crates/benilla-world/src/terrain_stream.rs` (line 788) zips `adt.chunks` with `adt.shading`;
  update the test caller (surface.rs line 558) with a 0 mask.
- `liquid_bevy_mesh` (line 213) writes `UV_1.y`, 0 today, as `rim_depth(vertex xy, north_west,
  rim_open)` from `benilla_assets`; 0 when the mask is 0. WMO liquids pass 0.

### Fog it (`crates/benilla-assets/src/shaders/liquid.wgsl`, `materials.rs`)

- `LiquidExt` (`materials.rs` line 342) gains a fourth `#[uniform(102)]` `Vec4`, `mist`:
  `RIM_FOG_YD`, `crate::mist::FOOT_DESAT`, `crate::mist::FOOT_VALUE`, 0. Add the field to the WGSL
  struct in the same order. Set it where `LiquidExt` is built (`surface.rs` line 384).
- The vertex stage passes `in.uv_b.y` on as a new `LiquidVsOut` field `rim_depth`.
- Each of the four returns that call `apply_fog` (lines 209, 234, 258 and 282) mixes its colour
  toward the foot colour first, as ticket 05's terrain does:
  `rim = smoothstep(0.0, mist.x, rim_depth)`,
  `foot = mix(fog, vec3(luma(fog)), mist.y) * mist.z` with `fog` the scene fog colour, and
  `rgb = mix(rgb, foot, rim)`. Where the return carries an alpha below 1, raise it the same way,
  `alpha = mix(alpha, 1.0, rim)`, so the water ends in solid grey like the land around it. A shared
  helper keeps the four alike.

The liquid collider (`liquid_collider`, line 196) is built for the rim's water too. That is fine:
the fence stops the walk.

## Failing tests first

- `crates/benilla-assets/src/adt.rs` `closed_tests`: a rim chunk keeps its liquids; a closed chunk
  with no open neighbour still loses them.
- `crates/benilla-world/src/liquid/surface.rs` tests, beside `one_sheet` (line 505): a sheet on a rim
  chunk with only its east neighbour open has `UV_1.y` 0 on its east column and 33.33 on its west
  column; a sheet with mask 0 has `UV_1.y` 0 everywhere.

## Acceptance

- `cargo test -q -p benilla-assets --lib 2>&1 | grep -E "FAILED|panicked|test result"` and
  `cargo test -q -p benilla-world --lib liquid 2>&1 | grep -E "FAILED|panicked|test result"` pass.
- `cargo check -q -p benilla-world` and `cargo check -q -p benilla-app` are clean. CLAUDE.md and
  `docs/agents/unattended.md` say how to run client builds.
- The gate cannot compile WGSL; read `liquid.wgsl` once more after the edit. Ticket 11 compiles it.
- What it looks like is ticket 11's and the spec's `unplayed` label's.

Assumptions allowed: how `spawn_liquids` takes the chunk data, the helper's name in WGSL. Record
them as `Assumed:` lines.
