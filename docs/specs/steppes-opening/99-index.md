# The Burning Steppes open, server-wide: index of the spec files

The parent (00), the ADR draft (`adr-draft.md`, numbered 0034), nine tickets labelled
`ready-for-agent` (01 to 08 and 10) and one `ready-for-human` (09). Paths are under
`crates/twilight-realm/` unless they start with `crates/`, `data/`, `docs/`, `tools/` or name a
root file. "Blocked by" names a file here; `RD:` names a ticket file in
`docs/specs/redridge-duskwood/`.

| File | Title | Blocked by | Edits | Runs alongside |
|---|---|---|---|---|
| 00-parent | The Burning Steppes open, server-wide (spec) | | | |
| 01-milestone-state | Milestones: the catalogue, the realm-written state file and the session event | | new `src/milestones.rs`, `src/lib.rs`, `src/save.rs`, `src/build_id.rs`, new `content/milestones.toml`, `crates/benilla-protocol/src/events.rs`, `crates/twilight-server/src/lib.rs`, `crates/benilla-app/src/twilight/mod.rs`, new `tests/milestones.rs`, `GLOSSARY.md`, new `docs/adr/0034-milestones-and-gated-zones.md` | 09; arc tickets that leave `src/lib.rs` alone |
| 02-gated-closed-zones | Closed Zones that a Milestone opens, and the four neighbours closed for good | 01 | `data/twilights-eve/closed_zones.toml`, `data/twilights-eve/closed_chunks.json`, `tools/twilights-eve/closed_zones.py`, `tools/twilights-eve/tests/test_closed_zones.py`, `src/world/closed.rs`, `src/world/phase.rs`, `src/lib.rs`, `src/population.rs`, `src/hub.rs`, `src/map_marks.rs`, `tests/closed_zones.rs`, `crates/benilla-formats/src/closed_zones.rs` (and any stray row in a newly closed zone) | 09 |
| 03-milestone-gated-content | Content that waits for a Milestone: after_milestone and until_milestone | 02, RD:02-level-bands-code, RD:03-quest-files | `src/hub.rs`, `src/quests/content.rs`, `src/quests/dialogs.rs`, `src/mobs.rs`, `src/population.rs`, `src/phased.rs`, `src/shard.rs`, `src/world/phase.rs`, `src/map_marks.rs`, `src/lib.rs`, headers of `content/hub.toml`, `content/zones.toml`, `content/quests.toml`, `content/ward_test.toml`, `content/stockade_riot.toml`, `tests/milestones.rs` | 07, 09 |
| 04-tally-and-readouts | The Tally, its two readout NPCs, and the opening's line on /1 and Toren's yell | 03, RD:12-redridge-35-40 | `src/milestones.rs`, `src/quests/dialogs.rs`, `src/hub.rs`, `src/chat.rs`, `src/lib.rs`, `content/milestones.toml`, `content/hub.toml` (new section after Goldshire's inn and shops), `GLOSSARY.md`, `tests/milestones.rs` | 07, 09 |
| 05-anchor-break | The break: an Encounter at the Ilgalar Anchor, and Toren's two quests | 04, RD:12-redridge-35-40, RD:03-quest-files | `src/encounters.rs`, `src/mobs.rs`, `src/lib.rs`, `src/quests/content.rs`, `src/build_id.rs`, `content/encounters.toml`, `content/ilgalar.toml`, `content/mobs.toml`, new `content/quests_blackrock_pass.toml`, `docs/art-assets.md`, new `tests/steppes_opening.rs` | 07, 09 |
| 06-pass-camp | The Blackrock Pass Watch camp: its captain, board, creatures, bounties and Writs | 05, RD:21-bands-45-50 | `content/hub.toml`, `content/writs.toml`, `content/mobs.toml`, `content/zones.toml` (Redridge), `content/quests_blackrock_pass.toml`, `content/milestones.toml`, `docs/art-assets.md`, `tests/steppes_opening.rs`, `tests/content_quests.rs` | 07, 09 |
| 07-client-opening | The client opens a gated zone at run time and fades its mist | 02 | `crates/benilla-formats/src/closed_zones.rs`, `crates/benilla-assets/src/adt.rs`, `crates/benilla-assets/src/lib.rs`, `crates/benilla-world/src/mist.rs`, the mist shader `mist.wgsl`, `crates/benilla-world/src/wdl.rs`, `crates/benilla-world/src/terrain_stream.rs`, `crates/benilla-app/src/twilight/mod.rs` (or wherever session events reach the app), `crates/benilla-app/src/minimap/mod.rs`, `crates/benilla-app/src/ui_world_map.rs` | 03, 04, 05, 06, 08, 09, 10 |
| 08-steppes-import | The Burning Steppes spawns, Level Bands 50-60, Flame Crest and Morgan's Vigil | 06, RD:02-level-bands-code, RD:21-bands-45-50 | new `data/twilights-eve/burning_steppes_spawns.json`, `examples/import_zone.rs`, new generated `content/burning_steppes.toml`, new `content/burning_steppes_bands.toml`, `src/bands.rs`, `src/mobs.rs`, `src/build_id.rs`, `content/zones.toml`, `content/mobs.toml`, `content/hub.toml`, `tests/content.rs`, `tests/content_quests.rs`, `tests/content_zones.rs`, `tests/closed_zones.rs`, `docs/reference/zone-spawns.md` | 07, 09, 10 |
| 09-steppes-map-art | Paint the Burning Steppes onto the world map, or decide to leave them off (ready-for-human) | | Davis's art; `docs/art-assets.md` when done | anything |
| 10-swap-stand-in | The break quest follows The Unsent Letter, and the stand-in goes | 06, RD:30-toren-unsent-letter | `content/quests_blackrock_pass.toml`, `tests/steppes_opening.rs`, and `src/save.rs` only if a save with an unknown quest id fails to load | 07, 08, 09 |

## What runs in parallel

- 01 to 06 are one chain: each shares `src/lib.rs`, a content file or a test file with the one
  before it. 08 follows 06 because both edit `content/mobs.toml`, `content/hub.toml` and
  `content/zones.toml`.
- 07 (client crates only) runs beside everything from 03 on. It shares
  `crates/benilla-app/src/twilight/mod.rs` with 01 and `crates/benilla-formats/src/closed_zones.rs`
  with 02, and both land before it.
- 09 is Davis's and blocks nothing.
- 10 waits on the arc's last quest (RD:30) and runs beside 07, 08 and 09.

## Notes for the filer

- Substitute `#PARENT` with the parent's number, each "Blocked by NN-name" with that ticket's
  number, and each `RD:<file>` with the arc ticket's number once filed. Add GitHub dependency
  links too if you like; the builders read both.
- Arc tickets that must land first: RD:02 and RD:03 (code files 03 and 05 share), RD:12 (it adds
  Toren in Lakeshire, 190083, who gives 05's quests and yells in 04), RD:21 (the last arc ticket on
  `content/zones.toml` and `src/bands.rs`), and RD:30 for 10 only.
- 01 and the arc's RD:02 and RD:03 all add lines to `src/build_id.rs`, at different places in its
  list. Git should merge them; a conflict requeues the ticket once.
- The dungeons spec (`docs/specs/prep/dungeons-30-50.md`, not filed) will also edit
  `content/ilgalar.toml`. Ticket 05 handles either order: if the dungeons spec placed an Anchor
  first, 05 gates that row instead of adding its own.
- Davis reviews the quoted text (Toren's two quests, the readouts, the captain, the camp's
  quests and Writs, Morgan's Vigil's greetings, the `/1` line and the yell) before filing; the
  builders paste it as written.
- The Writs spec's row "Burning Steppes, 3, written by the spec that brings in the zone's mobs"
  (`docs/specs/writs/README.md`) is still open: the camp's Writs are a different pool on a
  different board.
- When the spec lands, the lore bible's "the mist rolls off the pass and the Burning Steppes
  open" has a Tally between the break and the opening, for the live doc.
- The spec closes `unplayed`. What the opening looks like in the game (the fade, the land loading,
  the yell) is for Davis's play, not any ticket's acceptance.
