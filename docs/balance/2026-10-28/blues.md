# Balance check 3, report 3: blues against two elites, the generic boss and every named boss

Issue #487. Rules: `docs/balance/rules.md`. Model: `tools/twilights-eve/balance/` from #484, with one bug fix in this ticket (27acd450, below).

## Command

Data and model from commit `27acd450`, 300 fights a cell. The whole run took 18 s, so it stays at 300 fights.

```
~/.local/bin/python3.15 tools/twilights-eve/balance/run.py --scenario blues --levels 10-30 --fights 300 --jobs 4 --out target/agent/blues-raw.md
~/.local/bin/python3.15 tools/twilights-eve/balance/run.py --scenario named --fights 300 --jobs 4 --out target/agent/named-raw.md
```

`run.py` has no `--gear` or `--append`: `named` always fights in blues, and each run writes its own file. The
tables below come from `target/agent/blues487.py` (not committed), which calls the same scenario functions
(`scenarios.b_2e`, `b_boss`, `b_named`) through the same 4-process pool and keeps the rows as JSON. Every cell's
seed comes from the cell itself, so `run.py` prints the same numbers; the two raw files were checked against it.

`run.py --scenario named` fights a boss below 10 at level 10. The ticket wants that boss fought by the Basic
Warrior in whites at its own level instead, so that one cell is
`scenarios.run_cell(scenarios.cell("basic_warrior", 7, "white", scenarios.named(vurx), 300))`, reported
under T0 below.

Win is the share of fights won, rounded to a whole percent, so 0% can hide one or two wins. Health left is the
mean share of maximum health at the end of the won fights and kill time the median length of a won fight, both
`-` when none was won. `Green in` names the slots where the Calling has no Rare row at that level and wears its
green.

### The model fix

`boss.py` gave the comment paragraph above a `[[template]]` to the template before it, so a boss's Zone Factor
came from a neighbour's comment. Abbot Ashworth was fought at Elwynn's 1.5 health and 1.3 damage though his
spawn is in `northshire_abbey.toml`; the Eye Below at Northshire's 1.0 though `jasperlode.toml` is zone 12. Each
boss now takes the zone of its spawn's Phased Dungeon file, else the Northshire words in its own comments, its
Level Band or its spawn row, else Elwynn. The Abbot's health went from 3418 to 2278 and the Eye's from 6152 to
9228. Scythemaw, the Rake and Ghost Howl spawn only in `barrens.toml`, the Barrens test zone, where the realm
applies no Zone Factor; the model cannot read their area from those rows and fights them at Elwynn's.

## B-2E: each Calling in blues against two elites

Each must win 80% or more against two elites on level.

### Cutthroat

| Level | Win | Health left | Kill s | Holds | Green in |
|---|---|---|---|---|---|
| 10 | 100% | 84% | 39.4 | yes | Weapon |
| 11 | 99% | 55% | 62.1 | yes | Weapon |
| 12 | 81% | 36% | 67.7 | yes | Weapon |
| 13 | 41% | 25% | 72.1 | no | Weapon |
| 14 | 12% | 19% | 74.7 | no | Weapon |
| 15 | 100% | 54% | 57.7 | yes | - |
| 16 | 96% | 41% | 62.1 | yes | - |
| 17 | 88% | 31% | 64.1 | yes | - |
| 18 | 78% | 29% | 60.8 | no | - |
| 19 | 98% | 53% | 56.1 | yes | - |
| 20 | 94% | 44% | 59.4 | yes | - |
| 21 | 100% | 60% | 53.3 | yes | - |
| 22 | 97% | 56% | 54.1 | yes | - |
| 23 | 100% | 79% | 52.1 | yes | - |
| 24 | 98% | 75% | 54.1 | yes | - |
| 25 | 97% | 69% | 54.4 | yes | - |
| 26 | 94% | 69% | 55.4 | yes | - |
| 27 | 100% | 87% | 44.5 | yes | - |
| 28 | 100% | 86% | 44.1 | yes | - |
| 29 | 100% | 84% | 44.7 | yes | - |
| 30 | 100% | 81% | 46.1 | yes | - |

### Ranger

| Level | Win | Health left | Kill s | Holds | Green in |
|---|---|---|---|---|---|
| 10 | 7% | 14% | 75.8 | no | - |
| 11 | 0% | - | - | no | - |
| 12 | 0% | - | - | no | - |
| 13 | 0% | - | - | no | - |
| 14 | 0% | - | - | no | - |
| 15 | 0% | - | - | no | - |
| 16 | 0% | - | - | no | - |
| 17 | 0% | - | - | no | - |
| 18 | 0% | - | - | no | - |
| 19 | 0% | - | - | no | - |
| 20 | 0% | - | - | no | - |
| 21 | 0% | - | - | no | - |
| 22 | 0% | - | - | no | - |
| 23 | 0% | - | - | no | - |
| 24 | 0% | - | - | no | - |
| 25 | 0% | - | - | no | - |
| 26 | 0% | - | - | no | - |
| 27 | 1% | 4% | 43.9 | no | - |
| 28 | 0% | - | - | no | - |
| 29 | 0% | - | - | no | - |
| 30 | 0% | - | - | no | - |

### Initiate

| Level | Win | Health left | Kill s | Holds | Green in |
|---|---|---|---|---|---|
| 10 | 0% | - | - | no | - |
| 11 | 0% | - | - | no | - |
| 12 | 0% | - | - | no | - |
| 13 | 0% | - | - | no | - |
| 14 | 0% | - | - | no | - |
| 15 | 0% | - | - | no | - |
| 16 | 0% | - | - | no | - |
| 17 | 0% | - | - | no | - |
| 18 | 0% | - | - | no | - |
| 19 | 0% | - | - | no | - |
| 20 | 0% | - | - | no | - |
| 21 | 0% | - | - | no | - |
| 22 | 0% | - | - | no | - |
| 23 | 0% | - | - | no | - |
| 24 | 0% | - | - | no | - |
| 25 | 0% | - | - | no | - |
| 26 | 0% | - | - | no | - |
| 27 | 0% | - | - | no | - |
| 28 | 0% | - | - | no | - |
| 29 | 0% | - | - | no | - |
| 30 | 0% | - | - | no | - |

### Vanguard

| Level | Win | Health left | Kill s | Holds | Green in |
|---|---|---|---|---|---|
| 10 | 52% | 14% | 51.0 | no | - |
| 11 | 0% | - | - | no | - |
| 12 | 0% | - | - | no | - |
| 13 | 0% | - | - | no | - |
| 14 | 0% | - | - | no | - |
| 15 | 1% | 6% | 56.8 | no | - |
| 16 | 0% | - | - | no | - |
| 17 | 0% | - | - | no | - |
| 18 | 0% | - | - | no | - |
| 19 | 0% | 24% | 56.8 | no | - |
| 20 | 0% | - | - | no | - |
| 21 | 0% | - | - | no | - |
| 22 | 0% | - | - | no | - |
| 23 | 0% | - | - | no | - |
| 24 | 0% | - | - | no | - |
| 25 | 0% | - | - | no | - |
| 26 | 0% | - | - | no | - |
| 27 | 2% | 13% | 58.8 | no | - |
| 28 | 0% | 7% | 44.5 | no | - |
| 29 | 0% | - | - | no | - |
| 30 | 0% | - | - | no | - |

### Acolyte

| Level | Win | Health left | Kill s | Holds | Green in |
|---|---|---|---|---|---|
| 10 | 0% | - | - | no | - |
| 11 | 0% | - | - | no | - |
| 12 | 0% | - | - | no | - |
| 13 | 0% | - | - | no | - |
| 14 | 0% | - | - | no | - |
| 15 | 0% | - | - | no | - |
| 16 | 0% | - | - | no | - |
| 17 | 0% | - | - | no | - |
| 18 | 0% | - | - | no | - |
| 19 | 0% | - | - | no | - |
| 20 | 0% | - | - | no | - |
| 21 | 0% | - | - | no | - |
| 22 | 0% | - | - | no | - |
| 23 | 0% | - | - | no | - |
| 24 | 0% | - | - | no | - |
| 25 | 0% | - | - | no | - |
| 26 | 0% | - | - | no | - |
| 27 | 0% | - | - | no | - |
| 28 | 0% | - | - | no | - |
| 29 | 0% | - | - | no | - |
| 30 | 0% | - | - | no | - |

## B-boss: each Calling in blues against the generic boss

Each must win 80% or more against the generic boss on level.

### Cutthroat

