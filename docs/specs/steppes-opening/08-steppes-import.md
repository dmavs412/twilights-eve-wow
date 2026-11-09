Title: The Burning Steppes spawns, Level Bands 50-60, Flame Crest and Morgan's Vigil (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 12 and 13. Blocked by 06-pass-camp (same `content/mobs.toml`,
`content/hub.toml` and `content/zones.toml`; it also needs ticket 02's rule that content may stand
on gated land), RD:02-level-bands-code (the per-zone band files) and RD:21-bands-45-50 (the last
arc ticket that edits `src/bands.rs`, the band files' tests and `content/zones.toml`).

## What

Bring the vanilla Burning Steppes (AreaTable zone 46) into the realm as Redridge, Duskwood and
Stormwind came in (spec #466, `docs/reference/zone-spawns.md`), behind the gate tickets 02 and 04
built: the spawns stand on gated land, so nobody sees them until `steppes_open` is reached. Give
them sub-zone Level Bands from 50 to 60. Turn Flame Crest's two orcs hostile, and make Morgan's
Vigil's eight NPCs Hub NPCs.

## The import

1. Run the importer with Redridge's flags (`docs/reference/zone-spawns.md` line 18), for zone 46:

   ```
   python3.15 -I tools/twilights-eve/import_barrens_spawns.py --db C:\Users\davis\wow-data\vmangos\mangos.sqlite --data C:\Users\davis\wow-data\Data --source-asset db-sqlite-4641790.zip --map 0 --zone 46 --box -9000 -6800 -4300 1100 --all-creatures --out data/twilights-eve/burning_steppes_spawns.json
   ```

   The prep's dry run (2026-11-07) found 500 spawns of 67 entries (418 hostile, 64 neutral, 18
   friendly) and dropped 181 Scourge Invasion spawns as game-event only. If the dump keeps any
   event spawn (Scourge Invasion, or the Lunar Festival's Elder Dawnstrider, entry 15585), remove it
   by entry in `zones.toml`'s `removed`.
2. Add a `Zone` to `ZONES` in `crates/twilight-realm/examples/import_zone.rs` (line 40): name
   "Burning Steppes", dump `burning_steppes_spawns.json`, toml `burning_steppes.toml`, example
   `import_zone`, test `tests/content_zones.rs`, doc `docs/reference/zone-spawns.md`, and
   `hub_npcs` the eight Morgan's Vigil entries below, with a comment saying they are Hub NPCs.
   Run `cargo run -q -p twilight-realm --example import_zone` and commit the new
   `content/burning_steppes.toml` and any change it makes to the other three (there should be
   none).
3. Load it wherever the realm and tests list the generated zones: `src/mobs.rs` (line 614 and the
   module doc on line 2), `src/build_id.rs` (beside `redridge.toml`, line 50), `tests/content.rs`
   (line 841), `tests/content_quests.rs` (line 629) and `tests/content_zones.rs`.
4. A `[[zone]]` for `burning_steppes` in `content/zones.toml`, after Stormwind's:
   - `removed_guids`: every spawn inside the footprints of Blackrock Stronghold
     (`content/blackrock_stronghold.toml`, dungeon 14) and Dreadmaul Rock
     (`content/dreadmaul_rock.toml`, dungeon 15), under one comment per dungeon in the form of
     Redridge's. `src/phased.rs`'s `lore_bible_dungeons` test names them.
   - `levels = { from = [50, 57], to = [50, 60] }`, the fallback, with a comment saying the band
     file covers the whole zone so it applies to nothing.
5. Update `docs/reference/zone-spawns.md`: the import line, the counts table row and the loaded
   totals, from what the tools print.

## Level Bands (`content/burning_steppes_bands.toml`)

A new band file in RD:02's format, registered in `src/bands.rs`'s per-zone table (RD:02 added
it) and in `src/build_id.rs`. Header as RD:02 requires (which zone file it covers, the fallback,
friendly NPCs and critters keep their levels, `level_overrides.toml` wins, first listed wins a
tie). Boxes are `[x0, y0, x1, y1]`, each the AreaTable sub-area's chunk extent from an ADT scan
(2026-11-08). They overlap, so the order below is the file's order, the small sub-areas first.

`bounds = [[-8700.0, -4300.0, -7350.0, 1100.0]]`, `exclude = []`.

| # | Band | Levels | Box (x0, y0, x1, y1) | Vanilla levels there |
|---|---|---|---|---|
| 1 | Morgan's Vigil | 50-52 | -8467, -2900, -8200, -2633 | 50-57 |
| 2 | Blackrock Pass | 50-51 | -8667, -2667, -8300, -2433 | 51-53 |
| 3 | Flame Crest | 55-57 | -7633, -2267, -7467, -2067 | 54-60 |
| 4 | Slither Rock | 52-54 | -7733, -3100, -7533, -2933 | 51-55 |
| 5 | The Pillar of Ash | 55-57 | -8200, -1967, -7967, -1700 | 54-57 |
| 6 | Altar of Storms | 56-58 | -7767, -900, -7433, -600 | 55-61 |
| 7 | Blackrock Stronghold | 56-58 | -7867, -1600, -7500, -1233 | 53-59 |
| 8 | Dreadmaul Rock | 53-55 | -8133, -2800, -7467, -2433 | 50-60 |
| 9 | Ruins of Thaurissan | 54-56 | -7967, -2433, -7400, -1767 | 53-57 |
| 10 | Draco'dar | 57-59 | -8467, -1100, -7900, -667 | 55-60 |
| 11 | Blackrock Mountain | 58-60 | -8100, -1367, -7400, -800 | 46-60 |
| 12 | Terror Wing Path | 51-53 | -8500, -3200, -7367, -2600 | 50-54 |

A spawn in no box (plain "Burning Steppes" chunks) takes the nearest box's band. Each band row's
comment names two or three anchors from `burning_steppes.toml`, as the Redridge band tickets do.

## Flame Crest's orcs (`content/mobs.toml`)

Gruna (3708) and Vahgruk (13177) turn hostile, as the Blackrock are (decision 13). Two `[[template]]`
rows that shadow the generated ones, as Garrick Padfoot's (`content/mobs.toml`) shadows
`content/elwynn.toml`'s: copy each generated row and change only `faction_template = 40` (the
Blackrock Soldier's), the NPC flags to none, `subname` to none, and levels to 55-56 (a template in
`mobs.toml` keeps its own levels, so it skips the band). A comment names this ticket. Ragged John
(9563), Mathredis Firestar (9836), the four goblins (Kibler 10260, Tinkee Steamboil 10267,
Maxwort Uberglint 9536, Yuka Screwspigot 9544), Opus (10262) and the Spirit Healer (6491) keep
their generated rows.

## Morgan's Vigil (`content/hub.toml`)

A new section `# ── Morgan's Vigil (#PARENT) ──` directly after ticket 04's readouts section
(which ticket 06's captain closes). Eight rows in the Goldshire pattern (their real names,
entries and displays, from vmangos), each `role = "ambient"`, `npc_flags = 0x1`, `map = 0`, at its
vanilla spot. No stock and no flights: the game has no flight paths, and stock lists are later
work. They stand on gated land, so ticket 02's rule keeps them out of view until the Steppes open.

| guid | entry | name | subname | level | display | faction | x, y, z | orientation | text_id |
|---|---|---|---|---|---|---|---|---|---|
| 9300092 | 9560 | Marshal Maxwell | | 55 | 8685 | 12 | -8378.5, -2748.9, 186.7 | 4.38 | 64331 |
| 9300093 | 9565 | Mayara Brightwing | | 57 | 8695 | 12 | -8380.6, -2751.9, 187.1 | 0.79 | 64332 |
| 9300094 | 9561 | Jalinda Sprig | | 57 | 8693 | 64 | -8398.9, -2768.6, 194.9 | 0.47 | 64333 |
| 9300095 | 3298 | Gabrielle Chase | Food & Drink | 53 | 1442 | 12 | -8354.4, -2734.1, 185.2 | 4.66 | 64334 |
| 9300096 | 2299 | Borgus Stoutarm | Gryphon Master | 55 | 1626 | 55 | -8365.1, -2736.9, 185.7 | 5.65 | 64335 |
| 9300097 | 1296 | Felder Stover | Weaponsmith | 54 | 1426 | 12 | -8366.4, -2762.5, 187.6 | 0.96 | 64336 |
| 9300098 | 9177 | Oralius | | 55 | 8872 | 55 | -8371.9, -2743.4, 186.2 | 5.59 | 64337 |
| 9300099 | 9562 | Helendis Riverhorn | | 55 | 8694 | 80 | -8377.2, -2780.5, 193.7 | 1.15 | 64338 |

Greetings, in the same order:

- Marshal Maxwell: "Morgan's Vigil holds, and the Blackrock hold everything else. If you came up that road to fight, $C, you've come to the right ridge."
- Mayara Brightwing: "Mind the drakes over the Terror Wing. They don't land where you can reach them, and they always see you first."
- Jalinda Sprig: "Everything up here is either on fire or about to be. I keep a bucket by my bedroll."
- Gabrielle Chase: "The bread's hard and the water tastes of ash, but it's food. Sit down before you fall down."
- Borgus Stoutarm: "My birds won't fly. Not while the sky past the ridges is grey. So I feed them, and I wait."
- Felder Stover: "Bring me Blackrock steel and I'll tell you what it's worth. Mostly it's worth not being hit with it."
- Oralius: "The mountain's angry. You can feel it through your boots at night. Something inside it is at war."
- Helendis Riverhorn: "The ogres at Dreadmaul take tribute from someone. Wagons go in full and come out empty. Find out who pays."

The Z values are vmangos's. Run `ground_check.py`, and `--fix` where it flags one.

## Failing tests first

- `tests/content_zones.rs`: `burning_steppes.toml` agrees with its dump, as the other three zones'
  tests check, and its spawn count after removals is what `zone-spawns.md` says.
- `src/phased.rs`'s `lore_bible_dungeons` test passes with the new zone: no Steppes spawn stands in
  dungeon 14's or 15's footprint.
- `src/bands.rs` tests, in the form of the Redridge band tickets' tests: a hostile spawn in
  Blackrock Pass gets 50 or 51, one in Draco'dar 57 to 59, one on Blackrock Mountain 58 to 60, and
  a friendly NPC (Ragged John, 9563) keeps its own level. Every hostile Steppes spawn's level is
  within 50 to 60.
- `tests/closed_zones.rs`: before `reach_milestone("steppes_open")`, a player on the Redridge side
  in view range of the generated Steppes spawn nearest the border gets no create for it, and a
  player at the pass gets none for Marshal Maxwell. After it, both come in on the next look once
  the player is in view. Pick the nearest spawn from the generated file in the test.
- `tests/content_hub.rs` passes with the eight new rows (role and flags agree).
- Gruna (3708) and Vahgruk (13177) load hostile with no NPC flags; Ragged John (9563) loads
  friendly.

## Acceptance

- The tests pass. `cargo test -q -p twilight-realm` passes. `cargo check -q -p twilight-realm` is
  clean.
- `python3.15 tools/twilights-eve/ground_check.py` and `python3.15 tools/twilights-eve/icon_check.py`
  pass.

Assumptions allowed: a band box trimmed where the anchors show a sub-area's spawns sit in another
band's box; an event spawn the dump keeps; a Morgan's Vigil Z that `ground_check.py` moves. Record
them as `Assumed:` lines.
