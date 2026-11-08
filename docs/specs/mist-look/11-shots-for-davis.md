Title: Mist look: take the before and after shots for Davis (#PARENT)
Labels: ready-for-human

Part of #PARENT, decision 9. Blocked by 01-mist-shots, 02-mist-shader, 03-mist-shells,
04-mist-chamfers, 05-rim-terrain, 06-rim-water, 07-mist-puffs, 08-tram-wall, 09-edge-fog and
10-hush. An interactive Build chat in the main checkout runs this with Davis, because it builds and
runs the client, which the night cannot. Davis asked for these capture runs in the grill (decision
9); that is the permission CLAUDE.md wants before a game run.

## Why

Davis judges the new mist from before and after shots at fixed viewpoints, approves them, then
plays it once (decision 9). The night builders could neither run the captures nor compile WGSL, so
this ticket is also the first time the shaders from tickets 02, 05, 06 and 08 compile.

## Before you start

- The script is `scripts/mist-shots.ps1` (ticket 01). It takes the build lock and the machine-wide
  client lock itself, as `scripts/publish-play.ps1` does, and waits up to 60 minutes for the client
  lock. One client build runs at a time on this machine; if a night run is mid-ticket, expect to
  wait or run this when the builders are idle.
- Each run builds the client, then takes 18 shots. It passes the Bash tool's 10-minute cap, so
  start it with `run_in_background` and wait for its notification (CLAUDE.md, "Wait for every
  command to finish"). Never stop it mid-compile (CLAUDE.md says why).
- The tree must be clean before switching commits; commit or stash your own edits first.

## Steps

1. **The before set.** Find ticket 01's last commit on `main` (its closing comment lists the
   shas). Tickets 02, 05 and 09, the first look change in each chain, were blocked by 01, so that
   commit has the viewpoints and none of the new look. `git switch --detach <sha>`, then
   `powershell -NoProfile -File scripts/mist-shots.ps1 -Label before`, then `git switch main`. Skip
   this step if `target/visual/mist/before/` already holds all 18 PNGs from such a commit.
2. **The after set.** On `main`: `powershell -NoProfile -File scripts/mist-shots.ps1 -Label after`.
3. **Fix what breaks.** If the client logs a shader error or a shot is missing, fix it here (this is
   the Build lane), commit with the check that ran, and repeat step 2. If `scripts/mist-shots.ps1`
   itself fails, fix it the same way. Read a shot with the Read tool before showing Davis, to see
   that the mist is in frame.
4. **Side by side.** `powershell -NoProfile -File scripts/mist-shots.ps1 -Compare` writes a
   before-and-after JPEG per shot under `target/visual/mist/compare/`.
5. **Show Davis.** Publish one private Artifact page with the 18 pairs, grouped by view (straight,
   step, stair, close, far, steppes) with noon, dusk and night side by side, each captioned with what
   it frames (ticket 01's table). Give Davis the link here and in a comment on this issue.
6. **Tune with Davis.** Davis says what to change. Tunables to reach for first, all `pub const`:
   - `mist.rs`: `SHELL_DEPTHS_YD`, `SHELL_CAPS`, `SHELL_TOP_YD`, `OPAQUE_YD`, `RAG_YD`, `BAND_FLOOR`,
     `BILLOW_LIGHT`, `DRIFT_YD_S` (keep `DRIFT_UP_YD_S` at half of it and `LAYER_DRIFT` in multiples
     of 0.5), `FOOT_DESAT`, `FOOT_VALUE`, `COLOUR_RAMP_YD`, `CHAMFER_YD`, `FRONT_CHAMFER_YD` (0 if
     fogged land shows in front of a chamfered corner, the trade-off ticket 04 names).
   - `mist_puffs.rs`: `PUFF_SIZE_YD`, `PUFFS_PER_SEGMENT`, `PUFF_ALPHA`, `PUFF_DEPTH_YD`.
   - `benilla-assets` `adt.rs`: `RIM_FOG_YD`.
   - `lighting/edge.rs`: `PULL_FOG_START_YD`, `PULL_FOG_END_YD`, `PULL_GREY`, `EDGE_NEAR_YD`.

   Each round: edit, `cargo check`, the touched crate's tests, re-shoot the after set (`-Only` for the
   views that matter), `-Compare`, republish the same Artifact. Commit each change Davis keeps as
   "Mist look: <what changed> (#this issue)" with the check that ran. A change beyond tunables
   (new geometry, a new layer) is a new ticket under #PARENT, not this one.
7. **Done.** Davis approves the shots in a comment on this issue or in the chat. Post the final
   Artifact link and the tunables' final values as a comment, close this issue, and close #PARENT
   with the `unplayed` label (its other tickets are closed by then). Then, if Davis asks,
   `scripts/publish-play.ps1` for his play pass.

## What the play pass covers

Not part of this ticket; for the note that removes `unplayed`. Davis walks up to the Westfall edge
(the river stretch south-west of Goldshire, around (-9700, 760)), the Burning Steppes border at
Blackrock Pass (-8735, -2590) while it is still closed, and the Deeprun Tram tunnel's wall, whose
colour, band and depths (ticket 08) no capture could show. He listens for the ambience fading near
the edge (ticket 10). If the Steppes have opened on his test server by then, he watches the mist
fade with its shells and puffs and the rim land pop clear when its tile reloads (the parent's
SO:07 section).

## Acceptance

- `target/visual/mist/before/` and `after/` each hold the 18 shots, and the comparisons are in an
  Artifact linked from this issue.
- Davis's approval is in a comment.
- Any fix or tuning made along the way is committed on `main` with the check that ran.
