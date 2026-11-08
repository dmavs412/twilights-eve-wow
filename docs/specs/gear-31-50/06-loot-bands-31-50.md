Title: Loot Bands 31-35 to 46-50 in loot_bands.toml, with no whites (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 2, 4 and 9. Blocked by 05-band-gear-41-50 (the greens this ticket lists
must exist in `gear.toml`).

`crates/twilight-realm/content/loot_bands.toml` ends at 27-30 today, and a mob above it rolls that
band (`band_of`, `src/rules/loot.rs` line 228). This ticket adds the four bands past 30. Adding a
band is data only.

## The bands

Four `[[band]]` tables after the 27-30 one, each with a comment line in the file's style
("Redridge and Duskwood 31-35: 20 greens at item level 33, no whites (spec #PARENT)."):

| levels | white | green |
|---|---|---|
| [31, 35] | `[]` | 902100 to 902119, each `level = 33` |
| [36, 40] | `[]` | 902120 to 902139, each `level = 38` |
| [41, 45] | `[]` | 902140 to 902159, each `level = 43` |
| [46, 50] | `[]` | 902160 to 902179, each `level = 48` |

- `white = []`: no whites past 30 (decision 9). `Loot::band` (`src/looting.rs` line 57) already
  drops nothing when the picked list is empty, so a kill of 31 or above drops a green 4% of the
  time and no white. No code change.
- A mob above 50 rolls the 46-50 band until the Burning Steppes content spec adds 51-60
  (decision 4).
- Fix the file's header (lines 1 to 7): say that the bands from 31 list no whites, so their kills
  roll only the 4% green.

`GLOSSARY.md`, **Loot Source** (line 33): add "No White drops from a mob above level 30 (spec
#PARENT)."

## Failing tests first

`crates/twilight-realm/tests/northshire_loot.rs`:

- `LATER_BANDS` (line 28) gains `([31, 35], 33)`, `([36, 40], 38)`, `([41, 45], 43)` and
  `([46, 50], 48)`. In `each_band_lists_its_whites_and_greens_from_the_gear_sheet` (line 37) the
  white count is 21 for the bands up to 27-30 and 0 for each band from 31; the green checks stay
  (20 Uncommon rows at the band's item level, required level the band's first).
- `a_level_35_mob_rolls_the_last_band` (line 96) becomes a level 55 mob rolling the 46-50 band,
  with the same green check.
- A new test: `band_whites(33)` is empty and `band_greens(33)` is exactly 902100 to 902119;
  `band_greens(50)` is exactly 902160 to 902179.

`src/looting.rs` gains a unit test: a thousand `Loot::band(33, rng)` rolls hold no white and
between 20 and 60 greens of 902100 to 902119.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `python3.15 -I
  tools/twilights-eve/ground_check.py` flags nothing; `python3.15 -I
  tools/twilights-eve/icon_check.py` is unchanged.
