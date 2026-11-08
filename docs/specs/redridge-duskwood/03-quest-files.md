Title: Six quest and quest-item files for Redridge and Duskwood (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 15. Blocked by 02-level-bands-code (both edit `src/build_id.rs`).

Wiring only: six empty content files the realm loads, so that the Redridge, Duskwood and dungeon
quest tickets each append to a file of their own and two builders never race on one file. Plus
the arc's art placeholders, listed once.

## The files

New, in `crates/twilight-realm/content/`, each with a header comment in the form of
`quests_stormwind.toml`'s first two lines (what it holds, that it has `quests.toml`'s schema, and
which ticket ranges go in it):

| File | Holds | Empty body |
|---|---|---|
| `quests_redridge.toml` | Redridge story quests and bounties, 98101-98499 | nothing (`quest` defaults to empty) |
| `quests_duskwood.toml` | Duskwood story quests and bounties, 98601-98999 | nothing |
| `quests_redridge_dungeons.toml` | Ilgalar's and Stonewatch Keep's boss quests, Toren's page translations and The Unsent Letter, 98501-98599 | nothing |
| `quests_duskwood_dungeons.toml` | Mistmantle Manor's and Raven Hill Deeps' boss quests, 99001-99099 | nothing |
| `quest_items_redridge.toml` | Redridge quest items, 980301-980399 | `item = []` |
| `quest_items_duskwood.toml` | Duskwood quest items, 980401-980499 | `item = []` |

Stormwind's hand-off quests stay in `quests_stormwind.toml` and their items in
`quest_items.toml`.

## The code

- `shipped()` in `crates/twilight-realm/src/quests/content.rs` (line 550): read the four quest
  files after `quests_stormwind.toml` and before `writs.toml`, in the order above. Update the doc
  comment on line 548.
- `shipped_items()` (line 948): read `quest_items.toml`, then `quest_items_redridge.toml`, then
  `quest_items_duskwood.toml`, concatenating `item` and `starter_drop`. Panic with the file name
  if one fails to parse, and panic if an item entry appears in two files.
- `src/build_id.rs`: add the six files beside `quests_stormwind.toml` (line 49) and
  `quest_items.toml` (line 46).
- `content/quests.toml` header (lines 12 to 14): name the new files where it says story quests
  live in `quests_elwynn.toml` and `quests_stormwind.toml`.
- `tests/content_hub.rs`, `elwynn_givers()` (line 102): include the four new quest files.
- `tests/seam.rs`, `shipped_quests()` (line 359): include the four new quest files.
- `scripts/story-text.py` needs no change: it only reads the Elwynn story rewrite.

## Art placeholders

Add one section to `docs/art-assets.md`, after the Deeprun Tram's, headed
`## Redridge and Duskwood 30-50 (spec #PARENT)`, in the Stockade section's form. No later ticket
of this arc edits that file. Rows:

| Asset | Where | Notes |
|---|---|---|
| Grey Robe look | creature display, stock until painted | Stands in as display 2492 (vmangos's Scarlet Abbot, a robed human). Wanted: a grey hooded robe with a small crowned mark on the breast, for the Grey Robe Diggers, Lantern-bearers, Surveyors, Agents, Wardens and the Grey Captain. |
| Crowned mark | quest item icons and the agents' camp banners | Stock icons stand in. Wanted: a plain crown over a closed eye, in the same hand on every item, so a player tells it from the cult's hammer and black sun. |
| Lakeshire Watch camp | GameObject displays, stock | Tents (6738) and a campfire (192), as Sergeant Keane's outpost. No new art needed. |
| Writ Board poster | GameObject display, stock | Stands in as vanilla's Wanted Poster (display 17). Wanted: a smaller notice pinned beside the board, "Hands wanted east". |

Below the table, one line: every creature (the Blackrock, the gnolls, the worgen, the dead, the
ogres, the Defias, the Twilight courier and outriders) and every quest item but the crowned ones
use stock displays and need no art.

## Failing tests first

`tests/content_quests.rs`: a test that the realm loads with the six files present and empty
(`cargo test` loading `shipped()` is enough), and a test that an item entry written in two item
files makes `shipped_items()` panic (through the test seam in `src/lib.rs` line 817 if that is
simpler, or by calling the merge function with two small tables).

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `cargo check -q -p twilight-realm` is
  clean; `icon_check.py` and `ground_check.py` are unchanged.
