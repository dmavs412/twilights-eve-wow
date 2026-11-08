# Mist look: index of the spec files

The parent (00), the ADR 0015 amendment draft (`adr-0015-amendment.md`, appended to the ADR by
ticket 05), ten tickets labelled `ready-for-agent` (01 to 10) and one labelled `ready-for-human`
(11). "Blocked by" names a file here; `SO:` names a ticket file of the Burning Steppes opening
(`docs/specs/steppes-opening/`). SO:07-client-opening is #633, filed and open.

| File | Title | Blocked by | Edits | Runs alongside |
|---|---|---|---|---|
| 00-parent | Mist look: a rolling grey cloud bank at the edge of the Closed Zones (spec) | | | |
| 01-mist-shots | Mist look: capture viewpoints at the Westfall and Steppes edges, and a script that takes them | | `crates/benilla-app/src/capture/scenarios.rs`, new `scripts/mist-shots.ps1` | anything, SO:07 included |
| 02-mist-shader | The mist shader: per-layer noise that wraps, billows, faster drift and distance fog | 01, SO:07-client-opening | `crates/benilla-world/src/mist.rs`, `crates/benilla-world/src/shaders/mist.wgsl` | 05, 06, 09, 10 |
| 03-mist-shells | Five shells into the closed side replace the backdrop, the wisps and the skirt | 02 | `mist.rs` (and `mist.wgsl` comments) | 05, 06, 09, 10 |
| 04-mist-chamfers | Chamfered corners on every mist shell | 03 | `mist.rs` | 05, 06, 09, 10 |
| 05-rim-terrain | The rim row: closed terrain beside the edge draws and fades into the mist | 01, SO:07-client-opening | `crates/benilla-assets/src/adt.rs`, `crates/benilla-assets/src/model.rs` (the attribute), `crates/benilla-assets/src/lib.rs`, `crates/benilla-assets/src/materials.rs` (`TerrainExtension`), `crates/benilla-assets/src/shaders/terrain.wgsl`, `crates/benilla-world/src/terrain_stream.rs` (line 734), `docs/adr/0015-closed-zones.md` | 02, 03, 04, 07, 08, 09, 10 |
| 06-rim-water | The rim row's water draws and fades into the mist | 05 | `crates/benilla-assets/src/adt.rs`, `crates/benilla-assets/src/materials.rs` (`LiquidExt`), `crates/benilla-assets/src/shaders/liquid.wgsl`, `crates/benilla-world/src/liquid/surface.rs`, `crates/benilla-world/src/terrain_stream.rs` (line 788) | 02, 03, 04, 07, 08, 09, 10 |
| 07-mist-puffs | Drifting puffs along the foot of the mist | 04 | `mist.rs`, new `crates/benilla-world/src/mist_puffs.rs`, `crates/benilla-world/src/lib.rs` (one `mod` line) | 05, 06, 09, 10 |
| 08-tram-wall | The tram's placed mist wall: the WMO fog's colour, its own band and distance fog | 07 | `mist.rs` (and `mist.wgsl` if a lane is renamed) | 05, 06, 09, 10 |
| 09-edge-fog | The scene fog pulls in and greys near the edge of the mist | 01, SO:07-client-opening | `crates/benilla-formats/src/closed_zones.rs`, new `crates/benilla-world/src/lighting/edge.rs`, `crates/benilla-world/src/lighting/mod.rs`, `crates/benilla-world/src/lighting/resolve.rs` | 02 to 08 |
| 10-hush | The Hush: the zone's ambience fades near the edge of the mist | 09 | `crates/benilla-app/src/sound/zone.rs` | 02 to 08 |
| 11-shots-for-davis | Mist look: take the before and after shots for Davis (ready-for-human) | 01 to 10 | fixes and tuning found in the run, `mist.rs`, `mist_puffs.rs`, `adt.rs`, `lighting/edge.rs`, `scripts/mist-shots.ps1` | nothing of this spec |

## What runs in parallel

- 01 runs first, on its own; it blocks nothing in the Steppes spec and nothing there blocks it.
- Three chains start once 01 and SO:07 (#633) have landed, and share no file:
  - the mist chain 02, 03, 04, 07, 08, all on `mist.rs`;
  - the rim chain 05, 06, both on `adt.rs` and `materials.rs`;
  - the edge chain 09, 10, which reuses 09's curve.
- 05 and 06 both touch `terrain_stream.rs`, at different lines; 06 follows 05 anyway.
- Every code ticket is a client build, so the machine-wide client lock lets one build at a time.
  Two builders still interleave the chains, and a rebase conflict between chains should not
  happen since they share no file.
- 11 waits for all ten and is run by hand with Davis.

## Notes for the filer

- Substitute `#PARENT` with the parent's number (in each title, each "Part of" line, ticket 01's
  scenario comment and the ADR amendment draft), each "Blocked by NN-name" with that ticket's
  number, and "SO:07-client-opening" with #633. Add GitHub dependency links too if you like; the
  builders read both.
- The before set depends on 02, 05 and 09 being blocked by 01. Keep those three blockers even if
  01 lands first anyway.
- Ticket 05 appends the ADR amendment, so the ADR text is part of what you file: review
  `adr-0015-amendment.md` with Davis before filing.
- The numbers in the parent's "Readings taken while writing the tickets" are Claude's. Davis can
  change any of them before filing, or tune them in ticket 11 from the shots.
- The spec closes `unplayed` when 11 closes. The play pass covers the Westfall edge, the Steppes
  border while closed, the tram tunnel and the sound.
