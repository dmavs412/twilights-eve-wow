Title: Mist look: a rolling grey cloud bank at the edge of the Closed Zones (spec)
Labels:

Grilled with Davis on 2026-11-08 from `docs/specs/prep/mist-look.md`. The grill's answers (the
prep's last section) are below and override the prep wherever they differ. The prep's survey of
the code (sections 1 and 2) is still the map of the mist: every file, line and tunable the tickets
name comes from it, checked again at `6da00cb7`. ADR: this spec amends ADR 0015; the draft is
`adr-0015-amendment.md` here, and ticket 05 appends it to `docs/adr/0015-closed-zones.md`. The
lore bible (`docs/lore-bible.md`, "The mist and the Anchors") wins over any line here: the mist is
a dead grey mass that pools on the ground, roads end in grey, and sound stops (the Hush).

## What Davis sees when it lands

Walking west toward Westfall, the land runs on past the edge for one chunk and sinks into grey.
The mist is a deep bank of cloud, not a sheet: five layers stand one behind the other, the front
ones thin and the back one solid, each moving its own way, so the front churns against the back.
Big soft puffs roll slowly along the foot. The steps of the chunk grid are cut off at the corners.
Within about 60 yards of the edge the air itself closes in and goes grey, and the zone's ambient
sound fades toward silence. The tram tunnel's wall is the same cloud in the tunnel's own blue.

## Decisions (grill of 2026-11-08)

Davis answered each. Those marked Claude were Claude's recommendations or readings that Davis
took.

1. **What is wrong now** (Davis). It reads as a flat painted wall, it follows the chunk grid in
   straight steps, and it is dead, with no motion or depth. The colour is not a complaint.
2. **The picture** (Davis). A rolling grey cloud bank.
3. **Land inside the mist** (Davis). One row of closed chunks (33.3 yd) is drawn inside the mist
   and fogged to full mist. This amends ADR 0015. The realm's wall still refuses the walk.
4. **What replaces the flat backdrop** (Davis). Layered shells reaching into the closed side, plus
   slow drifting puffs along the foot so it churns.
5. **Colour** (Claude). The scene fog colour, as now.
6. **The approach** (Davis). Within about 60 yd of the edge the scene fog pulls in and greys, and
   the ambient sound fades (the Hush). The sound is its own ticket.
7. **The steps** (Claude, following answer 1). Chamfer each corner across the closed chunk; more if
   the shots still show steps.
8. **The tram wall** (Claude). The same shader, with its own colour from the WMO fog and its own
   depths. Trams still burst through it (ADR 0033).
9. **Judging** (Davis). The builder takes before and after `WOW_CAPTURE` shots at fixed viewpoints
   and times, Davis approves from the shots, then one play pass.
10. **Maps** (Claude). Out of scope; the colour does not change.

### Readings taken while writing the tickets (Claude)

Each is stated in its ticket. Davis can change any before filing.

- **The shells replace all four pieces.** Five shells replace the backdrop, the two wisp sheets
  and the ground skirt. The shells stand on the closed chunk's vertex rows 0, 2, 4, 6 and 8, so
  0, 8.3, 16.7, 25 and 33.3 yd past the edge, with alpha ceilings 0.25, 0.35, 0.5, 0.7 and 1.0 and
  tops 35, 55, 75, 95 and 120 yd above their ground. The back shell is solid and seals the cut, as
  the backdrop does today. The puffs take the skirt's job at the foot.
- **Chamfers only ever cross closed land.** Where a closed chunk sticks out into open land, every
  shell cuts the corner 16.7 yd back along each side. Where open land sticks out into the closed
  side, the front shell keeps its corner and each deeper shell cuts its own, up to twice its depth,
  which is the most it can cut without standing over open land. A one-chunk staircase becomes a
  straight diagonal from the second shell back. Trade-off: at a corner where a closed chunk sticks
  out, the front shell stands up to 11.8 yd behind the realm's wall, so a player can see fogged
  land there that they cannot walk on. The front shell's chamfer is its own constant
  (`FRONT_CHAMFER_YD`) so ticket 11 can set it to 0 if the shots show it.
