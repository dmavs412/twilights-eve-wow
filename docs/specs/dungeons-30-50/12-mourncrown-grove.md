Title: Mourncrown over Twilight Grove, gated on a capture (#PARENT)
Labels: ready-for-human

Part of #PARENT, decision 13. Blocked by RD:23-duskwood-45-50 (the last arc ticket to write
`content/quests_duskwood.toml`, which this ticket adds a row to; it also puts the Grey Robe
agents' camp in Twilight Grove, under where the Necropolis will hang) and SO:07-client-opening
(it changes `cut_closed` in `crates/benilla-assets/src/adt.rs`, which this ticket also changes).

**Run this ticket in an interactive Build chat, not overnight.** It needs client capture runs
(`cargo run -q -p benilla` with `WOW_CAPTURE`), which `scripts/overnight-settings.json` does not
allow. The builder of this ticket may run those captures; that is part of the ticket. An
overnight session that picks it up blocks it at once with the comment "needs capture runs; run in
a Build chat" and stops.

## What

Mourncrown, the Necropolis, hangs over Twilight Grove in Duskwood and replaces the Emerald Dream
portal area: the World Tree and the moonwell go, and the Necropolis model floats above the
clearing. It ships only if the capture shows it looks right (below). Nothing here names it to the
player; it is a shape in the sky.

## The game data

- The portal area is two ADT WMO placements on map 0 (`wmo_floor.py`'s placement list for the
  tile at (-10340, -420)):
  - `world\wmo\azeroth\buildings\worldtree\duskworldtree.wmo`, origin (-10316, -422, 44),
    bounds x -10580 to -10087, y -670 to -200 (its floors stand at z 221 to 277);
  - `world\wmo\kalimdor\buildings\nightelfmoonwell\dsnightelfmoonwellornate.wmo`, origin
    (-10317, -492, 51).
- The Necropolis model: `GameObjectDisplayInfo` 6678, `World\Goober\G_Necropolis.m2` (in the
  1.12.1 data, 343 KB). At scale 1.0 it is about 40 by 43 yd and 73 yd tall (bounds z -12.6 to
  61.0 about its origin). vmangos's six main invasion necropolises (entry 181154) stand at scale
  1.0, so this one does too and needs no scale field.

## The client drops the portal area

In `cut_closed` (`crates/benilla-assets/src/adt.rs`, after the `tile.wmos.retain` on closed
chunks, line 329), drop the WMO placements of a short list as well:

```rust
/// ADT WMO placements the Twilight's Eve world does not have (#PARENT): the Emerald Dream portal
/// area in Twilight Grove, where Mourncrown hangs instead. (map, model path in lower case, origin
/// x and y; a placement matches within 2 yd.)
const REMOVED_WMOS: &[(u32, &str, [f32; 2])] = &[
    (0, "world\\wmo\\azeroth\\buildings\\worldtree\\duskworldtree.wmo", [-10316.0, -422.0]),
    (0, "world\\wmo\\kalimdor\\buildings\\nightelfmoonwell\\dsnightelfmoonwellornate.wmo", [-10317.0, -492.0]),
];
```

Use the WMO path and origin fields the tile's `wmos` already carry (read the struct; the
origins above are rounded from the ADT's MODF, so match on distance). A dropped placement draws
nothing and welds no collision, as a closed chunk's do. The ADT's doodads stay.

Mirror the list in `tools/twilights-eve/wmo_floor.py` (`Floors._placed`, line 314), so
`ground_check.py` sees the grove as the client does, with a test in
`tools/twilights-eve/tests/` that neither placement is listed over (-10340, -420) and the terrain
still is.

## The Necropolis row

In `crates/twilight-realm/content/quests_duskwood.toml`, right after the file's header comment
(not at the end), a heading `# ── Mourncrown over Twilight Grove (#PARENT) ──` and one row in the
form of Keane's tents (`quests_elwynn.toml` line 1596):

```toml
[[gameobject]]
guid = 983071
entry = 980711
display = 6678
map = 0
position = [-10340.0, -420.0, 150.0]
grounded = false
orientation = 0.0
respawn = 60.0
```

The point is the grove's centre, with the model's underside about 90 yd above the ground (z 46).
It hangs far out of any character's reach, so nobody can use it, though the cursor may still
show a hand over it. Assumed: that is acceptable.

## The capture gate

Every capture is a client build: it takes the build lock and the machine-wide client lock
(`scripts/client-lock.ps1`) and runs with `run_in_background`, waiting for its notification
(CLAUDE.md). A server-less capture shows no realm GameObject, so the shot seeds the model as a
capture fixture, as the lighting matrix seeds its chest:

- `crates/benilla-app/src/capture/fixtures.rs`: a `SubjectKind::Necropolis` beside
  `SubjectKind::Chest` (line 1332) that spawns a GameObject of display 6678 at the fixture's
  point, scale 1.0.
- `crates/benilla-app/src/capture/scenarios.rs`, `ON_DEMAND` (line 285): three scenarios on
  `MAP_AZEROTH`, each with `ui: Some(UiFixture::Subject { kind: SubjectKind::Necropolis, at:
  [-10340.0, -420.0, 150.0] })`:
  - `mourncrown-near`: eye at the grove's west edge, about (-10520, -560, ground + 3), looking at
    (-10340, -420, 160), minute 1170 (dusk);
  - `mourncrown-under`: eye about (-10400, -470, ground + 3), looking up at (-10340, -420, 180),
    minute 1170;
  - `mourncrown-far`: eye on the road north of Darkshire, about (-10560, -950, ground + 10),
    looking at (-10340, -420, 160), minute 0 (night).
  Take each ground z from `python3.15 tools/twilights-eve/wmo_floor.py <x> <y>` ("terrain z").
- Run each: `WOW_CAPTURE=mourncrown-near WOW_CAPTURE_OUT=target/visual/mourncrown/near.png cargo
  run -q -p benilla` (the form in `crates/benilla-app/src/capture/mod.rs`'s header), and the same
  for `under` and `far`.

Look at each PNG (the Read tool shows images). It looks right when all of these hold:

1. The whole Necropolis is in frame in `near` and `far`: no part clipped by terrain, by the far
   plane or by the fog.
2. It is textured: no magenta, flat white or black faces, and not drawn inside out.
3. No trunk, branch or root of the World Tree and no moonwell is left in the clearing.
4. It hangs clear of the canopy in `near` and reads as a shape in the sky from `far`.

If one fails, change only the row's Z (between 120 and 220) and the fixture's point to match, and
capture again, at most three rounds. If it still fails, ship nothing: commit none of this
ticket, add `blocked`, and comment with the PNG paths and what looks wrong. If it passes, commit
the code, the row and the scenarios, and list the PNG paths (`target/visual/mourncrown/`) in the
closing comment so Davis can open them.

## Failing tests first

- `crates/benilla-assets` (the tests beside `cut_closed`, or a new one in its test module): a
  tile with the two portal-area placements and one other WMO keeps only the other after
  `cut_closed`.
- `crates/twilight-realm/tests/content_quests.rs` (or a new `tests/mourncrown.rs`): the quest
  files hold one GameObject of display 6678 at (-10340, -420), `grounded = false`, on map 0.
- The `wmo_floor.py` test above.

## Acceptance

- The tests pass; `cargo check -q -p benilla-assets` and `cargo check -q -p benilla-app` pass
  (background, as CLAUDE.md says for benilla-app); `cargo test -q -p twilight-realm` passes; the
  pipeline unittests pass; `ground_check.py` flags nothing; the three captures meet the four
  points above.

## Assumptions allowed

- The eye points and minutes moved to frame the model better, each recorded.
- The row's Z, within 120 to 220, as the captures settle it.
