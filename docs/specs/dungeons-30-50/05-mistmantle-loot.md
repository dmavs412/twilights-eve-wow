Title: Mistmantle Manor's set: twenty blues and the bosses' rolls (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 3. Blocked by 04-mistmantle-monsters (it writes the bosses' rows whose
`rolls` this ticket sets) and RD:20-gear-45-50 (the Redridge and Duskwood arc goes first on
`gear.csv` and `gear.toml`; its last gear ticket must land before any of this spec's).

Twenty blues, one per line of the Hogger's camp set, and the three bosses' rolls, as #579 did
for the Stockade. The set's four Chests (cloth, leather, mail, plate) are the rewards of the
arc's dungeon-ending quest 99001 (RD:26-mistmantle-quests), so they must exist.

## Data

- Twenty rows in `data/twilights-eve/gear.csv` through
  `python3.15 tools/twilights-eve/sheet.py add-row`, entries 905140 to 905159, item level 36,
  required level 33. Each copies the slot, `line`, `display_id`, `icon`, stat shape and wardrobe
  of the matching Hogger's camp row (entries 905040 to 905059, in the same order;
  `content/gear.toml` lines 786 to 805 show them), with the budgets the Power Curve gives
  (`build_gear.py`'s check gates them). Three name stems rotate over the slots as Hogger's, Gnoll
  and Riverpaw do: **Mistmantle**, **Tutor's** and **Candlewick** (placeholders).
- Rows 905140 to 905143 are the four Chests (cloth, leather, mail, plate), as 905040 to 905043
  are (`dungeon-chest-cloth` to `dungeon-chest-plate`).
- Regenerate `content/gear.toml` with `python3.15 tools/twilights-eve/build_gear.py`. Run
  `python3.15 -I tools/twilights-eve/icon_check.py`: the Lines share icons by ADR 0026, so no new
  icon and no new clash.
- In `crates/twilight-realm/content/mistmantle_manor.toml`: Steward Aldric (974010) and Lady
  Corrine Mistmantle (974013) get `rolls = [{ chance = 50, one_of = [905140, ..., 905159] }]`, and
  Stalvan Mistmantle (974014) `rolls = [{ chance = 100, one_of = [the same] }]`, written out in
  full. They replace ticket 04's stand-in. No epics (the gear 31-60 spec's). Leave every `drop`
  list alone.

## Failing tests first

`crates/twilight-realm/tests/dungeon_gear.rs` gains the Mistmantle pool beside the Stockade's and
the Deeprun Tram's (its header names them; add `mistmantle_manor.toml` to its file constants):

- the set's twenty entries exist in `gear.toml` at quality 3, item level 36, required level 33;
- among them, exactly four Chests, one each of cloth, leather, mail and plate;
- every boss's roll lists exactly the twenty, Stalvan's at 100 and the other two at 50;
- no stand-in remains (no 905080 to 905099 in the file).

## Acceptance

- The test passes; `cargo test -q -p twilight-realm` passes; `build_gear.py`'s check passes;
  `icon_check.py` reports no new clash; `ground_check.py` is unchanged.

## Assumptions allowed

- The stems, if `build_gear.py` refuses a name (length or a clash with an existing name): pick
  another in the dungeon's theme and record it.
