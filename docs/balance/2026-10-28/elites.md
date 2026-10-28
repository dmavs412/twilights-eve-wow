# Balance check 3, report 2: one elite in greens, levels 10 to 30

Issue #486. Rules: `docs/balance/rules.md`. Model: `tools/twilights-eve/balance/` from #484, unchanged.

## Command

Data from commit `14e0f410`, 300 fights a cell. The run wrote `run.py`'s own report to `target/agent/` and the
tables below are rebuilt from it in the order the ticket asks. Every cell's seed comes from the cell itself, so
the same command at the same commit prints the same numbers. The run took about a minute.

```
~/.local/bin/python3.15 tools/twilights-eve/balance/run.py --scenario elites --levels 10-30 --fights 300 --jobs 4 --out target/agent/elites-raw.md
```

Win is the share of fights won, rounded to a whole percent, so 0% can hide one or two wins. Health left is the
mean share of maximum health at the end of the won fights and kill time the median length of a won fight, both
`-` when none was won.

## E-dmg: Cutthroat, Ranger, Initiate in greens against one elite

Each must win between 35% and 65%.

| Level | Cutthroat win | Cutthroat health left | Cutthroat kill s | Cutthroat holds | Ranger win | Ranger health left | Ranger kill s | Ranger holds | Initiate win | Initiate health left | Initiate kill s | Initiate holds |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 10 | 100% | 61% | 26.1 | no | 5% | 20% | 37.6 | no | 5% | 7% | 28.3 | no |
| 11 | 87% | 42% | 38.1 | no | 0% | - | - | no | 0% | - | - | no |
| 12 | 63% | 31% | 40.1 | yes | 0% | - | - | no | 0% | - | - | no |
| 13 | 43% | 24% | 42.1 | yes | 0% | - | - | no | 0% | - | - | no |
| 14 | 38% | 27% | 41.4 | yes | 0% | - | - | no | 0% | - | - | no |
| 15 | 47% | 25% | 42.1 | yes | 0% | - | - | no | 0% | - | - | no |
| 16 | 29% | 24% | 42.1 | no | 0% | - | - | no | 0% | - | - | no |
| 17 | 14% | 16% | 43.7 | no | 0% | - | - | no | 0% | - | - | no |
| 18 | 10% | 21% | 42.1 | no | 0% | - | - | no | 0% | - | - | no |
| 19 | 50% | 29% | 40.1 | yes | 0% | - | - | no | 0% | - | - | no |
| 20 | 29% | 22% | 40.8 | no | 0% | - | - | no | 0% | - | - | no |
| 21 | 60% | 37% | 36.1 | yes | 0% | - | - | no | 0% | - | - | no |
| 22 | 46% | 35% | 36.1 | yes | 0% | - | - | no | 0% | - | - | no |
| 23 | 90% | 51% | 35.9 | no | 3% | 9% | 35.8 | no | 0% | - | - | no |
| 24 | 77% | 46% | 37.2 | no | 1% | 9% | 36.7 | no | 0% | - | - | no |
| 25 | 85% | 52% | 34.1 | no | 2% | 19% | 34.2 | no | 0% | - | - | no |
| 26 | 79% | 49% | 34.9 | no | 1% | 14% | 36.5 | no | 0% | - | - | no |
| 27 | 91% | 57% | 34.1 | no | 3% | 11% | 34.4 | no | 0% | - | - | no |
| 28 | 88% | 54% | 33.6 | no | 5% | 13% | 33.4 | no | 0% | - | - | no |
| 29 | 89% | 54% | 33.2 | no | 3% | 13% | 34.9 | no | 0% | - | - | no |
| 30 | 87% | 51% | 33.9 | no | 2% | 6% | 37.6 | no | 0% | - | - | no |

## E-tank: Vanguard, Acolyte in greens against one elite

Each must win 70% or more, and more than every damage Calling wins at that level. `Above damage Callings` is yes
when its win beats the best of the three damage Callings' wins in the table above.

