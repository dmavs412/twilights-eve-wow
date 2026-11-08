Title: Stonewatch Keep's set, the bosses' rolls and Singe's Scale (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 3 and 16. Blocked by 10-stonewatch-monsters (the bosses' rows),
09-raven-hill-loot (both edit `gear.csv`, `gear.toml` and `tests/dungeon_gear.rs`),
RD:20-gear-45-50 (the arc goes first on `gear.csv`) and RD:22-redridge-45-50 (the last arc ticket
to write `content/quest_items_redridge.toml` before this one adds a row to it).

Twenty blues and the four bosses' rolls, in ticket 05's form, and the scale Singe always drops.
The set's four Chests are the rewards of the arc's dungeon-ending quest 98511
(RD:29-stonewatch-quests).

## Data

- Twenty rows in `data/twilights-eve/gear.csv` through `sheet.py add-row`, entries 905200 to
  905219, item level 48, required level 45. Each copies the matching Hogger's camp row (905040 to
  905059, same order; `gear.toml` lines 786 to 805): slot, `line`, `display_id`, `icon`, stat
  shape and wardrobe, with the Power Curve's budgets. Rows 905200 to 905203 are the four Chests
  (cloth, leather, mail, plate). Stems rotate as Hogger's, Gnoll and Riverpaw do: **Stonewatch**,
  **Warlord's** and **Scorched** (placeholders).
- Regenerate `gear.toml` with `build_gear.py`; `icon_check.py` shows no new clash.
- In `crates/twilight-realm/content/stonewatch_keep.toml`: the Blackrock Quartermaster (974308)
  and the Dragonspawn Envoy (974313) get `rolls = [{ chance = 50, one_of = [905200, ...,
  905219] }]`; Gath'Ilzogg (974316) and Singe (974317) each `rolls = [{ chance = 100, one_of =
  [the same] }]`, written out in full, replacing ticket 10's stand-in. No epics.

## Singe's Scale (decision 16)

The one proof of the dragons the player carries out: a scale, nothing written.

- A row in `content/quest_items_redridge.toml` (made by RD:03-quest-files), under a new heading
  `# ── Stonewatch Keep (#PARENT) ──` at the end of the file:
  `{ entry = 980701, name = "Singe's Scale", quality = 1, item_level = 48, class = 12,
  display_id = <see below>, max_count = 1, description = "Black, as wide as a shield, and still
  warm. It came off something that flew." }`
- Its display: a stock item display whose icon is a dark scale (an `INV_Misc_MonsterScales_*`
  icon or the like) and that no other item uses. `python3.15 -I tools/twilights-eve/icon_check.py`
  must report no new clash (the no-duplicate-icons rule).
- Singe's creature row (974317) gets `drop = [980701]`: a fixed drop, on every Singe corpse for
  every looter (`src/world.rs` line 1860 adds a creature row's `drop` items unconditionally). No
  quest collects it yet.
- Leave Gath'Ilzogg's `drop` alone: the arc's ticket 29 adds page three to it.

## Failing tests first

`crates/twilight-realm/tests/dungeon_gear.rs` gains the Stonewatch pool (add
`stonewatch_keep.toml` to its file constants and its header):

- the twenty entries exist at quality 3, item level 48, required level 45;
- exactly four Chests among them, one each of cloth, leather, mail and plate;
- the Quartermaster's and the Envoy's rolls list exactly the twenty at 50, Gath'Ilzogg's and
  Singe's at 100;
- no stand-in remains (no 905080 to 905099 in the file).

`crates/twilight-realm/tests/stonewatch_keep.rs` (ticket 10) gains: Singe's row drops 980701, and
980701 is a known item named "Singe's Scale".

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `build_gear.py`'s check passes;
  `icon_check.py` reports no new clash; `ground_check.py` is unchanged.

## Assumptions allowed

- The stems, if `build_gear.py` refuses a name.
- If a load check refuses a class 12 item that no quest collects, make the scale class 15
  (Miscellaneous) at quality 1 and record it.
