# Balance check 3, report 4: greens five levels above the generic and named bosses

Issue #488. Rules: `docs/balance/rules.md`. Model: `tools/twilights-eve/balance/` from #484, with one bug fix in this ticket (bc220fe2, below).

## Command

Data and model from commit `bc220fe2`, 300 fights a cell. The whole run took 12 s, so it stays at 300 fights.

```
~/.local/bin/python3.15 tools/twilights-eve/balance/run.py --scenario plus5 --levels 15-30 --fights 300 --jobs 4 --out target/agent/plus5-raw.md
~/.local/bin/python3.15 tools/twilights-eve/balance/run.py --scenario named --gear green --above 5 --fights 300 --jobs 4 --out docs/balance/2026-10-28/plus-five.md --append
```

The first command runs both rules: `--scenario plus5` is G+5 and G+5-named. The second one stops at once, since
`run.py` has no `--gear`, `--above` or `--append`; `--scenario named` is B-named, in blues at the boss's level.
The tables below come from `target/agent/plus5488.py` (not committed), which calls `scenarios.g_5` and
`scenarios.g_5_named` through the same 4-process pool and keeps the rows as JSON. Every cell's seed comes from
the cell itself, so the first command's raw file prints the same numbers; it was checked against them.

Win is the share of fights won, rounded to a whole percent, so 0% can hide one or two wins. Health left is the
mean share of maximum health at the end of the won fights and kill time the median length of a won fight, both
`-` when none was won.

No named boss sits below level 5, so no character is below 10 and no fight goes to the Basic Warrior: Geomancer
Vurx (7) is fought at 12. Two are fought above 30: Acolyte Marren (26) at 31 and Hogger (30) at 35. The level
table above 30 is provisional (ADR 0012). There the character reads its class's base stats at its level, keeps
its Calling's level 30 talents and wears the best green it can, which is the level 30 kit. No Tier Step applies
on the character side, and none to the boss, since neither boss is above 30.

### The model fix

`scenarios._named` clamped the character to 30, so Marren was fought at 30 and Hogger at 30, not at 31 and 35.
Lifting the clamp exposed a second fault: `model.calling_level` raised `StopIteration` for a Power Level above
30, since no Calling level maps there. A Tier 1 Calling past the top of its band now stays at its cap.

## G+5: each Calling in greens against the generic boss five levels below

Each must win 75% or more against the generic boss at its level minus 5, levels 15 to 30.

### Cutthroat

| Level | Boss level | Win | Health left | Kill s | Holds |
|---|---|---|---|---|---|
| 15 | 10 | 100% | 56% | 36.0 | yes |
| 16 | 11 | 31% | 16% | 56.6 | no |
| 17 | 12 | 6% | 11% | 60.1 | no |
| 18 | 13 | 3% | 12% | 57.7 | no |
| 19 | 14 | 19% | 19% | 58.1 | no |
| 20 | 15 | 6% | 15% | 61.5 | no |
| 21 | 16 | 29% | 26% | 52.4 | no |
| 22 | 17 | 23% | 24% | 52.1 | no |
| 23 | 18 | 69% | 40% | 55.1 | no |
| 24 | 19 | 56% | 38% | 54.5 | no |
| 25 | 20 | 50% | 32% | 53.9 | no |
| 26 | 21 | 47% | 36% | 53.0 | no |
| 27 | 22 | 67% | 42% | 52.8 | no |
| 28 | 23 | 60% | 39% | 50.1 | no |
| 29 | 24 | 58% | 36% | 52.1 | no |
| 30 | 25 | 49% | 35% | 52.1 | no |

### Ranger

| Level | Boss level | Win | Health left | Kill s | Holds |
|---|---|---|---|---|---|
| 15 | 10 | 1% | 4% | 66.2 | no |
| 16 | 11 | 0% | - | - | no |
| 17 | 12 | 0% | - | - | no |
| 18 | 13 | 0% | - | - | no |
| 19 | 14 | 0% | - | - | no |
| 20 | 15 | 0% | - | - | no |
| 21 | 16 | 0% | - | - | no |
| 22 | 17 | 0% | - | - | no |
| 23 | 18 | 0% | - | - | no |
| 24 | 19 | 0% | - | - | no |
| 25 | 20 | 0% | - | - | no |
| 26 | 21 | 0% | - | - | no |
| 27 | 22 | 0% | - | - | no |
| 28 | 23 | 0% | - | - | no |
| 29 | 24 | 0% | - | - | no |
| 30 | 25 | 0% | - | - | no |

### Initiate

| Level | Boss level | Win | Health left | Kill s | Holds |
|---|---|---|---|---|---|
| 15 | 10 | 0% | - | - | no |
| 16 | 11 | 0% | - | - | no |
| 17 | 12 | 0% | - | - | no |
| 18 | 13 | 0% | - | - | no |
| 19 | 14 | 0% | - | - | no |
| 20 | 15 | 0% | - | - | no |
| 21 | 16 | 0% | - | - | no |
| 22 | 17 | 0% | - | - | no |
| 23 | 18 | 0% | - | - | no |
| 24 | 19 | 0% | - | - | no |
| 25 | 20 | 0% | - | - | no |
| 26 | 21 | 0% | - | - | no |
| 27 | 22 | 0% | - | - | no |
| 28 | 23 | 0% | - | - | no |
| 29 | 24 | 0% | - | - | no |
| 30 | 25 | 0% | - | - | no |

### Vanguard

| Level | Boss level | Win | Health left | Kill s | Holds |
|---|---|---|---|---|---|
| 15 | 10 | 98% | 35% | 48.6 | yes |
| 16 | 11 | 0% | - | - | no |
| 17 | 12 | 0% | - | - | no |
| 18 | 13 | 0% | - | - | no |
| 19 | 14 | 0% | - | - | no |
| 20 | 15 | 0% | - | - | no |
| 21 | 16 | 0% | - | - | no |
| 22 | 17 | 0% | - | - | no |
| 23 | 18 | 0% | - | - | no |
| 24 | 19 | 0% | - | - | no |
| 25 | 20 | 0% | - | - | no |
| 26 | 21 | 0% | - | - | no |
| 27 | 22 | 0% | - | - | no |
| 28 | 23 | 0% | - | - | no |
| 29 | 24 | 0% | - | - | no |
| 30 | 25 | 0% | - | - | no |

### Acolyte

| Level | Boss level | Win | Health left | Kill s | Holds |
|---|---|---|---|---|---|
| 15 | 10 | 100% | 52% | 44.0 | yes |
| 16 | 11 | 0% | - | - | no |
| 17 | 12 | 0% | - | - | no |
| 18 | 13 | 0% | - | - | no |
| 19 | 14 | 0% | - | - | no |
| 20 | 15 | 0% | - | - | no |
| 21 | 16 | 0% | - | - | no |
| 22 | 17 | 0% | - | - | no |
| 23 | 18 | 0% | - | - | no |
| 24 | 19 | 0% | - | - | no |
| 25 | 20 | 0% | - | - | no |
| 26 | 21 | 0% | - | - | no |
| 27 | 22 | 0% | - | - | no |
| 28 | 23 | 0% | - | - | no |
| 29 | 24 | 0% | - | - | no |
| 30 | 25 | 0% | - | - | no |

### The generic boss at each level

