Title: Fit the Tier 2 Tier Bonus and the Tier Step together at Power Levels 30, 40 and 50 (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 13. Blocked by 23-balance-sims-damage (all ten sims) and
19-bloodmender (the last ticket to edit `master.csv` before this one).

The ten Tier 2 Callings carry placeholder Tier Bonuses (twice their Tier 1 Calling's), and the
Tier Step on every creature of level 31 and up is a placeholder (`crates/twilight-realm/content/mobs.toml`
`[difficulty.tier_step]`, line 28: `from = 31`, `health = 1.5`, `damage = 1.5`). ADR 0012 says the
two are set together. This ticket sets them.

## The fit

1. Run `~/.local/bin/python3.15 tools/twilights-eve/balance/run.py --levels 30 40 50 --fights 300
   --callings all` and read every Tier 2 rule (`docs/balance/rules.md`, the Tier 2 section ticket
   21 wrote, H-1 included) and every Tier 1 rule at 30.
2. Search the two levers only: one Tier Step (health and damage, the same for every Calling) and
   one Tier Bonus per Tier 2 Calling (five stats, keeping each Calling's stat shape: scale the
   placeholder by one factor per Calling, then adjust one stat if a single rule still fails).
   The Tier 1 rules at Power Level 30 must still hold (the Tier Step starts at 31, so they should).
3. The smallest change that makes every Tier 2 rule hold at 30, 40 and 50 wins, as
   `docs/balance/rules.md`'s "Levers" says. When no Tier Bonus and Tier Step make a rule hold
   (an ordering rule such as DPS or E-tank that needs a skill change), keep the best fit, list
   the failing rule and the skill-side change that would fix it in the closing comment, and do
   not change skills here.
4. Write the result: each Tier 2 `calling` row's `tier_bonus` with
   `~/.local/bin/python3.15 tools/twilights-eve/sheet.py set <calling>/<calling> tier_bonus=...`,
   then `build_spells.py --data "$WOW_DATA"`; the Tier Step in `mobs.toml` with its comment
   updated (no longer a placeholder; say what it was fitted against and when).
5. Update ADR 0012's Tier Step paragraph (`docs/adr/0012-power-level-and-level-free-combat.md`,
   line 73) with the fitted numbers, as its 2026-11-01 amendment recorded the Difficulty.

## Where

- `data/twilights-eve/master.csv` (ten `calling` rows), both generated JSON files.
- `crates/twilight-realm/content/mobs.toml` (`[difficulty.tier_step]`).
- `docs/adr/0012-power-level-and-level-free-combat.md`.
- The realm reads the Tier Step through `Difficulty` in `crates/twilight-realm/src/mobs.rs`; tests
  that pin creature numbers above level 30 (search `tier_step` in `crates/twilight-realm/tests`)
  may need their expected values updated.

## Acceptance

- The closing comment holds the before and after tables at 30, 40 and 50, the chosen Tier Step
  and Tier Bonuses, and every rule that still fails with its proposed skill-side fix.
- `cargo test -q -p twilight-realm` passes; the pipeline unittests pass; `build_spells.py --check`
  is clean.

Assumptions allowed: the search method, ties between equally small fits.
