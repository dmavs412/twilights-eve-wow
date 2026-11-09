Title: gear_sheet.py: read, set, scale and restyle gear.csv rows by entry (#PARENT)
Labels: ready-for-agent

Part of #PARENT. No blocker. This ticket edits no data.

`tools/twilights-eve/sheet.py` reads and edits `data/twilights-eve/master.csv` by `calling/key`
(`Sheet.find`, line 63) and cannot add or change a `gear.csv` row. #579 and #592 each wrote a
one-off script to append their forty rows; commit 1ec271f5's message says so and records the
scaling method it used. This spec adds 116 rows and restyles about a hundred more, so it gets a
tool. The data tickets after it (04, 05, 07, 10) use it.

## The tool

New `tools/twilights-eve/gear_sheet.py`, stdlib only. It imports `Sheet` from `sheet.py` for its
byte-for-byte record handling (untouched lines written back as they were, `write_bytes`, the
file's own line ending) and finds a row by its `entry` cell instead of `calling/key`. It imports
`power_curve`, `BUDGET`, `WEAPON_DPS`, `ARMOUR_TYPES` and `classify` from `build_gear.py`; no
copies of those tables. Default CSV `data/twilights-eve/gear.csv`, `--csv` overrides.

```
python3.15 tools/twilights-eve/gear_sheet.py get <entry> [column ...]
python3.15 tools/twilights-eve/gear_sheet.py set <entry> column=value [column=value ...]
python3.15 tools/twilights-eve/gear_sheet.py scale <source> <entry> item_level=L required_level=R \
    [quality=Q] [name=N] [column=value ...] [--display D]
python3.15 tools/twilights-eve/gear_sheet.py looks <slot> <type> <item level> [--spread 5] [--db PATH]
python3.15 tools/twilights-eve/gear_sheet.py look-check <entry or a-b> [...] [--spread 5] [--db PATH]
```

- **get** prints the row's non-empty cells, or the columns named.
- **set** changes cells of one row. It refuses an unknown entry or column.
- **scale** copies row `<source>` to a new row `<entry>` at item level L and required level R,
  and writes it after the last row whose entry is below `<entry>`, so the sheet stays in entry
  order. It refuses an `<entry>` that exists. The numbers follow #592's method:
  - The budget is `round_half_up(BUDGET[quality] x P(L))`, doubled for a `2H` row (the rule
    `check_tier` applies, line 230).
  - Each stat cell (`str`, `agi`, `sta`, `int`, `spi`) and `spell_power` scale by the budget
    over the source's points, rounded half up. The remainder then goes on, or comes off, the
    largest stat, so the row's points (`str + agi + sta + int + spi + ceil(spell_power / 2)`)
    equal the budget exactly. The `budget` cell is set to the points.
  - A weapon's `dmg_min` and `dmg_max` scale by the target DPS (its `WEAPON_DPS` anchor x P(L))
    over the source's DPS, `speed` kept, rounded to whole numbers. The result is within 1% of the
    target.
  - A Chest's `armor` is the armour rule exactly: cloth (and Robe) 100, leather 175, mail 300,
    plate 375, x 1.5 x P(L), x 1.15 on a Rare, rounded half up. It refuses an Epic Chest, which no
    rule covers yet.
  - A shield's `armor` and `block` scale by P(L) / P(source item level), and on a quality change
    by the quality factor's ratio: Uncommon 1 / 1.15, Rare 1, Epic 1.13 (Hogger's Pen Gate 791
    over Hogger's Bulwark 699, the sheet's own epic and blue shields at 30).
  - `sell_price` scales by (P(L) / P(source item level)) squared, rounded. `durability`,
    `line`, `icon`, `display_id`, the wardrobe cells and everything else copy, unless set on the
    command line. A quality change keeps the stat shape and takes the new quality's budget.
  - `--display D` sets `display_id` to D. When the source's `icon` cell is blank, it also writes
    the icon name the source's display shows (field 5 of its ItemDisplayInfo.dbc row, read with
    `icon_check.py`'s `read_icons`, line 53), so the row keeps its Line's icon. Game data comes
    from `$WOW_DATA` or `--data`.
- **looks** lists vanilla items for a new model or Wardrobe. It reads the vmangos `item_template`
  table (default `C:\Users\davis\wow-data\vmangos\mangos.sqlite`, the
  Windows path; `--db` overrides). It maps the slot and type to class, subclass and inventory
  type with `classify`, and keeps items of quality 2 or better whose `item_level` is within
  `--spread` of the level asked and whose `display_id` no `gear.csv` row uses, as `display_id`
  or in any `wardrobe_*` cell. It sorts them by distance from the level. For a Chest it groups
  them by family: items sharing a nonzero `set_id`, else sharing their name less its last word
  ("Chief Brigadier Armor" and "Chief Brigadier Coif"). Each family line prints the chest's
  display, the family's item level, and the eight wardrobe cells in the sheet's column order
  (head, shoulders, back, wrists, hands, waist, legs, feet; inventory types 1, 3, 16, 9, 10, 6,
  7, 8), 0 where the family has no such piece. A Cloth Chest prefers the family's robe (inventory
  type 20) where it has one (ADR 0030).
