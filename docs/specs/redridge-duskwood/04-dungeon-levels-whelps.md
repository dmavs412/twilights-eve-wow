Title: Dungeon levels from the bible, and the whelps leave Lake Everstill (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 8 and 9. Blocked by 02-level-bands-code (both edit
`content/zones.toml`).

Two small content fixes before the band tickets.

## Dungeon levels (decision 9, the bible wins)

- `crates/twilight-realm/content/raven_hill_deeps.toml`: `levels = [44, 48]` (line 16) becomes
  `[40, 44]`; the header's "levels 44 to 48" (line 1) becomes "40 to 44"; the Phase Tester's
  `level = 44` (line 99) becomes 40.
- `crates/twilight-realm/content/stonewatch_keep.toml`: `levels = [44, 48]` (line 16) becomes
  `[45, 48]`; the header's "levels 44 to 48" becomes "45 to 48"; the Phase Tester's `level = 44`
  (line 120) becomes 45.
- Search `src/` and `tests/` for any other place that names these two dungeons' levels (a map
  mark test, the `lore_bible_dungeons` module in `src/phased.rs`) and update it.

## Black Dragon Whelps (decision 8)

Lake Everstill's south shore holds 35 Black Dragon Whelps (entry 441, x -9731 to -9003, y -3020
to -2089 in `redridge.toml`). They tell a level 35 player "dragons" ten levels before Stonewatch
Keep reveals them. Remove them from the open world:

- In `content/zones.toml`, Redridge's `[[zone]]` (line 30), `removed = []` becomes
  `removed = [441]`, with a comment: "Black Dragon Whelp (441): 35 spawns on Lake Everstill's
  shore, removed (#PARENT decision 8); the model is the Stonewatch Keep dungeon's to use."
- Check that no Writ (`content/writs.toml`), quest or encounter names entry 441. Today none does.

## Failing tests first

- In `src/phased.rs`'s `lore_bible_dungeons` module (or `tests/` beside the other dungeon shell
  checks): dungeon 11's levels are 40 to 44 and dungeon 9's are 45 to 48.
- In `tests/content_zones.rs` (not `src/bands.rs`, which ticket 06 edits at the same time): no
  creature of entry 441 spawns on map 0 in the open world.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `ground_check.py` flags nothing.