`model.mob_stats(level, rank=1, health_mul=2.0, damage_mul=1.1)`: a warrior-class elite with no spells, in
the on-level area's Zone Factor (Northshire at 10, Elwynn's 1.5 health and 1.3 damage from 11). It swings every
2.0 s. `Curve margin` is P(character level) / P(boss level).

| Level | Boss level | Curve margin | Zone Factor | Boss health | Mean swing | Armour |
|---|---|---|---|---|---|---|
| 15 | 10 | 1.50 | 1 / 1 | 2534 | 75.4 | 492 |
| 16 | 11 | 1.45 | 1.5 / 1.3 | 4182 | 107.9 | 518 |
| 17 | 12 | 1.42 | 1.5 / 1.3 | 4562 | 117.7 | 552 |
| 18 | 13 | 1.38 | 1.5 / 1.3 | 4942 | 127.5 | 585 |
| 19 | 14 | 1.36 | 1.5 / 1.3 | 5322 | 137.3 | 619 |
| 20 | 15 | 1.33 | 1.5 / 1.3 | 5702 | 147.1 | 653 |
| 21 | 16 | 1.31 | 1.5 / 1.3 | 6083 | 156.9 | 688 |
| 22 | 17 | 1.29 | 1.5 / 1.3 | 6463 | 166.7 | 722 |
| 23 | 18 | 1.28 | 1.5 / 1.3 | 6843 | 176.5 | 755 |
| 24 | 19 | 1.26 | 1.5 / 1.3 | 7223 | 186.3 | 789 |
| 25 | 20 | 1.25 | 1.5 / 1.3 | 7603 | 196.1 | 822 |
| 26 | 21 | 1.24 | 1.5 / 1.3 | 7983 | 205.9 | 858 |
| 27 | 22 | 1.23 | 1.5 / 1.3 | 8364 | 215.7 | 892 |
| 28 | 23 | 1.22 | 1.5 / 1.3 | 8744 | 225.5 | 925 |
| 29 | 24 | 1.21 | 1.5 / 1.3 | 9124 | 235.4 | 959 |
| 30 | 25 | 1.20 | 1.5 / 1.3 | 9504 | 245.2 | 992 |

## G+5-named: each Calling in greens five levels above every named boss

Each must win 75% or more against each named boss, at the boss's level plus 5. `Holds` is yes when all
five do, else it names the Callings that miss.

| Boss | Boss level | Character level | Cutthroat win | Ranger win | Initiate win | Vanguard win | Acolyte win | Holds |
|---|---|---|---|---|---|---|---|---|
| Geomancer Vurx (96002) | 7 | 12 | 100% | 100% | 100% | 100% | 100% | yes |
| Abbot Ashworth (96005) | 10 | 15 | 100% | 23% | 21% | 100% | 100% | no: Ranger, Initiate |
| Bloodtalon Scythemaw (3123) | 10 | 15 | 100% | 100% | 100% | 100% | 100% | yes |
| Garrick Padfoot (103) | 10 | 15 | 100% | 24% | 14% | 100% | 100% | no: Ranger, Initiate |
| The Rake (5807) | 10 | 15 | 100% | 97% | 83% | 100% | 100% | yes |
| Ghost Howl (3056) | 12 | 17 | 100% | 21% | 0% | 99% | 64% | no: Ranger, Initiate, Acolyte |
| Princess (330) | 12 | 17 | 38% | 0% | 0% | 1% | 0% | no: Cutthroat, Ranger, Initiate, Vanguard, Acolyte |
| Morgan the Collector (96012) | 14 | 19 | 64% | 0% | 0% | 0% | 3% | no: Cutthroat, Ranger, Initiate, Vanguard, Acolyte |
| Narg the Taskmaster (79) | 14 | 19 | 64% | 0% | 0% | 0% | 2% | no: Cutthroat, Ranger, Initiate, Vanguard, Acolyte |
| Gllrg the Tidecaller (96014) | 15 | 20 | 100% | 4% | 0% | 100% | 100% | no: Ranger, Initiate |
| Ritualist Serane (96025) | 18 | 23 | 100% | 90% | 0% | 95% | 80% | no: Initiate |
| The Eye Below (96022) | 18 | 23 | 88% | 0% | 0% | 0% | 89% | no: Ranger, Initiate, Vanguard |
| Gresh the Drowned (96034) | 20 | 25 | 99% | 74% | 0% | 92% | 100% | no: Ranger, Initiate |
| Lady Anacondra (3671) | 20 | 25 | 100% | 100% | 100% | 100% | 100% | yes |
| Speaker Thane (96035) | 21 | 26 | 99% | 61% | 0% | 66% | 100% | no: Ranger, Initiate, Vanguard |
| Shadow of Morganth (96037) | 22 | 27 | 96% | 0% | 0% | 36% | 100% | no: Ranger, Initiate, Vanguard |
| The Kept (96036) | 22 | 27 | 100% | 20% | 0% | 97% | 100% | no: Ranger, Initiate |
| Speaker Vael (96040) | 24 | 29 | 100% | 100% | 1% | 100% | 100% | no: Initiate |
| Acolyte Marren (96043) | 26 | 31 | 100% | 100% | 0% | 99% | 100% | no: Initiate |
| Hogger (96010) | 30 | 35 | 0% | 0% | 0% | 0% | 0% | no: Cutthroat, Ranger, Initiate, Vanguard, Acolyte |

Spells the model skipped as unmodelled, per boss (`boss.py` models periodic damage, ground patches, novas,
gazes, hits, heals and stances):

- Geomancer Vurx (7): frost_armor_vurx, frost_chill_vurx.
- The Eye Below (18): eye_mind_flay.
- Gresh the Drowned (20): drowning_grasp.
- Speaker Thane (21): eye_mind_flay.
- Shadow of Morganth (22): terrify_creature.
- The Kept (22): void_shield_creature.
- Hogger (30): eat, rushing_charge.
- Every other boss: none.

### Each named boss against the generic boss at its level

The named boss's numbers against the generic boss's at the same level, and each Calling's G+5-named win minus
its G+5 win at the same character level, in points (`-` where G+5 has no row: below 15 or above 30). A large
plus is a boss far easier than its generic neighbour.