| Level | Win | Health left | Kill s | Holds | Green in |
|---|---|---|---|---|---|
| 10 | 100% | 91% | 38.1 | yes | Weapon |
| 11 | 100% | 66% | 62.1 | yes | Weapon |
| 12 | 92% | 45% | 68.1 | yes | Weapon |
| 13 | 65% | 33% | 72.8 | no | Weapon |
| 14 | 29% | 21% | 76.1 | no | Weapon |
| 15 | 100% | 63% | 56.1 | yes | - |
| 16 | 99% | 49% | 61.6 | yes | - |
| 17 | 95% | 41% | 64.1 | yes | - |
| 18 | 94% | 36% | 64.1 | yes | - |
| 19 | 100% | 56% | 58.0 | yes | - |
| 20 | 99% | 48% | 61.5 | yes | - |
| 21 | 100% | 71% | 52.1 | yes | - |
| 22 | 100% | 62% | 53.7 | yes | - |
| 23 | 100% | 78% | 52.3 | yes | - |
| 24 | 100% | 74% | 53.8 | yes | - |
| 25 | 100% | 70% | 54.1 | yes | - |
| 26 | 99% | 68% | 55.4 | yes | - |
| 27 | 100% | 88% | 42.9 | yes | - |
| 28 | 100% | 86% | 42.1 | yes | - |
| 29 | 100% | 84% | 43.9 | yes | - |
| 30 | 100% | 81% | 44.1 | yes | - |

### Ranger

| Level | Win | Health left | Kill s | Holds | Green in |
|---|---|---|---|---|---|
| 10 | 79% | 25% | 70.5 | no | - |
| 11 | 0% | 1% | 97.3 | no | - |
| 12 | 0% | - | - | no | - |
| 13 | 0% | - | - | no | - |
| 14 | 0% | - | - | no | - |
| 15 | 0% | - | - | no | - |
| 16 | 0% | - | - | no | - |
| 17 | 0% | - | - | no | - |
| 18 | 0% | - | - | no | - |
| 19 | 0% | - | - | no | - |
| 20 | 0% | - | - | no | - |
| 21 | 0% | - | - | no | - |
| 22 | 0% | - | - | no | - |
| 23 | 12% | 12% | 51.9 | no | - |
| 24 | 5% | 11% | 54.2 | no | - |
| 25 | 2% | 11% | 49.5 | no | - |
| 26 | 2% | 9% | 54.2 | no | - |
| 27 | 47% | 20% | 47.6 | no | - |
| 28 | 47% | 18% | 47.0 | no | - |
| 29 | 31% | 15% | 47.6 | no | - |
| 30 | 25% | 17% | 47.6 | no | - |

### Initiate

| Level | Win | Health left | Kill s | Holds | Green in |
|---|---|---|---|---|---|
| 10 | 0% | 1% | 43.2 | no | - |
| 11 | 0% | - | - | no | - |
| 12 | 0% | - | - | no | - |
| 13 | 0% | - | - | no | - |
| 14 | 0% | - | - | no | - |
| 15 | 0% | - | - | no | - |
| 16 | 0% | - | - | no | - |
| 17 | 0% | - | - | no | - |
| 18 | 0% | - | - | no | - |
| 19 | 0% | - | - | no | - |
| 20 | 0% | - | - | no | - |
| 21 | 0% | - | - | no | - |
| 22 | 0% | - | - | no | - |
| 23 | 0% | - | - | no | - |
| 24 | 0% | - | - | no | - |
| 25 | 0% | - | - | no | - |
| 26 | 0% | - | - | no | - |
| 27 | 0% | - | - | no | - |
| 28 | 0% | - | - | no | - |
| 29 | 0% | - | - | no | - |
| 30 | 0% | - | - | no | - |

### Vanguard

| Level | Win | Health left | Kill s | Holds | Green in |
|---|---|---|---|---|---|
| 10 | 92% | 25% | 63.2 | yes | - |
| 11 | 0% | - | - | no | - |
| 12 | 0% | - | - | no | - |
| 13 | 0% | - | - | no | - |
| 14 | 0% | - | - | no | - |
| 15 | 10% | 10% | 71.1 | no | - |
| 16 | 1% | 12% | 74.2 | no | - |
| 17 | 0% | - | - | no | - |
| 18 | 0% | - | - | no | - |
| 19 | 14% | 8% | 71.1 | no | - |
| 20 | 6% | 11% | 73.2 | no | - |
| 21 | 3% | 10% | 68.1 | no | - |
| 22 | 0% | 3% | 69.1 | no | - |
| 23 | 0% | 2% | 69.1 | no | - |
| 24 | 0% | 5% | 69.7 | no | - |
| 25 | 0% | - | - | no | - |
| 26 | 0% | - | - | no | - |
| 27 | 20% | 12% | 67.0 | no | - |
| 28 | 10% | 14% | 67.0 | no | - |
| 29 | 4% | 12% | 70.1 | no | - |
| 30 | 0% | - | - | no | - |

### Acolyte

| Level | Win | Health left | Kill s | Holds | Green in |
|---|---|---|---|---|---|
| 10 | 83% | 50% | 44.5 | yes | - |
| 11 | 0% | - | - | no | - |
| 12 | 0% | - | - | no | - |
| 13 | 0% | - | - | no | - |
| 14 | 0% | - | - | no | - |
| 15 | 0% | - | - | no | - |
| 16 | 0% | - | - | no | - |
| 17 | 0% | - | - | no | - |
| 18 | 0% | - | - | no | - |
| 19 | 0% | - | - | no | - |
| 20 | 0% | - | - | no | - |
| 21 | 0% | - | - | no | - |
| 22 | 0% | - | - | no | - |
| 23 | 0% | - | - | no | - |
| 24 | 0% | - | - | no | - |
| 25 | 0% | - | - | no | - |
| 26 | 0% | - | - | no | - |
| 27 | 0% | - | - | no | - |
| 28 | 0% | - | - | no | - |
| 29 | 0% | - | - | no | - |
| 30 | 0% | - | - | no | - |

### The generic boss at each level

`model.mob_stats(level, rank=1, health_mul=2.0, damage_mul=1.1)`: a warrior-class elite with no spells,
in the on-level area's Zone Factor (Northshire at 10, Elwynn from 11). The two elites of B-2E are the
same creature with multipliers 1.0. It swings every 2.0 s.

| Level | Generic boss health | Mean swing | Armour | Elite health | Elite mean swing |
|---|---|---|---|---|---|
| 10 | 2534 | 75.4 | 492 | 1267 | 68.6 |
| 11 | 4182 | 107.9 | 518 | 2091 | 98.1 |
| 12 | 4562 | 117.7 | 552 | 2281 | 107.0 |
| 13 | 4942 | 127.5 | 585 | 2471 | 115.9 |
| 14 | 5322 | 137.3 | 619 | 2661 | 124.8 |
| 15 | 5702 | 147.1 | 653 | 2851 | 133.7 |
| 16 | 6083 | 156.9 | 688 | 3041 | 142.6 |
| 17 | 6463 | 166.7 | 722 | 3231 | 151.6 |
| 18 | 6843 | 176.5 | 755 | 3421 | 160.5 |
| 19 | 7223 | 186.3 | 789 | 3612 | 169.4 |
| 20 | 7603 | 196.1 | 822 | 3802 | 178.3 |
| 21 | 7983 | 205.9 | 858 | 3992 | 187.2 |
| 22 | 8364 | 215.7 | 892 | 4182 | 196.1 |
| 23 | 8744 | 225.5 | 925 | 4372 | 205.0 |
| 24 | 9124 | 235.4 | 959 | 4562 | 214.0 |
| 25 | 9504 | 245.2 | 992 | 4752 | 222.9 |
| 26 | 9884 | 255.0 | 1026 | 4942 | 231.8 |
| 27 | 10264 | 264.8 | 1061 | 5132 | 240.7 |
| 28 | 10644 | 274.6 | 1093 | 5322 | 249.6 |
| 29 | 11025 | 284.4 | 1127 | 5512 | 258.5 |
| 30 | 11405 | 294.2 | 1160 | 5702 | 267.4 |

## B-named: each Calling in blues against every named boss

Each must win 80% or more against each named boss, at the boss's level. `Holds` is yes when all five do, else
it names the Callings that miss.