- **The rim row.** A rim chunk is a closed chunk with an open chunk among its eight neighbours.
  Its terrain draws and fades toward the mist's foot colour by its distance past the edge, pure
  mist at 30 yd. It grows no clutter, keeps no doodad or WMO, stays impassable and keeps its
  fence. Its horizon cell stays cut.
- **The rim row draws its water too**, fogged the same way. The Elwynn and Westfall border runs
  down a river (water at z 21.2 to 21.4 from y 840 to 880 along x -9640 to -9900), and a river that
  stops at a straight line inside fogged land shows the cut that the row exists to hide. Its own
  ticket (06), after the terrain.
- **The noise moves more.** Sideways drift 1.2 yd/s (now 0.4) and upward 0.6 (now 0.3), each shell
  at its own multiple, alternate shells the opposite way, so the layers slide past each other.
  Billows lighten and darken the colour by up to 15%. The noise is gradient noise and wraps every
  5,888 yd, so the hourly jump goes.
- **The mist takes distance fog**, as the terrain does, so the dark foot fades into the haze far
  away.
- **Puffs** are camera-facing quads drawn through the effect stream with the stock
  `textures/weather/snowmist01.blp`, 14 to 28 yd across, 3 alive per 33-yd segment within 160 yd of
  the camera, at most 256, living 25 to 45 s, seeded so a capture repeats, and born at a random age
  so a fresh edge looks settled. Placed walls get none: a puff is wider than the tunnel.
- **The tram wall** takes shells 0, 1.5, 3, 4.5 and 6 yd behind its line, an opaque band about 8
  yd over the floor and its colour and distance fog from `WowLighting::wmo_fog_color`,
  `wmo_fog_start` and `wmo_fog_end`. Its numbers are guesses for play to judge, since a server-less
  capture cannot show map 369 (prep section 5).
- **The fog pull-in.** At the edge the scene fog runs from 20 to 200 yd (Elwynn by day is 125 to
  500) and its colour is 60% of the way to its own grey. The pull fades out by 60 yd from the
  nearest closed chunk, eases at most 0.25 a second so a zone that opens or a teleport does not
  snap it, and greys the sky dome's colours by the same amount so the mist's top still meets the
  sky. Underwater it does nothing; the underwater fog owns the screen there.
- **The Hush** fades the ambience bed to 15% of its volume at the edge, by the same curve and ease.
  Music, footsteps, creature and spell sounds and positional emitters keep their volume.
- **Viewpoints.** Five on the Westfall edge (the straight river stretch, the step at x -9600, the
  one-chunk staircase, 15 yd from the wall, and 280 yd out) and one at the Burning Steppes border
  from the Blackrock Pass camp, each at noon, dusk and night: 18 shots. Every eye and look was
  checked for a clear sight line against the terrain heights. The tram wall is left to the play
  pass.
- **Before and after.** The look tickets that start each chain (02, 05, 09) wait on ticket 01, so
  ticket 01's last commit is a "before" build with the viewpoints and none of the new look.
  Ticket 11 builds that commit for the before set and `main` for the after set.

## How it fits the Burning Steppes opening (#626)

