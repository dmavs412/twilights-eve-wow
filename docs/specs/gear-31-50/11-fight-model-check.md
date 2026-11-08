Title: Fight-model check: gear on the Power Curve against the Tier Step at 33, 38, 43 and 48 (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 3. Blocked by 07-dungeon-epics (by then every 31-50 row this spec, the
arc and the dungeons spec write is in `gear.csv`: band greens, Elite blues, Quest Lines, dungeon
sets, epics).

Decision 3: gear from 31 stays on the Power Curve and does not take the Tier Step (x1.5 health and
x1.5 damage on every mob from 31, `content/mobs.toml` `[difficulty.tier_step]`, line 28); the Tier
2 Tier Bonus closes the gap. This ticket measures that gap with the balance model and writes a
report. It changes no data and sets no Tier Bonus: the Tier 2 Callings spec fits that.

## The model past 30

`tools/twilights-eve/balance/scenarios.py` caps every Calling rule at level 30 (`_levels(levels,
10, 30)` in `p_2`, `p_3`, `dps`, `e_dmg`, `e_tank`, `b_2e` and `b_boss`, and 15 to 30 in `g_5`).
Raise those caps to 50 with one module constant, so `--levels 33 38 43 48` runs them; a run of
1-30 must give the same cells as before. `model.py` already handles a Power Level above a Calling's
tier cap (`calling_level`, line 1977) and the Tier Step (line 392). The rules in
`docs/balance/rules.md` stay written for 10 to 30; past 30 the report is for reading, not a gate.

## The runs

300 fights a cell, `--jobs 4`, levels 33, 38, 43 and 48, scenarios `pulls`, `dps`, `elites` and
`blues`, and `named` (which picks up the four dungeons' final bosses at their own levels).
Callings: every Calling in `data/twilights-eve/master.csv` with a tier of 1 or 2. If the Tier 2
Callings are in the sheet when this runs, they fight with the Tier Bonus they carry; if not, the
five Tier 1 Callings fight at their cap, as the model already does past 30.

1. **As shipped**: the repo as it is.
2. **No Tier Step**: the same, with `--data-root` pointing at a copy of the repo's
   `data/twilights-eve/` and `crates/twilight-realm/content/` under `target/agent/no-step/`
   whose `mobs.toml` has `tier_step` health and damage 1.0. Write the copy with a script in
   `target/agent/` (the unattended rules allow no redirect); never edit the repo's `mobs.toml`.

Run 2 shows what gear on the curve gives against mobs on the curve; the distance from run 1 to
run 2 is what the Tier Bonus has to close.

## The report

`docs/balance/<date of the run>/gear-31-50.md`, in the style of `docs/balance/2026-10-31/`:

- the commands and the commit;
- for each rule and each level, the win rate and health left in run 1 and run 2 side by side, with
  whether the rule's 10-30 band holds;
- one paragraph per level: which rules fail in run 1 and hold in run 2, and the share of run 2's
  win rate run 1 keeps;
- the four named bosses' cells (Stalvan, Morganth, the Grey Deacon, Gath'Ilzogg with Singe if the
  model fights them together, else each alone) in both runs;
- the kits the model picked at each level (`describe_kit`), so a reader can see the band greens,
  Quest Lines, dungeon blues and epics in use;
- a closing list, "For the Tier Bonus fit": the rules that fail only because of the Tier Step,
  by level. No proposal of numbers.

## Failing tests first

`tools/twilights-eve/tests/test_balance.py` gains: with levels `[33, 48]`, `p_2`'s cells cover 33
and 48 (the cap is gone), and with levels `range(1, 31)` the cells are the same levels as before.

## Acceptance

- `~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests` passes.
- The report exists and holds both runs for all four levels.
- `git status` shows no change under `data/` or `crates/`.

## Assumptions allowed

- If a run takes over an hour, drop to 150 fights a cell and say so in the report.
- If the model cannot fight Gath'Ilzogg and Singe together, fight each alone and say so.
- If a scenario fails past 30 for a reason other than the cap (a missing table row above 30),
  fix it in the model if small and record it, else leave that scenario out and say why.
