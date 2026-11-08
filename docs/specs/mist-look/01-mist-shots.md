Title: Mist look: capture viewpoints at the Westfall and Steppes edges, and a script that takes them (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 9. No blocker. Client and `scripts/` only; no look change.

## Why

Davis judges the new mist from before and after shots at fixed viewpoints (decision 9). No
scenario shows the mist today. This ticket adds the viewpoints and a script that takes them all.
Ticket 11, run by hand later, runs the script. You cannot run it: the night denies PowerShell and
game runs.

The look tickets that start each chain (02, 05, 09) are blocked by this one, so this ticket's last
commit is the build ticket 11 uses for the "before" set. Change nothing that draws.

## The scenarios

Add 18 rows to `ON_DEMAND` in `crates/benilla-app/src/capture/scenarios.rs` (line 285), at the end of
the table, under a comment `// ---- The mist at the Closed Zones' edge (spec #PARENT) ----`.
Each is `map: Some(MAP_AZEROTH)`, `ui: None`. Name each `mist-<view>-<time>`, where time is `noon`
(minute 720), `dusk` (1170) or `night` (0), so every view has three rows. Put the eye and look of
each view in `pub(super) const` pairs beside `SCAR_EYE` (line 932), each with a doc comment saying
what it frames, as the other pairs do.

Raw WoW coordinates `(x, y, z)`. Each z is 2.5 to 8 yd over the ground or water there, from
`wowmpq.Terrain.ground_z` and `water_z` (`tools/twilights-eve/wowmpq.py`); every sight line was
checked against the terrain at 60 steps.

| View | Eye | Look | What it frames |
|---|---|---|---|
| `straight` | (-9700.0, 760.0, 34.1) | (-9700.0, 880.0, 35.4) | the straight river stretch of the Westfall edge on y 866.7, 107 yd away, x -9600 to -9800 |
| `step` | (-9680.0, 820.0, 29.1) | (-9600.0, 880.0, 30.0) | the step at x -9600: the edge turns from y 866.7 north to y 1000; the closed chunk's corner at (-9600, 866.7) faces the eye |
| `stair` | (-9800.0, 700.0, 34.8) | (-9880.0, 790.0, 33.2) | the one-chunk staircase from (-9800, 833.3) down to (-9933, 733.3) |
| `close` | (-9980.0, 718.0, 24.3) | (-9980.0, 760.0, 28.0) | 15 yd from the wall on y 733.3, at the river's edge |
| `far` | (-9560.0, 600.0, 55.1) | (-9680.0, 900.0, 40.7) | the straight stretch and the step from about 280 yd |
| `steppes` | (-8760.0, -2590.0, 134.9) | (-8640.0, -2590.0, 157.3) | the Burning Steppes border on x -8666.7 from the Blackrock Pass camp, 93 yd away |

The Steppes border exists once SO:02-gated-closed-zones (#628) has put zone 46 in
`closed_chunks.json`. Before that the `steppes` rows show open land, which is fine: add them now.
A capture runs with no server, so no Milestone is reached and the border stays closed in a shot
even after SO:07-client-opening (#633).

## The script

New `scripts/mist-shots.ps1`, for Windows PowerShell 5.1. Header comment: what it does, the three
ways to run it below, and that it builds the client in this checkout's `target`.

```text
powershell -NoProfile -File scripts/mist-shots.ps1 -Label before
powershell -NoProfile -File scripts/mist-shots.ps1 -Label after -Only mist-close-*
powershell -NoProfile -File scripts/mist-shots.ps1 -Compare
```

- `$Shots` at the top: the 18 scenario names, one per line, in table order (view, then noon, dusk,
  night). `WOW_CAPTURE=list` prints only the golden scenarios, not `ON_DEMAND`, so the list lives
  here, and the test below keeps it in step with the Rust table.
- `-Label <word>`: take the build lock `target\.claude-build.lock` and the machine-wide client lock
  exactly as `scripts/publish-play.ps1` does (lines 20 to 45: refuse when another session holds the
  build lock; `Enter-ClientLock` with a 60-minute wait; release both in a `finally`). Run
  `cargo build -q -p benilla` once, then for each shot set `$env:WOW_CAPTURE` to its name and
  `$env:WOW_CAPTURE_OUT` to `target\visual\mist\<Label>\<name>.png` and run
  `cargo run -q -p benilla`. Print one line per shot, written or missing. Make the folder first.
- `-Only <wildcard>`: limit the shots to names that match (`-like`).
- `-Compare`: no cargo and no locks. For each name with a PNG in both `target\visual\mist\before`
  and `target\visual\mist\after`, read the image size with `System.Drawing`, run
  `scripts/img.ps1 compare` over the whole frame into `target\visual\mist\compare\<name>.png`, then
  `scripts/img.ps1 shrink -Width 1600` of that into `<name>.jpg`, small enough to upload.
- Exit 1 when any shot is missing. Do not set `$ErrorActionPreference = 'Stop'`
  (`publish-play.ps1` line 15 says why).

## Failing tests first

In `crates/benilla-app/src/capture/scenarios.rs`, a new `#[cfg(test)] mod mist_tests`:

- `every_mist_view_has_noon_dusk_and_night`: the `mist-` rows are 18, six views by three times, each
  at minute 720, 1170 or 0 by its suffix, on map 0, with no UI.
- `mist_eyes_stand_on_open_land`: no `mist-` eye is on a closed chunk
  (`benilla_formats::closed_zones::closed_at`).
- `mist_looks_face_the_edge`: every Westfall view's look point has a closed chunk within 60 yd
  (`closed_chunks_near(0, x, y, 60.0)` is not empty). The `steppes` view checks the same only when
  `zone_closed(0, 46)` is true, so the test passes before and after SO:02 lands.
- `every_mist_scenario_is_in_the_shot_script`: `include_str!("../../../../scripts/mist-shots.ps1")`;
  every `mist-` scenario name appears in it, and every word in it that starts with `mist-` and ends
  in `-noon`, `-dusk` or `-night` names a scenario. Split the text on characters outside `a-z`,
  `0-9` and `-`; no regex crate.

If SO:07-client-opening has landed, these readers take a reached set: pass an empty one.

## Acceptance

- `cargo test -q -p benilla-app --lib mist_tests 2>&1 | grep -E "FAILED|panicked|test result"`
  passes. CLAUDE.md and `docs/agents/unattended.md` say how to run a client build.
- `cargo check -q -p benilla-app` is clean.
- `scripts/mist-shots.ps1` exists as described. Nobody runs it in this ticket; ticket 11 runs it
  and fixes it if it fails.

Assumptions allowed: the constant names, the comment wording, and the script's messages. Record
them as `Assumed:` lines.
