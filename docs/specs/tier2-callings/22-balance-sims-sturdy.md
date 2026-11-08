Title: Balance sims for the sturdy Tier 2 Callings: Ironclad, Zealot, Blightcaller and Bloodmender (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 6, 10 and 13. Blocked by 21-balance-framework, 11-ironclad,
12-zealot, 18-blightcaller and 19-bloodmender.

Each Tier 1 Calling has a `Sim` subclass in `tools/twilights-eve/balance/model.py` (`Vanguard`
line 741, `Acolyte` line 1561) that plays its kit's rotation from `server_spells.json`. This
ticket adds four: `Ironclad` and `Zealot` in the form of `Vanguard`, `Blightcaller` and
`Bloodmender` in the form of `Acolyte`, and registers them where the Tier 1 sims are mapped (the
`dict(..., acolyte=Acolyte)` near line 1960).

## What each sim plays

Read the numbers from `data/twilights-eve/server_spells.json`, never from this ticket. The
rotation follows the kit's design (tickets 11, 12, 18 and 19):

- **Ironclad.** Righteous Smash on every swing it can afford; Shield Slam on cooldown when
  Conviction is at 3 or more; Retort whenever a block opened its window; Shield of the Penitent
  and Steel Resolve on cooldown; Iron Wall under 40% health; Rebuke on a cast; Hold the Line on a
  pull of 2 or more.
- **Zealot.** Seal of Iron kept up; Judgement on cooldown when the Seal is up, then the Seal
  again; Hallowed Ground on cooldown and stood in; Bulwark Charge to open; Holy Wrath on cooldown
  (double damage and the stun only on undead mobs, so the cell's mob type matters: on-level mobs
  are humanoid unless a named boss says otherwise); Righteous Fury kept up (no effect in a solo
  sim; list it as ignored); Rebuke on a cast; Hammer of Justice on a cast it cannot Rebuke.
- **Blightcaller.** Creeping Blight on every mob; Plague Tithe as filler; Harvest at 8 stacks or
  more; Spread Blight on a pull of 2 or more once the target has stacks; Miasma and Wasting
  Communion on cooldown on a pull; Hush on a cast. Solo, every heal that reaches "the most hurt
  friend" or "friends within N yd" reaches the caster.
- **Bloodmender.** Blood Lance as damage; Blood Feast whenever health is under 60%; Dark Mending
  and Transfusion on the caster when health is under 40% and health stays above the cost; Blood
  Ward and Circle of Blood on cooldown in a pull; Hush on a cast.
- **H-1 (ticket 21).** For the two healers, the same rotation aimed at keeping the dummy friend
  up: the friend is the lowest-health friend for every `ally=lowest_health`; Transfusion, Dark
  Mending and Blood Ward go on the friend.

Mechanics the sim cannot express go to the report's "Ignored choices" (threat, taunts, run
speed, roots on a mob already in melee, resurrection), as `docs/balance/rules.md` lists them.

## Where

- `tools/twilights-eve/balance/model.py`: the four classes and the registry.
- `tools/twilights-eve/tests/test_balance.py`: tests.
- No data change: this ticket does not edit `master.csv`, `gear.csv` or any content file.

## Failing tests first

In `tools/twilights-eve/tests/test_balance.py`:

- Each of the four sims, at Power Level 30 and 50 against 1 normal on level in greens, finishes a
  fight on one seed and reports damage done and health left (the form of the Tier 1 sims' tests).
- The Ironclad spends Conviction with Shield Slam (its log shows a Shield Slam after three or more
  stacks).
- The Zealot's Holy Wrath deals double to an undead mob and normal to a humanoid one.
- The Blightcaller's and the Bloodmender's H-1 cells report win and health left for the dummy
  friend.

## Acceptance

- `~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests` passes.
- `~/.local/bin/python3.15 tools/twilights-eve/balance/run.py --levels 30 40 50 --fights 50
  --callings ironclad zealot blightcaller bloodmender` runs to the end; paste its table into the
  closing comment. Rules may fail here: ticket 24 fits them.

Assumptions allowed: rotation priorities where the kit allows two readings.