The Steppes spec adds a Closed Zone that a Milestone opens at run time. Its client ticket
SO:07-client-opening (#633) gives every closed-zone reader a reached set, fades a curtain over
`OPEN_FADE_SECS` (6.0) with a fade factor in `MistExt`, despawns it, and reloads the tiles behind
it. This spec builds on top of that and waits for it wherever it edits the same code:

- **Shells fade with their curtain.** Every shell is part of the curtain mesh, built by
  `curtain_mesh` from `Segment` values alone. Everything a shell needs (the corner types, the
  closed chunk's heights, the depths) lives in `Segment`, so a curtain built from any subset of
  segments, such as SO:07's fading copy, has its shells and fades as one.
- **Puffs fade with their curtain.** A puff belongs to a segment of a curtain. While the curtain
  fades, its puffs take the same fade and no new ones spawn; when it despawns, they go.
- **The rim row follows the reached set.** The rim is decided in `cut_closed` with the same
  reached set the cut uses. When a zone opens, its tiles reload and the old rim chunks come back as
  ordinary open land. The fogged rim pops clear when its tile reloads, partway through the 6-second
  fade. That is accepted here and judged in play; a per-tile fade would need its own ticket.
- **New borders** appear with the reloaded tiles, with their rim, shells and puffs, and no fade in,
  as SO:07 already does for curtains.
- **The fog pull-in and the Hush** read the reached set, so near a zone that opens they ease away
  over about 4 s while the mist fades.
- **Captures** run with no server, so no Milestone is reached and the Steppes border is always
  closed in a shot.

Tickets 02 (mist shader), 05 (rim terrain) and 09 (edge fog) say "Blocked by
SO:07-client-opening": 02 edits `MistExt`, `mist.rs` and `mist.wgsl`; 05 edits `cut_closed`,
`terrain_stream.rs` and `benilla-assets/src/lib.rs`; 09 adds a reader to `closed_zones.rs`. Every
other code ticket follows one of those.

## Scope

- Capture viewpoints at the Westfall and Steppes edges and `scripts/mist-shots.ps1`.
- The mist shader: per-layer noise, gradient noise that wraps, billows, faster drift, distance fog,
  draw order by layer.
- Five shells into the closed side, and chamfered corners.
- The rim row's terrain and water, fogged into the mist; the ADR 0015 amendment.
- Drifting puffs along the foot.
- The tram wall's own colour, depths and band.
- The scene fog pulling in and greying near the edge.
- The ambience bed fading near the edge.
- The before and after shots and Davis's approval.

## Out of scope

- **Maps** (decision 10): the minimap's closed squares and the world map's mist art stay as they
  are.
- **A new colour** (decision 5): no fixed mist grey. The fog pull-in greys the scene fog near the
  edge, and the mist follows the scene fog, as it always has.
- **A depth prepass** or soft intersection with any geometry (prep, direction B).
- **Mist that rolls back over a zone** (ADR 0034's open consequence).
- **Opening and closing zones**: SO:07 (#633) does the run-time opening and the 6-second fade.
- **A WGSL validation test.** The gate cannot compile WGSL (prep section 1), so a broken shader
  shows only when ticket 11 runs the captures. Ticket 11 fixes any shader error first.
- **Sound beyond the ambience bed**: music, emitters and creature sounds.

## Child tickets

Build order. "Blocked by" names a file in this folder; `SO:` names a ticket file of the Burning
Steppes opening (`docs/specs/steppes-opening/`), and SO:07-client-opening is #633.

- 01 Capture viewpoints at the Westfall and Steppes edges, and a script that takes them (no
  blocker)
- 02 The mist shader: per-layer noise that wraps, billows, faster drift and distance fog (01,
  SO:07-client-opening)
- 03 Five shells into the closed side replace the backdrop, the wisps and the skirt (02)
- 04 Chamfered corners on every shell (03)
- 05 The rim row: closed terrain beside the edge draws and fades into the mist (01,
  SO:07-client-opening)
- 06 The rim row's water draws and fades into the mist (05)
- 07 Drifting puffs along the foot of the mist (04)
- 08 The tram's placed wall: the WMO fog's colour, its own depths and band (07)
- 09 The scene fog pulls in and greys near the edge (01, SO:07-client-opening)
- 10 The Hush: the ambience fades near the edge (09)
- 11 Mist look: take the before and after shots for Davis (ready-for-human; 01 to 10)

The index (`99-index.md`) lists each ticket's files and what can run alongside it. The spec
closes `unplayed` when 11 closes; Davis's play pass at the Westfall edge, the Steppes border and
in the tram tunnel removes the label.
