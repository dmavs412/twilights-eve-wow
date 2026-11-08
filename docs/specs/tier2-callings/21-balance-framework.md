Title: The balance model to Power Level 50: Tier 2 levels, gear and rules, with a healing rule (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 10 and 13. Blocked by RD:20-gear-45-50 (the Redridge and Duskwood
arc's last gear ticket, `docs/specs/redridge-duskwood/20-gear-45-50.md`; `gear.csv` stops at item
level 42 until the arc's gear tickets land).

`tools/twilights-eve/balance/` judges levels 1 to 30 (`docs/balance/rules.md`). Ticket 24 fits
the Tier 2 Tier Bonus and the Tier Step (`content/mobs.toml` `[difficulty.tier_step]`, line 28,
"a placeholder until Tier 2's Tier Bonus exists to fit it against") at Power Levels 30, 40 and
50. This ticket grows the model's frame to 50; tickets 22 and 23 add the ten Tier 2 sims.

## What changes

- **Levels.** The model's level axis is the Power Level. `power_level` and `calling_level`
  (`balance/model.py` lines 1967 and 1977) already map a Tier's levels onto its band from
  `tier_cap`; check they give Tier 2 real level 1 at Power Level 30 and real level 50 at 50, as
  the realm's `rules/stats.rs` does, and fix them if not. `open_rows` (line 1987) opens Tier 2
  rows by real level. `run.py`'s `--levels` default stays `1-30`; `--levels 31-50` (or `30-50`)
  runs Tier 2. At Power Level 30 itself both a capped Tier 1 Calling and a fresh Tier 2 Calling
  are judged.
- **Mobs.** Above 30 the mobs take the Tier Step from `mobs.toml` as `mob_stats` (line 384)
  reads it today. "On level" above 30 means a creature in the Redridge Zone Factor (Duskwood's is
  the same, `mobs.toml` line 74); write that in the rules.
- **Gear.** `kit` (line 172) picks greens and blues from `gear.csv` by level; with the arc's rows
  to item level 50 it reaches 50. Where a slot has no row at a level, the report says which (the
  `fell_back` list already does this for blues).
- **Rules.** `docs/balance/rules.md` gains a "Tier 2" section with the same rules at Power Levels
  30 to 50 for the ten Tier 2 Callings: P-2 and P-3 for all ten; DPS and E-dmg for the damage
  pairs (Bloodletter, Nightblade, Pyromancer, Leyweaver, Sharpshooter, Trapper); E-tank for the
  sturdy pairs (Ironclad, Zealot, Blightcaller, Bloodmender); B-2E, B-boss and G+5 for all ten;
  B-named and G+5-named against every `boss = true` template of a 30-50 dungeon at its level.
  Assumed: the thresholds stay Tier 1's.
- **A healing rule, H-1.** Blightcaller and Bloodmender are the main healers (decision 10). H-1:
  each healer, in greens at each level, keeps a dummy friend with the health of an on-level
  character in greens alive for 60 s against the damage one elite on level deals it, with at
  least 30% health left at the end, in at least 80% of 300 fights, while the healer itself is
  not attacked. The healer uses its kit on mobs as it would (its heals reach the friend through
  `ally=lowest_health`, `heal_allies_pct`, ground areas and shields). Write H-1 into the rules
  table. Ticket 22 implements the healers' half.
- **The report** (`run.py`'s `report`, line 50) groups Tier 2 rows apart from Tier 1's.

## Where

- `tools/twilights-eve/balance/model.py`, `run.py`, `scenarios.py` (`t0_1` at line 211 shows how a
  rule is a function over cells).
- `docs/balance/rules.md`.
- Tests: `tools/twilights-eve/tests/test_balance.py`.

## Failing tests first

In `tools/twilights-eve/tests/test_balance.py`, in the form of its tests (they read a data root;
use a fixture with a Tier 2 Calling if the shipped data has none yet, as the file builds its
fixtures):

- `power_level(2, 1) == 30` and `power_level(2, 50) == 50`; `calling_level` of a Tier 2 Calling at
  Power Level 40 is the real level the realm gives.
- `mob_stats(31, ...)` includes the Tier Step and `mob_stats(30, ...)` does not.
- A run with `--levels 31-50` and no Tier 2 sim yet lists each Tier 2 Calling as "no sim" and
  fails nothing else.
- H-1's cell runs for a fixture healer and reports win and health left.

## Acceptance

- `~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests` passes.
- `docs/balance/rules.md` has the Tier 2 section and H-1.

Assumptions allowed: H-1's numbers, the Redridge Zone Factor as "on level" above 30, how a level
with no gear row falls back.