| Boss | Level | Class | Rank | Health x / damage x | Area | Health | Mean swing | Generic health | Generic swing | Cutthroat vs generic | Ranger vs generic | Initiate vs generic | Vanguard vs generic | Acolyte vs generic |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Geomancer Vurx | 7 | mage | 1 | 0.2528 / 0.2826 | Northshire | 202 | 8.3 | 1774 | 44.0 | - | - | - | - | - |
| Abbot Ashworth | 10 | mage | 1 | 2 / 1.1 | Northshire | 2278 | 55.3 | 2534 | 75.4 | +0 | +22 | +21 | +2 | +0 |
| Bloodtalon Scythemaw | 10 | warrior | 0 | 1 / 1 | Elwynn | 950 | 59.4 | 2534 | 75.4 | +0 | +99 | +100 | +2 | +0 |
| Garrick Padfoot | 10 | warrior | 1 | 1.6 / 1.1 | Northshire | 2028 | 75.4 | 2534 | 75.4 | +0 | +23 | +14 | +2 | +0 |
| The Rake | 10 | warrior | 4 | 1 / 0.6 | Elwynn | 1426 | 44.6 | 2534 | 75.4 | +0 | +96 | +83 | +2 | +0 |
| Ghost Howl | 12 | warrior | 4 | 1 / 1.25 | Elwynn | 1711 | 111.4 | 4562 | 117.7 | +94 | +21 | +0 | +99 | +64 |
| Princess | 12 | warrior | 1 | 1.6 / 1.1 | Elwynn | 3650 | 117.7 | 4562 | 117.7 | +32 | +0 | +0 | +1 | +0 |
| Morgan the Collector | 14 | warrior | 1 | 1.6 / 1.1 | Elwynn | 4258 | 137.3 | 5322 | 137.3 | +45 | +0 | +0 | +0 | +3 |
| Narg the Taskmaster | 14 | warrior | 1 | 1.6 / 1.1 | Elwynn | 4258 | 137.3 | 5322 | 137.3 | +45 | +0 | +0 | +0 | +2 |
| Gllrg the Tidecaller | 15 | mage | 1 | 1.4 / 0.9 | Elwynn | 3588 | 88.2 | 5702 | 147.1 | +94 | +4 | +0 | +100 | +100 |
| Ritualist Serane | 18 | mage | 1 | 1.6 / 1 | Elwynn | 4921 | 117.5 | 6843 | 176.5 | +31 | +90 | +0 | +95 | +80 |
| The Eye Below | 18 | mage | 1 | 3 / 1 | Elwynn | 9228 | 117.5 | 6843 | 176.5 | +19 | +0 | +0 | +0 | +89 |
| Gresh the Drowned | 20 | mage | 1 | 2 / 1 | Elwynn | 6835 | 130.6 | 7603 | 196.1 | +49 | +74 | +0 | +92 | +100 |
| Lady Anacondra | 20 | paladin | 1 | 0.4547 / 0.2494 | Wailing Caverns | 1083 | 31.8 | 7603 | 196.1 | +50 | +100 | +100 | +100 | +100 |
| Speaker Thane | 21 | mage | 1 | 2 / 1 | Elwynn | 7177 | 137.1 | 7983 | 205.9 | +52 | +61 | +0 | +66 | +100 |
| Shadow of Morganth | 22 | mage | 1 | 4 / 1 | Elwynn | 15037 | 143.7 | 8364 | 215.7 | +29 | +0 | +0 | +36 | +100 |
| The Kept | 22 | warrior | 1 | 2.5 / 0.5 | Elwynn | 10454 | 98.1 | 8364 | 215.7 | +33 | +20 | +0 | +97 | +100 |
| Speaker Vael | 24 | mage | 1 | 1.3 / 0.8 | Elwynn | 5331 | 125.4 | 9124 | 235.4 | +42 | +100 | +1 | +100 | +100 |
| Acolyte Marren | 26 | mage | 1 | 1.4 / 0.9 | Elwynn | 6220 | 152.8 | 9884 | 255.0 | - | - | - | - | - |
| Hogger | 30 | warrior | 1 | 3 / 1.5 | Elwynn | 17107 | 401.2 | 11405 | 294.2 | - | - | - | - | - |

## Failures

122 of 180 (rule, Calling, level or boss) cells fail: G+5 77 of 80, G+5-named 45 of 100.