| Level | Vanguard win | Vanguard health left | Vanguard kill s | Vanguard above damage Callings | Vanguard holds | Acolyte win | Acolyte health left | Acolyte kill s | Acolyte above damage Callings | Acolyte holds |
|---|---|---|---|---|---|---|---|---|---|---|
| 10 | 85% | 22% | 36.9 | no | no | 96% | 45% | 27.3 | no | no |
| 11 | 0% | - | - | no | no | 0% | 69% | 38.8 | no | no |
| 12 | 0% | - | - | no | no | 0% | - | - | no | no |
| 13 | 0% | - | - | no | no | 0% | - | - | no | no |
| 14 | 0% | - | - | no | no | 0% | - | - | no | no |
| 15 | 0% | - | - | no | no | 0% | - | - | no | no |
| 16 | 0% | - | - | no | no | 0% | - | - | no | no |
| 17 | 0% | - | - | no | no | 0% | - | - | no | no |
| 18 | 0% | - | - | no | no | 0% | - | - | no | no |
| 19 | 0% | - | - | no | no | 0% | - | - | no | no |
| 20 | 0% | - | - | no | no | 0% | - | - | no | no |
| 21 | 0% | - | - | no | no | 0% | - | - | no | no |
| 22 | 0% | - | - | no | no | 0% | - | - | no | no |
| 23 | 1% | 34% | 42.5 | no | no | 0% | - | - | no | no |
| 24 | 0% | - | - | no | no | 0% | - | - | no | no |
| 25 | 0% | - | - | no | no | 0% | - | - | no | no |
| 26 | 0% | - | - | no | no | 0% | - | - | no | no |
| 27 | 1% | 5% | 44.8 | no | no | 0% | - | - | no | no |
| 28 | 0% | - | - | no | no | 0% | - | - | no | no |
| 29 | 0% | - | - | no | no | 0% | - | - | no | no |
| 30 | 0% | - | - | no | no | 0% | - | - | no | no |

## The elite at each level

`model.mob_stats(level, rank=1)`: a warrior-class creature with `[difficulty.rank]` elite 2.0 health and 1.5
damage, the Zone Factor of the on-level area (Northshire 1.0/1.0 at 10, Elwynn 1.5/1.3 from 11), template
multipliers 1.0, and the vanilla row's armour at its level (rank does not touch armour). It swings every 2.0 s and
a swing rolls evenly between the two damage numbers. The normal's health and mean swing are there to compare.

| Level | Health | Damage per swing | Mean swing | Armour | Normal health | Normal mean swing |
|---|---|---|---|---|---|---|
| 10 | 1267 | 59.0 to 78.2 | 68.6 | 492 | 634 | 45.7 |
| 11 | 2091 | 84.3 to 111.8 | 98.1 | 518 | 1045 | 65.4 |
| 12 | 2281 | 92.0 to 122.0 | 107.0 | 552 | 1140 | 71.3 |
| 13 | 2471 | 99.7 to 132.1 | 115.9 | 585 | 1236 | 77.3 |
| 14 | 2661 | 107.3 to 142.3 | 124.8 | 619 | 1331 | 83.2 |
| 15 | 2851 | 115.0 to 152.4 | 133.7 | 653 | 1426 | 89.1 |
| 16 | 3041 | 122.7 to 162.6 | 142.6 | 688 | 1521 | 95.1 |
| 17 | 3231 | 130.3 to 172.8 | 151.6 | 722 | 1616 | 101.0 |
| 18 | 3421 | 138.0 to 182.9 | 160.5 | 755 | 1711 | 107.0 |
| 19 | 3612 | 145.7 to 193.1 | 169.4 | 789 | 1806 | 112.9 |
| 20 | 3802 | 153.3 to 203.3 | 178.3 | 822 | 1901 | 118.9 |
| 21 | 3992 | 161.0 to 213.4 | 187.2 | 858 | 1996 | 124.8 |
| 22 | 4182 | 168.7 to 223.6 | 196.1 | 892 | 2091 | 130.8 |
| 23 | 4372 | 176.3 to 233.7 | 205.0 | 925 | 2186 | 136.7 |
| 24 | 4562 | 184.0 to 243.9 | 214.0 | 959 | 2281 | 142.6 |
| 25 | 4752 | 191.7 to 254.1 | 222.9 | 992 | 2376 | 148.6 |
| 26 | 4942 | 199.3 to 264.2 | 231.8 | 1026 | 2471 | 154.5 |
| 27 | 5132 | 207.0 to 274.4 | 240.7 | 1061 | 2566 | 160.5 |
| 28 | 5322 | 214.7 to 284.6 | 249.6 | 1093 | 2661 | 166.4 |
| 29 | 5512 | 222.3 to 294.7 | 258.5 | 1127 | 2756 | 172.4 |
| 30 | 5702 | 230.0 to 304.9 | 267.4 | 1160 | 2851 | 178.3 |

