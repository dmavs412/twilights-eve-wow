Title: Closed Zones that a Milestone opens, and the four neighbours closed for good (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 5 and 6. ADR 0034 and ADR 0015. Blocked by 01-milestone-state (the
reached set, and `src/lib.rs`).

## What

Today `data/twilights-eve/closed_zones.toml` closes only Westfall, and the closed list is fixed at
build time. Make a Closed Zone able to wait for a Milestone: zone 46 (the Burning Steppes) stays
closed until `steppes_open` is reached. Close Searing Gorge, the Badlands, the Swamp of Sorrows
and the Wetlands for good. The realm's wall follows the reached set at once. The client's reader
learns the new file shape and, until ticket 07, treats gated land as closed.

## Data and the tool

`data/twilights-eve/closed_zones.toml` becomes:

```toml
# Zones cut out of the world (ADR 0015, ADR 0034). AreaTable zone ids; a chunk is closed when its
# area rolls up through AreaTable's parents to one of these. Rerun closed_zones.py after an edit.
map = 0                      # Azeroth
# Closed for good: Westfall, the Badlands, the Swamp of Sorrows, the Wetlands, Searing Gorge.
zones = [40, 3, 8, 11, 51]

# Closed until a Milestone (content/milestones.toml) is reached, then open for good.
[[opens]]
zone = 46                    # Burning Steppes
milestone = "steppes_open"
```

`tools/twilights-eve/closed_zones.py`:

- `load_config` returns the `opens` rows too. A zone listed both in `zones` and in an `opens` row,
  or in two `opens` rows, is an error that names it.
- `render` writes the closed-for-good chunks under `chunks`, as today, and a new `gated` list, one
  entry per `opens` row: `{"key": "steppes_open", "zone": 46, "chunks": [[gx, gy], ...]}`, chunks
  sorted and one per line like `chunks`. With no `opens` rows, `gated` is `[]`.
- The per-zone counts it prints cover the gated zone too, marked "(until steppes_open)".
- Extend `tools/twilights-eve/tests/test_closed_zones.py`: the config parse with an `opens` row,
  the duplicate-zone error, and `render`'s `gated` list on a tiny input. Run the unittest line in
  `docs/spell_pipeline.md` (line 249).

Then run `python3.15 tools/twilights-eve/closed_zones.py` (it reads the game data from `WOW_DATA`)
and commit the new `data/twilights-eve/closed_chunks.json`. The prep scan counted 4,661 Steppes
chunks; the tool's own count is the one to trust.

## Realm

- `src/world/closed.rs`: `File` reads `gated` (`#[serde(default)]`), and `Closed` keeps each gated
  chunk with its key. `pub fn closed_at(map, x, y, reached: &[&str]) -> bool` is true on a chunk
  closed for good, or on a gated chunk whose key is not in `reached`. Add
  `pub fn closed_for_good(map, x, y) -> bool` for the content test. Re-export both from
  `src/lib.rs` (line 118).
- `InWorld::closed_off` (`src/world/phase.rs` line 263) takes the reached keys and passes them to
  `closed_at`. Its caller in `src/lib.rs` (line 1898) passes `self.milestones`' reached keys.
- **Nothing on closed land is sent.** While a chunk is closed, gated or for good, the realm brings
  in no spawn standing on it (`src/population.rs`, `update` at line 89), no Hub NPC
  (`src/hub.rs`, `InView::update` at line 377), no quest GameObject (`src/world/phase.rs`, the
  creates at line 749), and no Phased Dungeon portal or stop portal on it. `src/map_marks.rs`
  (`marks` at line 180, and the Hub and object loops at lines 31, 59 and 206) leaves out marks on
  closed land. Once the land opens, the next look brings them in, with no restart.
- **A character on closed land at login** (a host reset `milestones.toml` with characters in the
  Steppes) enters the world at their bind point instead. Do it where `enter_world` (`src/lib.rs`
  line 2922) already moves a save out of a dungeon (`world::come_out`), and log one line naming the
  character.
- **The content test.** `tests/closed_zones.rs`'s check that nothing the content places stands on
  a closed chunk (it reads `placed()` and `audit_rows()` in `src/world/closed.rs`) now fails only
  on land closed for good. Content on gated land passes. If any shipped row stands in one of the
  four newly closed zones, the test names it: list each in the commit message and move it onto
  open land at the nearest point, or remove it when it is a stray vanilla spawn. Record either as
  an `Assumed:` line.
- Every other caller of `closed_at` passes the realm's reached keys, or `&[]` in a test that means
  "nothing reached".

## Client reader (benilla-formats only)

`crates/benilla-formats/src/closed_zones.rs` parses `gated`. Until ticket 07 gives it a reached
set, it treats every gated chunk as closed: `closed_at`, `chunk_closed` and `closed_chunks_near`
are true on gated chunks, and `zone_closed` is true for a gated zone. Their signatures do not
change, so no other client crate changes in this ticket.

## Test points

Use these vanilla spots (vmangos spawn positions on map 0):

| Point | Where | Before `steppes_open` | After |
|---|---|---|---|
| (-8378.5, -2748.9) | Morgan's Vigil, Marshal Maxwell | closed | open |
| (-7495.0, -2170.9) | Flame Crest, Ragged John | closed | open |
| (-6679.9, -1194.4) | Searing Gorge, Kalaran Windblade | closed | closed |
| (-6633.7, -2724.6) | the Badlands, Lotwil Veriatus | closed | closed |
| (-10632.3, -3009.4) | the Swamp of Sorrows, Fallen Hero of the Horde | closed | closed |
| (-3732.5, -797.3) | the Wetlands, Captain Stoutfist | closed | closed |
| (-10508.8, 1045.23) | Westfall, Gryan Stoutmantle | closed | closed |
| (-9219.0, -2222.0) | Lakeshire, Magistrate Solomon | open | open |

If the tool puts one of the closed-for-good points on an open chunk, pick another spawn of that
zone from the vmangos dump and record it.

## Failing tests first

- `tests/closed_zones.rs`: the table above through `closed_at(0, x, y, &[])` and
  `closed_at(0, x, y, &["steppes_open"])`.
- `tests/closed_zones.rs`: a player at (-8690.0, -2560.0, 132.0) on the Redridge side of Blackrock
  Pass who steps to (-8650.0, -2560.0, 132.0), on a Steppes chunk, is snapped back and told "You
  cannot go that way." After `realm.reach_milestone("steppes_open")` the same step is allowed.
  Build it like `at_the_edge` and `step` in that file.
- `tests/closed_zones.rs`: a quest GameObject placed with `realm.with_quests(toml)` at Morgan's
  Vigil (-8378.5, -2748.9) sends no create to a player standing in view of it on the Redridge side
  before `steppes_open`; after `reach_milestone`, the next look sends it. If the view range does
  not reach across the border from open land, place the test object on the first Steppes chunk
  past (-8650.0, -2560.0) instead.
- `src/world/closed.rs` tests: `closed_for_good` is true in Searing Gorge and false at Morgan's
  Vigil.
- `crates/benilla-formats/src/closed_zones.rs` tests: Morgan's Vigil is closed, Lakeshire is open,
  `zone_closed(0, 46)` is true.
- `tests/saving.rs` or `tests/closed_zones.rs`: a character saved at Morgan's Vigil enters the
  world at its bind point while `steppes_open` is not reached.

## Acceptance

- The tests pass. `cargo test -q -p twilight-realm` passes. `cargo check -q -p twilight-realm` is
  clean. `cargo test -q -p benilla-formats closed_zones` passes.
- The pipeline unittests pass, and `closed_chunks.json` is the tool's output.

Assumptions allowed: inner names in the readers; how the reached keys reach `closed_off` and the
streaming code (a slice, a closure or a small type); how a stray row in a newly closed zone is
handled. Record them as `Assumed:` lines.