- G+5, Cutthroat, level 16 (boss 11): win 31%, 44 points under 75%.
- G+5, Cutthroat, level 17 (boss 12): win 6%, 69 points under 75%.
- G+5, Cutthroat, level 18 (boss 13): win 3%, 72 points under 75%.
- G+5, Cutthroat, level 19 (boss 14): win 19%, 56 points under 75%.
- G+5, Cutthroat, level 20 (boss 15): win 6%, 69 points under 75%.
- G+5, Cutthroat, level 21 (boss 16): win 29%, 46 points under 75%.
- G+5, Cutthroat, level 22 (boss 17): win 23%, 52 points under 75%.
- G+5, Cutthroat, level 23 (boss 18): win 69%, 6 points under 75%.
- G+5, Cutthroat, level 24 (boss 19): win 56%, 19 points under 75%.
- G+5, Cutthroat, level 25 (boss 20): win 50%, 25 points under 75%.
- G+5, Cutthroat, level 26 (boss 21): win 47%, 28 points under 75%.
- G+5, Cutthroat, level 27 (boss 22): win 67%, 8 points under 75%.
- G+5, Cutthroat, level 28 (boss 23): win 60%, 15 points under 75%.
- G+5, Cutthroat, level 29 (boss 24): win 58%, 17 points under 75%.
- G+5, Cutthroat, level 30 (boss 25): win 49%, 26 points under 75%.
- G+5, Ranger, level 15 (boss 10): win 1%, 74 points under 75%.
- G+5, Ranger, level 16 (boss 11): win 0%, 75 points under 75%.
- G+5, Ranger, level 17 (boss 12): win 0%, 75 points under 75%.
- G+5, Ranger, level 18 (boss 13): win 0%, 75 points under 75%.
- G+5, Ranger, level 19 (boss 14): win 0%, 75 points under 75%.
- G+5, Ranger, level 20 (boss 15): win 0%, 75 points under 75%.
- G+5, Ranger, level 21 (boss 16): win 0%, 75 points under 75%.
- G+5, Ranger, level 22 (boss 17): win 0%, 75 points under 75%.
- G+5, Ranger, level 23 (boss 18): win 0%, 75 points under 75%.
- G+5, Ranger, level 24 (boss 19): win 0%, 75 points under 75%.
- G+5, Ranger, level 25 (boss 20): win 0%, 75 points under 75%.
- G+5, Ranger, level 26 (boss 21): win 0%, 75 points under 75%.
- G+5, Ranger, level 27 (boss 22): win 0%, 75 points under 75%.
- G+5, Ranger, level 28 (boss 23): win 0%, 75 points under 75%.
- G+5, Ranger, level 29 (boss 24): win 0%, 75 points under 75%.
- G+5, Ranger, level 30 (boss 25): win 0%, 75 points under 75%.
- G+5, Initiate, level 15 (boss 10): win 0%, 75 points under 75%.
- G+5, Initiate, level 16 (boss 11): win 0%, 75 points under 75%.
- G+5, Initiate, level 17 (boss 12): win 0%, 75 points under 75%.
- G+5, Initiate, level 18 (boss 13): win 0%, 75 points under 75%.
- G+5, Initiate, level 19 (boss 14): win 0%, 75 points under 75%.
- G+5, Initiate, level 20 (boss 15): win 0%, 75 points under 75%.
- G+5, Initiate, level 21 (boss 16): win 0%, 75 points under 75%.
- G+5, Initiate, level 22 (boss 17): win 0%, 75 points under 75%.
- G+5, Initiate, level 23 (boss 18): win 0%, 75 points under 75%.
- G+5, Initiate, level 24 (boss 19): win 0%, 75 points under 75%.
- G+5, Initiate, level 25 (boss 20): win 0%, 75 points under 75%.
- G+5, Initiate, level 26 (boss 21): win 0%, 75 points under 75%.
- G+5, Initiate, level 27 (boss 22): win 0%, 75 points under 75%.
- G+5, Initiate, level 28 (boss 23): win 0%, 75 points under 75%.
- G+5, Initiate, level 29 (boss 24): win 0%, 75 points under 75%.
- G+5, Initiate, level 30 (boss 25): win 0%, 75 points under 75%.
- G+5, Vanguard, level 16 (boss 11): win 0%, 75 points under 75%.
- G+5, Vanguard, level 17 (boss 12): win 0%, 75 points under 75%.
- G+5, Vanguard, level 18 (boss 13): win 0%, 75 points under 75%.
- G+5, Vanguard, level 19 (boss 14): win 0%, 75 points under 75%.
- G+5, Vanguard, level 20 (boss 15): win 0%, 75 points under 75%.
- G+5, Vanguard, level 21 (boss 16): win 0%, 75 points under 75%.
- G+5, Vanguard, level 22 (boss 17): win 0%, 75 points under 75%.
- G+5, Vanguard, level 23 (boss 18): win 0%, 75 points under 75%.
- G+5, Vanguard, level 24 (boss 19): win 0%, 75 points under 75%.
- G+5, Vanguard, level 25 (boss 20): win 0%, 75 points under 75%.
- G+5, Vanguard, level 26 (boss 21): win 0%, 75 points under 75%.
- G+5, Vanguard, level 27 (boss 22): win 0%, 75 points under 75%.
- G+5, Vanguard, level 28 (boss 23): win 0%, 75 points under 75%.
- G+5, Vanguard, level 29 (boss 24): win 0%, 75 points under 75%.
- G+5, Vanguard, level 30 (boss 25): win 0%, 75 points under 75%.
- G+5, Acolyte, level 16 (boss 11): win 0%, 75 points under 75%.
- G+5, Acolyte, level 17 (boss 12): win 0%, 75 points under 75%.
- G+5, Acolyte, level 18 (boss 13): win 0%, 75 points under 75%.
- G+5, Acolyte, level 19 (boss 14): win 0%, 75 points under 75%.
- G+5, Acolyte, level 20 (boss 15): win 0%, 75 points under 75%.
- G+5, Acolyte, level 21 (boss 16): win 0%, 75 points under 75%.
- G+5, Acolyte, level 22 (boss 17): win 0%, 75 points under 75%.
- G+5, Acolyte, level 23 (boss 18): win 0%, 75 points under 75%.
- G+5, Acolyte, level 24 (boss 19): win 0%, 75 points under 75%.
- G+5, Acolyte, level 25 (boss 20): win 0%, 75 points under 75%.
- G+5, Acolyte, level 26 (boss 21): win 0%, 75 points under 75%.
- G+5, Acolyte, level 27 (boss 22): win 0%, 75 points under 75%.
- G+5, Acolyte, level 28 (boss 23): win 0%, 75 points under 75%.
- G+5, Acolyte, level 29 (boss 24): win 0%, 75 points under 75%.
- G+5, Acolyte, level 30 (boss 25): win 0%, 75 points under 75%.
- G+5-named, Ranger, Abbot Ashworth (boss 10, character 15): win 23%, 52 points under 75%.
- G+5-named, Initiate, Abbot Ashworth (boss 10, character 15): win 21%, 54 points under 75%.
- G+5-named, Ranger, Garrick Padfoot (boss 10, character 15): win 24%, 51 points under 75%.
- G+5-named, Initiate, Garrick Padfoot (boss 10, character 15): win 14%, 61 points under 75%.
- G+5-named, Ranger, Ghost Howl (boss 12, character 17): win 21%, 54 points under 75%.
- G+5-named, Initiate, Ghost Howl (boss 12, character 17): win 0%, 75 points under 75%.
- G+5-named, Acolyte, Ghost Howl (boss 12, character 17): win 64%, 11 points under 75%.
- G+5-named, Cutthroat, Princess (boss 12, character 17): win 38%, 37 points under 75%.
- G+5-named, Ranger, Princess (boss 12, character 17): win 0%, 75 points under 75%.
- G+5-named, Initiate, Princess (boss 12, character 17): win 0%, 75 points under 75%.
- G+5-named, Vanguard, Princess (boss 12, character 17): win 1%, 74 points under 75%.
- G+5-named, Acolyte, Princess (boss 12, character 17): win 0%, 75 points under 75%.
- G+5-named, Cutthroat, Morgan the Collector (boss 14, character 19): win 64%, 11 points under 75%.
- G+5-named, Ranger, Morgan the Collector (boss 14, character 19): win 0%, 75 points under 75%.
- G+5-named, Initiate, Morgan the Collector (boss 14, character 19): win 0%, 75 points under 75%.
- G+5-named, Vanguard, Morgan the Collector (boss 14, character 19): win 0%, 75 points under 75%.
- G+5-named, Acolyte, Morgan the Collector (boss 14, character 19): win 3%, 72 points under 75%.
- G+5-named, Cutthroat, Narg the Taskmaster (boss 14, character 19): win 64%, 11 points under 75%.
- G+5-named, Ranger, Narg the Taskmaster (boss 14, character 19): win 0%, 75 points under 75%.
- G+5-named, Initiate, Narg the Taskmaster (boss 14, character 19): win 0%, 75 points under 75%.
- G+5-named, Vanguard, Narg the Taskmaster (boss 14, character 19): win 0%, 75 points under 75%.
- G+5-named, Acolyte, Narg the Taskmaster (boss 14, character 19): win 2%, 73 points under 75%.
- G+5-named, Ranger, Gllrg the Tidecaller (boss 15, character 20): win 4%, 71 points under 75%.
- G+5-named, Initiate, Gllrg the Tidecaller (boss 15, character 20): win 0%, 75 points under 75%.
- G+5-named, Initiate, Ritualist Serane (boss 18, character 23): win 0%, 75 points under 75%.
- G+5-named, Ranger, The Eye Below (boss 18, character 23): win 0%, 75 points under 75%.
- G+5-named, Initiate, The Eye Below (boss 18, character 23): win 0%, 75 points under 75%.
- G+5-named, Vanguard, The Eye Below (boss 18, character 23): win 0%, 75 points under 75%.
- G+5-named, Ranger, Gresh the Drowned (boss 20, character 25): win 74%, 1 points under 75%.
- G+5-named, Initiate, Gresh the Drowned (boss 20, character 25): win 0%, 75 points under 75%.
- G+5-named, Ranger, Speaker Thane (boss 21, character 26): win 61%, 14 points under 75%.
- G+5-named, Initiate, Speaker Thane (boss 21, character 26): win 0%, 75 points under 75%.
- G+5-named, Vanguard, Speaker Thane (boss 21, character 26): win 66%, 9 points under 75%.
- G+5-named, Ranger, Shadow of Morganth (boss 22, character 27): win 0%, 75 points under 75%.
- G+5-named, Initiate, Shadow of Morganth (boss 22, character 27): win 0%, 75 points under 75%.
- G+5-named, Vanguard, Shadow of Morganth (boss 22, character 27): win 36%, 39 points under 75%.
- G+5-named, Ranger, The Kept (boss 22, character 27): win 20%, 55 points under 75%.
- G+5-named, Initiate, The Kept (boss 22, character 27): win 0%, 75 points under 75%.
- G+5-named, Initiate, Speaker Vael (boss 24, character 29): win 1%, 74 points under 75%.
- G+5-named, Initiate, Acolyte Marren (boss 26, character 31): win 0%, 75 points under 75%.
- G+5-named, Cutthroat, Hogger (boss 30, character 35): win 0%, 75 points under 75%.
- G+5-named, Ranger, Hogger (boss 30, character 35): win 0%, 75 points under 75%.
- G+5-named, Initiate, Hogger (boss 30, character 35): win 0%, 75 points under 75%.
- G+5-named, Vanguard, Hogger (boss 30, character 35): win 0%, 75 points under 75%.
- G+5-named, Acolyte, Hogger (boss 30, character 35): win 0%, 75 points under 75%.

