Title: Balance sims for the damage Tier 2 Callings: Bloodletter, Nightblade, Pyromancer, Leyweaver, Sharpshooter and Trapper (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 7, 8, 9 and 13. Blocked by 22-balance-sims-sturdy (same file,
`balance/model.py`), 10-sharpshooter, 13-bloodletter, 14-nightblade, 15-pyromancer,
16-leyweaver and 17-trapper.

Six `Sim` subclasses in `tools/twilights-eve/balance/model.py`, in the form of the Tier 1 sims
of their Line (`Cutthroat` line 914, `Initiate` line 1100, `Ranger` line 1326), registered beside
them.

## What each sim plays

Read the numbers from `data/twilights-eve/server_spells.json`, never from this ticket.

- **Bloodletter.** Hemorrhage as the builder; Rupture at 5 points when it is down, Open Veins at 5
  points when two bleeds are up, Garrote kept up; Blood Fan on a pull of 2 or more; droplets
  picked up as the Cutthroat sim picks them up; Red Frenzy at 5 Bloodlust stacks; Blood Pool on
  cooldown; Kick on a cast.
- **Nightblade.** Opens from Stealth with Ambush (from behind; the sim may assume the position,
  as the Cutthroat sim does for its openers); Backstab as the builder; Eviscerate at 5 points;
  Vanish on cooldown and Ambush again from it; Cold Blood before an Eviscerate at 5 points; Kidney
  Shot on a cast it cannot Kick.
- **Pyromancer.** Searing Ember to open, Fireball as filler, Pyroblast on cooldown, Catalyst at 3
  Ember stacks; on a pull of 2 or more Kindle once the target has 3 stacks, Flamestrike and Blast
  Wave on cooldown; Counterspell on a cast.
- **Leyweaver.** Ley-Spark while the bar is above 80% after the cost, Arcane Missiles otherwise;
  Arcane Surge from a full bar; Ley Tap when the bar is under 30% and the mob is not in melee (the
  sim may assume range, as the Initiate sim does); Attunement on cooldown; Counterspell on a cast;
  Ley Bind on an add in a pull of 2 or more (list it as ignored if the sim has no adds to bind).
- **Sharpshooter.** Stands still: Steady Shot as filler building Steady Aim, Aimed Shot and
  Puncturing Shaft on cooldown, Sunder Arrow kept at 5 stacks, Rapid Fire on cooldown, Hold Your
  Ground when the mob reaches melee, Pinning Shot to keep it at range; Silencing Shot on a cast.
- **Trapper.** Moves: lays Immolation Trap and Frost Trap where the mob will step, Snapshot as
  filler with Auto Shot, Caltrops when the mob reaches melee, Spring the Traps when two traps are
  armed, Vault away from melee; Silencing Shot on a cast. Traps in a solo sim spring on the one
  mob; list "springs on a group" as ignored on a single-mob cell.

## Where

- `tools/twilights-eve/balance/model.py`: the six classes and the registry.
- `tools/twilights-eve/tests/test_balance.py`: tests.
- No data change.

## Failing tests first

In `tools/twilights-eve/tests/test_balance.py`:

- Each of the six sims, at Power Level 30 and 50 against 1 normal on level in greens, finishes a
  fight on one seed and reports damage done and health left.
- The Leyweaver's Arcane Surge hits harder from a full bar than from a half bar.
- The Sharpshooter's Steady Aim stacks reach 5 in a fight where it never moves.

## Acceptance

- `~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests` passes.
- `~/.local/bin/python3.15 tools/twilights-eve/balance/run.py --levels 30 40 50 --fights 50
  --callings all` runs to the end with all ten Tier 2 sims; paste its table into the closing
  comment. Rules may fail here: ticket 24 fits them.

Assumptions allowed: rotation priorities where the kit allows two readings.
