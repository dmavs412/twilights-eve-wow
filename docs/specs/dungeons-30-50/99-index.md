# The 30-50 dungeons: index of the spec files

The parent (00) and 12 tickets (01 to 12) labelled `ready-for-agent`. Paths are under
`crates/twilight-realm/` unless they start with `crates/`, `data/`, `docs/` or `tools/`. "Blocked
by" names a file here; `RD:` names a ticket file of the Redridge and Duskwood arc
(`docs/specs/redridge-duskwood/`), `SO:` one of the Steppes opening spec
(`docs/specs/steppes-opening/`).

| File | Title | Blocked by | Edits | Runs alongside |
|---|---|---|---|---|
| 00-parent | The 30-50 dungeons (spec) | | | |
| 01-dungeon-templates | Templates in dungeon files, spell sections per dungeon, and a lore-bible test that accepts a filled dungeon | RD:02-level-bands-code, RD:04-dungeon-levels-whelps, SO:03-milestone-gated-content | `src/phased.rs`, `src/mobs.rs`, `tests/common/mod.rs`, `content/mobs.toml` (header), `content/creature_spells.toml` (four sections at the end), `data/twilights-eve/client_dbc_patch.json`, `docs/art-assets.md` (a section before the Deeprun Tram's), a new test | arc tickets that leave `src/mobs.rs` and `src/phased.rs` alone |
| 02-frozen-until | Frozen until a boss's health mark: a dungeon creature key, and the tram moved onto it | 01 | `src/phased.rs`, `src/mobs.rs`, `src/shard.rs`, `src/tram.rs`, a system under `src/world/`, `content/ward_test.toml`, `content/deeprun_tram.toml` (a comment), `docs/adr/0033-tram-waves-and-placed-mist-walls.md`, `tests/phased_wards.rs` | 04, 05, 06, 07 |
| 03-partner-death-link | A partner-death link: the survivor enrages | 02 | `src/phased.rs`, `src/mobs.rs`, a system under `src/world/`, `content/ward_test.toml`, `content/creature_spells.toml` (header, a test row after Stonewatch's section), `client_dbc_patch.json`, `tests/phased_wards.rs` | 04, 05, 06, 07, 08 |
| 04-mistmantle-monsters | Mistmantle Manor: the yard, the steward, the lady and Stalvan | 01, RD:11-bands-35-40 | `content/mistmantle_manor.toml`, `content/zones.toml` (Duskwood's `removed_guids`), `content/creature_spells.toml` (Mistmantle section), `client_dbc_patch.json`, new `tests/mistmantle_manor.rs` | 02, 03, 06, 08, 10 |
| 05-mistmantle-loot | Mistmantle Manor's set: twenty blues and the bosses' rolls | 04, RD:20-gear-45-50 | `data/twilights-eve/gear.csv`, `content/gear.toml`, `content/mistmantle_manor.toml` (rolls), `tests/dungeon_gear.rs` | 02, 03, 06, 08, 10, 12 |
| 06-ilgalar-monsters | The Tower of Ilgalar: the valves, the overseer and Morganth | 01 | `content/ilgalar.toml`, `content/azora.toml` (a comment), `content/quests_elwynn.toml` (line 981), `content/creature_spells.toml` (Ilgalar section), `client_dbc_patch.json`, new `tests/ilgalar.rs` | 02, 03, 04, 05, 08, 10 |
| 07-ilgalar-loot | The Tower of Ilgalar's set: twenty blues and the bosses' rolls | 06, 05, RD:20-gear-45-50 | `gear.csv`, `gear.toml`, `content/ilgalar.toml` (rolls), `tests/dungeon_gear.rs` | 02, 03, 04, 08, 10, 12 |
| 08-raven-hill-monsters | Raven Hill Deeps: Felstrom, the dig face and the Grey Deacon | 02, RD:04-dungeon-levels-whelps | `content/raven_hill_deeps.toml`, `data/twilights-eve/mist_walls.json`, `content/creature_spells.toml` (Raven Hill section), `client_dbc_patch.json`, new `tests/raven_hill_deeps.rs` | 03, 04, 05, 06, 07, 10 |
| 09-raven-hill-loot | Raven Hill Deeps' set: twenty blues and the bosses' rolls | 08, 07, RD:20-gear-45-50 | `gear.csv`, `gear.toml`, `content/raven_hill_deeps.toml` (rolls), `tests/dungeon_gear.rs` | 04, 06, 10, 12 |
| 10-stonewatch-monsters | Stonewatch Keep: the quartermaster, the envoy, Gath'Ilzogg and Singe | 03, RD:04-dungeon-levels-whelps | `content/stonewatch_keep.toml`, `content/creature_spells.toml` (Stonewatch section), `client_dbc_patch.json`, new `tests/stonewatch_keep.rs` | 04, 05, 06, 07, 08, 09 |
| 11-stonewatch-loot | Stonewatch Keep's set, the bosses' rolls and Singe's Scale | 10, 09, RD:20-gear-45-50, RD:22-redridge-45-50 | `gear.csv`, `gear.toml`, `content/stonewatch_keep.toml` (rolls, Singe's `drop`), `content/quest_items_redridge.toml`, `tests/dungeon_gear.rs`, `tests/stonewatch_keep.rs` | 12 |
| 12-mourncrown-grove | Mourncrown over Twilight Grove, gated on a capture | RD:23-duskwood-45-50, SO:07-client-opening | `crates/benilla-assets/src/adt.rs`, `crates/benilla-app/src/capture/fixtures.rs`, `crates/benilla-app/src/capture/scenarios.rs`, `tools/twilights-eve/wmo_floor.py` and a test under `tools/twilights-eve/tests/`, `content/quests_duskwood.toml` (a row after the header), a realm test | everything here; an interactive Build chat only |

## What the arc's DUNGEONS: placeholders mean

The arc's tickets 26 to 29 say "Blocked by DUNGEONS:<name>". Each maps to one file here:

| Placeholder | This spec's file |
|---|---|
| DUNGEONS:mistmantle-monsters | 04-mistmantle-monsters |
| DUNGEONS:mistmantle-loot | 05-mistmantle-loot |
| DUNGEONS:ilgalar-monsters | 06-ilgalar-monsters |
| DUNGEONS:ilgalar-loot | 07-ilgalar-loot |
| DUNGEONS:raven-hill-monsters | 08-raven-hill-monsters |
| DUNGEONS:raven-hill-loot | 09-raven-hill-loot |
| DUNGEONS:stonewatch-monsters | 10-stonewatch-monsters |
| DUNGEONS:stonewatch-loot | 11-stonewatch-loot |

## What runs in parallel

- 01, 02 and 03 are one chain: each edits `src/phased.rs`, and 02 and 03 share
  `content/ward_test.toml` and `tests/phased_wards.rs`.
- After 01, the monster tickets run two at a time: 04 and 06 at once, then 08 (after 02) and 10
  (after 03). Each writes its own dungeon file, its own new test file and its own section of
  `creature_spells.toml`. They all regenerate `client_dbc_patch.json`, but ticket 01's first row
  in each section keeps each dungeon's client rows apart in it, so two builders' edits merge
  without a conflict.
- The loot tickets are one chain on `gear.csv`, `gear.toml` and `tests/dungeon_gear.rs`: 05, 07,
  09, 11, all after the arc's last gear ticket (RD:20). Each also waits for its own dungeon's
  monster ticket, and runs beside the other dungeons' monster tickets.
- 12 touches no file another ticket here touches, and needs an interactive Build chat for its
  captures.

## Notes for the filer

- Substitute `#PARENT` with the parent's number, each "Blocked by NN-name" with that ticket's
  number, and each `RD:` and `SO:` name with that ticket's number once filed. Add GitHub
  dependency links too if you like; the builders read both.
- In the arc's tickets 26 to 29, replace each `DUNGEONS:` placeholder with the number of the
  ticket in the table above.
- **SO:05-anchor-break** edits `content/ilgalar.toml` too. File it with "Blocked by
  06-ilgalar-monsters". Its text already handles an Anchor row that exists: 06 places the whole
  Anchor as guid 794101 at SO:05's position and display, and SO:05 adds `until_milestone` and the
  broken Anchor.
- **RD:08-duskwood-30-35**'s quest 98607 explores the old Mistmantle Portal at the front door
  (-10362.6, -1257.0). Ticket 04 moves the Portal to the edge of a walled yard, so that point is
  inside the wall. Point 98607's explore at 04's new Portal (its first cut is (-10345, -1184);
  the builder records the final spot), or file RD:08 after 04 and let it read the file.
- **RD:27-ilgalar-quests** adds both the failure notes (980381) and Letters Under a Crowned Seal
  (980382) to Morganth's `drop`. The grill's decision 8 says Morganth carries only the failure
  notes. Decide whether the letters stay on him before filing RD:27.
- The arc's tickets 26 to 29 own the page drops on the bosses. This spec's tickets leave those
  `drop` lists empty and name each boss's guid: Stalvan 974014, Morganth 974114, the Grey Deacon
  974213, Gath'Ilzogg 974316.
- The gear 31-60 draft reserves the same rows (905140 to 905219) for these four sets and plans to
  write the pools. Those rows are this spec's. The gear spec's tickets that edit `gear.csv` should
  come after 11 (or before 05) so the two chains do not interleave.
- Other files these tickets share with the sibling specs, where a clash would only cost a
  requeue: `content/zones.toml` (04; the arc's 16 and 21 append Duskwood rows elsewhere in it),
  `docs/art-assets.md` (01; the arc's 03 and the Steppes 05 and 06 add sections elsewhere).