- **look-check** judges the rows named (single entries or `a-b` ranges). For each Chest, Weapon,
  Offhand or Ranged row it finds the vanilla items with the row's `display_id` and prints the row's
  item level beside the nearest such item's level. Rings, Amulets and Trinkets are skipped. It
  exits 1 when:
  - a row's model is more than `--spread` item levels from the row's item level, or no vanilla
    item has that display. One exception: a Plate Chest below item level 41 passes with a plate
    look from 41 to 50, since vanilla has no plate chest below 41;
  - two named Chest rows of one armour type are out of order (the higher item level wears the
    lower-level vanilla look);
  - two named Chest rows share a `display_id`, or one row's eight wardrobe cells equal another
    Chest row's anywhere in the sheet.

Write the usage in the module docstring, in `sheet.py`'s style. Add one paragraph after
`sheet.py`'s in `docs/spell_pipeline.md` (line 255) naming the tool and its five commands.

## Failing tests first

New `tools/twilights-eve/tests/test_gear_sheet.py`, driving the script through its command line
against a small CSV in a temp dir, as `tests/test_sheet.py` does. The CSV holds the real header of
`gear.csv` and copies of rows 902080 (cloth Chest), 902084 (sword), 902085 (shield), 902088
(staff) and 905906 (epic shield).

- `get 902080 armor` prints 420; `set 902080 name=X` changes only that cell and leaves every
  other line byte for byte.
- `scale 902080 902100 item_level=33 required_level=31` writes a row after 902080 with budget 39,
  points 39, armour 492 (100 x 1.5 x P(33), P(33) = 3 x 1.03^3).
- `scale 902088 902108 item_level=33 required_level=31` writes a staff with budget 78, points
  78, `spell_power` about twice `int`, and DPS within 1% of 55.1.
- `scale 902085 902105 item_level=33 required_level=31` scales the shield's armour and block by
  P(33) / P(28).
- `scale 902084 904100 item_level=38 required_level=35 quality=Rare` writes budget 99 and DPS
  within 1% of 62.7.
- `scale` refuses an entry that exists and an Epic Chest.
- `looks` and `look-check` run against an in-memory or temp-file sqlite the test builds with a
  few `item_template` rows (the columns `entry, name, class, subclass, inventory_type,
  item_level, display_id, quality, set_id`): a family is grouped, a used display is left out, a
  far model fails `look-check`, an out-of-order pair fails, a shared wardrobe fails.
- `--display` with a blank source icon is tested with a stub for the icon reader, or skipped with
  `unittest.skipUnless` when `$WOW_DATA` is unset (CI has no game data).

## Acceptance

- `~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests` passes.
- `python3.15 tools/twilights-eve/gear_sheet.py get 902080` prints the row.
- `python3.15 tools/twilights-eve/build_gear.py` still writes an unchanged `gear.toml` (this
  ticket edits no data).

## Assumptions allowed

- The family grouping, if a name-stem rule misgroups a vanilla set the session can see: refine
  it and record the rule.
- Rounding of weapon damage, as long as DPS lands within 1% of the target.
