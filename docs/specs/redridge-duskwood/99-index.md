# Redridge and Duskwood 30-50: index of the spec files

The parent (00), one stub spec to grill (01) and 29 tickets (02 to 30) labelled
`ready-for-agent`. Paths are under `crates/twilight-realm/` unless they start with `data/`,
`docs/` or `tools/`. "Blocked by" names a file here; `#593` is the open Deeprun ticket that also
edits `hub.toml` and `quests_stormwind.toml`; `DUNGEONS:` names the 30-50 dungeons spec's tickets
for that dungeon (its creatures and its loot), not filed yet.

| File | Title | Blocked by | Edits | Runs alongside |
|---|---|---|---|---|
| 00-parent | Redridge and Duskwood 30-50 arc (spec) | | | |
| 01-escort-spec | Escort quests (to grill), stub, no label | | nothing until grilled | anything |
| 02-level-bands-code | Level Bands for Redridge and Duskwood: a band file per zone | | `src/bands.rs`, `src/mobs.rs`, `src/build_id.rs`, new `content/redridge_bands.toml` and `content/duskwood_bands.toml`, `content/zones.toml` (header) | 05 |
| 03-quest-files | Six quest and quest-item files for Redridge and Duskwood | 02 | `src/quests/content.rs`, `src/build_id.rs`, `content/quests.toml` (header), six new content files, `tests/content_hub.rs`, `tests/seam.rs`, `tests/content_quests.rs`, `docs/art-assets.md` | 04, 05, 06 |
| 04-dungeon-levels-whelps | Dungeon levels from the bible, and the whelps leave Lake Everstill | 02 | `content/raven_hill_deeps.toml`, `content/stonewatch_keep.toml`, `content/zones.toml`, `src/phased.rs` (tests), `tests/content_zones.rs` | 03, 05, 06 |
| 05-gear-30-35 | Quest Lines and the 30-35 quest gear | | `data/twilights-eve/gear.csv`, `content/gear.toml`, new `tests/quest_gear_30_50.rs` | 02, 03, 04, 06 |
| 06-bands-30-35 | Level Bands 30-35 | 02 | `content/redridge_bands.toml`, `content/duskwood_bands.toml`, `src/bands.rs` (tests) | 03, 04, 05 |
| 07-redridge-30-35 | Redridge 30-35 | 03, 05, 06 | `content/quests_redridge.toml`, `content/quest_items_redridge.toml`, new `tests/redridge_30_50.rs` | 08, 10, 11 |
| 08-duskwood-30-35 | Duskwood 30-35 | 03, 05, 06 | `content/quests_duskwood.toml`, `content/quest_items_duskwood.toml`, new `tests/duskwood_30_50.rs` | 07, 09, 10, 11 |
| 09-stormwind-30-35 | Stormwind 30-35: the way in | 07, #593 | `content/quests_stormwind.toml`, `content/quest_items.toml`, new `tests/stormwind_30_50.rs` | 08, 10, 11, 12 |
| 10-gear-35-40 | The 35-40 quest gear | 05 | `gear.csv`, `gear.toml`, `tests/quest_gear_30_50.rs` | 07, 08, 09, 11 |
| 11-bands-35-40 | Level Bands 35-40 and the Grey Robes | 04, 06 | band files, `src/bands.rs` (tests), `content/mobs.toml`, `content/zones.toml`, `content/level_overrides.toml` | 07, 08, 09, 10 |
| 12-redridge-35-40 | Redridge 35-40: Toren at the inn | 07, 10, 11, #593 | `quests_redridge.toml`, `quest_items_redridge.toml`, `content/hub.toml`, `tests/redridge_30_50.rs` | 13, 14, 15, 16 |
| 13-duskwood-35-40 | Duskwood 35-40: the Grey Robes | 08, 10, 11 | `quests_duskwood.toml`, `quest_items_duskwood.toml`, `tests/duskwood_30_50.rs` | 12, 14, 15, 16 |
| 14-stormwind-35-40 | Stormwind 35-40 | 08, 09, 10 | `quests_stormwind.toml`, `quest_items.toml`, `tests/stormwind_30_50.rs` | 12, 13, 15, 16 |
| 15-gear-40-45 | The 40-45 quest gear | 10 | `gear.csv`, `gear.toml`, `tests/quest_gear_30_50.rs` | 12, 13, 14, 16 |
| 16-bands-40-45 | Level Bands 40-45 and the Surveyors | 11 | band files, `src/bands.rs` (tests), `mobs.toml`, `zones.toml`, `level_overrides.toml` | 12, 13, 14, 15 |
| 17-redridge-40-45 | Redridge 40-45: Tharil'zun's orders | 12, 15, 16 | `quests_redridge.toml`, `quest_items_redridge.toml`, `tests/redridge_30_50.rs` | 18, 20, 21 |
| 18-duskwood-40-45 | Duskwood 40-45: a machine that went wrong | 13, 15, 16 | `quests_duskwood.toml`, `quest_items_duskwood.toml`, `tests/duskwood_30_50.rs` | 17, 19, 20, 21 |
| 19-stormwind-40-45 | Stormwind 40-45: the same desk | 14, 17 | `quests_stormwind.toml`, `quest_items.toml`, `tests/stormwind_30_50.rs` | 18, 20, 21, 22, 23 |
| 20-gear-45-50 | The 45-50 quest gear | 15 | `gear.csv`, `gear.toml`, `tests/quest_gear_30_50.rs` | 17, 18, 19, 21 |
| 21-bands-45-50 | Level Bands 45-50, the camp ground, the agents and the courier | 16 | band files, `src/bands.rs` (tests), `mobs.toml`, `zones.toml` | 17, 18, 19, 20 |
| 22-redridge-45-50 | Redridge 45-50: the Stonewatch Falls camp | 17, 20, 21 | `quests_redridge.toml`, `quest_items_redridge.toml`, `hub.toml`, `tests/redridge_30_50.rs` | 19, 23, 24, 26 to 28 |
| 23-duskwood-45-50 | Duskwood 45-50: the agents' camp and Mor'Ladim | 18, 20, 21 | `quests_duskwood.toml`, `quest_items_duskwood.toml`, `tests/duskwood_30_50.rs` | 19, 22, 24 to 27 |
| 24-stormwind-45-50 | Stormwind 45-50: relief that never came | 19, 20 | `quests_stormwind.toml`, `quest_items.toml`, `tests/stormwind_30_50.rs` | 22, 23, 25 to 30 |
| 25-keeshan | Keeshan: Missing in Action | 01, 22 | `quests_redridge.toml`, `hub.toml`, the escort spec's files as its tickets left them, `tests/redridge_30_50.rs` | 23, 24, 26, 27, 28 |
| 26-mistmantle-quests | Mistmantle Manor's quests | 03, 05, 08, DUNGEONS:mistmantle-monsters, DUNGEONS:mistmantle-loot | `content/quests_duskwood_dungeons.toml`, `content/mistmantle_manor.toml` (one `drop`), new `tests/duskwood_dungeon_quests.rs` | 09 to 25, 27, 29 |
| 27-ilgalar-quests | The Tower of Ilgalar's quests | 10, 12, DUNGEONS:ilgalar-monsters, DUNGEONS:ilgalar-loot | `content/quests_redridge_dungeons.toml`, `content/ilgalar.toml` (one `drop`), new `tests/redridge_dungeon_quests.rs` | 13 to 26, 28 |
| 28-raven-hill-deeps-quests | Raven Hill Deeps' quests | 12, 15, 18, 26, DUNGEONS:raven-hill-monsters, DUNGEONS:raven-hill-loot | `quests_duskwood_dungeons.toml`, `content/raven_hill_deeps.toml` (one `drop`), `tests/duskwood_dungeon_quests.rs` | 22 to 25, 27, 29 |
| 29-stonewatch-quests | Stonewatch Keep's quests | 20, 22, 27, DUNGEONS:stonewatch-monsters, DUNGEONS:stonewatch-loot | `quests_redridge_dungeons.toml`, `content/stonewatch_keep.toml` (one `drop`), `tests/redridge_dungeon_quests.rs` | 23 to 26, 28 |
| 30-toren-unsent-letter | The Unsent Letter | 22, 28, 29 | `quests_redridge_dungeons.toml`, `tests/redridge_dungeon_quests.rs` | 23 to 26 |

