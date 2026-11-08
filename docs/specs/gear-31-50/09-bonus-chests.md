Title: The four bonus chests: Mistmantle's clock, Ilgalar's open valves, the Deeps' dig face and Stonewatch's roof (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 7. Blocked by 03-bonus-chest-code (the `[bonus_chest]` key),
07-dungeon-epics (the last ticket of this spec to edit the four dungeon files and
`tests/dungeon_gear.rs`), DUNGEONS:05-mistmantle-loot, DUNGEONS:07-ilgalar-loot,
DUNGEONS:09-raven-hill-loot and DUNGEONS:11-stonewatch-loot (each writes its dungeon's set, which
the chest draws from).

One bonus chest per 30-50 dungeon, earned by an optional challenge, holding one blue of the
dungeon's set and coin. The challenges are this spec's reading (Davis left them to it). Each one
uses a part the dungeon already has.

## The chests

Each dungeon file gains a `[bonus_chest]` table (ticket 03's schema) with a comment naming this
ticket. `display = 259` and `east_yd = 2.0` in every one, as the tram's chests. Coin is 100 x the
set's item level in copper, a placeholder. `one_of` is the dungeon's twenty set entries, written
out in full.

| File | `boss` | `entry` | `one_of` | `money` | `challenge` |
|---|---|---|---|---|---|
| `content/mistmantle_manor.toml` | 974014 (Stalvan Mistmantle) | 980721 | 905140-905159 | 3600 | `{ kind = "within_s", seconds = 720 }` |
| `content/ilgalar.toml` | 974114 (Morganth) | 980722 | 905160-905179 | 3800 | `{ kind = "unused", levers = [983041, 983042, 983043] }` |
| `content/raven_hill_deeps.toml` | 974213 (the Grey Deacon) | 980723 | 905180-905199 | 4400 | `{ kind = "unused", levers = [983051] }` |
| `content/stonewatch_keep.toml` | 974316 (Gath'Ilzogg) | 980724 | 905200-905219 | 4800 | `{ kind = "together_s", guids = [974316, 974317], seconds = 10 }` |

- **Mistmantle Manor.** Its fights are plain (the dungeons spec's decision 11), so the challenge is
  speed: Stalvan dies within 12 minutes of the Run's start, in a run sized for about 15.
  `found_text = "Under the torn boards by the humming stone, a strongbox nobody has opened since
  the dark came."`
- **The Tower of Ilgalar.** Morganth dies with all three feed valves (983041 to 983043) still
  open, so every one of his timed calls ran. `found_text = "With every valve still open, the
  tower's strongbox stands unguarded."`
- **Raven Hill Deeps.** The Grey Deacon dies with the shoring lever (983051) never pulled, so the
  Dig Overseer's ghouls kept coming out of the side tunnel. `found_text = "The diggers' pay chest
  sits by the seam, its keeper gone."`
- **Stonewatch Keep.** Gath'Ilzogg (974316) and Singe (974317) die within 10 seconds of each
  other, so the survivor's enrage barely lands. The chest stands by Gath'Ilzogg's corpse.
  `found_text = "Gath'Ilzogg's war chest falls open beside him."` Leave his `drop` list alone: the
  arc's ticket 29 puts page three there, and the chest holds no page.

Names in `found_text` follow the dungeons spec's reveal rules: nobody says "Anchor" in Mistmantle,
names the cult, or names Sareth.

`docs/art-assets.md`: one line under the 30-50 dungeons' section (or a new "Gear 31-50" section if
there is none), saying the four bonus chests wear vanilla's small wooden chest (display 259) as a
placeholder for a chest look of their own.

## Failing tests first

- `crates/twilight-realm/tests/dungeon_gear.rs` gains, for each of the four files: the
  `[bonus_chest]` exists with the table's `boss`, `entry`, `money` and challenge, and its
  `one_of` is exactly the dungeon's twenty set entries (the same list its final boss's set roll
  names).
- `crates/twilight-realm/tests/ilgalar.rs` (DUNGEONS:06's file and harness) gains: a Run where
  Morganth dies with no valve pulled makes one chest of entry 980722 near his corpse holding 3800
  copper and one of 905160 to 905179; a Run where valve A is pulled first makes none.

The other three challenge kinds are tested on the Ward Test by ticket 03.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `python3.15 -I
  tools/twilights-eve/ground_check.py` flags nothing; `python3.15 -I
  tools/twilights-eve/icon_check.py` is unchanged.

## Assumptions allowed

- A boss or lever guid that the dungeon file holds under another number when this runs: use the
  file's creature or lever that the table names by role, and record it.
- If killing Morganth in `tests/ilgalar.rs`'s harness is impractical, test the chest on Raven Hill
  Deeps' Deacon or Mistmantle's Stalvan instead, with the same two cases, and record it.
