Title: Five shells into the closed side replace the backdrop, the wisps and the skirt (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 2 and 4. Blocked by 02-mist-shader (the layer number, per-layer noise
and draw order this ticket builds on). Client only: `crates/benilla-world/src/mist.rs`, and
`crates/benilla-world/src/shaders/mist.wgsl` only if a comment there names the old pieces.

## Why

Today each segment has a solid backdrop on the edge line, two half-alpha wisp sheets 4.2 and 8.3
yd in front of it on open ground, and a ground skirt. The wall has no inside: behind the backdrop
is a flat field of fog colour. Davis wants a rolling cloud bank built from layered shells reaching
into the closed side (decision 4). Five shells stand one behind the other, the front ones thin and
the back one solid, each with its own noise and drift from ticket 02, so they slide past each other.
The puffs in ticket 07 take the skirt's job at the foot; this ticket removes the wisps and the
skirt.

Corners here join with plain mitres, so the shells meet without gaps. Ticket 04 then cuts the
corners.

## The shells

Five shells per segment, numbered 0 (front) to 4 (back). Shell `k` stands `SHELL_DEPTHS_YD[k]` past
the edge line, into the closed chunk.

| Constant | Value | Meaning |
|---|---|---|
| `SHELL_DEPTHS_YD` | [0.0, 8.333, 16.667, 25.0, 33.333] | yards past the edge: the closed chunk's outer vertex rows 0, 2, 4, 6 and 8 (`CHUNK_SIZE / 8` apart) |
| `SHELL_CAPS` | [0.25, 0.35, 0.5, 0.7, 1.0] | each shell's alpha ceiling (`UV_1.x`) |
| `SHELL_TOP_YD` | [35.0, 55.0, 75.0, 95.0, 120.0] | each shell's top above the highest ground under it; the last is today's `ABOVE_YD` |
| `SHELL_SINK_YD` | 4.0 | shells 0 to 3 reach this far under the ground at each column |
| `WALL_SHELL_DEPTHS_YD` | [0.0, 1.5, 3.0, 4.5, 6.0] | a placed wall's shells, behind its line (away from `open_side`) |

The layer number (ticket 02's `ATTRIBUTE_COLOR.x`) of shell `k` is `k`, so it takes
`LAYER_DRIFT[k]` and its own noise offset. The index buffer runs shell 4 of every segment first,
then shell 3, down to shell 0 last, which is back to front from the open side.

### What a segment carries

`segments()` (mist.rs line 140) has the closed predicate and the tile's chunks, so it records on
each `Segment` everything a shell needs. A curtain built from any subset of segments (SO:07's
fading copy, #633) must get its shells from the `Segment` values alone. Add:

- `inward: [f32; 2]`: the unit vector from the edge line into the closed chunk.
- `ends: [Corner; 2]`, at `from` and at `to`, where
  `pub(crate) enum Corner { Straight, ClosedOut, OpenOut }`.
- `closed: Option<[[[f32; 3]; 9]; 9]>`: the closed neighbour's 9 outer vertex rows, row 0 on the
  shared side and row 8 on its far side, each ordered from `from` to `to`. `cut_closed` clears a
  closed chunk's indices but keeps its 145 positions, so the heights are there when the closed
  chunk is in the same tile's `chunks`. `None` when it is in another tile.
- `depths: [f32; 5]`: `SHELL_DEPTHS_YD` for a chunk segment, `WALL_SHELL_DEPTHS_YD` for a placed
  wall (`wall_segment`, line 223, which also sets `inward` away from `open_side`, both ends
  `Straight` and `closed` to `None`).

`rows` (the open chunk's rows 0 to 2) can stay for the fallback heights and the puffs; drop rows 1
and 2 if nothing reads them once the wisps go.

### Corner types

Look along the segment past one end, from the end point outward. Two chunks sit beyond it: the one
on the open side and the one on the closed side. The closed predicate decides each:

- open side open, closed side closed: `Straight`; the edge runs on in line.
- both open: `ClosedOut`; the closed chunk ends here and the edge turns into the closed side.
- both closed: `OpenOut`; the edge turns toward the open side.
- open side closed, closed side open (two closed chunks meet only at the corner): `ClosedOut`.

A chunk off the 1024 grid counts as open, as in `segments` today.

### Where shell k runs

With `u` the unit vector from `from` to `to`, `n = inward` and `d = depths[k]`, shell `k` is the
segment moved `d` along `n`, its ends moved along `u` by the corner rule:

- start = `from + n*d + u*(s*d)` and end = `to + n*d - u*(e*d)`, where `s` (and `e`) is +1 for
  `ClosedOut`, 0 for `Straight` and -1 for `OpenOut`.

So at a `ClosedOut` corner both shells stop at the same point `d` inside the closed chunk on both
axes, and at an `OpenOut` corner both run on to the mitre point. A shell shorter than 1 yd is left
out (a lone closed chunk's back shells meet in its middle).

Nine columns run evenly from start to end.

### Heights

The ground under a column is bilinear in `closed`'s 9 by 9 grid at the column's point, the point
clamped into the closed chunk's square. With `closed` `None`, use `rows[0]`'s height at the nearest
column along the segment.

- Shell 4 (the back) seals the cut, as the backdrop did: every column's foot is the lowest ground
  under the whole shell minus `BELOW_YD` (10).
- Shells 0 to 3: each column's foot is its own ground minus `SHELL_SINK_YD`.
- Every shell's top is the highest ground under it plus `SHELL_TOP_YD[k]`.
- A placed wall's shells all run from the wall's `bottom` to its `top`, as its backdrop does today:
  the tunnel's roof and floor hide the rest.

`UV_0` and `UV_1` keep their meaning (yards above the column's ground and the top's; the cap and
the yards above the piece's own foot), so the shader's height band and foot fade work unchanged.
`CAMERA_FADE_YD` still fades every piece whose cap is under 0.99, now shells 0 to 3.

### What goes

`WISP_CAP`, `SKIRT_CAP`, `SKIRT_RAISE_YD`, the wisp and skirt code in `curtain_mesh` (lines 359 to
372) and their tests. Update the module's and the shader's header comments, and the doc comments
on `Segment` and `curtain_mesh`, to describe shells.

## Failing tests first

In mist.rs's `mod tests`, with the existing `chunk(gx, gy)` helper:

- `shells_stand_at_their_depths_with_their_caps`: one open chunk whose south, south-west and
  south-east neighbours are closed, so both ends are `Straight`, with the south one in `chunks`:
  five shells, 18 vertices each, at depths 0, 8.33, 16.67, 25 and 33.33 yd past
  the side; `UV_1.x` is `SHELL_CAPS[k]` and `ATTRIBUTE_COLOR.x` is `k`; the first 48 indices are
  shell 4's.
- `shells_of_a_straight_edge_meet_end_to_end`: two open chunks side by side, both with closed
  chunks to the south: each shell of the first segment ends where the same shell of the second
  begins (within 0.01 yd).
- `closed_out_shells_stop_inside_the_closed_chunk`: an open chunk with only its east neighbour
  closed: both ends are `ClosedOut`, shell `k` runs from `d` to `33.33 - d` along the side, and
  shells 2 to 4 (d of 16.67 yd and more) are left out.
- `open_out_shells_meet_at_the_mitre`: an open chunk whose east, north and north-east neighbours are
  closed: the east and north segments' shells `k` share the end point corner + `d` along each
  inward normal.
- `no_shell_stands_on_open_land`: in the last three cases and a one-chunk staircase, every shell
  vertex's `(x, y)` is on a closed chunk or within 0.01 yd of a chunk grid line.
- `shell_heights_come_from_the_closed_chunk`: with the closed chunk in `chunks`, a shell 4 column
  stands on its row-8 heights; with it absent, on the open chunk's row 0.
- `a_placed_walls_shells_step_away_from_its_open_side`: `test_wall()` (line 630) gives shells at 0,
  1.5, 3, 4.5 and 6 yd on the side away from `open_side`.

## Acceptance

- `cargo test -q -p benilla-world --lib mist 2>&1 | grep -E "FAILED|panicked|test result"` passes.
- `cargo check -q -p benilla-world` and `cargo check -q -p benilla-app` are clean. CLAUDE.md and
  `docs/agents/unattended.md` say how to run client builds.
- What it looks like is ticket 11's and the spec's `unplayed` label's.

Assumptions allowed: how `Segment` stores the new fields, the bilinear helper's shape, whether
`rows` keeps rows 1 and 2. Record them as `Assumed:` lines.
