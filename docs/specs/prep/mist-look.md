# Mist look: grill prep

Prep for a grill on making the Closed Zones' mist look better. Read-only survey of the code at
`b578f014`; nothing here is decided. Guesses are marked "Assumed:".

Background: #303 built the curtain (f1c964a). Davis played the Westfall edge on 2026-11-02 and
called it "a flat white sheet". #554 reworked it (a526d4b7..2243ef4f, closed 2026-11-03): a dark
foot, a ragged top, a ground skirt and two wisp sheets. #588 reused the same curtain for placed
walls (ADR 0033, the Deeprun Tram tunnel). No play note on the #554 look exists in any issue, and
no mist screenshot exists under `docs/`, `.claude/smoke/` or `target/`. The lore bible's target
(`docs/lore-bible.md`, "The mist and the Anchors"): the Nether's grey edge, "roads end in grey",
a dead grey mass that pools on the ground, not a wall.

## 1. How the mist is built today

All in `crates/benilla-world/src/mist.rs` (667 lines) and `crates/benilla-world/src/shaders/mist.wgsl`
(92 lines). Bevy 0.18.1 (`Cargo.toml:23`).

Where it stands. `segments()` (mist.rs:140) makes one segment per side of an open map chunk whose
neighbour is closed (`data/twilights-eve/closed_chunks.json`). A segment is a straight 33.3-yard
run on a chunk-grid line, so the wall follows the chunk grid as a staircase. At the Goldshire
latitude the edge runs at about y 1000 to 1033; between x -8933 and -10800 the edge line jumps
between y 1200 and 633 in steps of one to five chunks (33 to 167 yards). Placed walls
(`wall_segment`, mist.rs:223) build one segment from `data/twilights-eve/mist_walls.json`; today
only `deeprun_tunnel` on map 369, bottom -20, top 30, floor -4.3.