## Failures

98 of 105 (rule, Calling, level) cells fail: E-dmg 56 of 63, E-tank 42 of 42.

- E-dmg, Cutthroat, level 10: win 100%, 35 points over 65%.
- E-dmg, Ranger, level 10: win 5%, 30 points under 35%.
- E-dmg, Initiate, level 10: win 5%, 30 points under 35%.
- E-dmg, Cutthroat, level 11: win 87%, 22 points over 65%.
- E-dmg, Ranger, level 11: win 0%, 35 points under 35%.
- E-dmg, Initiate, level 11: win 0%, 35 points under 35%.
- E-dmg, Ranger, level 12: win 0%, 35 points under 35%.
- E-dmg, Initiate, level 12: win 0%, 35 points under 35%.
- E-dmg, Ranger, level 13: win 0%, 35 points under 35%.
- E-dmg, Initiate, level 13: win 0%, 35 points under 35%.
- E-dmg, Ranger, level 14: win 0%, 35 points under 35%.
- E-dmg, Initiate, level 14: win 0%, 35 points under 35%.
- E-dmg, Ranger, level 15: win 0%, 35 points under 35%.
- E-dmg, Initiate, level 15: win 0%, 35 points under 35%.
- E-dmg, Cutthroat, level 16: win 29%, 6 points under 35%.
- E-dmg, Ranger, level 16: win 0%, 35 points under 35%.
- E-dmg, Initiate, level 16: win 0%, 35 points under 35%.
- E-dmg, Cutthroat, level 17: win 14%, 21 points under 35%.
- E-dmg, Ranger, level 17: win 0%, 35 points under 35%.
- E-dmg, Initiate, level 17: win 0%, 35 points under 35%.
- E-dmg, Cutthroat, level 18: win 10%, 25 points under 35%.
- E-dmg, Ranger, level 18: win 0%, 35 points under 35%.
- E-dmg, Initiate, level 18: win 0%, 35 points under 35%.
- E-dmg, Ranger, level 19: win 0%, 35 points under 35%.
- E-dmg, Initiate, level 19: win 0%, 35 points under 35%.
- E-dmg, Cutthroat, level 20: win 29%, 6 points under 35%.
- E-dmg, Ranger, level 20: win 0%, 35 points under 35%.
- E-dmg, Initiate, level 20: win 0%, 35 points under 35%.
- E-dmg, Ranger, level 21: win 0%, 35 points under 35%.
- E-dmg, Initiate, level 21: win 0%, 35 points under 35%.
- E-dmg, Ranger, level 22: win 0%, 35 points under 35%.
- E-dmg, Initiate, level 22: win 0%, 35 points under 35%.
- E-dmg, Cutthroat, level 23: win 90%, 25 points over 65%.
- E-dmg, Ranger, level 23: win 3%, 32 points under 35%.
- E-dmg, Initiate, level 23: win 0%, 35 points under 35%.
- E-dmg, Cutthroat, level 24: win 77%, 12 points over 65%.
- E-dmg, Ranger, level 24: win 1%, 34 points under 35%.
- E-dmg, Initiate, level 24: win 0%, 35 points under 35%.
- E-dmg, Cutthroat, level 25: win 85%, 20 points over 65%.
- E-dmg, Ranger, level 25: win 2%, 33 points under 35%.
- E-dmg, Initiate, level 25: win 0%, 35 points under 35%.
- E-dmg, Cutthroat, level 26: win 79%, 14 points over 65%.
- E-dmg, Ranger, level 26: win 1%, 34 points under 35%.
- E-dmg, Initiate, level 26: win 0%, 35 points under 35%.
- E-dmg, Cutthroat, level 27: win 91%, 26 points over 65%.
- E-dmg, Ranger, level 27: win 3%, 32 points under 35%.
- E-dmg, Initiate, level 27: win 0%, 35 points under 35%.
- E-dmg, Cutthroat, level 28: win 88%, 23 points over 65%.
- E-dmg, Ranger, level 28: win 5%, 30 points under 35%.
- E-dmg, Initiate, level 28: win 0%, 35 points under 35%.
- E-dmg, Cutthroat, level 29: win 89%, 24 points over 65%.
- E-dmg, Ranger, level 29: win 3%, 32 points under 35%.
- E-dmg, Initiate, level 29: win 0%, 35 points under 35%.
- E-dmg, Cutthroat, level 30: win 87%, 22 points over 65%.
- E-dmg, Ranger, level 30: win 2%, 33 points under 35%.
- E-dmg, Initiate, level 30: win 0%, 35 points under 35%.
- E-tank, Vanguard, level 10: win 85%, 15 points under the Cutthroat's 100%.
- E-tank, Acolyte, level 10: win 96%, 4 points under the Cutthroat's 100%.
- E-tank, Vanguard, level 11: win 0%, 70 points under 70% and 87 points under the Cutthroat's 87%.
- E-tank, Acolyte, level 11: win 0%, 70 points under 70% and 87 points under the Cutthroat's 87%.
- E-tank, Vanguard, level 12: win 0%, 70 points under 70% and 63 points under the Cutthroat's 63%.
- E-tank, Acolyte, level 12: win 0%, 70 points under 70% and 63 points under the Cutthroat's 63%.
- E-tank, Vanguard, level 13: win 0%, 70 points under 70% and 43 points under the Cutthroat's 43%.
- E-tank, Acolyte, level 13: win 0%, 70 points under 70% and 43 points under the Cutthroat's 43%.
- E-tank, Vanguard, level 14: win 0%, 70 points under 70% and 38 points under the Cutthroat's 38%.
- E-tank, Acolyte, level 14: win 0%, 70 points under 70% and 38 points under the Cutthroat's 38%.
- E-tank, Vanguard, level 15: win 0%, 70 points under 70% and 47 points under the Cutthroat's 47%.
- E-tank, Acolyte, level 15: win 0%, 70 points under 70% and 47 points under the Cutthroat's 47%.
- E-tank, Vanguard, level 16: win 0%, 70 points under 70% and 29 points under the Cutthroat's 29%.
- E-tank, Acolyte, level 16: win 0%, 70 points under 70% and 29 points under the Cutthroat's 29%.
- E-tank, Vanguard, level 17: win 0%, 70 points under 70% and 14 points under the Cutthroat's 14%.
- E-tank, Acolyte, level 17: win 0%, 70 points under 70% and 14 points under the Cutthroat's 14%.
- E-tank, Vanguard, level 18: win 0%, 70 points under 70% and 10 points under the Cutthroat's 10%.
- E-tank, Acolyte, level 18: win 0%, 70 points under 70% and 10 points under the Cutthroat's 10%.
- E-tank, Vanguard, level 19: win 0%, 70 points under 70% and 50 points under the Cutthroat's 50%.
- E-tank, Acolyte, level 19: win 0%, 70 points under 70% and 50 points under the Cutthroat's 50%.
- E-tank, Vanguard, level 20: win 0%, 70 points under 70% and 29 points under the Cutthroat's 29%.
- E-tank, Acolyte, level 20: win 0%, 70 points under 70% and 29 points under the Cutthroat's 29%.
- E-tank, Vanguard, level 21: win 0%, 70 points under 70% and 60 points under the Cutthroat's 60%.
- E-tank, Acolyte, level 21: win 0%, 70 points under 70% and 60 points under the Cutthroat's 60%.
- E-tank, Vanguard, level 22: win 0%, 70 points under 70% and 46 points under the Cutthroat's 46%.
- E-tank, Acolyte, level 22: win 0%, 70 points under 70% and 46 points under the Cutthroat's 46%.
- E-tank, Vanguard, level 23: win 1%, 69 points under 70% and 89 points under the Cutthroat's 90%.
- E-tank, Acolyte, level 23: win 0%, 70 points under 70% and 90 points under the Cutthroat's 90%.
- E-tank, Vanguard, level 24: win 0%, 70 points under 70% and 77 points under the Cutthroat's 77%.
- E-tank, Acolyte, level 24: win 0%, 70 points under 70% and 77 points under the Cutthroat's 77%.
- E-tank, Vanguard, level 25: win 0%, 70 points under 70% and 85 points under the Cutthroat's 85%.
- E-tank, Acolyte, level 25: win 0%, 70 points under 70% and 85 points under the Cutthroat's 85%.
- E-tank, Vanguard, level 26: win 0%, 70 points under 70% and 79 points under the Cutthroat's 79%.
- E-tank, Acolyte, level 26: win 0%, 70 points under 70% and 79 points under the Cutthroat's 79%.
- E-tank, Vanguard, level 27: win 1%, 69 points under 70% and 90 points under the Cutthroat's 91%.
- E-tank, Acolyte, level 27: win 0%, 70 points under 70% and 91 points under the Cutthroat's 91%.
- E-tank, Vanguard, level 28: win 0%, 70 points under 70% and 88 points under the Cutthroat's 88%.
- E-tank, Acolyte, level 28: win 0%, 70 points under 70% and 88 points under the Cutthroat's 88%.
- E-tank, Vanguard, level 29: win 0%, 70 points under 70% and 89 points under the Cutthroat's 89%.
- E-tank, Acolyte, level 29: win 0%, 70 points under 70% and 89 points under the Cutthroat's 89%.
- E-tank, Vanguard, level 30: win 0%, 70 points under 70% and 87 points under the Cutthroat's 87%.
- E-tank, Acolyte, level 30: win 0%, 70 points under 70% and 87 points under the Cutthroat's 87%.

