# Tier 2 Callings: index of the spec files

The parent (00) and 24 tickets (01 to 24) labelled `ready-for-agent`. Paths are under
`crates/twilight-realm/` unless they start with `crates/`, `data/`, `docs/`, `tools/` or
`GLOSSARY.md`. "Blocked by" names a file here; `RD:` names a ticket file of the Redridge and
Duskwood arc (`docs/specs/redridge-duskwood/`). No ticket waits on the 30-50 dungeons spec: the
one dungeon trainer stands in the Riot, which is built.

| File | Title | Blocked by | Edits | Runs alongside |
|---|---|---|---|---|
| 00-parent | Tier 2 Callings, hidden trainers and the level 30 Calling Choice (spec) | | | |
| 01-calling-choice-per-tier | The Calling Choice at every Tier Cap: a per-Tier record, Tier 1 options, and Zealot in the popup | | `src/calling_choice.rs`, `src/characters.rs`, `src/save.rs`, `src/quests/content.rs`, `content/quests.toml` (64003), `data/twilights-eve/callings.csv` (zealot), both generated JSON files, `docs/adr/0028-...md`, `GLOSSARY.md`, `tests/calling_choice.rs`, `tests/saving_quests.rs` | 03 to 07, 09, 21 |
| 02-trainer-search | Hidden trainers, hint lines and trials | 01 | `src/hub.rs`, `src/map_marks.rs`, `src/quests/content.rs`, `src/quests/mod.rs`, `src/quests/dialogs.rs`, `src/characters.rs`, `src/save.rs`, `content/quests.toml` (header, a fixture), `content/hub.toml` (header, a fixture), `GLOSSARY.md`, new `tests/trainer_search.rs` | 03 to 07, 09, 21 |
| 03-holy-tank-kinds | Holy tank kinds: judging Seal of Iron, a threat aura, creature-type bonuses and per-target stacks | | `src/callings.rs`, `src/world/cast.rs`, `tools/twilights-eve/build_spells.py`, `tools/twilights-eve/tests/test_build_spells.py`, `docs/spell_pipeline.md`, `docs/agents/calling-script-guide.md`, new `tests/holy_tank_kinds.rs` | 01, 02, 09, 10, 21 |
| 04-blade-kinds | Blade kinds: bleed damage taken, ticking bleeds now, spending your own stacks, spilling droplets and Vanish | 03 | as 03, plus threat code (`src/rules/threat.rs`, `src/shard.rs`); new `tests/blade_kinds.rs` | 01, 02, 09, 10, 11, 21 |
| 05-aura-copies | Aura copies: copy an aura with its stacks, and spread it when its wearer dies | 04 | as 03; new `tests/aura_copies.rs` | 01, 02, 09, 10 to 13, 21 |
| 06-mana-kinds | Mana kinds: damage by a full bar, and damage paid by the mana spent | 05 | as 03; new `tests/mana_kinds.rs` | 01, 02, 09, 10 to 14, 21 |
| 07-trap-springs | Trap springs: a trap that catches a group, and a shot that sets off your traps | 06 | as 03; `tests/ground_area.rs` | 01, 02, 09, 10 to 15, 21 |
| 08-resurrect-and-blood-heals | Resurrection, and a heal over time paid by the health a cast cost | 07, 02 | as 03, plus `src/world/death.rs`, `src/lib.rs` (`RealmCommand`), `crates/benilla-app/src/twilight/mod.rs` (one line and a test); new `tests/resurrect.rs` | 09, 10 to 16, 21 |
| 09-client-choice-window | The Calling Choice window at Tier 1's cap, and a ten-row Talents panel | | `crates/benilla-app/assets/ui/CallingChoice.xml`, `crates/benilla-app/assets/ui/Talents.xml` (if needed), `crates/benilla-app/src/ui_script/calling_choice_tests.rs`, `crates/benilla-app/src/ui_script/talent_rows_tests.rs` | everything (a client build waits on the machine-wide client lock, not on files) |
| 10-sharpshooter | Sharpshooter | 01, 02 | `data/twilights-eve/master.csv`, `data/twilights-eve/callings.csv`, both generated JSON files, `content/hub.toml`, `content/quests.toml`, `docs/art-assets.md`, new `tests/tier2_sharpshooter.rs` | 03 to 09, 21 |
| 11-ironclad | Ironclad | 03, 10 | as 10, plus Wray's row in `hub.toml` and 64002, `content/quest_items.toml`; new `tests/tier2_ironclad.rs` | 04 to 09, 21 |
| 12-zealot | Zealot rebuilt as a holy tank | 03, 11 | as 11, plus `content/callings.toml` (Zealot removed), the old Zealot tests (`tests/zealot_skills.rs`, `tests/rules_seals.rs`, `tests/advancement.rs`, `tests/content_callings.rs`, `tests/tier1_skills.rs`, `tests/rules_power.rs`, `tests/tier_xp_rate.rs`, `tests/common/mod.rs`); new `tests/tier2_zealot.rs` | 05 to 09, 21 |
| 13-bloodletter | Bloodletter | 04, 12 | as 11; new `tests/tier2_bloodletter.rs` | 06 to 09, 21 |
| 14-nightblade | Nightblade | 04, 13 | as 11, but `content/mobs.toml` (template 97290) and `content/stockade_riot.toml` (one spawn) in place of `hub.toml`; new `tests/tier2_nightblade.rs` | 06 to 09, 21 |
| 15-pyromancer | Pyromancer | 05, 14 | as 11; new `tests/tier2_pyromancer.rs` | 07 to 09, 21 |
| 16-leyweaver | Leyweaver | 06, 15 | as 10; new `tests/tier2_leyweaver.rs` | 08, 09, 21 |
| 17-trapper | Trapper | 07, 16 | as 11; new `tests/tier2_trapper.rs` | 08, 09, 21 |
| 18-blightcaller | Blightcaller | 05, 08, 17 | as 11; new `tests/tier2_blightcaller.rs` | 09, 21 |
| 19-bloodmender | Bloodmender | 08, 18 | as 10; new `tests/tier2_bloodmender.rs` | 09, 21, 22 (once 11, 12 and 18 are in) |
| 20-trainer-deliveries | Each Tier 2 trainer sends the character on to Lakeshire and Darkshire | RD:09-stormwind-30-35, 19 | `content/quests_stormwind.toml`, `content/quest_items.toml`, `tests/stormwind_30_50.rs` | 21 to 24; not RD:14, RD:19 or RD:24 (same files) |
| 21-balance-framework | The balance model to Power Level 50 | RD:20-gear-45-50 | `tools/twilights-eve/balance/model.py`, `run.py`, `scenarios.py`, `tools/twilights-eve/tests/test_balance.py`, `docs/balance/rules.md` | every realm, client and data ticket here |
| 22-balance-sims-sturdy | Balance sims: Ironclad, Zealot, Blightcaller and Bloodmender | 21, 11, 12, 18, 19 | `balance/model.py`, `tools/twilights-eve/tests/test_balance.py` | 20 |
| 23-balance-sims-damage | Balance sims: Bloodletter, Nightblade, Pyromancer, Leyweaver, Sharpshooter and Trapper | 22, 10, 13 to 17 | as 22 | 20 |
| 24-tier-fit | Fit the Tier 2 Tier Bonus and the Tier Step | 23, 19 | `master.csv` (ten `calling` rows), both generated JSON files, `content/mobs.toml` (`[difficulty.tier_step]`), `docs/adr/0012-...md`, realm tests that pin creature numbers above 30 | 20 |