| Boss | Level | Cutthroat win | Ranger win | Initiate win | Vanguard win | Acolyte win | Holds |
|---|---|---|---|---|---|---|---|
| Abbot Ashworth (96005) | 10 | 100% | 100% | 3% | 100% | 100% | no: Initiate |
| Bloodtalon Scythemaw (3123) | 10 | 100% | 100% | 100% | 100% | 100% | yes |
| Garrick Padfoot (103) | 10 | 100% | 98% | 13% | 99% | 95% | no: Initiate |
| The Rake (5807) | 10 | 100% | 100% | 68% | 100% | 100% | no: Initiate |
| Ghost Howl (3056) | 12 | 100% | 85% | 0% | 87% | 38% | no: Initiate, Acolyte |
| Princess (330) | 12 | 97% | 0% | 0% | 0% | 0% | no: Ranger, Initiate, Vanguard, Acolyte |
| Morgan the Collector (96012) | 14 | 66% | 0% | 0% | 0% | 0% | no: Cutthroat, Ranger, Initiate, Vanguard, Acolyte |
| Narg the Taskmaster (79) | 14 | 60% | 0% | 0% | 0% | 0% | no: Cutthroat, Ranger, Initiate, Vanguard, Acolyte |
| Gllrg the Tidecaller (96014) | 15 | 100% | 100% | 91% | 100% | 100% | yes |
| Ritualist Serane (96025) | 18 | 100% | 5% | 0% | 98% | 82% | no: Ranger, Initiate |
| The Eye Below (96022) | 18 | 96% | 0% | 0% | 3% | 65% | no: Ranger, Initiate, Vanguard, Acolyte |
| Gresh the Drowned (96034) | 20 | 100% | 35% | 10% | 100% | 100% | no: Ranger, Initiate |
| Lady Anacondra (3671) | 20 | 100% | 100% | 100% | 100% | 100% | yes |
| Speaker Thane (96035) | 21 | 100% | 17% | 0% | 100% | 100% | no: Ranger, Initiate |
| Shadow of Morganth (96037) | 22 | 100% | 0% | 0% | 68% | 100% | no: Ranger, Initiate, Vanguard |
| The Kept (96036) | 22 | 100% | 0% | 0% | 100% | 100% | no: Ranger, Initiate |
| Speaker Vael (96040) | 24 | 100% | 100% | 79% | 100% | 100% | no: Initiate |
| Acolyte Marren (96043) | 26 | 100% | 100% | 16% | 100% | 100% | no: Initiate |
| Hogger (96010) | 30 | 96% | 0% | 0% | 0% | 0% | no: Ranger, Initiate, Vanguard, Acolyte |

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

The named boss's own numbers, the generic boss's at the same level, and each Calling's win against the named
boss minus its B-boss win at that level, in points. A large plus is a boss far easier than the generic one.

| Boss | Level | Class | Rank | Health x / damage x | Area | Health | Mean swing | Generic health | Generic swing | Cutthroat vs generic | Ranger vs generic | Initiate vs generic | Vanguard vs generic | Acolyte vs generic |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Abbot Ashworth | 10 | mage | 1 | 2 / 1.1 | Northshire | 2278 | 55.3 | 2534 | 75.4 | +0 | +21 | +3 | +8 | +17 |
| Bloodtalon Scythemaw | 10 | warrior | 0 | 1 / 1 | Elwynn | 950 | 59.4 | 2534 | 75.4 | +0 | +21 | +100 | +8 | +17 |
| Garrick Padfoot | 10 | warrior | 1 | 1.6 / 1.1 | Northshire | 2028 | 75.4 | 2534 | 75.4 | +0 | +19 | +13 | +7 | +12 |
| The Rake | 10 | warrior | 4 | 1 / 0.6 | Elwynn | 1426 | 44.6 | 2534 | 75.4 | +0 | +21 | +68 | +8 | +17 |
| Ghost Howl | 12 | warrior | 4 | 1 / 1.25 | Elwynn | 1711 | 111.4 | 4562 | 117.7 | +8 | +85 | +0 | +87 | +38 |
| Princess | 12 | warrior | 1 | 1.6 / 1.1 | Elwynn | 3650 | 117.7 | 4562 | 117.7 | +5 | +0 | +0 | +0 | +0 |
| Morgan the Collector | 14 | warrior | 1 | 1.6 / 1.1 | Elwynn | 4258 | 137.3 | 5322 | 137.3 | +37 | +0 | +0 | +0 | +0 |
| Narg the Taskmaster | 14 | warrior | 1 | 1.6 / 1.1 | Elwynn | 4258 | 137.3 | 5322 | 137.3 | +31 | +0 | +0 | +0 | +0 |
| Gllrg the Tidecaller | 15 | mage | 1 | 1.4 / 0.9 | Elwynn | 3588 | 88.2 | 5702 | 147.1 | +0 | +100 | +91 | +90 | +100 |
| Ritualist Serane | 18 | mage | 1 | 1.6 / 1 | Elwynn | 4921 | 117.5 | 6843 | 176.5 | +6 | +5 | +0 | +98 | +82 |
| The Eye Below | 18 | mage | 1 | 3 / 1 | Elwynn | 9228 | 117.5 | 6843 | 176.5 | +2 | +0 | +0 | +3 | +65 |
| Gresh the Drowned | 20 | mage | 1 | 2 / 1 | Elwynn | 6835 | 130.6 | 7603 | 196.1 | +1 | +35 | +10 | +94 | +100 |
| Lady Anacondra | 20 | paladin | 1 | 0.4547 / 0.2494 | Wailing Caverns | 1083 | 31.8 | 7603 | 196.1 | +1 | +100 | +100 | +94 | +100 |
| Speaker Thane | 21 | mage | 1 | 2 / 1 | Elwynn | 7177 | 137.1 | 7983 | 205.9 | +0 | +17 | +0 | +97 | +100 |
| Shadow of Morganth | 22 | mage | 1 | 4 / 1 | Elwynn | 15037 | 143.7 | 8364 | 215.7 | +0 | +0 | +0 | +68 | +100 |
| The Kept | 22 | warrior | 1 | 2.5 / 0.5 | Elwynn | 10454 | 98.1 | 8364 | 215.7 | +0 | +0 | +0 | +100 | +100 |
| Speaker Vael | 24 | mage | 1 | 1.3 / 0.8 | Elwynn | 5331 | 125.4 | 9124 | 235.4 | +0 | +95 | +79 | +100 | +100 |
| Acolyte Marren | 26 | mage | 1 | 1.4 / 0.9 | Elwynn | 6220 | 152.8 | 9884 | 255.0 | +1 | +98 | +16 | +100 | +100 |
| Hogger | 30 | warrior | 1 | 3 / 1.5 | Elwynn | 17107 | 401.2 | 11405 | 294.2 | -4 | -25 | +0 | +0 | +0 |

### T0: named bosses below level 10

Fought by the Basic Warrior in starter gear and whites at the boss's own level, as the ticket asks. No rule in
`docs/balance/rules.md` judges a Tier 0 boss, so this row has no verdict and is not in the failures.

| Boss | Level | Win | Health left | Kill s | Boss health | Unmodelled |
|---|---|---|---|---|---|---|
| Geomancer Vurx (96002) | 7 | 100% | 89% | 10.2 | 202 | frost_armor_vurx, frost_chill_vurx |

## Failures

211 of 305 (rule, Calling, level or boss) cells fail: B-2E 87 of 105, B-boss 84 of 105, B-named 40 of 95.