## Reading

- The elite outlasts every Calling's health without a survival tool. Its swing (68.6 at 10, 98.1 at 11, 267.4 at 30, every 2 s) kills the Initiate and the Acolyte in 6 to 12 s after armour alone, the Cutthroat and the Ranger in 13 to 17 s and the Vanguard in 18 to 30 s. A kill takes 26 to 51 s at their DPS. Each win comes from what the Calling does about that gap.
- The Cutthroat is the only Calling inside the E-dmg band, at 12 to 15, 19, 21 and 22. Evasion (+50% dodge for 15 s, used under 60% health), Rupture's 40% leech and the Garrote opener carry it. It is over the band at 10 (100%, Northshire's 1.0 Zone Factor) and 11, and again at 23 to 30 (77-91%), where the level-23 greens lift its health from 1010 to 1185 and its DPS from 105 to 120, and a kill drops to 33 to 37 s. It is under the band at 16 to 18 and 20 (10-29%), when a kill takes 41 to 44 s and Evasion covers a smaller share of it. At 18 the kit picker also gives it Prospector's Chestpiece, a stock chest that drops its armour from 627 to 365.
- The Ranger wins 0 to 5% at every level. Below 23 it fights in melee with a staff at 30 to 59 DPS (the minimum range, as in #485), so a kill would take 43 s at 10 and 62 to 72 s from 11. Point Blank at 23 doubles its DPS to 108-142, but a kill still takes 34 to 38 s against about 14 s of life, and Field Dressing's 20% over 10 s does not close that.
- The Initiate wins 5% at 10 and 0% from 11. From 11 its 40 to 72 DPS needs 52 to 79 s a kill and it lives 6 to 8 s. Its armour falls to 99-118 at 16 to 18, 22 and 26 to 30 from stock vanilla chests, the kit-picker problem #485 found.
- E-tank holds at no level. At 10 the Vanguard (85%) and the Acolyte (96%) clear 70%, but the Cutthroat wins 100%. At 11 Elwynn's Zone Factor takes the elite from 1267 to 2091 health and from 68.6 to 98.1 a swing, and both drop to 0-1% for good. The Vanguard's damage build has no defensive talent, its 41 to 112 DPS needs about 50 s a kill, and Seal's 4% heals and Shield of the Penitent's block do not cover the 18 to 23 s it lives. The Acolyte has cloth armour (101 to 335) and leech heals that do not keep up.
- Moving `[difficulty.rank]` elite alone cannot fix E-dmg. An in-memory sweep (150 fights a cell, levels 10, 12, 16, 20, 23, 26, 30; not committed) tried six health/damage pairs from 2.0/1.5 down to 0.75/0.6, and none puts all three damage Callings in 35-65% at any level. At 1.0/1.0 the Cutthroat wins 100% everywhere while the Initiate wins 0-17% from 12; at 1.0/0.75 the Initiate reaches 22-87% from 12 to 23, but the Cutthroat and the Ranger are at 58-100%. A rank factor changes the elite the same for every Calling, and the Cutthroat's elite DPS is 1.3 to 2.3 times the Initiate's, so the spread between them at one level stays wider than the 30-point band. One factor also applies at every level, so it cannot lower the Cutthroat at 23 to 30 and raise it at 16 to 20.
- Rank cannot fix E-tank's ordering either. At every pair in the sweep the Cutthroat's win matches or beats both tanks' at every sampled level. At the weak end (0.75/0.6) all three win 100%, a tie that still fails "above". The ordering needs character-side levers: the tanks' survival (armour, block, self-healing, a defensive talent in the build) or the Cutthroat's Evasion.

## Kits and builds

As `run.py` printed them, in greens. Talents are the damage build at every point the level gives.

| Calling | Level | Kit | Talents |
|---|---|---|---|
| Cutthroat | 10 | Trapper's Leather Jerkin, Poacher's Locket, Prowler's Signet, Echo Ridge Loop, Marshal's Broadsword, Padfoot's Shiv (OH) | none |
| Cutthroat | 11 | Stonefield Leather Jerkin, Brackwell Talisman, Brackwell Loop, Prowler's Signet, Brackwell Cutlass, Maclure Dirk (OH) | reflexive_strike 1 |
| Cutthroat | 12 | Stonefield Leather Jerkin, Brackwell Talisman, Brackwell Loop, Prowler's Signet, Brackwell Cutlass, Maclure Dirk (OH) | reflexive_strike 1, blood_frenzy 1, bloodletting 1 |
| Cutthroat | 13 | Stonefield Leather Jerkin, Brackwell Talisman, Brackwell Loop, Fencemender Loop, Brackwell Cutlass, Maclure Dirk (OH) | reflexive_strike 1, blood_frenzy 1, bloodletting 1, executioners_toll 1 |
| Cutthroat | 14 | Stonefield Leather Jerkin, Brackwell Talisman, Courier's Loop, Brackwell Loop, Morgan's Cutlass, Morgan's Dirk (OH) | reflexive_strike 2, blood_frenzy 2, bloodletting 1, executioners_toll 1 |
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
| Ranger | 10 | Trapper's Leather Jerkin, Poacher's Locket, Prowler's Signet, Echo Ridge Loop, Padfoot's Ill-Gotten Staff, Vineyard Poacher's Bow | none |
| Ranger | 11 | Stonefield Leather Jerkin, Brackwell Talisman, Brackwell Loop, Prowler's Signet, Brackwell Quarterstaff, Lion's Pride Shortbow | piercing_force 1 |
| Ranger | 12 | Stonefield Leather Jerkin, Brackwell Talisman, Brackwell Loop, Prowler's Signet, Brackwell Quarterstaff, Lion's Pride Shortbow | piercing_force 1, jagged_barbs 1, guerilla_tactics 1 |
| Ranger | 13 | Stonefield Leather Jerkin, Brackwell Talisman, Brackwell Loop, Fencemender Loop, Brackwell Quarterstaff, Lion's Pride Shortbow | piercing_force 1, jagged_barbs 1, guerilla_tactics 1, rhythmic_draw 1 |
| Ranger | 14 | Stonefield Leather Jerkin, Brackwell Talisman, Courier's Loop, Brackwell Loop, Narg's Crook, Lion's Pride Shortbow | piercing_force 2, jagged_barbs 2, guerilla_tactics 1, rhythmic_draw 1 |
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
| Initiate | 10 | Abbey Scholar's Robe, Scribe's Pendant, Echo Ridge Circle, Scribe's Band, Padfoot's Ill-Gotten Staff, Familiar's Spark | none |
| Initiate | 11 | Brackwell Wool Robe, Stonefield Pendant, Maclure Circle, Lion's Pride Seal, Stonefield Cudgel, Vigil Candle, Familiar's Spark | ley_line_tap 1 |
| Initiate | 12 | Brackwell Wool Robe, Stonefield Pendant, Maclure Circle, Lion's Pride Seal, Stonefield Cudgel, Vigil Candle, Familiar's Spark | ley_line_tap 1, volatile_embers 1, thermal_feedback 1 |
| Initiate | 13 | Brackwell Wool Robe, Stonefield Pendant, Maclure Circle, Fencepost Band, Stonefield Cudgel, Vigil Candle, Familiar's Spark | ley_line_tap 2, volatile_embers 1, thermal_feedback 1 |
| Initiate | 14 | Brackwell Wool Robe, Stonefield Pendant, Courier's Seal, Maclure Circle, Stonefield Cudgel, Vigil Candle, Familiar's Spark | ley_line_tap 2, volatile_embers 2, thermal_feedback 2 |
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
| Vanguard | 10 | Stonefield Plate Vest, Vintner's Locket, Brawler's Signet, Echo Ridge Band, Marshal's Broadsword, Veteran Shield | none |
| Vanguard | 11 | Lion's Pride Plate Cuirass, Lion's Pride Torc, Maclure Band, Lion's Pride Signet, Brackwell Cutlass, Stonefield Kite Shield | divine_strength 1 |
| Vanguard | 12 | Lion's Pride Plate Cuirass, Lion's Pride Torc, Maclure Band, Lion's Pride Signet, Brackwell Cutlass, Stonefield Kite Shield | divine_strength 2, wrathful_iron 1 |
| Vanguard | 13 | Lion's Pride Plate Cuirass, Lion's Pride Torc, Maclure Band, Fencewright Ring, Brackwell Cutlass, Stonefield Kite Shield | divine_strength 2, wrathful_iron 2 |
| Vanguard | 14 | Lion's Pride Plate Cuirass, Lion's Pride Torc, Courier's Signet, Maclure Band, Morgan's Cutlass, Stonefield Kite Shield | divine_strength 3, wrathful_iron 3 |
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
| Acolyte | 10 | Abbey Scholar's Robe, Scribe's Pendant, Scribe's Signet, Echo Ridge Circle, Padfoot's Ill-Gotten Staff, Familiar's Spark | none |
| Acolyte | 11 | Brackwell Wool Robe, Stonefield Pendant, Lion's Pride Seal, Maclure Circle, Stonefield Cudgel, Vigil Candle, Familiar's Spark | virulence 1 |
| Acolyte | 12 | Brackwell Wool Robe, Stonefield Pendant, Lion's Pride Seal, Maclure Circle, Stonefield Cudgel, Vigil Candle, Familiar's Spark | virulence 1, vitality 1, masochism 1 |
| Acolyte | 13 | Brackwell Wool Robe, Stonefield Pendant, Lion's Pride Seal, Maclure Circle, Stonefield Cudgel, Vigil Candle, Familiar's Spark | virulence 2, vitality 1, masochism 1 |
| Acolyte | 14 | Brackwell Wool Robe, Stonefield Pendant, Lion's Pride Seal, Courier's Seal, Stonefield Cudgel, Vigil Candle, Familiar's Spark | virulence 2, vitality 2, masochism 2 |
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
