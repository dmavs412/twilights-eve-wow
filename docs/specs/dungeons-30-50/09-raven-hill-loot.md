Title: Raven Hill Deeps' set: twenty blues and the bosses' rolls (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 3. Blocked by 08-raven-hill-monsters (the bosses' rows), 07-ilgalar-loot
(both edit `gear.csv`, `gear.toml` and `tests/dungeon_gear.rs`) and RD:20-gear-45-50 (the arc goes
first on `gear.csv`).

Twenty blues and the three bosses' rolls, in ticket 05's form. The set's four Chests are the
rewards of the arc's dungeon-ending quest 99011 (RD:28-raven-hill-deeps-quests).

## Data

- Twenty rows in `data/twilights-eve/gear.csv` through `sheet.py add-row`, entries 905180 to
  905199, item level 44, required level 41. Each copies the matching Hogger's camp row (905040 to
  905059, same order; `gear.toml` lines 786 to 805): slot, `line`, `display_id`, `icon`, stat
  shape and wardrobe, with the Power Curve's budgets. Rows 905180 to 905183 are the four Chests
  (cloth, leather, mail, plate). Stems rotate as Hogger's, Gnoll and Riverpaw do: **Gravebound**,
  **Deacon's** and **Seamdigger's** (placeholders).
- Regenerate `gear.toml` with `build_gear.py`; `icon_check.py` shows no new clash.
- In `crates/twilight-realm/content/raven_hill_deeps.toml`: Commander Felstrom (974204) and the
  Dig Overseer (974210) get `rolls = [{ chance = 50, one_of = [905180, ..., 905199] }]`, and the
  Grey Deacon (974213) `rolls = [{ chance = 100, one_of = [the same] }]`, written out in full,
  replacing ticket 08's stand-in. No epics. Leave every `drop` list alone.

## Failing tests first

`crates/twilight-realm/tests/dungeon_gear.rs` gains the Raven Hill pool (add
`raven_hill_deeps.toml` to its file constants and its header):

- the twenty entries exist at quality 3, item level 44, required level 41;
- exactly four Chests among them, one each of cloth, leather, mail and plate;
- Felstrom's and the overseer's rolls list exactly the twenty at 50, the Deacon's at 100;
- no stand-in remains (no 905080 to 905099 in the file).

## Acceptance

- The test passes; `cargo test -q -p twilight-realm` passes; `build_gear.py`'s check passes;
  `icon_check.py` reports no new clash; `ground_check.py` is unchanged.

## Assumptions allowed

- The stems, if `build_gear.py` refuses a name; pick another in the crypt's theme and record it.

Blocked by GEAR:01-gear-sheet-tool. `sheet.py add-row` edits only `master.csv`, so write and change `gear.csv` rows with the `gear_sheet.py` tool that ticket adds, wherever this ticket says `sheet.py`.