- B-2E, Cutthroat, level 13: win 41%, 39 points under 80%.
- B-2E, Cutthroat, level 14: win 12%, 68 points under 80%.
- B-2E, Cutthroat, level 18: win 78%, 2 points under 80%.
- B-2E, Ranger, level 10: win 7%, 73 points under 80%.
- B-2E, Ranger, level 11: win 0%, 80 points under 80%.
- B-2E, Ranger, level 12: win 0%, 80 points under 80%.
- B-2E, Ranger, level 13: win 0%, 80 points under 80%.
- B-2E, Ranger, level 14: win 0%, 80 points under 80%.
- B-2E, Ranger, level 15: win 0%, 80 points under 80%.
- B-2E, Ranger, level 16: win 0%, 80 points under 80%.
- B-2E, Ranger, level 17: win 0%, 80 points under 80%.
- B-2E, Ranger, level 18: win 0%, 80 points under 80%.
- B-2E, Ranger, level 19: win 0%, 80 points under 80%.
- B-2E, Ranger, level 20: win 0%, 80 points under 80%.
- B-2E, Ranger, level 21: win 0%, 80 points under 80%.
- B-2E, Ranger, level 22: win 0%, 80 points under 80%.
- B-2E, Ranger, level 23: win 0%, 80 points under 80%.
- B-2E, Ranger, level 24: win 0%, 80 points under 80%.
- B-2E, Ranger, level 25: win 0%, 80 points under 80%.
- B-2E, Ranger, level 26: win 0%, 80 points under 80%.
- B-2E, Ranger, level 27: win 1%, 79 points under 80%.
- B-2E, Ranger, level 28: win 0%, 80 points under 80%.
- B-2E, Ranger, level 29: win 0%, 80 points under 80%.
- B-2E, Ranger, level 30: win 0%, 80 points under 80%.
- B-2E, Initiate, level 10: win 0%, 80 points under 80%.
- B-2E, Initiate, level 11: win 0%, 80 points under 80%.
- B-2E, Initiate, level 12: win 0%, 80 points under 80%.
- B-2E, Initiate, level 13: win 0%, 80 points under 80%.
- B-2E, Initiate, level 14: win 0%, 80 points under 80%.
- B-2E, Initiate, level 15: win 0%, 80 points under 80%.
- B-2E, Initiate, level 16: win 0%, 80 points under 80%.
- B-2E, Initiate, level 17: win 0%, 80 points under 80%.
- B-2E, Initiate, level 18: win 0%, 80 points under 80%.
- B-2E, Initiate, level 19: win 0%, 80 points under 80%.
- B-2E, Initiate, level 20: win 0%, 80 points under 80%.
- B-2E, Initiate, level 21: win 0%, 80 points under 80%.
- B-2E, Initiate, level 22: win 0%, 80 points under 80%.
- B-2E, Initiate, level 23: win 0%, 80 points under 80%.
- B-2E, Initiate, level 24: win 0%, 80 points under 80%.
- B-2E, Initiate, level 25: win 0%, 80 points under 80%.
- B-2E, Initiate, level 26: win 0%, 80 points under 80%.
- B-2E, Initiate, level 27: win 0%, 80 points under 80%.
- B-2E, Initiate, level 28: win 0%, 80 points under 80%.
- B-2E, Initiate, level 29: win 0%, 80 points under 80%.
- B-2E, Initiate, level 30: win 0%, 80 points under 80%.
- B-2E, Vanguard, level 10: win 52%, 28 points under 80%.
- B-2E, Vanguard, level 11: win 0%, 80 points under 80%.
- B-2E, Vanguard, level 12: win 0%, 80 points under 80%.
- B-2E, Vanguard, level 13: win 0%, 80 points under 80%.
- B-2E, Vanguard, level 14: win 0%, 80 points under 80%.
- B-2E, Vanguard, level 15: win 1%, 79 points under 80%.
- B-2E, Vanguard, level 16: win 0%, 80 points under 80%.
- B-2E, Vanguard, level 17: win 0%, 80 points under 80%.
- B-2E, Vanguard, level 18: win 0%, 80 points under 80%.
- B-2E, Vanguard, level 19: win 0%, 80 points under 80%.
- B-2E, Vanguard, level 20: win 0%, 80 points under 80%.
- B-2E, Vanguard, level 21: win 0%, 80 points under 80%.
- B-2E, Vanguard, level 22: win 0%, 80 points under 80%.
- B-2E, Vanguard, level 23: win 0%, 80 points under 80%.
- B-2E, Vanguard, level 24: win 0%, 80 points under 80%.
- B-2E, Vanguard, level 25: win 0%, 80 points under 80%.
- B-2E, Vanguard, level 26: win 0%, 80 points under 80%.
- B-2E, Vanguard, level 27: win 2%, 78 points under 80%.
- B-2E, Vanguard, level 28: win 0%, 80 points under 80%.
- B-2E, Vanguard, level 29: win 0%, 80 points under 80%.
- B-2E, Vanguard, level 30: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 10: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 11: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 12: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 13: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 14: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 15: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 16: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 17: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 18: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 19: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 20: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 21: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 22: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 23: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 24: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 25: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 26: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 27: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 28: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 29: win 0%, 80 points under 80%.
- B-2E, Acolyte, level 30: win 0%, 80 points under 80%.
- B-boss, Cutthroat, level 13: win 65%, 15 points under 80%.
- B-boss, Cutthroat, level 14: win 29%, 51 points under 80%.
- B-boss, Ranger, level 10: win 79%, 1 points under 80%.
- B-boss, Ranger, level 11: win 0%, 80 points under 80%.
- B-boss, Ranger, level 12: win 0%, 80 points under 80%.
- B-boss, Ranger, level 13: win 0%, 80 points under 80%.
- B-boss, Ranger, level 14: win 0%, 80 points under 80%.
- B-boss, Ranger, level 15: win 0%, 80 points under 80%.
- B-boss, Ranger, level 16: win 0%, 80 points under 80%.
- B-boss, Ranger, level 17: win 0%, 80 points under 80%.
- B-boss, Ranger, level 18: win 0%, 80 points under 80%.
- B-boss, Ranger, level 19: win 0%, 80 points under 80%.
- B-boss, Ranger, level 20: win 0%, 80 points under 80%.
- B-boss, Ranger, level 21: win 0%, 80 points under 80%.
- B-boss, Ranger, level 22: win 0%, 80 points under 80%.
- B-boss, Ranger, level 23: win 12%, 68 points under 80%.
- B-boss, Ranger, level 24: win 5%, 75 points under 80%.
- B-boss, Ranger, level 25: win 2%, 78 points under 80%.
- B-boss, Ranger, level 26: win 2%, 78 points under 80%.
- B-boss, Ranger, level 27: win 47%, 33 points under 80%.
- B-boss, Ranger, level 28: win 47%, 33 points under 80%.
- B-boss, Ranger, level 29: win 31%, 49 points under 80%.
- B-boss, Ranger, level 30: win 25%, 55 points under 80%.
- B-boss, Initiate, level 10: win 0%, 80 points under 80%.
- B-boss, Initiate, level 11: win 0%, 80 points under 80%.
- B-boss, Initiate, level 12: win 0%, 80 points under 80%.
- B-boss, Initiate, level 13: win 0%, 80 points under 80%.
- B-boss, Initiate, level 14: win 0%, 80 points under 80%.
- B-boss, Initiate, level 15: win 0%, 80 points under 80%.
- B-boss, Initiate, level 16: win 0%, 80 points under 80%.
- B-boss, Initiate, level 17: win 0%, 80 points under 80%.
- B-boss, Initiate, level 18: win 0%, 80 points under 80%.
- B-boss, Initiate, level 19: win 0%, 80 points under 80%.
- B-boss, Initiate, level 20: win 0%, 80 points under 80%.
- B-boss, Initiate, level 21: win 0%, 80 points under 80%.
- B-boss, Initiate, level 22: win 0%, 80 points under 80%.
- B-boss, Initiate, level 23: win 0%, 80 points under 80%.
- B-boss, Initiate, level 24: win 0%, 80 points under 80%.
- B-boss, Initiate, level 25: win 0%, 80 points under 80%.
- B-boss, Initiate, level 26: win 0%, 80 points under 80%.
- B-boss, Initiate, level 27: win 0%, 80 points under 80%.
- B-boss, Initiate, level 28: win 0%, 80 points under 80%.
- B-boss, Initiate, level 29: win 0%, 80 points under 80%.
- B-boss, Initiate, level 30: win 0%, 80 points under 80%.
- B-boss, Vanguard, level 11: win 0%, 80 points under 80%.
- B-boss, Vanguard, level 12: win 0%, 80 points under 80%.
- B-boss, Vanguard, level 13: win 0%, 80 points under 80%.
- B-boss, Vanguard, level 14: win 0%, 80 points under 80%.
- B-boss, Vanguard, level 15: win 10%, 70 points under 80%.
- B-boss, Vanguard, level 16: win 1%, 79 points under 80%.
- B-boss, Vanguard, level 17: win 0%, 80 points under 80%.
- B-boss, Vanguard, level 18: win 0%, 80 points under 80%.
- B-boss, Vanguard, level 19: win 14%, 66 points under 80%.
- B-boss, Vanguard, level 20: win 6%, 74 points under 80%.
- B-boss, Vanguard, level 21: win 3%, 77 points under 80%.
- B-boss, Vanguard, level 22: win 0%, 80 points under 80%.
- B-boss, Vanguard, level 23: win 0%, 80 points under 80%.
- B-boss, Vanguard, level 24: win 0%, 80 points under 80%.
- B-boss, Vanguard, level 25: win 0%, 80 points under 80%.
- B-boss, Vanguard, level 26: win 0%, 80 points under 80%.
- B-boss, Vanguard, level 27: win 20%, 60 points under 80%.
- B-boss, Vanguard, level 28: win 10%, 70 points under 80%.
- B-boss, Vanguard, level 29: win 4%, 76 points under 80%.
- B-boss, Vanguard, level 30: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 11: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 12: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 13: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 14: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 15: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 16: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 17: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 18: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 19: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 20: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 21: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 22: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 23: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 24: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 25: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 26: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 27: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 28: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 29: win 0%, 80 points under 80%.
- B-boss, Acolyte, level 30: win 0%, 80 points under 80%.
- B-named, Initiate, Abbot Ashworth (level 10): win 3%, 77 points under 80%.
- B-named, Initiate, Garrick Padfoot (level 10): win 13%, 67 points under 80%.
- B-named, Initiate, The Rake (level 10): win 68%, 12 points under 80%.
- B-named, Initiate, Ghost Howl (level 12): win 0%, 80 points under 80%.
- B-named, Acolyte, Ghost Howl (level 12): win 38%, 42 points under 80%.
- B-named, Ranger, Princess (level 12): win 0%, 80 points under 80%.
- B-named, Initiate, Princess (level 12): win 0%, 80 points under 80%.
- B-named, Vanguard, Princess (level 12): win 0%, 80 points under 80%.
- B-named, Acolyte, Princess (level 12): win 0%, 80 points under 80%.
- B-named, Cutthroat, Morgan the Collector (level 14): win 66%, 14 points under 80%.
- B-named, Ranger, Morgan the Collector (level 14): win 0%, 80 points under 80%.
- B-named, Initiate, Morgan the Collector (level 14): win 0%, 80 points under 80%.
- B-named, Vanguard, Morgan the Collector (level 14): win 0%, 80 points under 80%.
- B-named, Acolyte, Morgan the Collector (level 14): win 0%, 80 points under 80%.
- B-named, Cutthroat, Narg the Taskmaster (level 14): win 60%, 20 points under 80%.
- B-named, Ranger, Narg the Taskmaster (level 14): win 0%, 80 points under 80%.
- B-named, Initiate, Narg the Taskmaster (level 14): win 0%, 80 points under 80%.
- B-named, Vanguard, Narg the Taskmaster (level 14): win 0%, 80 points under 80%.
- B-named, Acolyte, Narg the Taskmaster (level 14): win 0%, 80 points under 80%.
- B-named, Ranger, Ritualist Serane (level 18): win 5%, 75 points under 80%.
- B-named, Initiate, Ritualist Serane (level 18): win 0%, 80 points under 80%.
- B-named, Ranger, The Eye Below (level 18): win 0%, 80 points under 80%.
- B-named, Initiate, The Eye Below (level 18): win 0%, 80 points under 80%.
- B-named, Vanguard, The Eye Below (level 18): win 3%, 77 points under 80%.
- B-named, Acolyte, The Eye Below (level 18): win 65%, 15 points under 80%.
- B-named, Ranger, Gresh the Drowned (level 20): win 35%, 45 points under 80%.
- B-named, Initiate, Gresh the Drowned (level 20): win 10%, 70 points under 80%.
- B-named, Ranger, Speaker Thane (level 21): win 17%, 63 points under 80%.
- B-named, Initiate, Speaker Thane (level 21): win 0%, 80 points under 80%.
- B-named, Ranger, Shadow of Morganth (level 22): win 0%, 80 points under 80%.
- B-named, Initiate, Shadow of Morganth (level 22): win 0%, 80 points under 80%.
- B-named, Vanguard, Shadow of Morganth (level 22): win 68%, 12 points under 80%.
- B-named, Ranger, The Kept (level 22): win 0%, 80 points under 80%.
- B-named, Initiate, The Kept (level 22): win 0%, 80 points under 80%.
- B-named, Initiate, Speaker Vael (level 24): win 79%, 1 points under 80%.
- B-named, Initiate, Acolyte Marren (level 26): win 16%, 64 points under 80%.
- B-named, Ranger, Hogger (level 30): win 0%, 80 points under 80%.
- B-named, Initiate, Hogger (level 30): win 0%, 80 points under 80%.
- B-named, Vanguard, Hogger (level 30): win 0%, 80 points under 80%.
- B-named, Acolyte, Hogger (level 30): win 0%, 80 points under 80%.

