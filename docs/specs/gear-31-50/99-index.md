# Gear 31-50: index of the spec files

The parent (00) and 11 tickets (01 to 11) labelled `ready-for-agent`. Paths are under
`crates/twilight-realm/` unless they start with `data/`, `docs/` or `tools/`. "Blocked by" names a
file here; `RD:` names a ticket file of the Redridge and Duskwood arc
(`docs/specs/redridge-duskwood/`, filed as #595 to #625), `DUNGEONS:` one of the 30-50 dungeons
spec (`docs/specs/dungeons-30-50/`, filed as #637 to #649).

Two builders. Tickets 04, 05, 07 and 10 edit `data/twilights-eve/gear.csv`, `content/gear.toml`
and `data/twilights-eve/client_dbc_patch.json` and run one after another, after the arc's last
gear ticket (RD:20, #615) and the dungeons' last loot ticket (DUNGEONS:11, #648). The others edit
other files and run beside them.

| File | Title | Blocked by | Edits | Runs alongside |
|---|---|---|---|---|
| 00-parent | Gear 31-50 (spec) | | | |
| 01-gear-sheet-tool | gear_sheet.py: read, set, scale and restyle gear.csv rows by entry | | new `tools/twilights-eve/gear_sheet.py`, new `tools/twilights-eve/tests/test_gear_sheet.py`, `docs/spell_pipeline.md` | 02, 03, any arc or dungeon ticket |
| 02-zone-rolls-code | A rolls table by entry in zones.toml | RD:02-level-bands-code | `src/mobs.rs`, `content/zones.toml` (header) | 01, 03 |
| 03-bonus-chest-code | A bonus chest for a Phased Dungeon challenge | DUNGEONS:03-partner-death-link | `src/phased.rs`, `src/shard.rs`, `src/lib.rs`, `content/ward_test.toml`, `tests/phased_wards.rs`, `GLOSSARY.md` | 01, 02, 04, 05 |
| 04-band-gear-31-40 | Loot Band gear 31-40: forty World greens and twelve Elite blues | 01, RD:20-gear-45-50, DUNGEONS:11-stonewatch-loot | `gear.csv`, `gear.toml`, `client_dbc_patch.json`, new `tests/band_gear_31_50.rs` | 02, 03 |
| 05-band-gear-41-50 | Loot Band gear 41-50: forty World greens and twelve Elite blues | 04 | `gear.csv`, `gear.toml`, `client_dbc_patch.json`, `tests/band_gear_31_50.rs` | 02, 03 |
| 06-loot-bands-31-50 | Loot Bands 31-35 to 46-50 in loot_bands.toml, with no whites | 05 | `content/loot_bands.toml`, `tests/northshire_loot.rs`, `src/looting.rs` (a test), `GLOSSARY.md` | 07, 08 |
| 07-dungeon-epics | Dungeon epics: twelve epics on the four 30-50 final bosses at 5% | 05, DUNGEONS:05-mistmantle-loot, DUNGEONS:07-ilgalar-loot, DUNGEONS:09-raven-hill-loot, DUNGEONS:11-stonewatch-loot | `gear.csv`, `gear.toml`, `client_dbc_patch.json`, `content/mistmantle_manor.toml`, `content/ilgalar.toml`, `content/raven_hill_deeps.toml`, `content/stonewatch_keep.toml` (final boss templates), `tests/dungeon_gear.rs` | 06, 08 |
| 08-elite-rolls-31-50 | Elite rolls in Redridge and Duskwood | 02, 05, RD:21-bands-45-50 | `content/zones.toml` (`[[zone.rolls]]`), `src/mobs.rs` (a test) | 06, 07 |
| 09-bonus-chests | The four bonus chests | 03, 07, DUNGEONS:05-mistmantle-loot, DUNGEONS:07-ilgalar-loot, DUNGEONS:09-raven-hill-loot, DUNGEONS:11-stonewatch-loot | the four dungeon files (`[bonus_chest]`), `tests/dungeon_gear.rs`, `tests/ilgalar.rs`, `docs/art-assets.md` | 10, 11 |
| 10-looks-31-50 | Looks that follow item level for the Quest Lines, the four dungeon sets and every 31-50 plate Chest | 07, RD:20-gear-45-50, DUNGEONS:11-stonewatch-loot | `gear.csv`, `gear.toml`, `client_dbc_patch.json`, new `tests/looks_31_50.rs` | 08, 09, 11 |
| 11-fight-model-check | Fight-model check: gear on the Power Curve against the Tier Step at 33, 38, 43 and 48 | 07 | `tools/twilights-eve/balance/scenarios.py`, `tools/twilights-eve/tests/test_balance.py`, new `docs/balance/<date>/gear-31-50.md` | 08, 09, 10 |

## Gates

- Every ticket: `cargo test -q -p twilight-realm 2>&1 | grep -E "FAILED|panicked|test result"`
  when it touches the realm, `data/` or `tools/`.
- A ticket that touches `crates/twilight-realm/content/` also runs `ground_check.py` and
  `icon_check.py`.
- A `gear.csv` ticket runs `build_gear.py`, `build_spells.py --data "$WOW_DATA"`,
  `icon_check.py`, `gear_sheet.py look-check` on its rows, and the pipeline unittests.
- A `tools/` ticket runs `~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests`.

## Counts

- New `gear.csv` rows: 80 World greens + 24 Elite blues + 12 epics = 116.
- New icons: 12 (the epics). Painted icons: 0.
- New Chest looks: 16 (this spec's band Chests, the 4 plate ones handed out in 10) + 16 (Quest
  Line greens) + 4 (Tharil'zun's blues) + 16 (dungeon sets) = 52.
- Elite roll rows: 18. Bonus chests: 4, plus the Ward Test's.