## Reading

- G+5 holds in 3 of 80 cells, all at 15 against the level 10 boss: Cutthroat 100%, Vanguard 98%, Acolyte 100%. The Ranger wins 1% and the Initiate 0% there. From 16 to 30 no cell holds. The Cutthroat is the only Calling that wins at all (3% to 69%); the other four win 0% at every level from 16.
- Five levels of the Power Curve are enough margin only while the boss stays in Northshire. At 15 against 10 the margin is 1.5x and the boss has no Zone Factor. At 16 against 11 the curve still gives 1.45x, but the level 11 boss is in Elwynn and takes 1.5 health and 1.3 damage: its health goes from 2534 to 4182 and its mean swing from 75 to 108. The Vanguard drops from 98% to 0%, the Acolyte from 100% to 0% and the Cutthroat from 100% to 31%. The margin is gone at 16, a jump from one level to the next rather than a thinning toward 30.
- The thinning ratio (1.45x at 16, 1.2x at 30) does not show in the results. The Cutthroat's win rises from 3% to 31% at 16 to 22 to 47% to 69% at 23 to 30: its DPS grows from 70 to 173 (2.5x) while the boss's health grows from 4182 to 9504 (2.3x). For the other four the gap is far wider than the ratio. A kill needs 59 to 132 s of their DPS, and the boss's mean swing every 2 s would take their full health in 6 to 16 s before armour and avoidance.
- The Ranger's Point Blank at 23 doubles its DPS (57 at 22, 116 at 23), and a kill then needs 59 to 64 s, close to the Cutthroat's 55 to 56 s. It still wins 0%, with 1085 to 1382 health against the Cutthroat's 1185 to 1488. The Initiate has the least health (447 to 775) and 47 to 73 DPS.
- G+5-named holds for all five Callings against four bosses: Geomancer Vurx, Bloodtalon Scythemaw, the Rake and Lady Anacondra. Per Calling it holds against 16 of 20 bosses for the Cutthroat, 15 for the Acolyte, 13 for the Vanguard, 7 for the Ranger and 4 for the Initiate.
- Health times mean swing against the generic boss at the same level gives a rough measure that ignores armour and spells. By it two bosses sit above their generic neighbour: Hogger (1.5x health, 1.36x swing, 2.05 together) and the Shadow of Morganth (1.8x health, 0.67x swing, 1.20). The Eye Below is at 0.90. Seven sit far below at 0.4 or less: Vurx and Anacondra (0.02), Scythemaw (0.30), Speaker Vael (0.31), Ghost Howl (0.36), Gllrg and Acolyte Marren (0.38). Princess, Morgan and Narg (0.80, warrior class at 1.6/1.1) play like the generic boss: at 17 and 19 the Cutthroat wins 38% and 64%, the rest 0% to 3%.
- The caster bosses at 18 to 22 (Serane, Gresh, Thane, the Kept) swing about two thirds of the generic boss and are where five levels pay off: the Vanguard wins 66% to 97% and the Acolyte 80% to 100% against them, where both win 0% against the generic boss at the same character level.
- Hogger at 35 beats every Calling: 0% for all five. He has 17107 health, so the Cutthroat's 184 DPS needs 93 s, and a 401 swing. At 35 the character keeps its level 30 kit and talents and gains only base stats (the Cutthroat goes from 1488 to 1663 health), and the level table above 30 is provisional (ADR 0012). His Eat and Rushing Charge are unmodelled, so play may be harder still. Acolyte Marren at 31 is easy for four Callings (99% to 100%); the Initiate wins 0%.

## Kits and builds

As the cells used them, all in greens. Talents are the damage build at every point the level gives; at 31 and
35 the Calling is at its cap and the kit is the level 30 one.

