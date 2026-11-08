Title: Drifting puffs along the foot of the mist (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 4. Blocked by 04-mist-chamfers (the last ticket on `mist.rs` before this
one). Client only: `crates/benilla-world/src/mist.rs`, a new
`crates/benilla-world/src/mist_puffs.rs`, and its `mod` line beside `pub mod mist;` in
`crates/benilla-world/src/lib.rs` (line 129). `MistPlugin` registers its systems.

## Why

Davis asked for slow drifting puffs along the foot of the cloud bank, so it churns (decision 4).
Ticket 03 removed the ground skirt and the wisp sheets; the puffs take their place at the foot,
where the shells meet the ground. Soft round silhouettes are what make the bank read as cloud
rather than as a layered wall.

## What the puffs are

Camera-facing quads drawn through the world effect stream (`crate::particles`, `EffectQuads` and
`EffectDrawSpec` in `particles/buffer.rs` line 207), with the stock texture
`textures/weather/snowmist01.blp`. Weather's mist puffs are the model: `push_mist` in
`crates/benilla-world/src/weather/precip/mist.rs` builds their quads, and
`crates/benilla-world/src/weather/precip/mod.rs` loads the texture (line 221) and commits the draw
(lines 456 to 467).

Constants, `pub const` with doc comments in `mist_puffs.rs`:

| Constant | Value | Meaning |
|---|---|---|
| `PUFF_RANGE_YD` | 160.0 | segments whose middle is this close to the camera keep puffs |
| `PUFFS_PER_SEGMENT` | 3 | puffs alive per 33-yd segment |
| `PUFF_CAP` | 256 | most puffs alive at once; the nearest segments win |
| `PUFF_SIZE_YD` | 14.0 to 28.0 | quad side |
| `PUFF_DEPTH_YD` | -3.0 to 10.0 | centre past the edge line along `inward`; negative is over open ground |
| `PUFF_LIFT` | 0.35 | centre height above the ground, as a share of the size |
| `PUFF_DRIFT_YD_S` | 0.4 to 1.0 | speed along the segment, either way |
| `PUFF_RISE_YD_S` | 0.15 | upward speed |
| `PUFF_SPIN_DEG_S` | -6.0 to 6.0 | turn in the screen plane |
| `PUFF_LIFE_S` | 25.0 to 45.0 | life |
| `PUFF_FADE_IN_S` / `PUFF_FADE_OUT_S` | 6.0 / 8.0 | alpha ramps at birth and death |
| `PUFF_ALPHA` | 0.45 | peak alpha |
| `PUFF_NEAR_YD` | 3.0 to 10.0 | alpha rises from 0 to full over this distance from the camera |

Ranges are uniform draws.

## Behaviour

- **Which curtains.** Every closed-zone curtain. Placed walls (`mist_walls.json`, ADR 0033) get
  none; a puff is wider than the tram tunnel.
- **Segments.** The curtain entity keeps its `Vec<Segment>` in a component (reuse one if
  SO:07-client-opening added it), so the puff system knows each segment's line, `inward` and
  ground (`rows[0]`, or `closed` from ticket 03).
- **Spawning.** Each segment within range keeps `PUFFS_PER_SEGMENT` alive. A dead puff is replaced
  at a new spot on the same segment.
- **Settled at once.** A puff spawned because its segment just came into range (or the curtain just
  loaded) starts at a random age in its life, not at 0, so a freshly loaded edge and a capture's
  first frames already look settled.
- **Repeatable.** All draws come from a seeded hash of the segment's `from` point, the slot (0 to
  2) and how many times that slot has respawned. No `thread_rng`, so a capture repeats.
- **Motion.** Each frame `pos += dt * (along * drift + up * rise)`; a puff that drifts past its
  segment's end keeps going (the next segment's puffs overlap it).
- **Colour.** The mist's own rule on the CPU: the foot colour (`fog` desaturated by `FOOT_DESAT`,
  times `FOOT_VALUE`) mixed toward the fog colour by the puff's height above the ground over
  `COLOUR_RAMP_YD`, from `WowLighting::fog_color`.
- **Alpha.** `PUFF_ALPHA` times the life ramps, times the camera ramp, times the curtain's fade.
- **A curtain that fades.** SO:07 fades a curtain over `OPEN_FADE_SECS` when its zone opens. Its
  puffs take the same fade factor, read where SO:07 stores it, and no new ones spawn; when the
  curtain despawns its puffs go.
- **Drawing.** One `commit_quads` draw a frame, the puffs sorted back to front by distance to the
  camera, `EffectBlend::Alpha`, `EffectFog::Scene` (distance fog, like the mist after ticket 02).
  The puffs stand in front of the shells, so they must draw after the curtain: check how the
  curtain entity sorts in `Transparent3d` (its `Transform` is the identity, the mesh in world
  coordinates) and set the draw's `anchor` and `bias` so the puffs sort nearer. Record what you
  found as an `Assumed:` line.
- **Schedule.** The simulation in `Update` after the curtains are built; the push in `PostUpdate`
  after `begin_effect_frame`, as `push_precip` does (`weather/precip/mod.rs` line 483).

Keep the simulation pure where you can (spawn, step, alpha and the sort as plain functions over
plain data), so the tests below need no app.

## Failing tests first

In `mist_puffs.rs`:

- `the_same_seed_gives_the_same_puffs`: two spawns for the same segment and slot match exactly;
  another slot differs.
- `puffs_stay_on_their_segment`: 1,000 spawns on one segment all lie within its span along `u`
  (plus half the largest size) and within `PUFF_DEPTH_YD` along `inward`.
- `alpha_rises_holds_and_falls`: alpha is 0 at birth, `PUFF_ALPHA` mid-life (camera far away), 0 at
  death; 0 at the camera, full at `PUFF_NEAR_YD`'s far end.
- `a_new_segment_starts_settled`: puffs spawned for a segment entering range have ages spread across
  their lives, not all 0.
- `the_cap_keeps_the_nearest`: 200 segments in range give `PUFF_CAP` puffs, all on the nearest
  segments.
- `a_fading_curtain_fades_its_puffs_and_spawns_none`: at fade 0.5 a puff's alpha halves; a dead puff
  on that curtain is not replaced.
- `placed_walls_have_no_puffs`.

## Acceptance

- `cargo test -q -p benilla-world --lib mist 2>&1 | grep -E "FAILED|panicked|test result"` passes.
- `cargo check -q -p benilla-world` and `cargo check -q -p benilla-app` are clean. CLAUDE.md and
  `docs/agents/unattended.md` say how to run client builds.
- What it looks like is ticket 11's and the spec's `unplayed` label's.

Assumptions allowed: the hash, the component's name, the anchor and bias. Record them as
`Assumed:` lines.
