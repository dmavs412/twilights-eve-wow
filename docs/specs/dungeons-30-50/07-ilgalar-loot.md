Title: The Tower of Ilgalar's set: twenty blues and the bosses' rolls (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 3. Blocked by 06-ilgalar-monsters (the bosses' rows), 05-mistmantle-loot
(both edit `gear.csv`, `gear.toml` and `tests/dungeon_gear.rs`) and RD:20-gear-45-50 (the arc goes
first on `gear.csv`).

Twenty blues and the two bosses' rolls, in ticket 05's form. The set's four Chests are the
rewards of the arc's dungeon-ending quest 98501 (RD:27-ilgalar-quests).

## Data

- Twenty rows in `data/twilights-eve/gear.csv` through `sheet.py add-row`, entries 905160 to
  905179, item level 38, required level 35. Each copies the matching Hogger's camp row (905040 to
  905059, same order; `gear.toml` lines 786 to 805): slot, `line`, `display_id`, `icon`, stat
  shape and wardrobe, with the Power Curve's budgets. Rows 905160 to 905163 are the four Chests
  (cloth, leather, mail, plate). Stems rotate as Hogger's, Gnoll and Riverpaw do: **Morganth's**,
  **Tuner's** and **Ilgalar** (placeholders).
- Regenerate `gear.toml` with `build_gear.py`; `icon_check.py` shows no new clash.
- In `crates/twilight-realm/content/ilgalar.toml`: the Blackrock Overseer (974111) gets
  `rolls = [{ chance = 50, one_of = [905160, ..., 905179] }]` and Morganth (974114)
  `rolls = [{ chance = 100, one_of = [the same] }]`, written out in full, replacing ticket 06's
  stand-in. No epics. Leave every `drop` list alone.

## Failing tests first

`crates/twilight-realm/tests/dungeon_gear.rs` gains the Ilgalar pool (add `ilgalar.toml` to its
file constants and its header):

- the twenty entries exist at quality 3, item level 38, required level 35;
- exactly four Chests among them, one each of cloth, leather, mail and plate;
- the overseer's roll lists exactly the twenty at 50, Morganth's at 100;
- no stand-in remains (no 905080 to 905099 in the file).

## Acceptance

- The test passes; `cargo test -q -p twilight-realm` passes; `build_gear.py`'s check passes;
  `icon_check.py` reports no new clash; `ground_check.py` is unchanged.

## Assumptions allowed

- The stems, if `build_gear.py` refuses a name; pick another in the tower's theme and record it.

Blocked by GEAR:01-gear-sheet-tool. `sheet.py add-row` edits only `master.csv`, so write and change `gear.csv` rows with the `gear_sheet.py` tool that ticket adds, wherever this ticket says `sheet.py`.