## Reading

- Blues hold for one Calling. The Cutthroat holds B-2E at 18 of 21 levels (not 13, 14 and 18) and B-boss at 19 (not 13 and 14). Every other Calling holds B-boss only at level 10 (Vanguard 92%, Acolyte 83%; the Ranger misses by 1 point at 79%) and B-2E at no level.
- Level 11 is the cliff, as in #486. Elwynn's Zone Factor takes the generic boss from 2534 to 4182 health and from 75 to 108 a swing, while a Calling's blues add 2% (Vanguard) to 22% (Acolyte) health. The Vanguard drops from 92% to 0% and the Acolyte from 83% to 0%. At 20 the Vanguard has 1789 health, takes 99 a swing after armour every 2 s and deals 98 DPS. It lives about 36 s unhealed, but a kill takes about 78 s. The Initiate (867 health, 162 a swing) and the Acolyte (1076, 162) live 11 to 13 s.
- The blue kit's one gap explains the Cutthroat's misses at 13 and 14. It has no Rare weapon until 15 and wears its green there; its DPS is 67 to 70 at 10 to 14 and 101 at 15, and its B-2E win goes from 12% at 14 to 100% at 15. Its 78% at 18 is not a gap. Every other Calling has a Rare row in every slot at every level, so no other failure comes from the kit.
- The Ranger, as in #485, cannot shoot inside 8 yd until Point Blank at 23. In blues its B-2E DPS goes from 60 at 22 to 150 at 23. Against the generic boss it still wins only 2% to 47% from 23. Against the six named bosses from 18 to 22 (all but Anacondra) it wins 0% to 35%, and against Vael at 24 and Marren at 26 it wins 100%.
- The Initiate fails every named boss but three. It wins 91% to 100% against Scythemaw, Gllrg and Anacondra, and 79% or less against the rest. Its health is 397 at 10 and 1285 at 30, the lowest of the five, and it takes the full swing on 217 to 570 armour.
- Most caster bosses are far easier than the generic boss at their level. A mage-class creature swings about 0.73 times a warrior-class one at the same damage multiplier (Serane, 1.0, swings 117.5 at 18; the generic boss, 1.1, swings 176.5). The tanks win 0% to 10% against the generic boss at those levels and 82% to 100% against Gllrg, Serane, Gresh, Thane, Vael and Marren. The Eye Below (3.0 health, 9228) still beats the Vanguard, who wins 3%. The Vanguard wins 68% against the Shadow of Morganth (4.0 health, 15037).
- Five bosses are much easier than the generic boss through their template, not a spell. Scythemaw is rank 0, a normal with 950 health. The Rake is rank 4 (rare, 1.5 health) at 0.6 damage and 1426 health. Ghost Howl is rank 4 with 1711 health against the generic boss's 4562. The Kept has 0.5 damage, a 98 swing against 216. Lady Anacondra keeps vanilla's 0.4547/0.2494 multipliers, is paladin class and sits in Wailing Caverns (zone 718, no Zone Factor). She has 1083 health and a 32 swing against the generic boss's 7603 and 196, and every Calling kills her in 7 to 14 s.
- The warrior-class bosses at 1.6/1.1 (Princess at 12, Morgan and Narg at 14) play like the generic boss: only the Cutthroat wins, at 97%, 66% and 60%. Hogger (3.0/1.5: 17107 health and a 401 swing against 11405 and 294) is the one boss harder than its generic neighbour. The Ranger's win drops 25 points, and only the Cutthroat (96%) beats him. His charge and Eat are unmodelled, so play may be harder still.
- Tier 0: Geomancer Vurx at 7 has 202 health, under half a level-7 normal warrior's 444, so the Basic Warrior in whites wins every fight in 10 s. His multipliers (0.2528/0.2826) leave him easier than a normal. Frost Armor and Frost Chill are unmodelled.

## Kits and builds

As the cells used them, in blues; `Green in` as in the tables above. Talents are the damage build at every
point the level gives. The Basic Warrior row is the T0 Vurx cell, in whites.