| Calling | Level | Kit | Talents |
|---|---|---|---|
| Cutthroat | 12 | Stonefield Leather Jerkin, Brackwell Talisman, Brackwell Loop, Prowler's Signet, Brackwell Cutlass, Maclure Dirk (OH) | reflexive_strike 1, blood_frenzy 1, bloodletting 1 |
| Cutthroat | 15 | Fargodeep Leather Jerkin, Crystal Lake Talisman, Crystal Lake Loop, Courier's Loop, Crystal Lake Cutlass, Kobold Dirk (OH) | reflexive_strike 2, blood_frenzy 2, bloodletting 2, executioners_toll 1 |
| Cutthroat | 16 | Fargodeep Leather Jerkin, Crystal Lake Talisman, Crystal Lake Loop, Courier's Loop, Vigil Saber, Vigil Dagger (OH) | reflexive_strike 2, blood_frenzy 2, bloodletting 2, executioners_toll 2 |
| Cutthroat | 17 | Fargodeep Leather Jerkin, Crystal Lake Talisman, Crystal Lake Loop, Courier's Loop, Vigil Saber, Vigil Dagger (OH) | reflexive_strike 3, blood_frenzy 3, bloodletting 2, executioners_toll 2 |
| Cutthroat | 18 | Prospector's Chestpiece, Crystal Lake Talisman, Stonerite Loop, Crystal Lake Loop, Vigil Saber, Vigil Dagger (OH) | reflexive_strike 3, blood_frenzy 3, bloodletting 2, executioners_toll 2, phantom_guile 1 |
| Cutthroat | 19 | Azora Leather Jerkin, Stone Cairn Talisman, Stone Cairn Loop, Stonerite Loop, Stone Cairn Cutlass, Murloc Dirk (OH) | reflexive_strike 3, blood_frenzy 3, bloodletting 3, executioners_toll 2, phantom_guile 1, thirst 1 |
| Cutthroat | 20 | Azora Leather Jerkin, Stone Cairn Talisman, Stone Cairn Loop, Stonerite Loop, Stone Cairn Cutlass, Murloc Dirk (OH) | reflexive_strike 3, blood_frenzy 3, bloodletting 3, executioners_toll 3, phantom_guile 1, thirst 1 |
| Cutthroat | 21 | Azora Leather Jerkin, Stone Cairn Talisman, Stone Cairn Loop, Stonerite Loop, Timberman's Axe-Sword, Timberman's Dagger (OH) | reflexive_strike 4, blood_frenzy 3, bloodletting 3, executioners_toll 3, phantom_guile 1, thirst 1, scent_of_blood 1 |
| Cutthroat | 22 | Azora Leather Jerkin, Stone Cairn Talisman, Ridgewatch Loop, Stone Cairn Loop, Timberman's Axe-Sword, Timberman's Dagger (OH) | reflexive_strike 4, blood_frenzy 4, bloodletting 3, executioners_toll 3, phantom_guile 1, thirst 1, scent_of_blood 1 |
| Cutthroat | 23 | Ridgepoint Leather Jerkin, Eastvale Talisman, Eastvale Loop, Ridgewatch Loop, Eastvale Cutlass, Mirror Lake Dirk (OH) | reflexive_strike 4, blood_frenzy 4, bloodletting 3, executioners_toll 3, phantom_guile 2, thirst 2, scent_of_blood 1 |
| Cutthroat | 24 | Ridgepoint Leather Jerkin, Eastvale Talisman, Ledger Loop, Eastvale Loop, Eastvale Cutlass, Mirror Lake Dirk (OH) | reflexive_strike 4, blood_frenzy 4, bloodletting 3, executioners_toll 3, phantom_guile 2, thirst 2, scent_of_blood 1, flurry_of_blades 1 |
| Cutthroat | 25 | Ridgepoint Leather Jerkin, Eastvale Talisman, Ledger Loop, Eastvale Loop, Pit Sword, Pit Dirk (OH) | reflexive_strike 4, blood_frenzy 4, bloodletting 4, executioners_toll 3, phantom_guile 2, thirst 2, scent_of_blood 2, flurry_of_blades 1 |
| Cutthroat | 26 | Ridgepoint Leather Jerkin, Eastvale Talisman, Boatburner's Loop, Ledger Loop, Pit Sword, Pit Dirk (OH) | reflexive_strike 5, blood_frenzy 4, bloodletting 4, executioners_toll 3, phantom_guile 2, thirst 2, scent_of_blood 2, flurry_of_blades 1 |
| Cutthroat | 27 | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Cutlass, Forest's Edge Dirk (OH) | reflexive_strike 5, blood_frenzy 5, bloodletting 4, executioners_toll 3, phantom_guile 2, thirst 2, scent_of_blood 2, flurry_of_blades 1 |
| Cutthroat | 28 | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Cutlass, Forest's Edge Dirk (OH) | reflexive_strike 5, blood_frenzy 5, bloodletting 4, executioners_toll 3, phantom_guile 3, thirst 2, scent_of_blood 2, flurry_of_blades 2 |
| Cutthroat | 29 | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Cutlass, Forest's Edge Dirk (OH) | reflexive_strike 5, blood_frenzy 5, bloodletting 5, executioners_toll 3, phantom_guile 3, thirst 2, scent_of_blood 2, flurry_of_blades 2 |
| Cutthroat | 30 | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Cutlass, Forest's Edge Dirk (OH) | reflexive_strike 5, blood_frenzy 5, bloodletting 5, executioners_toll 3, phantom_guile 3, thirst 2, scent_of_blood 3, flurry_of_blades 2, ruthlessness 1 |
| Cutthroat | 31 | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Cutlass, Forest's Edge Dirk (OH) | reflexive_strike 5, blood_frenzy 5, bloodletting 5, executioners_toll 3, phantom_guile 3, thirst 2, scent_of_blood 3, flurry_of_blades 2, ruthlessness 1 |
| Cutthroat | 35 | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Cutlass, Forest's Edge Dirk (OH) | reflexive_strike 5, blood_frenzy 5, bloodletting 5, executioners_toll 3, phantom_guile 3, thirst 2, scent_of_blood 3, flurry_of_blades 2, ruthlessness 1 |
| Ranger | 12 | Stonefield Leather Jerkin, Brackwell Talisman, Brackwell Loop, Prowler's Signet, Brackwell Quarterstaff, Lion's Pride Shortbow | piercing_force 1, jagged_barbs 1, guerilla_tactics 1 |
| Ranger | 15 | Fargodeep Leather Jerkin, Crystal Lake Talisman, Crystal Lake Loop, Courier's Loop, Crystal Lake Quarterstaff, Grik's Shortbow | piercing_force 2, jagged_barbs 2, guerilla_tactics 2, rhythmic_draw 1 |
| Ranger | 16 | Fargodeep Leather Jerkin, Crystal Lake Talisman, Crystal Lake Loop, Courier's Loop, Vigil Staff, Grik's Shortbow | piercing_force 2, jagged_barbs 2, guerilla_tactics 2, rhythmic_draw 2 |
| Ranger | 17 | Fargodeep Leather Jerkin, Crystal Lake Talisman, Crystal Lake Loop, Courier's Loop, Vigil Staff, Kobold King's Longbow | piercing_force 3, jagged_barbs 3, guerilla_tactics 2, rhythmic_draw 2 |
| Ranger | 18 | Fargodeep Leather Jerkin, Crystal Lake Talisman, Stonerite Loop, Crystal Lake Loop, Vigil Staff, Kobold King's Longbow | piercing_force 3, jagged_barbs 3, guerilla_tactics 2, rhythmic_draw 2, steadfast_aim 1 |
| Ranger | 19 | Azora Leather Jerkin, Stone Cairn Talisman, Stone Cairn Loop, Stonerite Loop, Stone Cairn Quarterstaff, Cairn Watch Shortbow | piercing_force 3, jagged_barbs 3, guerilla_tactics 3, rhythmic_draw 3, steadfast_aim 1 |
| Ranger | 20 | Azora Leather Jerkin, Stone Cairn Talisman, Stone Cairn Loop, Stonerite Loop, Stone Cairn Quarterstaff, Cairn Watch Shortbow | piercing_force 3, jagged_barbs 3, guerilla_tactics 3, rhythmic_draw 3, steadfast_aim 1, twin_arrows 1 |
| Ranger | 21 | Azora Leather Jerkin, Stone Cairn Talisman, Stone Cairn Loop, Stonerite Loop, Timberman's Staff, Apprentice's Crossbow | piercing_force 4, jagged_barbs 4, guerilla_tactics 3, rhythmic_draw 3, steadfast_aim 1, twin_arrows 1 |
| Ranger | 22 | Azora Leather Jerkin, Stone Cairn Talisman, Ridgewatch Loop, Stone Cairn Loop, Timberman's Staff, Apprentice's Crossbow | piercing_force 4, jagged_barbs 4, guerilla_tactics 3, rhythmic_draw 3, steadfast_aim 2, twin_arrows 1 |
| Ranger | 23 | Ridgepoint Leather Jerkin, Eastvale Talisman, Eastvale Loop, Ridgewatch Loop, Eastvale Quarterstaff, Smuggler's Shortbow | piercing_force 4, jagged_barbs 4, guerilla_tactics 3, rhythmic_draw 3, steadfast_aim 2, twin_arrows 1, shattered_defenses 1, point_blank 1 |
| Ranger | 24 | Ridgepoint Leather Jerkin, Eastvale Talisman, Ledger Loop, Eastvale Loop, Eastvale Quarterstaff, Smuggler's Shortbow | piercing_force 4, jagged_barbs 4, guerilla_tactics 4, rhythmic_draw 3, steadfast_aim 2, twin_arrows 1, shattered_defenses 1, point_blank 1 |
| Ranger | 25 | Ridgepoint Leather Jerkin, Eastvale Talisman, Ledger Loop, Eastvale Loop, Eastvale Quarterstaff, Smuggler's Shortbow | piercing_force 4, jagged_barbs 4, guerilla_tactics 4, rhythmic_draw 4, steadfast_aim 2, twin_arrows 2, shattered_defenses 1, point_blank 1 |
| Ranger | 26 | Ridgepoint Leather Jerkin, Eastvale Talisman, Boatburner's Loop, Ledger Loop, Eastvale Quarterstaff, Smuggler's Shortbow | piercing_force 4, jagged_barbs 4, guerilla_tactics 4, rhythmic_draw 4, steadfast_aim 2, twin_arrows 2, shattered_defenses 1, point_blank 1, adrenaline_rush 1 |
| Ranger | 27 | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Quarterstaff, Deepwater Shortbow | piercing_force 5, jagged_barbs 4, guerilla_tactics 4, rhythmic_draw 4, steadfast_aim 2, twin_arrows 2, shattered_defenses 1, point_blank 1, adrenaline_rush 1 |
| Ranger | 28 | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Quarterstaff, Deepwater Shortbow | piercing_force 5, jagged_barbs 5, guerilla_tactics 4, rhythmic_draw 4, steadfast_aim 2, twin_arrows 2, shattered_defenses 2, point_blank 1, adrenaline_rush 1 |
| Ranger | 29 | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Quarterstaff, Deepwater Shortbow | piercing_force 5, jagged_barbs 5, guerilla_tactics 4, rhythmic_draw 4, steadfast_aim 2, twin_arrows 2, shattered_defenses 2, point_blank 2, adrenaline_rush 1 |
| Ranger | 30 | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Quarterstaff, Deepwater Shortbow | piercing_force 5, jagged_barbs 5, guerilla_tactics 5, rhythmic_draw 5, steadfast_aim 2, twin_arrows 2, shattered_defenses 2, point_blank 2, adrenaline_rush 1 |
| Ranger | 31 | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Quarterstaff, Deepwater Shortbow | piercing_force 5, jagged_barbs 5, guerilla_tactics 5, rhythmic_draw 5, steadfast_aim 2, twin_arrows 2, shattered_defenses 2, point_blank 2, adrenaline_rush 1 |
| Ranger | 35 | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Quarterstaff, Deepwater Shortbow | piercing_force 5, jagged_barbs 5, guerilla_tactics 5, rhythmic_draw 5, steadfast_aim 2, twin_arrows 2, shattered_defenses 2, point_blank 2, adrenaline_rush 1 |
| Initiate | 12 | Brackwell Wool Robe, Stonefield Pendant, Maclure Circle, Lion's Pride Seal, Stonefield Cudgel, Vigil Candle, Familiar's Spark | ley_line_tap 1, volatile_embers 1, thermal_feedback 1 |
| Initiate | 15 | Crystal Lake Wool Robe, Fargodeep Pendant, Kobold Circle, Courier's Seal, Crystal Lake Quarterstaff, Familiar's Spark | ley_line_tap 3, volatile_embers 2, thermal_feedback 2 |
| Initiate | 16 | Seer's Robe, Fargodeep Pendant, Kobold Circle, Courier's Seal, Crystal Lake Quarterstaff, Familiar's Spark | ley_line_tap 3, volatile_embers 3, thermal_feedback 2 |
| Initiate | 17 | Seer's Robe, Fargodeep Pendant, Kobold Circle, Courier's Seal, Crystal Lake Quarterstaff, Familiar's Spark | ley_line_tap 3, volatile_embers 3, thermal_feedback 2, ley_line_attunement 1, frigid_conservation 1 |
| Initiate | 18 | Mystic's Wrap, Fargodeep Pendant, Rite Band, Kobold Circle, Crystal Lake Quarterstaff, Familiar's Spark | ley_line_tap 3, volatile_embers 3, thermal_feedback 3, ley_line_attunement 1, frigid_conservation 1 |
| Initiate | 19 | Stone Cairn Wool Robe, Azora Pendant, Murloc Circle, Rite Band, Azora Cudgel, Murloc Lantern, Familiar's Spark | ley_line_tap 4, volatile_embers 4, thermal_feedback 3, ley_line_attunement 1, frigid_conservation 1 |
| Initiate | 20 | Stone Cairn Wool Robe, Azora Pendant, Murloc Circle, Rite Band, Azora Cudgel, Murloc Lantern, Familiar's Spark | ley_line_tap 4, volatile_embers 4, thermal_feedback 3, ley_line_attunement 2, frigid_conservation 1 |
| Initiate | 21 | Stone Cairn Wool Robe, Azora Pendant, Murloc Circle, Rite Band, Timberman's Staff, Familiar's Spark | ley_line_tap 4, volatile_embers 4, thermal_feedback 3, ley_line_attunement 2, frigid_conservation 2, catalytic_resonance 1 |
| Initiate | 22 | Bright Armor, Azora Pendant, Ridgewatch Seal, Murloc Circle, Timberman's Staff, Familiar's Spark | ley_line_tap 4, volatile_embers 4, thermal_feedback 3, ley_line_attunement 2, frigid_conservation 2, catalytic_resonance 1, arcane_momentum 1 |
| Initiate | 23 | Eastvale Wool Robe, Ridgepoint Pendant, Mirror Lake Circle, Ridgewatch Seal, Ridgepoint Cudgel, Mirror Lake Lantern, Familiar's Spark | ley_line_tap 5, volatile_embers 4, thermal_feedback 4, ley_line_attunement 2, frigid_conservation 2, catalytic_resonance 1, arcane_momentum 1 |
| Initiate | 24 | Eastvale Wool Robe, Ridgepoint Pendant, Ledger Band, Mirror Lake Circle, Ridgepoint Cudgel, Mirror Lake Lantern, Familiar's Spark | ley_line_tap 5, volatile_embers 5, thermal_feedback 4, ley_line_attunement 2, frigid_conservation 2, catalytic_resonance 1, arcane_momentum 1 |
| Initiate | 25 | Eastvale Wool Robe, Ridgepoint Pendant, Ledger Band, Mirror Lake Circle, Pit Mace, Mirror Lake Lantern, Familiar's Spark | ley_line_tap 5, volatile_embers 5, thermal_feedback 4, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 2, arcane_momentum 1 |
| Initiate | 26 | Silver-thread Robe, Ridgepoint Pendant, Boatburner's Seal, Ledger Band, Pit Mace, Mirror Lake Lantern, Familiar's Spark | ley_line_tap 5, volatile_embers 5, thermal_feedback 4, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 2, arcane_momentum 2 |
| Initiate | 27 | Silver-thread Robe, Riverpaw Pendant, Boatburner's Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | ley_line_tap 5, volatile_embers 5, thermal_feedback 5, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 2, arcane_momentum 2 |
| Initiate | 28 | Silver-thread Robe, Riverpaw Pendant, Boatburner's Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | ley_line_tap 5, volatile_embers 5, thermal_feedback 5, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 3, arcane_momentum 3 |
| Initiate | 29 | Silver-thread Robe, Riverpaw Pendant, Boatburner's Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | ley_line_tap 5, volatile_embers 5, thermal_feedback 5, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 4, arcane_momentum 3 |
| Initiate | 30 | Silver-thread Robe, Riverpaw Pendant, Boatburner's Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | ley_line_tap 5, volatile_embers 5, thermal_feedback 5, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 5, arcane_momentum 3 |
| Initiate | 31 | Silver-thread Robe, Riverpaw Pendant, Boatburner's Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | ley_line_tap 5, volatile_embers 5, thermal_feedback 5, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 5, arcane_momentum 3 |
| Initiate | 35 | Silver-thread Robe, Riverpaw Pendant, Boatburner's Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | ley_line_tap 5, volatile_embers 5, thermal_feedback 5, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 5, arcane_momentum 3 |
| Vanguard | 12 | Lion's Pride Plate Cuirass, Lion's Pride Torc, Maclure Band, Lion's Pride Signet, Brackwell Cutlass, Stonefield Kite Shield | divine_strength 2, wrathful_iron 1 |
| Vanguard | 15 | Grik's Plate Cuirass, Grik's Torc, Kobold Band, Courier's Signet, Crystal Lake Cutlass, Fargodeep Kite Shield | divine_strength 3, wrathful_iron 3, hallowed_ground 1 |
| Vanguard | 16 | Grik's Plate Cuirass, Grik's Torc, Kobold Band, Courier's Signet, Vigil Saber, Fargodeep Kite Shield | divine_strength 4, wrathful_iron 3, hallowed_ground 1 |
| Vanguard | 17 | Grik's Plate Cuirass, Grik's Torc, Kobold Band, Courier's Signet, Vigil Saber, Kobold King's Aegis | divine_strength 4, wrathful_iron 4, hallowed_ground 2 |
| Vanguard | 18 | Grik's Plate Cuirass, Grik's Torc, Standing Stone Ring, Kobold Band, Vigil Saber, Kobold King's Aegis | divine_strength 5, wrathful_iron 4, hallowed_ground 2 |
| Vanguard | 19 | Cairn Watch Plate Cuirass, Cairn Watch Torc, Murloc Band, Standing Stone Ring, Stone Cairn Cutlass, Azora Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 2, righteous_fury 1 |
| Vanguard | 20 | Cairn Watch Plate Cuirass, Cairn Watch Torc, Murloc Band, Standing Stone Ring, Stone Cairn Cutlass, Azora Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 2, righteous_fury 1, zealous_restoration 1 |
| Vanguard | 21 | Cairn Watch Plate Cuirass, Cairn Watch Torc, Murloc Band, Standing Stone Ring, Timberman's Axe-Sword, Apprentice's Bulwark | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 1, zealous_restoration 1, hammer_of_vanguard 1 |
| Vanguard | 22 | Cairn Watch Plate Cuirass, Cairn Watch Torc, Ridgewatch Signet, Murloc Band, Timberman's Axe-Sword, Apprentice's Bulwark | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 2, zealous_restoration 1, hammer_of_vanguard 1 |
| Vanguard | 23 | Smuggler's Plate Cuirass, Smuggler's Torc, Mirror Lake Band, Ridgewatch Signet, Eastvale Cutlass, Ridgepoint Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 3, zealous_restoration 2, hammer_of_vanguard 1 |
| Vanguard | 24 | Smuggler's Plate Cuirass, Smuggler's Torc, Ledger Signet, Mirror Lake Band, Eastvale Cutlass, Ridgepoint Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 3, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 25 | Smuggler's Plate Cuirass, Smuggler's Torc, Ledger Signet, Mirror Lake Band, Pit Sword, Ridgepoint Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 26 | Smuggler's Plate Cuirass, Smuggler's Torc, Boatburner's Signet, Ledger Signet, Pit Sword, Ridgepoint Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 27 | Deepwater Plate Cuirass, Deepwater Torc, Forest's Edge Band, Deepwater Signet, Westbrook Cutlass, Riverpaw Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 28 | Deepwater Plate Cuirass, Deepwater Torc, Forest's Edge Band, Deepwater Signet, Westbrook Cutlass, Riverpaw Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 29 | Deepwater Plate Cuirass, Deepwater Torc, Forest's Edge Band, Deepwater Signet, Westbrook Cutlass, Riverpaw Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 30 | Deepwater Plate Cuirass, Deepwater Torc, Forest's Edge Band, Deepwater Signet, Westbrook Cutlass, Riverpaw Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 31 | Deepwater Plate Cuirass, Deepwater Torc, Forest's Edge Band, Deepwater Signet, Westbrook Cutlass, Riverpaw Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 35 | Deepwater Plate Cuirass, Deepwater Torc, Forest's Edge Band, Deepwater Signet, Westbrook Cutlass, Riverpaw Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Acolyte | 12 | Brackwell Wool Robe, Stonefield Pendant, Lion's Pride Seal, Maclure Circle, Stonefield Cudgel, Vigil Candle, Familiar's Spark | virulence 1, vitality 1, masochism 1 |
| Acolyte | 15 | Crystal Lake Wool Robe, Fargodeep Pendant, Kobold Circle, Grik's Seal, Crystal Lake Quarterstaff, Familiar's Spark | virulence 3, vitality 2, masochism 2 |
| Acolyte | 16 | Seer's Robe, Fargodeep Pendant, Kobold Circle, Grik's Seal, Crystal Lake Quarterstaff, Familiar's Spark | virulence 3, vitality 3, masochism 2 |
| Acolyte | 17 | Seer's Robe, Fargodeep Pendant, Kobold Circle, Grik's Seal, Crystal Lake Quarterstaff, Familiar's Spark | virulence 3, vitality 3, masochism 3, crimson_surge 1 |
| Acolyte | 18 | Seer's Robe, Fargodeep Pendant, Rite Band, Kobold Circle, Crystal Lake Quarterstaff, Familiar's Spark | virulence 4, vitality 3, masochism 3, crimson_surge 1 |
| Acolyte | 19 | Stone Cairn Wool Robe, Azora Pendant, Murloc Circle, Cairn Watch Seal, Azora Cudgel, Murloc Lantern, Familiar's Spark | virulence 4, vitality 4, masochism 4, crimson_surge 1 |
| Acolyte | 20 | Stone Cairn Wool Robe, Azora Pendant, Murloc Circle, Cairn Watch Seal, Azora Cudgel, Murloc Lantern, Familiar's Spark | virulence 4, vitality 4, masochism 4, crimson_surge 2 |
| Acolyte | 21 | Stone Cairn Wool Robe, Azora Pendant, Murloc Circle, Cairn Watch Seal, Timberman's Staff, Familiar's Spark | virulence 5, vitality 4, masochism 4, crimson_surge 2, will_to_survive 1 |
| Acolyte | 22 | Bright Robe, Azora Pendant, Ridgewatch Seal, Murloc Circle, Timberman's Staff, Familiar's Spark | virulence 5, vitality 4, masochism 4, crimson_surge 2, will_to_survive 1, morbid_fascination 1 |
| Acolyte | 23 | Bright Robe, Ridgepoint Pendant, Mirror Lake Circle, Smuggler's Seal, Ridgepoint Cudgel, Mirror Lake Lantern, Familiar's Spark | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 1, morbid_fascination 1 |
| Acolyte | 24 | Bright Robe, Ridgepoint Pendant, Ledger Band, Mirror Lake Circle, Ridgepoint Cudgel, Mirror Lake Lantern, Familiar's Spark | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 2, morbid_fascination 1 |
| Acolyte | 25 | Bright Robe, Ridgepoint Pendant, Ledger Band, Mirror Lake Circle, Pit Mace, Mirror Lake Lantern, Familiar's Spark | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 2 |
| Acolyte | 26 | Silver-thread Robe, Ridgepoint Pendant, Boatburner's Seal, Ledger Band, Pit Mace, Mirror Lake Lantern, Familiar's Spark | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 3 |
| Acolyte | 27 | Silver-thread Robe, Riverpaw Pendant, Deepwater Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 3 |
| Acolyte | 28 | Silver-thread Robe, Riverpaw Pendant, Deepwater Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 3 |
| Acolyte | 29 | Silver-thread Robe, Riverpaw Pendant, Deepwater Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 3 |
| Acolyte | 30 | Silver-thread Robe, Riverpaw Pendant, Deepwater Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 3 |
| Acolyte | 31 | Silver-thread Robe, Riverpaw Pendant, Deepwater Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 3 |
| Acolyte | 35 | Silver-thread Robe, Riverpaw Pendant, Deepwater Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 3 |