## Notes for the filer

- Substitute `#PARENT` with the parent's number, and each "Blocked by NN-name" with that ticket's
  number. Add GitHub dependency links too if you like; the builders read both.
- The `DUNGEONS:` blockers do not exist yet. Until the 30-50 dungeons spec files its tickets,
  leave tickets 26 to 30 unfiled, or file them with a "Blocked by" on the dungeons spec's parent
  issue, which closes only when its tickets land.
- This arc goes first on `gear.csv`. The gear 31-60 spec's tickets, and the 30-50 dungeons spec's
  loot tickets, that edit `gear.csv` must be blocked by 20-gear-45-50, this arc's last gear ticket.
  Each band's gear ticket (05, 10, 15, 20) comes before that band's quest tickets, which name its
  rows in `choices`.
- The four dungeon-ending quests (99001, 98501, 99011, 98511) offer their dungeon set's four
  Chests, so they stay blocked by the dungeons spec's loot tickets as well as its monster tickets.
- The dungeons spec can drop its X3 (ticket 04 sets Raven Hill Deeps to 40-44 and Stonewatch Keep
  to 45-48) and should block its Raven Hill and Stonewatch tickets on 04.
- The Steppes opening spec's break quest names 98521 in `after` and Toren 190083 as giver. In that spec the break
  no longer opens the land: it starts a server-wide progress bar, filled by Writs and bounties at a
  Blackrock Pass camp, and the Steppes open when it fills. Nothing in this arc says otherwise.
- Davis reviews the quoted spine text (Toren's lines, the letter in 22, the Stormwind hand-offs)
  before filing; the builders paste it as written.
- When the arc lands: the lore bible's open question "Where Toren lives after his rescue, and who
  protects him" has an answer (Lakeshire's inn; Marris's Watch on his stair) for the live doc.
