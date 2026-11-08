Title: Loot Band gear 31-40: forty World greens and twelve Elite blues (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 2, 8 and 9. Blocked by 01-gear-sheet-tool (the tool this ticket writes
rows with), RD:20-gear-45-50 and DUNGEONS:11-stonewatch-loot. Those two are the last tickets of
the Redridge and Duskwood arc and of the 30-50 dungeons spec to edit `data/twilights-eve/gear.csv`
and `content/gear.toml`. A new display pair renumbers the synthetic displays (998000 up) through
`gear.toml`, so gear tickets never run side by side.

The first two Loot Bands past 30: one row on each of the 20 `world-*` Lines and the six `elite-*`
Lines, for 31-35 and for 36-40. 52 rows. No whites (decision 9). Ticket 06 lists the greens in
`loot_bands.toml`; ticket 08 gives the Elite blues to the named elites.

## The rows

Write every row with `python3.15 tools/twilights-eve/gear_sheet.py scale <source> <entry>
item_level=L required_level=R name=...`, from the 27-30 band's row on the same Line. The tool
sets the budget, stats, weapon damage, armour, block and sell price by the Power Curve. Keep the
source's `line`, `bonding` (Equip), `durability`, `quality` and `icon`.

| Band | Item level | Required | Greens (Uncommon) | Elite blues (Rare) | Green stems, in order | Elite stem |
|---|---|---|---|---|---|---|
| 31-35 | 33 | 31 | 902100-902119 | 904024-904029 | Lakeshire, Everstill, Darkshire, Tranquil | Everstill Warden's |
| 36-40 | 38 | 36 | 902120-902139 | 904030-904035 | Canyon, Alther's, Yorgen, Addle's | Canyon Chief's |

- **Greens.** Entry `902100 + 20 x band index + i` (band index 0 for 31-35, 1 for 36-40), source
  `902080 + i`, for i = 0 to 19 in the source's Line order: `world-chest-cloth`,
  `world-chest-leather`, `world-chest-mail`, `world-chest-plate`, `world-sword`, `world-shield`,
  `world-dagger`, `world-bow`, `world-staff`, `world-mace`, `world-held`, `world-amulet-might`,
  `world-amulet-swift`, `world-amulet-mind`, `world-ring-might-a`, `world-ring-might-b`,
  `world-ring-swift-a`, `world-ring-swift-b`, `world-ring-mind-a`, `world-ring-mind-b`.
- **Elite blues.** Entry `904024 + 6 x band index + i`, source `904018 + i`, for i = 0 to 5:
  `elite-amulet-might`, `elite-amulet-swift`, `elite-amulet-mind`, `elite-ring-might`,
  `elite-ring-swift`, `elite-ring-mind`.
- **Names** are placeholders. A green takes stem `i mod 4` of its band's four, as 902080 to
  902099 rotate Westbrook, Riverpaw, Forest's Edge and Deepwater, and keeps its source's noun
  ("Westbrook Wool Robe" becomes "Lakeshire Wool Robe"). An Elite blue takes its band's stem and
  its source's noun ("Bookkeeper's Torc" becomes "Everstill Warden's Torc"). If `build_gear.py`
  refuses a name, pick another in the band's places and record it.

## Numbers the build checks

Budget `round_half_up(base x P(L))`, base green 12 and blue 26, a two-hander doubled;
P(L) = 3 x 1.03^(L - 30). `build_gear.py` refuses a row more than 4 points or 10% of DPS off.

| Item level | Green | Green staff | Blue | Green DPS 1H / staff / bow | Chest armour cloth / leather / mail / plate |
|---|---|---|---|---|---|
| 33 | 39 | 78 | 85 | 41.3 / 55.1 / 31.5 | 492 / 861 / 1475 / 1844 |
| 38 | 46 | 92 | 99 | 47.9 / 63.8 / 36.5 | 570 / 998 / 1710 / 2138 |

## Looks (decision 8)

Looks follow item level: each new model or Wardrobe comes from vanilla items near the row's item
level, and a higher row wears a higher look.

- **Cloth, leather and mail Chests** (6 rows): each takes its own vanilla family from
  `gear_sheet.py looks Chest <Cloth|Leather|Mail> <item level>`: the chest's display as
  `display_id` and the family's pieces as the eight `wardrobe_*` cells (`gear_sheet.py set`). A
  cloth Chest wears its family's robe where it has one (ADR 0030). The 38 row of an armour type
  wears a higher-level family than its 33 row. No family any other `gear.csv` row wears.
- **Plate Chests** (902103, 902123) keep the 27-30 row's look for now. Vanilla has no plate chest
  below item level 41, so ticket 10 hands out the 13 plate looks of these levels in one order.
- **Weapons, shields, held items and bows** (14 rows): each takes a model from `gear_sheet.py
  looks <slot> <type> <item level>`, written with `scale ... --display D` so the row keeps its
  Line's icon in its `icon` cell. No model any other row uses.
- **Rings and amulets** keep their source's display: no one sees them worn.

A Line still shows one icon (ADR 0026); a new model never changes it. No new icon.

## Failing tests first

New `crates/twilight-realm/tests/band_gear_31_50.rs` (ticket 05 extends it), reading
`content/gear.toml` as `tests/quest_gear_30_50.rs` and `tests/dungeon_gear.rs` do:

- 902100 to 902139 exist at quality 2, item levels 33 and 38, required levels 31 and 36, each on
  the `world-*` Line of its source (`902080 + i`);
- 904024 to 904035 exist at quality 3, item levels 33 and 38, required levels 31 and 36, on the
  `elite-*` Lines;
- each Chest's armour is the table's;
- every `world-*` and `elite-*` Line shows one icon across all its rows;
- the cloth, leather and mail Chests of 902100 to 902139 have wardrobes no other Chest row in
  `gear.toml` has.

## Acceptance

- `python3.15 tools/twilights-eve/build_gear.py` passes and regenerates `content/gear.toml`;
  `python3.15 tools/twilights-eve/build_spells.py --data "$WOW_DATA"` regenerates
  `data/twilights-eve/client_dbc_patch.json` (the new model and icon pairs become
  `item_displays` rows); `python3.15 -I tools/twilights-eve/icon_check.py` reports no new clash.
- `python3.15 tools/twilights-eve/gear_sheet.py look-check 902100-902102 902104-902110
  902120-902122 902124-902130` exits 0.
- The new test passes; `cargo test -q -p twilight-realm` passes; `python3.15 -I
  tools/twilights-eve/ground_check.py` flags nothing; the pipeline unittests pass.

## Assumptions allowed

- The vanilla family or model chosen, from `looks`'s list, each named in the commit with its
  vanilla item level.
- If a slot has no free vanilla model within 5 levels, widen to 8 and record it.