## Why the Calling tickets run in a chain

All ten Calling tickets (10 to 19) append to `master.csv` and rebuild both generated JSON files
whole, take `spell_id`s from the top of the sheet, and add rows to `callings.csv`, `quests.toml`
and `hub.toml` (14 to `mobs.toml` and `stockade_riot.toml` instead). Two at once would collide on
the JSON and on the ids, so each is blocked by the one before. The order puts the Calling that
needs no new realm kind first (Sharpshooter), then follows the realm kinds as they land:
Ironclad and Zealot after 03, Bloodletter and Nightblade after 04, Pyromancer after 05,
Leyweaver after 06, Trapper after 07, Blightcaller and Bloodmender after 08.

## Notes for the filer

- Substitute `#PARENT` with the parent's number, and each "Blocked by NN-name" with that ticket's
  number. Add GitHub dependency links too if you like; the builders read both.
- File the parent as the replacement for #556, then close #556 with a comment that points to it
  ("Closed into #<parent>, grilled 2026-11-08").
- `RD:09-stormwind-30-35` and `RD:20-gear-45-50` are the Redridge and Duskwood arc's ticket files.
  If they are not filed when this spec is, file 20 and 21 with a "Blocked by" on the arc's parent
  issue, or hold them until those two exist.
- #555 and #558 stay open as their own specs; nothing here overlaps them.
- Ticket 12 deletes four rows from `master.csv` (the old Zealot's Seal of Command, Seal of the
  Crusader, Mortal Strike and Charge). It says how; it is the only ticket that removes sheet rows.
