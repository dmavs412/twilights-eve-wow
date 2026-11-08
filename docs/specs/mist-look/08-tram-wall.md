Title: The tram's placed mist wall: the WMO fog's colour, its own band and distance fog (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 8. ADR 0033. Blocked by 07-mist-puffs (the last ticket on `mist.rs`
before this one). Client only: `crates/benilla-world/src/mist.rs`, and
`crates/benilla-world/src/shaders/mist.wgsl` only if a lane needs renaming.

## Why

The Deeprun Tram's tunnel has a placed mist wall (`data/twilights-eve/mist_walls.json`, key
`deeprun_tunnel`, map 369, from (-75, 2400) to (35, 2400), bottom -20, top 30, floor -4.3). Map 369
has no `Light.dbc` row. Its atmosphere is the Subway WMO's own fog, RGB(30, 53, 100), end 236.1 yd
(`crates/benilla-app/src/capture/scenarios.rs`, the `tram-undersea` comment at line 286). The wall
shares the one mist material, and `tint_mist` (line 443 at `6da00cb7`) writes the outdoor scene
fog colour into it, so it is the wrong colour for the tunnel. Its opaque band is the outdoor one,
25 yd (plus or minus 20) over a floor in a tunnel that is far lower, so the whole wall is one flat
opaque rectangle.

Davis took Claude's reading: the same shader, with its own colour from the WMO fog and its own
depths (decision 8). Ticket 03 already gave placed walls their own shell depths (0, 1.5, 3, 4.5 and
6 yd behind the line) and ticket 07 gave them no puffs. This ticket gives them their own material.
Trams must still burst through it: the wall's solid box is on the walk layers only, and nothing
here changes it.

## What to change

- **A second material.** `setup_mist` (line 388) makes two `MistMaterial`s: the closed-zone one as
  now, and a placed-wall one. `Mist` (line 111) holds both. `build_placed_walls` (line 264) gives
  its curtains the placed-wall material.
- **Its own band.** The placed-wall material's `shape` and `rag` and `foot` lanes take:

  | Constant | Value | Closed-zone value | Meaning |
  |---|---|---|---|
  | `WALL_OPAQUE_YD` | 8.0 | `OPAQUE_YD` 25.0 | solid band above the floor |
  | `WALL_RAG_YD` | 3.0 | `RAG_YD` 20.0 | how far the noise moves the fade's start |
  | `WALL_COLOUR_RAMP_YD` | 15.0 | `COLOUR_RAMP_YD` 60.0 | the dark foot returns to the fog colour over this |

  The drift, warp, billows, foot desaturation and brightness, layer drift and noise stay the
  closed-zone values.
- **Its own colour and distance fog.** `tint_mist` writes the placed-wall material from
  `WowLighting::wmo_fog_color`, `wmo_fog_start` and `wmo_fog_end` (`lighting/mod.rs` lines 47 to
  49), the interior fog triple that the WMO crossfade blends toward the claimed WMO's MFOG.
  Ticket 02's `scene` uniform carries start and end. A SO:07 fade factor, if any, stays at 1 on the
  placed-wall material: placed walls never fade.
- **Its height.** Ticket 03 runs a placed wall's shells from the wall's `bottom` to its `top`. Keep
  that; the test below pins it.

## Failing tests first

In mist.rs's `mod tests`:

- `placed_walls_take_their_own_material`: a pure function that builds the two materials' extension
  values (split it out of `setup_mist` if needed) gives the placed-wall one `WALL_OPAQUE_YD`,
  `WALL_RAG_YD` and `WALL_COLOUR_RAMP_YD` in the lanes where the closed-zone one has `OPAQUE_YD`,
  `RAG_YD` and `COLOUR_RAMP_YD`, and the same values everywhere else.
- `the_placed_wall_reads_the_wmo_fog`: a pure function that takes a `WowLighting`'s fog fields and
  returns the two materials' colour and fog uniforms gives the placed-wall one the WMO triple and
  the closed-zone one the scene triple. Build the inputs from plain arrays if `WowLighting` is
  awkward to construct in a test.
- `a_placed_walls_shells_span_the_wall`: `test_wall()` (line 630) gives shells whose top vertices
  stand at 30.0 and whose feet stand at -20.0.

## Acceptance

- `cargo test -q -p benilla-world --lib mist 2>&1 | grep -E "FAILED|panicked|test result"` passes.
- `cargo check -q -p benilla-world` and `cargo check -q -p benilla-app` are clean. CLAUDE.md and
  `docs/agents/unattended.md` say how to run client builds.
- A server-less capture cannot show map 369 (the `tram-undersea` comment says why), so the tunnel's
  look is for Davis's play pass and the spec's `unplayed` label. The numbers above are guesses for
  that pass to judge.

Assumptions allowed: how `Mist` holds the two handles, the pure functions' shapes. Record them as
`Assumed:` lines.