The mesh. `curtain_mesh()` (mist.rs:354) builds four pieces per segment into one mesh per ADT tile
(child of the tile's terrain root, despawns with it, `ExteriorScene` at mist.rs:433):

- Backdrop: 9 columns on the side line (row 0), from `bottom` to `top`, alpha ceiling 1.
- Two wisp sheets: on the open chunk's vertex rows 1 and 2 (4.2 and 8.3 yd in from the side), each
  column from its own ground minus 1 yd (mist.rs:360) up to `top`, ceiling `WISP_CAP`.
- Ground skirt: a 9 by 3 grid over rows 0 to 2, raised `SKIRT_RAISE_YD`, ceiling `SKIRT_CAP`,
  fading to 0 by row 2.
- 81 vertices and 40 quads per segment. `UV_0` = (yards above ground, sheet top above ground);
  `UV_1` = (ceiling, yards above the piece's own foot).

The material (`setup_mist`, mist.rs:388): `ExtendedMaterial<StandardMaterial, MistExt>`, unlit,
`AlphaMode::Blend`, both faces (`cull_mode: None`, :392), no depth write (:103). One shared
material. `tint_mist` (:443) writes `WowLighting::fog_color` (Light.dbc IntBand row 7, the
scene's distance fog colour, raw gamma) and `time.elapsed_secs_wrapped()` (:454) each frame.

The shader (`mist.wgsl` `fragment`, :59). One warped fractal value noise `n` (3 octaves of
`value_noise`, cells 23, 11.5 and 5.75 yd, `NOISE_SCALE` :28; warp from two more fractal samples,
:73), sampled at world `(x + z, y)` drifting sideways and up. Below `start = OPAQUE_YD + (n - 0.5)
* 2 * RAG_YD` alpha is 1; above it alpha falls linearly to 0 at the sheet top, times
`mix(FADE_FLOOR, 1, n)` (:75 to :80). Colour = fog colour desaturated and darkened at the foot,
back to the pure fog colour over `COLOUR_RAMP_YD` (:81 to :83). Wisps and skirt fade out within
`CAMERA_FADE_YD` of the camera (:87). The colour itself carries no noise.

Tunables (mist.rs):

| Constant | Value | Line | Meaning |
|---|---|---|---|
| `BELOW_YD` | 10.0 | 34 | backdrop reaches below the side's lowest ground |
| `ABOVE_YD` | 120.0 | 36 | sheet top above the side's highest ground |
| `OPAQUE_YD` | 25.0 | 39 | solid band height where the noise is middling |
| `RAG_YD` | 20.0 | 41 | noise moves the fade start by this much, so 5 to 45 yd |
| `FADE_FLOOR` | 0.3 | 43 | fade's ceiling where the noise is lowest |
| `DRIFT_YD_S` | 0.4 | 45 | sideways drift, yd/s |
| `DRIFT_UP_YD_S` | 0.3 | 46 | upward drift, yd/s |
| `WARP_YD` | 8.0 | 48 | domain warp |
| `FOOT_DESAT` | 0.6 | 50 | foot desaturation |
| `FOOT_VALUE` | 0.35 | 52 | foot brightness |
| `COLOUR_RAMP_YD` | 60.0 | 54 | foot colour returns to fog colour over this |
| `FOOT_RAMP_YD` | 6.0 | 56 | each piece fades in above its own foot |
| `WISP_CAP` | 0.5 | 58 | wisp sheets' alpha ceiling |
| `SKIRT_CAP` | 0.6 | 60 | skirt's alpha ceiling |
| `SKIRT_RAISE_YD` | 1.5 | 62 | skirt height above the outer-row vertices |
| `CAMERA_FADE_YD` | 3.0 | 65 | wisps and skirt vanish this close to the camera |
| `WALL_THICK_YD` | 0.5 | 218 | placed wall's collider thickness |
| `NOISE_SCALE` (wgsl) | 1/23 | 28 | first octave cells per yard |

What surrounds it. Closed chunks draw no terrain, water, clutter, placements or horizon (ADR 0015,
#302). The clear colour behind that void is the same fog colour (`apply_sky_backdrop`,
`lighting/resolve.rs:377`), so everything past the wall is one flat field of fog colour. The
minimap paints closed chunks in the fog colour (`benilla-app/src/minimap/mod.rs:618`); the world
map draws stock mist art (`GetMapMists`, `benilla-ui/src/script/worldmap.rs:912`).

What the renderer offers:

- Custom WGSL: yes, many (`benilla-world/src/shaders/`: sky, cloud, static_gx, wow_effect,
  ffx_glow, mist; `benilla-assets/src/shaders/`: terrain, wdl, liquid, wow_model). A new shader
  or a change to `mist.wgsl` is routine. The gate cannot compile WGSL; a capture run does.
- Distance fog: custom, linear, in gamma space, in each shader (`fog_params` in `terrain.wgsl:39`;
  `WowLighting::fog_start/fog_end`). Bevy's `DistanceFog` is deliberately unused
  (`lighting/mod.rs:37`). The mist shader applies no distance fog.
- Particles: yes. A world effect stream (`crate::particles`, `EffectDrawSpec` in
  `particles/buffer.rs:207`) with alpha, add and multiply blends, scene fog, and stock BLP
  textures. Weather already draws up to 128 fog-coloured 12 by 12 yd puffs with
  `textures/weather/snowmist01.blp` (`weather/precip/mist.rs`). Billboard puffs are cheap to add.
- Procedural textures: the cloud layer builds a coverage image on the CPU and uploads it
  (`clouds/kernel.rs`, `clouds/layer.rs`). A CPU-made noise texture for the mist would follow it.
- Post-processing: one, the FFX glow composite (`ffx_glow.rs`, `Tonemapping::None`). No bloom,
  no volumetric fog, no depth prepass (no `DepthPrepass` anywhere). Soft edges that read scene
  depth would need a prepass for every material, including the custom terrain and model ones.
  MSAA is on (`MsaaSetting`).

## 2. Likely reasons it looks off, judged from the code

1. Flat colour in the solid band. Alpha is 1 from the ground to 5 to 45 yd up, and colour varies
   only with height. The noise touches alpha above the band, never colour. Up close the lower
   wall is one flat grey with a vertical gradient. Fix: modulate brightness by a second noise
   (billows lighter on top, darker underneath, about plus or minus 15%), and let the solid band
   thin to 0.85 to 0.9 in places where the void behind is the same colour anyway.
2. Straight panels and right-angle corners. Each segment is a flat vertical plane on the chunk
   grid, and the edge steps up to 167 yd at right angles. Nothing in a mist is a straight line.
   Fix: build the backdrop on a smoothed edge line (chamfer each corner across the closed chunk,
   or a moving average of the edge path), or hide the steps behind layered depth (direction B).
3. No depth. Behind the backdrop is a uniform fog-colour void, so the wall has no inside. The two
   wisp sheets sit only 4.2 and 8.3 yd in front and read nearly the same noise: `along = x + z`
   shifts by only the row offset (mist.wgsl:68), so each wisp is the backdrop's pattern moved 4
   yd. Fix: give each piece its own noise offset and scale (pack a layer index in `UV_1`), and add
   sheets behind the backdrop, inside the closed chunk.
4. The mist ignores distance fog. Terrain fades to the fog colour with distance; the mist's dark
   foot (35% brightness) does not. Assumed: from Goldshire the far wall shows a dark band under
   haze that lights nothing else. Fix: mix the mist colour toward the fog colour by the same
   `fog_start`/`fog_end` linear factor the terrain uses (both are `pub(crate)` on `WowLighting`,
   same crate).
5. Draw order across segments. One mesh per tile, no depth write, triangles in index order:
   segment A's backdrop, wisps, skirt, then segment B's. Where B's backdrop is behind A's wisps
   on screen, B draws later and covers them. Assumed: wisps and skirt vanish in strips near
   staircase corners. Fix: order the index buffer by piece across all segments (every backdrop,
   then every row-1 wisp, then every row-2 wisp, then every skirt). The camera is always on the
   open side, so that order is back to front.
6. Corners double up or leave notches. At a convex corner the two segments' wisp sheets and
   skirts cross, doubling alpha in a visible plus shape; at a concave corner they stop short.
   Fix: clip wisp and skirt rows at the corner diagonal, or replace them with puffs.
7. Blobby, gridded noise. Three octaves of value noise with the finest cell 5.75 yd give soft
   blobs and no fine wisps, and value noise shows its square lattice. Fix: 5 octaves of gradient
   noise, or one tileable noise texture made on the CPU at startup (cheaper per pixel than the
   current 9 noise calls times 4 pieces of overdraw).
8. Hard skirt lines on hills. The skirt floats 1.5 yd above the outer-row vertices only; inner
   terrain vertices between them can rise through it, and with no depth prepass the crossing is a
   hard line. Fix: raise it and lower its ceiling, ramp it by ground slope, or swap it for ground
   puffs.
9. Colour comes only from the scene fog. By day in Elwynn that colour is pale. Assumed: a pale
   blue-white, which is why #303 read as white. The lore wants a dead grey. Fix: mix the fog
   colour toward a fixed grey (`MIST_GREY`, `MIST_GREY_MIX`), keeping the top in the fog colour so
   it still meets the sky.
10. The placed tram wall uses the outdoor scene fog. Map 369 has no Light.dbc row; its atmosphere
    is the WMO's own fog, RGB(30,53,100) (`capture/scenarios.rs:288`). `tint_mist` writes the
    scene `fog_color`, not `wmo_fog_color`. Assumed: the tunnel wall is a colour that does not
    match the tunnel. Also its wisps stand only 1 and 2 yd in front, and the tunnel is probably
    lower than the 25-yd solid band, so the whole wall is one flat opaque rectangle. Fix: a second
    material instance for placed walls that reads `wmo_fog_color`, and wall-specific depths.
11. The drift jumps once an hour. `elapsed_secs_wrapped` wraps hourly (#303 note). Minor. Fix:
    wrap at a period long enough that no one sees it, or step the noise offset on the CPU.

## 3. Look directions

Every direction should take fixes 4, 5, 9 and 10 first; they are small and fix bugs, not taste.

**A. Layered shells (volumetric-style).** Replace the backdrop with 4 to 6 parallel sheets
stepping into the closed chunk, about 6 to 10 yd apart, each with its own noise offset and
scale. Front sheets are thin (alpha 0.3), the last is solid. The closed chunk's heights are in
`adt.chunks` already (`segments` only skips them), so each shell can stand on the ground under it.
Colour noise gives billows. What Davis sees: walking up, the mist has an inside; the front
thins and moves against the back (parallax); the staircase steps blur because each shell's
corner sits deeper. Cost: a mesh builder rewrite and a shader tweak, one to two tickets; overdraw
rises from 4 layers to about 8 near the wall. Assumed: fine on Davis's GPU, since it is a strip
of screen, not full screen, except when the camera faces the wall up close.

**B. A WGSL shader with better noise and fake soft edges.** Keep today's mesh and spend the work
in the shader: a CPU-made tileable noise texture, 5 octaves, billow lighting (darker underside,
brighter top), per-piece offsets, distance fog, a smoothed staircase by chamfering corners in the
mesh. The ground contact already fakes depth fade through `UV_0` (yards above the ground). What
Davis sees: the same wall, but alive, with soft rolling lumps and no flat area; still a wall.
Cost: smallest, one ticket. A real depth-fade (soft intersection with any geometry) needs a depth
prepass across all materials; not recommended.

**C. The land fades into grey.** Draw the terrain of the first one or two closed chunk rows
(33 to 67 yd) and fog it toward the mist grey by distance past the edge, in `terrain.wgsl`, with
the curtain moved to the far side of those rows. Roads and grass run on and dissolve, which is
the lore's line "roads end in grey". The staircase disappears from view because the cut is
inside dense grey. Cost: amends ADR 0015 ("the client draws no terrain ... on a closed chunk"),
changes the `terrain_stream.rs` cut and the terrain shader, and the realm already refuses moves
there, so collision stays. Two to three tickets and a grill on the ADR. Clutter and trees stay
cut. Assumed: the WDL horizon stays cut, so far away nothing changes.

**D. Billboard puff clouds.** Large camera-facing puffs (10 to 30 yd) along the foot of the wall,
drawn through the effect stream with `snowmist01.blp` or another stock smoke BLP, fog-tinted,
turning and rising slowly; a few drift out onto the open ground and fade. What Davis sees: real
rounded cloud shapes with soft silhouettes, closest to a WoW cloud bank. Cost: a CPU puff pool per
visible segment (like weather's 128-puff pool), sorting, and puffs clip hard into hills without a
depth prepass, so they float above the ground. One to two tickets. Best as a layer on top of A or
B, not alone.

A modifier for any direction: **the world drains near the wall.** Within about 60 yd of the edge,
pull the scene fog in (`fog_start`/`fog_end` toward the camera) and shift its colour toward the
mist grey. This was held back from #554 because it crosses lighting. It is cheap (the lighting
resolve already blends fog for storms and WMOs) and makes the whole screen feel the Hush, not
just the wall.

## 4. Open decisions for the grill

1. What bothers you most about the mist as it is now (#554 build)? Options: (a) it still reads as
   a flat painted wall; (b) the straight panels and right-angle steps; (c) the colour, too bright
   or the wrong hue against the sky; (d) it looks dead, nothing moves or has depth. Recommended:
   look at the before-shots from section 5 first, then pick. My guess from the code is (a) and
   (b): the solid band has no colour variation, and the chunk staircase is a row of flat panels.
2. What one picture should the mist be? (a) a rolling grey cloud bank; (b) land that fades into
   grey, the road running on and dissolving; (c) a low ground fog with grey void above; (d) a thin
   veil you can half see through. Recommended: (b) at the ground with (a) above it. It matches
   "roads end in grey" and hides the cut better than any sheet.
3. May the client draw the first closed chunk rows' terrain inside the mist (amends ADR 0015)?
   (a) no, keep the cut where it is; (b) yes, one row (33 yd), fogged to full mist; (c) yes, two
   rows. Recommended: (b). It is the biggest gain in depth, and the realm wall already blocks the
   ground, so only the look changes.
4. What should replace the flat backdrop? (a) keep it and improve the shader (direction B);
   (b) layered shells into the closed side (A); (c) shells plus puffs at the foot (A + D).
   Recommended: (b) first, puffs later only if the shots still look flat. Shells reuse the
   existing mesh code and material.
5. What colour is the mist? (a) the scene fog colour as now; (b) the fog colour mixed half-way to
   a fixed dead grey, with the top still pure fog colour; (c) its own colour table by time of day.
   Recommended: (b). It reads grey in every light and still meets the sky without a seam.
6. Should the world change near the wall? (a) no; (b) the scene fog pulls in and greys within
   about 60 yd of the edge; (c) (b) plus the ambience fading (the Hush). Recommended: (b) in this
   spec, sound in its own ticket. Cheap, and it makes the approach feel different.
7. Should the staircase be smoothed in the mesh, even if shells or terrain hide it? (a) no; (b)
   chamfer each corner across the closed chunk; (c) a smoothed edge line through closed chunks.
   Recommended: (b). One rule in `segments`, and corners are where the panels show most.
8. Does the tram's placed wall get the same look? (a) same material, same numbers; (b) same shader
   but its own colour from the WMO fog and its own depths; (c) its own look entirely. Recommended:
   (b). The tunnel has no scene fog row, and the trams must still burst out of it (ADR 0033).
9. How is the look judged before you play it? (a) you play each change; (b) the builder produces
   before and after capture shots at fixed viewpoints and times, and you approve from the shots;
   (c) both. Recommended: (b), then one play pass at the end. It needs the ticket to allow capture
   runs, which CLAUDE.md otherwise reserves for your request.
10. Are the maps in scope? (a) no; (b) the minimap's closed chunks take the new grey; (c) the world
    map mist art is redone too. Recommended: (b) only if decision 5 changes the colour, since the
    minimap copies the fog colour today.

## 5. Ticket sketch

Judging a look needs pictures, and no mist screenshot exists today. The capture harness can make
them without Davis:

- `WOW_CAPTURE=<scenario>` boots the client with no server, pins the camera eye and look (raw WoW
  coords) and the game minute, freezes the clock and runs 150 fixed steps, waits for a stable
  frame and writes one PNG to `WOW_CAPTURE_OUT` (`crates/benilla-app/src/capture/mod.rs` header).
  The mist drift reads `Time`, so a frozen clock makes the noise repeatable. Viewpoints being
  worked on go in `ON_DEMAND` (`capture/scenarios.rs:285`) and run by name.
  Run: `WOW_CAPTURE=mist-edge-noon WOW_CAPTURE_OUT=target/visual/mist/before-noon.png cargo run -q -p benilla`.
- Assumed: a server-less capture streams the map 0 tiles at the eye and builds the curtain, since
  the closed chunk list is client data. No existing scenario shows the mist, so the first run
  proves it.
- Viewpoints for map 0 (edge found from `closed_chunks.json`; heights are guesses): Assumed: eye
  (-9600, 780, ground + 3) looking at (-9600, 900, ground + 15) for straight-on at 120 yd; a
  corner where the edge steps, near x -9533 (y 1033 to 867); a far view from Goldshire
  (-9460, 62) looking west; a close view 10 yd from the wall. Each at minute 720 (noon), 1170
  (dusk) and 0 (night). The ground z comes from the terrain; `tools/twilights-eve/ground_check.py`
  computes it for content points.
- The tram wall cannot be shot server-less: `tram-undersea` renders only the sky dome without a
  server (scenarios.rs:291). It needs a live run with `WOW_LIVE_SHOT`, `WOW_PROBE_GOTO` and
  `WOW_PROBE_CAM` (`capture/live_shot.rs`). Assumed: leave it to the play pass.
- Compare: `scripts/img.ps1 compare -File before.png -Other after.png -Rect ... -Out c.png` lays
  two crops side by side. The capture doc names `scripts/visual.sh` and `benilla-visual`; neither
  exists.
- Every capture is a client build: it takes the build lock and the machine-wide client lock
  (`scripts/client-lock.ps1`), runs in the background (CLAUDE.md), and the ticket must say capture
  runs are allowed.

Ticket order (assuming the recommendations above):

1. Mist shots: add the map 0 `ON_DEMAND` scenarios, capture the before set, attach the PNGs (or
   crops via `img.ps1 shrink`) to the spec issue. Client, no look change. Acceptance: 4 viewpoints
   by 3 times written and attached.
2. Bug fixes: distance fog on the mist, index order by piece, per-piece noise offsets, grey mix,
   placed walls on the WMO fog, the hourly jump. `mist.rs`, `mist.wgsl`. Gate plus the after set.
3. The chosen direction (shells, or the closed terrain rows after an ADR 0015 amendment), with
   corner chamfers. After set at the same viewpoints.
4. Optional: fog pull-in near the wall (lighting resolve), then puffs, then the Hush, each its own
   ticket with its own after set.

Gate for each: `cargo check -q -p benilla-world` and `cargo test -q -p benilla-world --lib mist`,
plus the capture set. The spec closes `unplayed`; Davis's play pass at the Westfall edge and in
the tram tunnel removes it.
