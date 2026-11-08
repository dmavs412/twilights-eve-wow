Title: Loot Band gear 41-50: forty World greens and twelve Elite blues (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 2, 4, 8 and 9. Blocked by 04-band-gear-31-40 (same files:
`data/twilights-eve/gear.csv`, `content/gear.toml`, `client_dbc_patch.json` and
`tests/band_gear_31_50.rs`).

The last two Loot Bands of this spec, 41-45 and 46-50: one row on each of the 20 `world-*` Lines
and the six `elite-*` Lines per band. 52 rows. No whites. Bands 51-60 wait for the Burning Steppes
content spec (decision 4); a mob above 50 rolls the 46-50 band.

## The rows

Exactly ticket 04's method and rules ("The rows", "Looks"): `gear_sheet.py scale` from the 27-30
band's row on the same Line, source `902080 + i` for a green and `904018 + i` for an Elite blue,
in the Line order ticket 04 lists. Keep the source's `line`, `bonding`, `durability`, `quality` and
`icon`.

| Band | Band index | Item level | Required | Greens (Uncommon) | Elite blues (Rare) | Green stems, in order | Elite stem |
|---|---|---|---|---|---|---|---|
| 41-45 | 2 | 43 | 41 | 902140-902159 | 904036-904041 | Render's, Brightwood, Vul'Gol, Rethban | Blackrock Captain's |
| 46-50 | 3 | 48 | 46 | 902160-902179 | 904042-904047 | Galardell, Orchard, Ebonlocke, Stonefall | Gravewatch |

Green entry `902100 + 20 x band index + i`; Elite entry `904024 + 6 x band index + i`. A green
takes stem `i mod 4` and its source's noun; an Elite blue its band's stem and its source's noun.
Names are placeholders.

## Numbers the build checks

| Item level | Green | Green staff | Blue | Green DPS 1H / staff / bow | Chest armour cloth / leather / mail / plate |
|---|---|---|---|---|---|
| 43 | 53 | 106 | 115 | 55.5 / 74.0 / 42.3 | 661 / 1156 / 1983 / 2478 |
| 48 | 61 | 122 | 133 | 64.4 / 85.8 / 49.0 | 766 / 1341 / 2298 / 2873 |

## Looks

As ticket 04. The cloth, leather and mail Chests (902140-902142, 902160-902162) each take their
own vanilla family near their item level, each higher than the same armour type's row a band
below. The plate Chests (902143, 902163) keep the 27-30 row's look until ticket 10. The 14
weapon, shield, held and bow rows take models near their level with `--display`; rings and
amulets keep their source's display.

## Failing tests first

`crates/twilight-realm/tests/band_gear_31_50.rs` (ticket 04) gains:

- 902140 to 902179 at quality 2, item levels 43 and 48, required levels 41 and 46, each on the
  `world-*` Line of `902080 + i`;
- 904036 to 904047 at quality 3, item levels 43 and 48, required levels 41 and 46, on the
  `elite-*` Lines;
- each Chest's armour is the table's;
- across 902100 to 902179, each `world-*` Line has exactly four rows, and across 904024 to
  904047 each `elite-*` Line exactly four;
- the cloth, leather and mail Chests of 902140 to 902179 have wardrobes no other Chest row has.

## Acceptance

- `build_gear.py` passes and regenerates `gear.toml`; `build_spells.py --data "$WOW_DATA"`
  regenerates the patch; `icon_check.py` reports no new clash.
- `python3.15 tools/twilights-eve/gear_sheet.py look-check 902100-902102 902120-902122
  902140-902142 902160-902162 902104-902110 902124-902130 902144-902150 902164-902170` exits 0
  (it also checks the order across all four bands).
- The test passes; `cargo test -q -p twilight-realm` passes; `ground_check.py` flags nothing; the
  pipeline unittests pass.

## Assumptions allowed

- The vanilla family or model chosen, each named in the commit with its vanilla item level.
- If a slot has no free vanilla model within 5 levels, widen to 8 and record it.
