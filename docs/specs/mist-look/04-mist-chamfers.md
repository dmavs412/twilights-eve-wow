Title: Chamfered corners on every mist shell (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 1 and 7. Blocked by 03-mist-shells (the shells, `Segment`'s `ends`,
`inward`, `closed` and `depths`). Client only: `crates/benilla-world/src/mist.rs`.

## Why

The edge follows the 33.3-yd chunk grid, so the mist turns in right angles. On the Westfall edge
it steps up to 167 yd at a time (x -9600, from y 866.7 to 1000) and runs down a one-chunk staircase
from (-9800, 833.3) to (-9933, 733.3). Davis named the straight steps as one of the three things
wrong (decision 1). This ticket cuts every corner with a diagonal across closed land (decision 7).
No shell may stand over open land: the realm's wall is on the chunk grid, and mist on walkable
ground would hide where it stops you.

## The rule

Two new constants:

| Constant | Value | Meaning |
|---|---|---|
| `CHAMFER_YD` | 16.667 (half a chunk) | how far back along each side a corner is cut |
| `FRONT_CHAMFER_YD` | 16.667 | the same for shell 0 at a `ClosedOut` corner; its own constant so the shots can set it to 0 |

Ticket 03 ends shell `k` (depth `d`) at a joint point `J` at each end. At a `ClosedOut` or
`OpenOut` end, cut the corner with legs of length `L`:

- `ClosedOut`: `L = FRONT_CHAMFER_YD` for shell 0, `CHAMFER_YD` for the others. The closed chunk
  sticks out into open land here, so the cut always lies on the closed chunk.
- `OpenOut`: `L = min(CHAMFER_YD, 2*d)`. Open land sticks out into the closed side; a cut deeper
  than twice the shell's depth would cross the open chunk. Shell 0 keeps its sharp corner.
- `Straight`: no cut.
- `L` is at most half the shell's length, so cuts at its two ends never cross.

At the start end, with `u` the unit vector along the segment and `w` the direction the
neighbouring shell leaves `J` (`+inward` at a `ClosedOut` start, `-inward` at an `OpenOut` start):
the shell now begins at `M = J + (u + w) * L / 2`, the middle of the diagonal, then runs to
`P = J + u * L`, then on along the side. The end at `to` is the mirror (`-u`). Each segment owns
half of the diagonal, so two segments built by different tiles still meet at `M` exactly: both
compute it from the same corner by the same rule.

A shell is now `M`, then nine columns evenly from `P` to the far cut point, then `M'`: 11 columns.
`M`'s ground comes from the same bilinear lookup in `closed` as the columns (ticket 03), with `M`
clamped into the closed chunk's square. The foot and top rules of ticket 03 are unchanged.

## What Davis sees

On a one-chunk staircase every segment is 33.3 yd and both its ends turn, so from shell 1 back the
cuts meet in the middle and the shell is one straight diagonal. Shell 0 still steps where open land
sticks out, but the shells behind it run diagonally, and the puffs (ticket 07) hide its foot. At a
large step the corner becomes a 45-degree cut 23.6 yd long.

Known trade-off, Davis to judge from the shots: where a closed chunk sticks out, shell 0's cut
stands up to 11.8 yd behind the realm's wall, so fogged land shows in front of it that a player
cannot walk on. Setting `FRONT_CHAMFER_YD` to 0 puts shell 0 back on the grid there.

## Failing tests first

In mist.rs's `mod tests`:

- `a_closed_out_corner_is_cut_on_the_closed_chunk`: an open chunk with its east and south neighbours
  open and its south-east neighbour closed, built from the two open chunks that border it (the east
  one's south side and the south one's east side): for each shell, the two segments' ends meet at
  the same `M` within 0.01 yd, and `M` is `L / 2` along both sides from `J`.
- `an_open_out_corner_keeps_its_front_and_cuts_its_back`: the open chunk with east, north and
  north-east neighbours closed (ticket 03's mitre case): shell 0's east and north segments still
  meet at the grid corner; shell 4's meet at `M` with `L = CHAMFER_YD`; shell 1's with `L = 16.667`
  (twice 8.333).
- `a_one_chunk_staircase_turns_no_sharper_than_135_degrees`: a staircase of four steps, its
  segments built from all its open chunks: in shells 1 to 4, joined across segments, every turn
  between consecutive pieces is 45 degrees or less (the angle between them 135 or more).
- `no_chamfer_stands_on_open_land`: in all three cases, every vertex is on a closed chunk or within
  0.01 yd of a chunk grid line.
- `a_straight_end_is_not_cut`: ticket 03's straight-edge test still passes with 11-column shells
  whose first and last columns sit on the joint.

Update ticket 03's vertex counts for 11 columns.

## Acceptance

- `cargo test -q -p benilla-world --lib mist 2>&1 | grep -E "FAILED|panicked|test result"` passes.
- `cargo check -q -p benilla-world` and `cargo check -q -p benilla-app` are clean. CLAUDE.md and
  `docs/agents/unattended.md` say how to run client builds.
- What it looks like is ticket 11's and the spec's `unplayed` label's.

Assumptions allowed: whether `M` repeats on a `Straight` end (a duplicated joint column is fine),
how the turn angle is measured in the test. Record them as `Assumed:` lines.