| Calling | Level | Kit | Green in | Talents |
|---|---|---|---|---|
| Cutthroat | 10 | Corrupted Brother's Jerkin, Shadowed Choker, Belltower Loop, Disciple's Loop, Twilight Falchion, Padfoot's Shiv (OH) | Weapon | none |
| Cutthroat | 11 | Corrupted Brother's Jerkin, Pasture Pendant, Pasture Loop, Belltower Loop, Twilight Falchion, Maclure Dirk (OH) | Weapon | reflexive_strike 1 |
| Cutthroat | 12 | Corrupted Brother's Jerkin, Pasture Pendant, Pasture Loop, Belltower Loop, Twilight Falchion, Maclure Dirk (OH) | Weapon | reflexive_strike 1, blood_frenzy 1, bloodletting 1 |
| Cutthroat | 13 | Corrupted Brother's Jerkin, Pasture Pendant, Pasture Loop, Belltower Loop, Twilight Falchion, Maclure Dirk (OH) | Weapon | reflexive_strike 1, blood_frenzy 1, bloodletting 1, executioners_toll 1 |
| Cutthroat | 14 | Corrupted Brother's Jerkin, Pasture Pendant, Pasture Loop, Belltower Loop, Twilight Falchion, Morgan's Dirk (OH) | Weapon | reflexive_strike 2, blood_frenzy 2, bloodletting 1, executioners_toll 1 |
| Cutthroat | 15 | Narg's Dread Jerkin, Eye's Talisman, Digger's Loop, Grik's Ring, Digger's Blade, Eye's Dirk (OH) | - | reflexive_strike 2, blood_frenzy 2, bloodletting 2, executioners_toll 1 |
| Cutthroat | 16 | Narg's Dread Jerkin, Eye's Talisman, Digger's Loop, Grik's Ring, Digger's Blade, Eye's Dirk (OH) | - | reflexive_strike 2, blood_frenzy 2, bloodletting 2, executioners_toll 2 |
| Cutthroat | 17 | Narg's Dread Jerkin, Eye's Talisman, Digger's Loop, Grik's Ring, Digger's Blade, Eye's Dirk (OH) | - | reflexive_strike 3, blood_frenzy 3, bloodletting 2, executioners_toll 2 |
| Cutthroat | 18 | Narg's Dread Jerkin, Eye's Talisman, Digger's Loop, Grik's Ring, Digger's Blade, Eye's Dirk (OH) | - | reflexive_strike 3, blood_frenzy 3, bloodletting 2, executioners_toll 2, phantom_guile 1 |
| Cutthroat | 19 | Pointed Jerkin of Azora, Void Talisman, Morganth's Loop, Shadowhide Ring, Morganth's Blade, Void Dirk (OH) | - | reflexive_strike 3, blood_frenzy 3, bloodletting 3, executioners_toll 2, phantom_guile 1, thirst 1 |
| Cutthroat | 20 | Pointed Jerkin of Azora, Void Talisman, Morganth's Loop, Shadowhide Ring, Morganth's Blade, Void Dirk (OH) | - | reflexive_strike 3, blood_frenzy 3, bloodletting 3, executioners_toll 3, phantom_guile 1, thirst 1 |
| Cutthroat | 21 | Pointed Jerkin of Azora, Void Talisman, Morganth's Loop, Shadowhide Ring, Morganth's Blade, Void Dirk (OH) | - | reflexive_strike 4, blood_frenzy 3, bloodletting 3, executioners_toll 3, phantom_guile 1, thirst 1, scent_of_blood 1 |
| Cutthroat | 22 | Pointed Jerkin of Azora, Void Talisman, Morganth's Loop, Shadowhide Ring, Morganth's Blade, Void Dirk (OH) | - | reflexive_strike 4, blood_frenzy 4, bloodletting 3, executioners_toll 3, phantom_guile 1, thirst 1, scent_of_blood 1 |
| Cutthroat | 23 | Pointed Jerkin of Azora, Sermon Pendant, Sermon Loop, Morganth's Loop, Morganth's Blade, Void Dirk (OH) | - | reflexive_strike 4, blood_frenzy 4, bloodletting 3, executioners_toll 3, phantom_guile 2, thirst 2, scent_of_blood 1 |
| Cutthroat | 24 | Pointed Jerkin of Azora, Sermon Pendant, Sermon Loop, Morganth's Loop, Morganth's Blade, Void Dirk (OH) | - | reflexive_strike 4, blood_frenzy 4, bloodletting 3, executioners_toll 3, phantom_guile 2, thirst 2, scent_of_blood 1, flurry_of_blades 1 |
| Cutthroat | 25 | Pointed Jerkin of Azora, Sermon Pendant, Sermon Loop, Morganth's Loop, Morganth's Blade, Void Dirk (OH) | - | reflexive_strike 4, blood_frenzy 4, bloodletting 4, executioners_toll 3, phantom_guile 2, thirst 2, scent_of_blood 2, flurry_of_blades 1 |
| Cutthroat | 26 | Pointed Jerkin of Azora, Sermon Pendant, Sermon Loop, Morganth's Loop, Morganth's Blade, Void Dirk (OH) | - | reflexive_strike 5, blood_frenzy 4, bloodletting 4, executioners_toll 3, phantom_guile 2, thirst 2, scent_of_blood 2, flurry_of_blades 1 |
| Cutthroat | 27 | Hogger's Dread Jerkin, Gnoll Talisman, Riverpaw Loop, Hogger's Ring, Riverpaw Blade, Gnoll Dirk (OH) | - | reflexive_strike 5, blood_frenzy 5, bloodletting 4, executioners_toll 3, phantom_guile 2, thirst 2, scent_of_blood 2, flurry_of_blades 1 |
| Cutthroat | 28 | Hogger's Dread Jerkin, Gnoll Talisman, Riverpaw Loop, Hogger's Ring, Riverpaw Blade, Gnoll Dirk (OH) | - | reflexive_strike 5, blood_frenzy 5, bloodletting 4, executioners_toll 3, phantom_guile 3, thirst 2, scent_of_blood 2, flurry_of_blades 2 |
| Cutthroat | 29 | Hogger's Dread Jerkin, Gnoll Talisman, Riverpaw Loop, Hogger's Ring, Riverpaw Blade, Gnoll Dirk (OH) | - | reflexive_strike 5, blood_frenzy 5, bloodletting 5, executioners_toll 3, phantom_guile 3, thirst 2, scent_of_blood 2, flurry_of_blades 2 |
| Cutthroat | 30 | Hogger's Dread Jerkin, Gnoll Talisman, Riverpaw Loop, Hogger's Ring, Riverpaw Blade, Gnoll Dirk (OH) | - | reflexive_strike 5, blood_frenzy 5, bloodletting 5, executioners_toll 3, phantom_guile 3, thirst 2, scent_of_blood 3, flurry_of_blades 2, ruthlessness 1 |
| Ranger | 10 | Abbey Vanguard's Hauberk, Shadowed Choker, Belltower Loop, Disciple's Loop, Bell-Ringer's Greatmace, Belltower Longbow | - | none |
| Ranger | 11 | Abbey Vanguard's Hauberk, Pasture Pendant, Pasture Loop, Belltower Loop, Bell-Ringer's Greatmace, Belltower Longbow | - | piercing_force 1 |
| Ranger | 12 | Abbey Vanguard's Hauberk, Pasture Pendant, Pasture Loop, Belltower Loop, Bell-Ringer's Greatmace, Belltower Longbow | - | piercing_force 1, jagged_barbs 1, guerilla_tactics 1 |
| Ranger | 13 | Abbey Vanguard's Hauberk, Pasture Pendant, Pasture Loop, Belltower Loop, Bell-Ringer's Greatmace, Belltower Longbow | - | piercing_force 1, jagged_barbs 1, guerilla_tactics 1, rhythmic_draw 1 |
| Ranger | 14 | Abbey Vanguard's Hauberk, Pasture Pendant, Pasture Loop, Belltower Loop, Bell-Ringer's Greatmace, Belltower Longbow | - | piercing_force 2, jagged_barbs 2, guerilla_tactics 1, rhythmic_draw 1 |
| Ranger | 15 | Narg's Dread Jerkin, Eye's Talisman, Digger's Loop, Grik's Ring, Grik's Staff, Digger's Longbow | - | piercing_force 2, jagged_barbs 2, guerilla_tactics 2, rhythmic_draw 1 |
| Ranger | 16 | Narg's Dread Jerkin, Eye's Talisman, Digger's Loop, Grik's Ring, Grik's Staff, Digger's Longbow | - | piercing_force 2, jagged_barbs 2, guerilla_tactics 2, rhythmic_draw 2 |
| Ranger | 17 | Narg's Dread Jerkin, Eye's Talisman, Digger's Loop, Grik's Ring, Grik's Staff, Digger's Longbow | - | piercing_force 3, jagged_barbs 3, guerilla_tactics 2, rhythmic_draw 2 |
| Ranger | 18 | Narg's Dread Jerkin, Eye's Talisman, Digger's Loop, Grik's Ring, Grik's Staff, Digger's Longbow | - | piercing_force 3, jagged_barbs 3, guerilla_tactics 2, rhythmic_draw 2, steadfast_aim 1 |
| Ranger | 19 | Pointed Jerkin of Azora, Void Talisman, Morganth's Loop, Shadowhide Ring, Shadowhide Staff, Morganth's Longbow | - | piercing_force 3, jagged_barbs 3, guerilla_tactics 3, rhythmic_draw 3, steadfast_aim 1 |
| Ranger | 20 | Pointed Jerkin of Azora, Void Talisman, Morganth's Loop, Shadowhide Ring, Shadowhide Staff, Morganth's Longbow | - | piercing_force 3, jagged_barbs 3, guerilla_tactics 3, rhythmic_draw 3, steadfast_aim 1, twin_arrows 1 |
| Ranger | 21 | Pointed Jerkin of Azora, Void Talisman, Morganth's Loop, Shadowhide Ring, Shadowhide Staff, Morganth's Longbow | - | piercing_force 4, jagged_barbs 4, guerilla_tactics 3, rhythmic_draw 3, steadfast_aim 1, twin_arrows 1 |
| Ranger | 22 | Pointed Jerkin of Azora, Void Talisman, Morganth's Loop, Shadowhide Ring, Shadowhide Staff, Morganth's Longbow | - | piercing_force 4, jagged_barbs 4, guerilla_tactics 3, rhythmic_draw 3, steadfast_aim 2, twin_arrows 1 |
| Ranger | 23 | Pointed Jerkin of Azora, Sermon Pendant, Sermon Loop, Morganth's Loop, Shadowhide Staff, Morganth's Longbow | - | piercing_force 4, jagged_barbs 4, guerilla_tactics 3, rhythmic_draw 3, steadfast_aim 2, twin_arrows 1, shattered_defenses 1, point_blank 1 |
| Ranger | 24 | Pointed Jerkin of Azora, Sermon Pendant, Sermon Loop, Morganth's Loop, Shadowhide Staff, Morganth's Longbow | - | piercing_force 4, jagged_barbs 4, guerilla_tactics 4, rhythmic_draw 3, steadfast_aim 2, twin_arrows 1, shattered_defenses 1, point_blank 1 |
| Ranger | 25 | Pointed Jerkin of Azora, Sermon Pendant, Sermon Loop, Morganth's Loop, Shadowhide Staff, Morganth's Longbow | - | piercing_force 4, jagged_barbs 4, guerilla_tactics 4, rhythmic_draw 4, steadfast_aim 2, twin_arrows 2, shattered_defenses 1, point_blank 1 |
| Ranger | 26 | Pointed Jerkin of Azora, Sermon Pendant, Sermon Loop, Morganth's Loop, Shadowhide Staff, Morganth's Longbow | - | piercing_force 4, jagged_barbs 4, guerilla_tactics 4, rhythmic_draw 4, steadfast_aim 2, twin_arrows 2, shattered_defenses 1, point_blank 1, adrenaline_rush 1 |
| Ranger | 27 | Hogger's Dread Jerkin, Gnoll Talisman, Riverpaw Loop, Hogger's Ring, Hogger's Staff, Riverpaw Longbow | - | piercing_force 5, jagged_barbs 4, guerilla_tactics 4, rhythmic_draw 4, steadfast_aim 2, twin_arrows 2, shattered_defenses 1, point_blank 1, adrenaline_rush 1 |
| Ranger | 28 | Hogger's Dread Jerkin, Gnoll Talisman, Riverpaw Loop, Hogger's Ring, Hogger's Staff, Riverpaw Longbow | - | piercing_force 5, jagged_barbs 5, guerilla_tactics 4, rhythmic_draw 4, steadfast_aim 2, twin_arrows 2, shattered_defenses 2, point_blank 1, adrenaline_rush 1 |
| Ranger | 29 | Hogger's Dread Jerkin, Gnoll Talisman, Riverpaw Loop, Hogger's Ring, Hogger's Staff, Riverpaw Longbow | - | piercing_force 5, jagged_barbs 5, guerilla_tactics 4, rhythmic_draw 4, steadfast_aim 2, twin_arrows 2, shattered_defenses 2, point_blank 2, adrenaline_rush 1 |
| Ranger | 30 | Hogger's Dread Jerkin, Gnoll Talisman, Riverpaw Loop, Hogger's Ring, Hogger's Staff, Riverpaw Longbow | - | piercing_force 5, jagged_barbs 5, guerilla_tactics 5, rhythmic_draw 5, steadfast_aim 2, twin_arrows 2, shattered_defenses 2, point_blank 2, adrenaline_rush 1 |
| Initiate | 10 | Abbot's Vestments, Psalmist's Choker, Psalter Circle, Scribe's Seal, Staff of the Black Psalm, Ashworth's Censer Wand | - | none |
| Initiate | 11 | Abbot's Vestments, Pasture Charm, Pasture Band, Psalter Circle, Staff of the Black Psalm, Ashworth's Censer Wand | - | ley_line_tap 1 |
| Initiate | 12 | Abbot's Vestments, Pasture Charm, Pasture Band, Psalter Circle, Staff of the Black Psalm, Ashworth's Censer Wand | - | ley_line_tap 1, volatile_embers 1, thermal_feedback 1 |
| Initiate | 13 | Abbot's Vestments, Pasture Charm, Pasture Band, Psalter Circle, Staff of the Black Psalm, Ashworth's Censer Wand | - | ley_line_tap 2, volatile_embers 1, thermal_feedback 1 |
| Initiate | 14 | Abbot's Vestments, Pasture Charm, Pasture Band, Psalter Circle, Staff of the Black Psalm, Ashworth's Censer Wand | - | ley_line_tap 2, volatile_embers 2, thermal_feedback 2 |
| Initiate | 15 | Narg's Dread Robe, Digger's Pendant, Eye's Circle, Digger's Seal, Eye's Cudgel, Digger's Lantern, Ashworth's Censer Wand | - | ley_line_tap 3, volatile_embers 2, thermal_feedback 2 |
| Initiate | 16 | Narg's Dread Robe, Digger's Pendant, Eye's Circle, Digger's Seal, Eye's Cudgel, Digger's Lantern, Ashworth's Censer Wand | - | ley_line_tap 3, volatile_embers 3, thermal_feedback 2 |
| Initiate | 17 | Narg's Dread Robe, Digger's Pendant, Eye's Circle, Digger's Seal, Eye's Cudgel, Digger's Lantern, Ashworth's Censer Wand | - | ley_line_tap 3, volatile_embers 3, thermal_feedback 2, ley_line_attunement 1, frigid_conservation 1 |
| Initiate | 18 | Narg's Dread Robe, Digger's Pendant, Eye's Circle, Digger's Seal, Eye's Cudgel, Digger's Lantern, Ashworth's Censer Wand | - | ley_line_tap 3, volatile_embers 3, thermal_feedback 3, ley_line_attunement 1, frigid_conservation 1 |
| Initiate | 19 | Pointed Robe of Azora, Morganth's Pendant, Void Circle, Morganth's Seal, Void Cudgel, Morganth's Lantern, Ashworth's Censer Wand | - | ley_line_tap 4, volatile_embers 4, thermal_feedback 3, ley_line_attunement 1, frigid_conservation 1 |
| Initiate | 20 | Pointed Robe of Azora, Morganth's Pendant, Void Circle, Morganth's Seal, Void Cudgel, Morganth's Lantern, Ashworth's Censer Wand | - | ley_line_tap 4, volatile_embers 4, thermal_feedback 3, ley_line_attunement 2, frigid_conservation 1 |
| Initiate | 21 | Pointed Robe of Azora, Morganth's Pendant, Void Circle, Morganth's Seal, Void Cudgel, Morganth's Lantern, Ashworth's Censer Wand | - | ley_line_tap 4, volatile_embers 4, thermal_feedback 3, ley_line_attunement 2, frigid_conservation 2, catalytic_resonance 1 |
| Initiate | 22 | Pointed Robe of Azora, Morganth's Pendant, Void Circle, Morganth's Seal, Void Cudgel, Morganth's Lantern, Ashworth's Censer Wand | - | ley_line_tap 4, volatile_embers 4, thermal_feedback 3, ley_line_attunement 2, frigid_conservation 2, catalytic_resonance 1, arcane_momentum 1 |
| Initiate | 23 | Pointed Robe of Azora, Sermon Charm, Sermon Band, Void Circle, Void Cudgel, Morganth's Lantern, Ashworth's Censer Wand | - | ley_line_tap 5, volatile_embers 4, thermal_feedback 4, ley_line_attunement 2, frigid_conservation 2, catalytic_resonance 1, arcane_momentum 1 |
| Initiate | 24 | Pointed Robe of Azora, Sermon Charm, Sermon Band, Void Circle, Void Cudgel, Morganth's Lantern, Ashworth's Censer Wand | - | ley_line_tap 5, volatile_embers 5, thermal_feedback 4, ley_line_attunement 2, frigid_conservation 2, catalytic_resonance 1, arcane_momentum 1 |
| Initiate | 25 | Pointed Robe of Azora, Sermon Charm, Sermon Band, Void Circle, Void Cudgel, Morganth's Lantern, Ashworth's Censer Wand | - | ley_line_tap 5, volatile_embers 5, thermal_feedback 4, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 2, arcane_momentum 1 |
| Initiate | 26 | Pointed Robe of Azora, Sermon Charm, Sermon Band, Void Circle, Void Cudgel, Morganth's Lantern, Ashworth's Censer Wand | - | ley_line_tap 5, volatile_embers 5, thermal_feedback 4, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 2, arcane_momentum 2 |
| Initiate | 27 | Hogger's Dread Robe, Riverpaw Pendant, Gnoll Circle, Riverpaw Seal, Hogger's Staff, Ashworth's Censer Wand | - | ley_line_tap 5, volatile_embers 5, thermal_feedback 5, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 2, arcane_momentum 2 |
| Initiate | 28 | Hogger's Dread Robe, Riverpaw Pendant, Gnoll Circle, Riverpaw Seal, Hogger's Staff, Ashworth's Censer Wand | - | ley_line_tap 5, volatile_embers 5, thermal_feedback 5, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 3, arcane_momentum 3 |
| Initiate | 29 | Hogger's Dread Robe, Riverpaw Pendant, Gnoll Circle, Riverpaw Seal, Hogger's Staff, Ashworth's Censer Wand | - | ley_line_tap 5, volatile_embers 5, thermal_feedback 5, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 4, arcane_momentum 3 |
| Initiate | 30 | Hogger's Dread Robe, Riverpaw Pendant, Gnoll Circle, Riverpaw Seal, Hogger's Staff, Ashworth's Censer Wand | - | ley_line_tap 5, volatile_embers 5, thermal_feedback 5, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 5, arcane_momentum 3 |
| Vanguard | 10 | Abbey Vanguard's Breastplate, Capstone Amulet of Might, Seal of the Vanguard, Brother's Iron Band, Disciple's Sacrificial Blade, Shield of the Nave | - | none |
| Vanguard | 11 | Abbey Vanguard's Breastplate, Pasture Torc, Pasture Signet, Seal of the Vanguard, Disciple's Sacrificial Blade, Shield of the Nave | - | divine_strength 1 |
| Vanguard | 12 | Abbey Vanguard's Breastplate, Pasture Torc, Pasture Signet, Seal of the Vanguard, Disciple's Sacrificial Blade, Shield of the Nave | - | divine_strength 2, wrathful_iron 1 |
| Vanguard | 13 | Abbey Vanguard's Breastplate, Pasture Torc, Pasture Signet, Seal of the Vanguard, Disciple's Sacrificial Blade, Shield of the Nave | - | divine_strength 2, wrathful_iron 2 |
| Vanguard | 14 | Abbey Vanguard's Breastplate, Pasture Torc, Pasture Signet, Seal of the Vanguard, Disciple's Sacrificial Blade, Shield of the Nave | - | divine_strength 3, wrathful_iron 3 |
| Vanguard | 15 | Narg's Dread Plate, Grik's Torc, Grik's Band, Eye's Signet, Digger's Blade, Grik's Bulwark | - | divine_strength 3, wrathful_iron 3, hallowed_ground 1 |
| Vanguard | 16 | Narg's Dread Plate, Grik's Torc, Grik's Band, Eye's Signet, Digger's Blade, Grik's Bulwark | - | divine_strength 4, wrathful_iron 3, hallowed_ground 1 |
| Vanguard | 17 | Narg's Dread Plate, Grik's Torc, Grik's Band, Eye's Signet, Digger's Blade, Grik's Bulwark | - | divine_strength 4, wrathful_iron 4, hallowed_ground 2 |
| Vanguard | 18 | Narg's Dread Plate, Grik's Torc, Grik's Band, Eye's Signet, Digger's Blade, Grik's Bulwark | - | divine_strength 5, wrathful_iron 4, hallowed_ground 2 |
| Vanguard | 19 | Pointed Plate of Azora, Shadowhide Torc, Shadowhide Band, Void Signet, Morganth's Blade, Shadowhide Bulwark | - | divine_strength 5, wrathful_iron 5, hallowed_ground 2, righteous_fury 1 |
| Vanguard | 20 | Pointed Plate of Azora, Shadowhide Torc, Shadowhide Band, Void Signet, Morganth's Blade, Shadowhide Bulwark | - | divine_strength 5, wrathful_iron 5, hallowed_ground 2, righteous_fury 1, zealous_restoration 1 |
| Vanguard | 21 | Pointed Plate of Azora, Shadowhide Torc, Shadowhide Band, Void Signet, Morganth's Blade, Shadowhide Bulwark | - | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 1, zealous_restoration 1, hammer_of_vanguard 1 |
| Vanguard | 22 | Pointed Plate of Azora, Shadowhide Torc, Shadowhide Band, Void Signet, Morganth's Blade, Shadowhide Bulwark | - | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 2, zealous_restoration 1, hammer_of_vanguard 1 |
| Vanguard | 23 | Pointed Plate of Azora, Sermon Torc, Sermon Signet, Shadowhide Band, Morganth's Blade, Shadowhide Bulwark | - | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 3, zealous_restoration 2, hammer_of_vanguard 1 |
| Vanguard | 24 | Pointed Plate of Azora, Sermon Torc, Sermon Signet, Shadowhide Band, Morganth's Blade, Shadowhide Bulwark | - | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 3, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 25 | Pointed Plate of Azora, Sermon Torc, Sermon Signet, Shadowhide Band, Morganth's Blade, Shadowhide Bulwark | - | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 26 | Pointed Plate of Azora, Sermon Torc, Sermon Signet, Shadowhide Band, Morganth's Blade, Shadowhide Bulwark | - | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 27 | Hogger's Dread Plate, Hogger's Torc, Hogger's Band, Gnoll Signet, Riverpaw Blade, Hogger's Bulwark | - | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 28 | Hogger's Dread Plate, Hogger's Torc, Hogger's Band, Gnoll Signet, Riverpaw Blade, Hogger's Bulwark | - | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 29 | Hogger's Dread Plate, Hogger's Torc, Hogger's Band, Gnoll Signet, Riverpaw Blade, Hogger's Bulwark | - | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 30 | Hogger's Dread Plate, Hogger's Torc, Hogger's Band, Gnoll Signet, Riverpaw Blade, Hogger's Bulwark | - | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Acolyte | 10 | Abbot's Vestments, Capstone Amulet of the Mind, Psalter Circle, Scribe's Seal, Staff of the Black Psalm, Ashworth's Censer Wand | - | none |
| Acolyte | 11 | Abbot's Vestments, Pasture Charm, Pasture Band, Psalter Circle, Staff of the Black Psalm, Ashworth's Censer Wand | - | virulence 1 |
| Acolyte | 12 | Abbot's Vestments, Pasture Charm, Pasture Band, Psalter Circle, Staff of the Black Psalm, Ashworth's Censer Wand | - | virulence 1, vitality 1, masochism 1 |
| Acolyte | 13 | Abbot's Vestments, Pasture Charm, Pasture Band, Psalter Circle, Staff of the Black Psalm, Ashworth's Censer Wand | - | virulence 2, vitality 1, masochism 1 |
| Acolyte | 14 | Abbot's Vestments, Pasture Charm, Pasture Band, Psalter Circle, Staff of the Black Psalm, Ashworth's Censer Wand | - | virulence 2, vitality 2, masochism 2 |
| Acolyte | 15 | Narg's Dread Robe, Digger's Pendant, Digger's Seal, Eye's Circle, Eye's Cudgel, Digger's Lantern, Ashworth's Censer Wand | - | virulence 3, vitality 2, masochism 2 |
| Acolyte | 16 | Narg's Dread Robe, Digger's Pendant, Digger's Seal, Eye's Circle, Eye's Cudgel, Digger's Lantern, Ashworth's Censer Wand | - | virulence 3, vitality 3, masochism 2 |
| Acolyte | 17 | Narg's Dread Robe, Digger's Pendant, Digger's Seal, Eye's Circle, Eye's Cudgel, Digger's Lantern, Ashworth's Censer Wand | - | virulence 3, vitality 3, masochism 3, crimson_surge 1 |
| Acolyte | 18 | Narg's Dread Robe, Digger's Pendant, Digger's Seal, Eye's Circle, Eye's Cudgel, Digger's Lantern, Ashworth's Censer Wand | - | virulence 4, vitality 3, masochism 3, crimson_surge 1 |
| Acolyte | 19 | Pointed Robe of Azora, Morganth's Pendant, Void Circle, Morganth's Seal, Void Cudgel, Morganth's Lantern, Ashworth's Censer Wand | - | virulence 4, vitality 4, masochism 4, crimson_surge 1 |
| Acolyte | 20 | Pointed Robe of Azora, Morganth's Pendant, Void Circle, Morganth's Seal, Void Cudgel, Morganth's Lantern, Ashworth's Censer Wand | - | virulence 4, vitality 4, masochism 4, crimson_surge 2 |
| Acolyte | 21 | Pointed Robe of Azora, Morganth's Pendant, Void Circle, Morganth's Seal, Void Cudgel, Morganth's Lantern, Ashworth's Censer Wand | - | virulence 5, vitality 4, masochism 4, crimson_surge 2, will_to_survive 1 |
| Acolyte | 22 | Pointed Robe of Azora, Morganth's Pendant, Void Circle, Morganth's Seal, Void Cudgel, Morganth's Lantern, Ashworth's Censer Wand | - | virulence 5, vitality 4, masochism 4, crimson_surge 2, will_to_survive 1, morbid_fascination 1 |
| Acolyte | 23 | Pointed Robe of Azora, Sermon Charm, Sermon Band, Void Circle, Void Cudgel, Morganth's Lantern, Ashworth's Censer Wand | - | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 1, morbid_fascination 1 |
| Acolyte | 24 | Pointed Robe of Azora, Sermon Charm, Sermon Band, Void Circle, Void Cudgel, Morganth's Lantern, Ashworth's Censer Wand | - | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 2, morbid_fascination 1 |
| Acolyte | 25 | Pointed Robe of Azora, Sermon Charm, Sermon Band, Void Circle, Void Cudgel, Morganth's Lantern, Ashworth's Censer Wand | - | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 2 |
| Acolyte | 26 | Pointed Robe of Azora, Sermon Charm, Sermon Band, Void Circle, Void Cudgel, Morganth's Lantern, Ashworth's Censer Wand | - | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 3 |
| Acolyte | 27 | Hogger's Dread Robe, Riverpaw Pendant, Riverpaw Seal, Gnoll Circle, Hogger's Staff, Ashworth's Censer Wand | - | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 3 |
| Acolyte | 28 | Hogger's Dread Robe, Riverpaw Pendant, Riverpaw Seal, Gnoll Circle, Hogger's Staff, Ashworth's Censer Wand | - | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 3 |
| Acolyte | 29 | Hogger's Dread Robe, Riverpaw Pendant, Riverpaw Seal, Gnoll Circle, Hogger's Staff, Ashworth's Censer Wand | - | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 3 |
| Acolyte | 30 | Hogger's Dread Robe, Riverpaw Pendant, Riverpaw Seal, Gnoll Circle, Hogger's Staff, Ashworth's Censer Wand | - | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 3 |
| Basic Warrior | 7 | Novice's Chain Vest, Pendant of Brawn, Band of Brawn, Band of Endurance, Trogg Club, Militia Buckler | - | none |
