# Balance check 3, report 1: pulls and DPS ordering, levels 1 to 30

Issue #485. Rules: `docs/balance/rules.md`. Model: `tools/twilights-eve/balance/` from #484, unchanged.

## Commands

Data from commit `03feb0c3` (`git rev-parse --short HEAD`), 300 fights a cell. `run.py` has no `--append`,
so each scenario wrote its own file and the tables below are rebuilt from the three. Every cell's seed comes
from the cell itself, so the same commands at the same commit print the same numbers.

```
~/.local/bin/python3.15 tools/twilights-eve/balance/run.py --scenario T0 --levels 1-9 --fights 300 --jobs 4 --out target/agent/t0.md
~/.local/bin/python3.15 tools/twilights-eve/balance/run.py --scenario pulls --levels 10-30 --fights 300 --jobs 4 --out target/agent/pulls.md
~/.local/bin/python3.15 tools/twilights-eve/balance/run.py --scenario dps --levels 10-30 --fights 300 --jobs 4 --out target/agent/dps.md
```

Each scenario ran in under a minute. Win is the share of fights won; health left is the mean share of
maximum health at the end of the won fights, `-` when none was won.

## T0-1 and T0-2: Basic Warrior, starter outfit and whites

T0-1 needs a 1-mob win of 97% or more; T0-2 needs a 2-mob win of 90% or more.

| Level | Mob health | 1 mob win | 1 mob health left | 2 mobs win | 2 mobs health left | T0-1 holds | T0-2 holds |
|---|---|---|---|---|---|---|---|
| 1 | 63 | 100% | 99% | 100% | 97% | yes | yes |
| 2 | 127 | 100% | 94% | 100% | 81% | yes | yes |
| 3 | 190 | 100% | 86% | 100% | 55% | yes | yes |
| 4 | 253 | 100% | 74% | 94% | 22% | yes | yes |
| 5 | 317 | 100% | 62% | 13% | 7% | yes | no |
| 6 | 380 | 100% | 44% | 0% | - | yes | no |
| 7 | 444 | 93% | 24% | 0% | - | no | no |
| 8 | 507 | 42% | 11% | 0% | - | no | no |
| 9 | 570 | 3% | 8% | 0% | - | no | no |

## P-2 and P-3: each Calling in greens

P-2 needs a 2-mob win of 80% or more with 20% or more health left; P-3 needs a 3-mob win between 15%
and 50%.

### Cutthroat

| Level | Mob health | 2 mobs win | 2 mobs health left | 3 mobs win | P-2 holds | P-3 holds |
|---|---|---|---|---|---|---|
| 10 | 634 | 100% | 52% | 70% | yes | no |
| 11 | 1045 | 90% | 34% | 5% | yes | no |
| 12 | 1140 | 70% | 27% | 0% | no | no |
| 13 | 1236 | 53% | 20% | 0% | no | no |
| 14 | 1331 | 32% | 18% | 0% | no | no |
| 15 | 1426 | 47% | 20% | 0% | no | no |
| 16 | 1521 | 33% | 17% | 0% | no | no |
| 17 | 1616 | 14% | 17% | 0% | no | no |
| 18 | 1711 | 12% | 18% | 0% | no | no |
| 19 | 1806 | 44% | 23% | 0% | no | no |
| 20 | 1901 | 31% | 22% | 0% | no | no |
| 21 | 1996 | 47% | 27% | 0% | no | no |
| 22 | 2091 | 38% | 25% | 0% | no | no |
| 23 | 2186 | 86% | 45% | 6% | yes | no |
| 24 | 2281 | 76% | 43% | 3% | no | no |
| 25 | 2376 | 75% | 43% | 3% | no | no |
| 26 | 2471 | 78% | 39% | 2% | no | no |
| 27 | 2566 | 88% | 46% | 6% | yes | no |
| 28 | 2661 | 93% | 48% | 13% | yes | no |
| 29 | 2756 | 85% | 46% | 8% | yes | no |
| 30 | 2851 | 85% | 46% | 4% | yes | no |

### Ranger

| Level | Mob health | 2 mobs win | 2 mobs health left | 3 mobs win | P-2 holds | P-3 holds |
|---|---|---|---|---|---|---|
| 10 | 634 | 0% | 2% | 0% | no | no |
| 11 | 1045 | 0% | - | 0% | no | no |
| 12 | 1140 | 0% | - | 0% | no | no |
| 13 | 1236 | 0% | - | 0% | no | no |
| 14 | 1331 | 0% | - | 0% | no | no |
| 15 | 1426 | 0% | - | 0% | no | no |
| 16 | 1521 | 0% | - | 0% | no | no |
| 17 | 1616 | 0% | - | 0% | no | no |
| 18 | 1711 | 0% | - | 0% | no | no |
| 19 | 1806 | 0% | - | 0% | no | no |
| 20 | 1901 | 0% | - | 0% | no | no |
| 21 | 1996 | 0% | - | 0% | no | no |
| 22 | 2091 | 0% | - | 0% | no | no |
| 23 | 2186 | 0% | - | 0% | no | no |
| 24 | 2281 | 0% | - | 0% | no | no |
| 25 | 2376 | 0% | 9% | 0% | no | no |
| 26 | 2471 | 0% | - | 0% | no | no |
| 27 | 2566 | 0% | - | 0% | no | no |
| 28 | 2661 | 0% | - | 0% | no | no |
| 29 | 2756 | 0% | - | 0% | no | no |
| 30 | 2851 | 0% | - | 0% | no | no |

### Initiate

| Level | Mob health | 2 mobs win | 2 mobs health left | 3 mobs win | P-2 holds | P-3 holds |
|---|---|---|---|---|---|---|
| 10 | 634 | 0% | - | 0% | no | no |
| 11 | 1045 | 0% | - | 0% | no | no |
| 12 | 1140 | 0% | - | 0% | no | no |
| 13 | 1236 | 0% | - | 0% | no | no |
| 14 | 1331 | 0% | - | 0% | no | no |
| 15 | 1426 | 0% | - | 0% | no | no |
| 16 | 1521 | 0% | - | 0% | no | no |
| 17 | 1616 | 0% | - | 0% | no | no |
| 18 | 1711 | 0% | - | 0% | no | no |
| 19 | 1806 | 0% | - | 0% | no | no |
| 20 | 1901 | 0% | - | 0% | no | no |
| 21 | 1996 | 0% | - | 0% | no | no |
| 22 | 2091 | 0% | - | 0% | no | no |
| 23 | 2186 | 0% | - | 0% | no | no |
| 24 | 2281 | 0% | - | 0% | no | no |
| 25 | 2376 | 0% | - | 0% | no | no |
| 26 | 2471 | 0% | - | 0% | no | no |
| 27 | 2566 | 0% | - | 0% | no | no |
| 28 | 2661 | 0% | - | 0% | no | no |
| 29 | 2756 | 0% | - | 0% | no | no |
| 30 | 2851 | 0% | - | 0% | no | no |

### Vanguard

| Level | Mob health | 2 mobs win | 2 mobs health left | 3 mobs win | P-2 holds | P-3 holds |
|---|---|---|---|---|---|---|
| 10 | 634 | 97% | 32% | 7% | yes | no |
| 11 | 1045 | 1% | 9% | 0% | no | no |
| 12 | 1140 | 0% | 10% | 0% | no | no |
| 13 | 1236 | 0% | - | 0% | no | no |
| 14 | 1331 | 0% | - | 0% | no | no |
| 15 | 1426 | 0% | 12% | 0% | no | no |
| 16 | 1521 | 0% | - | 0% | no | no |
| 17 | 1616 | 0% | - | 0% | no | no |
| 18 | 1711 | 0% | - | 0% | no | no |
| 19 | 1806 | 0% | - | 0% | no | no |
| 20 | 1901 | 0% | - | 0% | no | no |
| 21 | 1996 | 0% | 12% | 0% | no | no |
| 22 | 2091 | 0% | - | 0% | no | no |
| 23 | 2186 | 4% | 5% | 0% | no | no |
| 24 | 2281 | 1% | 13% | 0% | no | no |
| 25 | 2376 | 0% | 5% | 0% | no | no |
| 26 | 2471 | 0% | 7% | 0% | no | no |
| 27 | 2566 | 9% | 13% | 0% | no | no |
| 28 | 2661 | 4% | 15% | 0% | no | no |
| 29 | 2756 | 2% | 13% | 0% | no | no |
| 30 | 2851 | 0% | 8% | 0% | no | no |

### Acolyte

| Level | Mob health | 2 mobs win | 2 mobs health left | 3 mobs win | P-2 holds | P-3 holds |
|---|---|---|---|---|---|---|
| 10 | 634 | 67% | 54% | 0% | no | no |
| 11 | 1045 | 0% | - | 0% | no | no |
| 12 | 1140 | 0% | - | 0% | no | no |
| 13 | 1236 | 0% | - | 0% | no | no |
| 14 | 1331 | 0% | - | 0% | no | no |
| 15 | 1426 | 0% | - | 0% | no | no |
| 16 | 1521 | 0% | - | 0% | no | no |
| 17 | 1616 | 0% | - | 0% | no | no |
| 18 | 1711 | 0% | - | 0% | no | no |
| 19 | 1806 | 0% | - | 0% | no | no |
| 20 | 1901 | 0% | - | 0% | no | no |
| 21 | 1996 | 0% | - | 0% | no | no |
| 22 | 2091 | 0% | - | 0% | no | no |
| 23 | 2186 | 0% | - | 0% | no | no |
| 24 | 2281 | 0% | - | 0% | no | no |
| 25 | 2376 | 0% | - | 0% | no | no |
| 26 | 2471 | 0% | - | 0% | no | no |
| 27 | 2566 | 0% | - | 0% | no | no |
| 28 | 2661 | 0% | - | 0% | no | no |
| 29 | 2756 | 0% | - | 0% | no | no |
| 30 | 2851 | 0% | - | 0% | no | no |

## DPS: single-mob DPS in greens

Holds when the Cutthroat, Ranger and Initiate each deal more DPS than both the Vanguard and the Acolyte.
DPS counts the whole fight, so a Calling that dies mid-fight is measured up to its death.

| Level | Cutthroat | Ranger | Initiate | Vanguard | Acolyte | Holds |
|---|---|---|---|---|---|---|
| 10 | 55.0 | 37.4 | 42.2 | 33.2 | 61.5 | no |
| 11 | 58.5 | 33.8 | 42.4 | 43.1 | 54.0 | no |
| 12 | 58.6 | 33.2 | 43.0 | 44.5 | 56.8 | no |
| 13 | 58.3 | 32.9 | 44.3 | 45.8 | 55.1 | no |
| 14 | 63.2 | 35.3 | 46.0 | 50.4 | 58.8 | no |
| 15 | 66.9 | 37.2 | 47.4 | 54.3 | 57.7 | no |
| 16 | 69.7 | 38.8 | 47.6 | 56.8 | 50.8 | no |
| 17 | 70.0 | 39.9 | 48.9 | 62.1 | 55.7 | no |
| 18 | 77.4 | 40.8 | 50.5 | 62.3 | 53.2 | no |
| 19 | 87.9 | 46.1 | 54.9 | 73.6 | 69.1 | no |
| 20 | 87.7 | 47.1 | 55.4 | 74.5 | 69.8 | no |
| 21 | 99.8 | 51.2 | 57.0 | 87.9 | 67.8 | no |
| 22 | 100.9 | 51.1 | 58.1 | 87.3 | 63.4 | no |
| 23 | 110.8 | 113.0 | 61.6 | 93.6 | 68.7 | no |
| 24 | 114.1 | 115.3 | 63.1 | 94.8 | 72.1 | no |
| 25 | 123.5 | 122.0 | 65.3 | 99.2 | 73.1 | no |
| 26 | 126.4 | 121.2 | 67.5 | 100.4 | 72.9 | no |
| 27 | 135.5 | 136.4 | 70.3 | 110.1 | 79.2 | no |
| 28 | 150.7 | 141.9 | 72.4 | 113.0 | 76.3 | no |
| 29 | 151.1 | 146.2 | 74.6 | 110.0 | 77.8 | no |
| 30 | 156.0 | 145.9 | 76.2 | 110.9 | 78.3 | no |

## Failures

245 (rule, Calling, level) cells miss.

- T0-1, Basic Warrior, level 7: win 93%, needs 97%, short by 4 points.
- T0-1, Basic Warrior, level 8: win 42%, needs 97%, short by 55 points.
- T0-1, Basic Warrior, level 9: win 3%, needs 97%, short by 94 points.
- T0-2, Basic Warrior, level 5: win 13%, needs 90%, short by 77 points.
- T0-2, Basic Warrior, level 6: win 0%, needs 90%, short by 90 points.
- T0-2, Basic Warrior, level 7: win 0%, needs 90%, short by 90 points.
- T0-2, Basic Warrior, level 8: win 0%, needs 90%, short by 90 points.
- T0-2, Basic Warrior, level 9: win 0%, needs 90%, short by 90 points.
- P-2, Cutthroat, level 12: win 70%, short of 80% by 10 points.
- P-2, Cutthroat, level 13: win 53%, short of 80% by 27 points.
- P-2, Cutthroat, level 14: win 32%, short of 80% by 48 points; health left 18%, short of 20% by 2 points.
- P-2, Cutthroat, level 15: win 47%, short of 80% by 33 points.
- P-2, Cutthroat, level 16: win 33%, short of 80% by 47 points; health left 17%, short of 20% by 3 points.
- P-2, Cutthroat, level 17: win 14%, short of 80% by 66 points; health left 17%, short of 20% by 3 points.
- P-2, Cutthroat, level 18: win 12%, short of 80% by 68 points; health left 18%, short of 20% by 2 points.
- P-2, Cutthroat, level 19: win 44%, short of 80% by 36 points.
- P-2, Cutthroat, level 20: win 31%, short of 80% by 49 points.
- P-2, Cutthroat, level 21: win 47%, short of 80% by 33 points.
- P-2, Cutthroat, level 22: win 38%, short of 80% by 42 points.
- P-2, Cutthroat, level 24: win 76%, short of 80% by 4 points.
- P-2, Cutthroat, level 25: win 75%, short of 80% by 5 points.
- P-2, Cutthroat, level 26: win 78%, short of 80% by 2 points.
- P-2, Ranger, level 10: win 0%, short of 80% by 80 points; health left 2%, short of 20% by 18 points.
- P-2, Ranger, level 11: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Ranger, level 12: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Ranger, level 13: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Ranger, level 14: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Ranger, level 15: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Ranger, level 16: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Ranger, level 17: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Ranger, level 18: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Ranger, level 19: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Ranger, level 20: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Ranger, level 21: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Ranger, level 22: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Ranger, level 23: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Ranger, level 24: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Ranger, level 25: win 0%, short of 80% by 80 points; health left 9%, short of 20% by 11 points.
- P-2, Ranger, level 26: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Ranger, level 27: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Ranger, level 28: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Ranger, level 29: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Ranger, level 30: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 10: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 11: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 12: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 13: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 14: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 15: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 16: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 17: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 18: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 19: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 20: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 21: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 22: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 23: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 24: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 25: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 26: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 27: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 28: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 29: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Initiate, level 30: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Vanguard, level 11: win 1%, short of 80% by 79 points; health left 9%, short of 20% by 11 points.
- P-2, Vanguard, level 12: win 0%, short of 80% by 80 points; health left 10%, short of 20% by 10 points.
- P-2, Vanguard, level 13: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Vanguard, level 14: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Vanguard, level 15: win 0%, short of 80% by 80 points; health left 12%, short of 20% by 8 points.
- P-2, Vanguard, level 16: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Vanguard, level 17: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Vanguard, level 18: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Vanguard, level 19: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Vanguard, level 20: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Vanguard, level 21: win 0%, short of 80% by 80 points; health left 12%, short of 20% by 8 points.
- P-2, Vanguard, level 22: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Vanguard, level 23: win 4%, short of 80% by 76 points; health left 5%, short of 20% by 15 points.
- P-2, Vanguard, level 24: win 1%, short of 80% by 79 points; health left 13%, short of 20% by 7 points.
- P-2, Vanguard, level 25: win 0%, short of 80% by 80 points; health left 5%, short of 20% by 15 points.
- P-2, Vanguard, level 26: win 0%, short of 80% by 80 points; health left 7%, short of 20% by 13 points.
- P-2, Vanguard, level 27: win 9%, short of 80% by 71 points; health left 13%, short of 20% by 7 points.
- P-2, Vanguard, level 28: win 4%, short of 80% by 76 points; health left 15%, short of 20% by 5 points.
- P-2, Vanguard, level 29: win 2%, short of 80% by 78 points; health left 13%, short of 20% by 7 points.
- P-2, Vanguard, level 30: win 0%, short of 80% by 80 points; health left 8%, short of 20% by 12 points.
- P-2, Acolyte, level 10: win 67%, short of 80% by 13 points.
- P-2, Acolyte, level 11: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Acolyte, level 12: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Acolyte, level 13: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Acolyte, level 14: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Acolyte, level 15: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Acolyte, level 16: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Acolyte, level 17: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Acolyte, level 18: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Acolyte, level 19: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Acolyte, level 20: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Acolyte, level 21: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Acolyte, level 22: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Acolyte, level 23: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Acolyte, level 24: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Acolyte, level 25: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Acolyte, level 26: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Acolyte, level 27: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Acolyte, level 28: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Acolyte, level 29: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-2, Acolyte, level 30: win 0%, short of 80% by 80 points; no fight won, so no health left.
- P-3, Cutthroat, level 10: win 70%, over 50% by 20 points.
- P-3, Cutthroat, level 11: win 5%, short of 15% by 10 points.
- P-3, Cutthroat, level 12: win 0%, short of 15% by 15 points.
- P-3, Cutthroat, level 13: win 0%, short of 15% by 15 points.
- P-3, Cutthroat, level 14: win 0%, short of 15% by 15 points.
- P-3, Cutthroat, level 15: win 0%, short of 15% by 15 points.
- P-3, Cutthroat, level 16: win 0%, short of 15% by 15 points.
- P-3, Cutthroat, level 17: win 0%, short of 15% by 15 points.
- P-3, Cutthroat, level 18: win 0%, short of 15% by 15 points.
- P-3, Cutthroat, level 19: win 0%, short of 15% by 15 points.
- P-3, Cutthroat, level 20: win 0%, short of 15% by 15 points.
- P-3, Cutthroat, level 21: win 0%, short of 15% by 15 points.
- P-3, Cutthroat, level 22: win 0%, short of 15% by 15 points.
- P-3, Cutthroat, level 23: win 6%, short of 15% by 9 points.
- P-3, Cutthroat, level 24: win 3%, short of 15% by 12 points.
- P-3, Cutthroat, level 25: win 3%, short of 15% by 12 points.
- P-3, Cutthroat, level 26: win 2%, short of 15% by 13 points.
- P-3, Cutthroat, level 27: win 6%, short of 15% by 9 points.
- P-3, Cutthroat, level 28: win 13%, short of 15% by 2 points.
- P-3, Cutthroat, level 29: win 8%, short of 15% by 7 points.
- P-3, Cutthroat, level 30: win 4%, short of 15% by 11 points.
- P-3, Ranger, level 10: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 11: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 12: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 13: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 14: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 15: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 16: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 17: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 18: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 19: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 20: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 21: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 22: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 23: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 24: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 25: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 26: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 27: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 28: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 29: win 0%, short of 15% by 15 points.
- P-3, Ranger, level 30: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 10: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 11: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 12: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 13: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 14: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 15: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 16: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 17: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 18: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 19: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 20: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 21: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 22: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 23: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 24: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 25: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 26: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 27: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 28: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 29: win 0%, short of 15% by 15 points.
- P-3, Initiate, level 30: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 10: win 7%, short of 15% by 8 points.
- P-3, Vanguard, level 11: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 12: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 13: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 14: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 15: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 16: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 17: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 18: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 19: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 20: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 21: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 22: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 23: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 24: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 25: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 26: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 27: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 28: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 29: win 0%, short of 15% by 15 points.
- P-3, Vanguard, level 30: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 10: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 11: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 12: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 13: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 14: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 15: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 16: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 17: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 18: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 19: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 20: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 21: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 22: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 23: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 24: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 25: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 26: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 27: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 28: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 29: win 0%, short of 15% by 15 points.
- P-3, Acolyte, level 30: win 0%, short of 15% by 15 points.
- DPS, Cutthroat, level 10: 55.0 DPS, 6.5 below the Acolyte's 61.5.
- DPS, Ranger, level 10: 37.4 DPS, 24.1 below the Acolyte's 61.5.
- DPS, Initiate, level 10: 42.2 DPS, 19.3 below the Acolyte's 61.5.
- DPS, Ranger, level 11: 33.8 DPS, 20.2 below the Acolyte's 54.0.
- DPS, Initiate, level 11: 42.4 DPS, 11.6 below the Acolyte's 54.0.
- DPS, Ranger, level 12: 33.2 DPS, 23.6 below the Acolyte's 56.8.
- DPS, Initiate, level 12: 43.0 DPS, 13.8 below the Acolyte's 56.8.
- DPS, Ranger, level 13: 32.9 DPS, 22.2 below the Acolyte's 55.1.
- DPS, Initiate, level 13: 44.3 DPS, 10.8 below the Acolyte's 55.1.
- DPS, Ranger, level 14: 35.3 DPS, 23.5 below the Acolyte's 58.8.
- DPS, Initiate, level 14: 46.0 DPS, 12.8 below the Acolyte's 58.8.
- DPS, Ranger, level 15: 37.2 DPS, 20.5 below the Acolyte's 57.7.
- DPS, Initiate, level 15: 47.4 DPS, 10.3 below the Acolyte's 57.7.
- DPS, Ranger, level 16: 38.8 DPS, 18.0 below the Vanguard's 56.8.
- DPS, Initiate, level 16: 47.6 DPS, 9.2 below the Vanguard's 56.8.
- DPS, Ranger, level 17: 39.9 DPS, 22.2 below the Vanguard's 62.1.
- DPS, Initiate, level 17: 48.9 DPS, 13.2 below the Vanguard's 62.1.
- DPS, Ranger, level 18: 40.8 DPS, 21.5 below the Vanguard's 62.3.
- DPS, Initiate, level 18: 50.5 DPS, 11.8 below the Vanguard's 62.3.
- DPS, Ranger, level 19: 46.1 DPS, 27.5 below the Vanguard's 73.6.
- DPS, Initiate, level 19: 54.9 DPS, 18.7 below the Vanguard's 73.6.
- DPS, Ranger, level 20: 47.1 DPS, 27.4 below the Vanguard's 74.5.
- DPS, Initiate, level 20: 55.4 DPS, 19.1 below the Vanguard's 74.5.
- DPS, Ranger, level 21: 51.2 DPS, 36.7 below the Vanguard's 87.9.
- DPS, Initiate, level 21: 57.0 DPS, 30.9 below the Vanguard's 87.9.
- DPS, Ranger, level 22: 51.1 DPS, 36.2 below the Vanguard's 87.3.
- DPS, Initiate, level 22: 58.1 DPS, 29.2 below the Vanguard's 87.3.
- DPS, Initiate, level 23: 61.6 DPS, 32.0 below the Vanguard's 93.6.
- DPS, Initiate, level 24: 63.1 DPS, 31.7 below the Vanguard's 94.8.
- DPS, Initiate, level 25: 65.3 DPS, 33.9 below the Vanguard's 99.2.
- DPS, Initiate, level 26: 67.5 DPS, 32.9 below the Vanguard's 100.4.
- DPS, Initiate, level 27: 70.3 DPS, 39.8 below the Vanguard's 110.1.
- DPS, Initiate, level 28: 72.4 DPS, 40.6 below the Vanguard's 113.0.
- DPS, Initiate, level 29: 74.6 DPS, 35.4 below the Vanguard's 110.0.
- DPS, Initiate, level 30: 76.2 DPS, 34.7 below the Vanguard's 110.9.

## Reading

- Tier 0 fails on the character's flat growth. From level 1 to 9 the on-level mob's health grows 9 times (63 to 570) and its swing from 2-3 to 33-44, while the Basic Warrior's DPS stays at 16.5 to 19. Fire Blast (20 + 2.5 a level), Gouge (10 + 1.5) and Heroic Strike (9.5 + 1.4) roughly double over those levels, the best Common weapon stops improving at level 6 (Trogg Club), and health grows only from 220 to 332. One mob stops holding at 7 and two mobs at 5.
- The Zone Factor step at 11 breaks P-2 for the tanks. Elwynn's 1.5 health and 1.3 damage take the mob from 634 to 1045 health between levels 10 and 11, and the 2-mob win drops from 97% to 1% for the Vanguard and from 67% to 0% for the Acolyte. Neither recovers at any level up to 30.
- The Cutthroat is the only Calling that ever holds P-2 (levels 10, 11, 23 and 27 to 30). Its 2-mob win falls to 12-47% at levels 14 to 22 and climbs back to 75-93% from 23, the level its single-mob health left jumps from 62% to 74% with the level-23 green set.
- The Ranger's gap below 23 is damage. Spells and shots carry a minimum range in the realm (`world/cast.rs`, `no_min_range`), so until Point Blank lands at 23 the Ranger fights a pull in melee with a staff at 33 to 51 DPS, and even one mob beats it 73-88% of the time at 17 to 22. Point Blank doubles its DPS (51 to 113) and wins every single fight from 23. Two mobs still win 0%, because a single kill takes 19 to 21 s and costs about half its health, so a second mob swinging through that kill costs more health than it has.
- The Initiate is the outlier on every rule. Ley Spark (31 + 2.5 a level) grows 1.9 times from 10 to 30 and green spell power only from 60 to 90, while on-level mob health grows 4.5 times. Its DPS goes from 42 to 76 and its single-mob win from 100% at 10 to 0-1% from 16. It is below both tanks' DPS at every level from 10 to 30, so the DPS rule never holds.
- Cloth survival gets worse after 22. The kit picker gives the casters stock vanilla chests with no stamina or spell power: Bright Armor (62 armour) for the Initiate at 22, Bright Robe (62) for the Acolyte at 22 to 25, and Silver-thread Robe (66) for both from 26. It picks them over the Loot Band robes (Eastvale Wool Robe has 345 armour and 5 stamina) because the casters' kit weights rank intellect far above armour and stamina. Their armour falls from about 333 at level 20 to 118 at 30.
- P-3 never holds. Against 3 mobs the Ranger, Initiate and Acolyte win 0% at every level, and the Vanguard wins 7% at 10 and 0% after. The Cutthroat wins 70% at 10, over the band, and 0-13% from 11.
- The DPS rule holds at no level. At 10 the Acolyte's 61.5 DPS beats all three damage Callings (Blood Lance's 36 + 2 a level is high at low levels). The Ranger misses from 10 to 22, and the Initiate misses at every level.

## Kits and builds

As `run.py` printed them. The Basic Warrior wears its starting outfit and whites; the Callings wear greens.
Talents are the damage build at every point the level gives.

| Calling | Level | Gear | Kit | Talents |
|---|---|---|---|---|
| Basic Warrior | 1 | white | Novice's Chain Vest, Pendant of Brawn, Band of Brawn, Band of Endurance, Simple Dagger, Militia Buckler | none |
| Basic Warrior | 2 | white | Novice's Chain Vest, Pendant of Brawn, Band of Brawn, Band of Endurance, Flanged Mace, Militia Buckler | none |
| Basic Warrior | 3 | white | Novice's Chain Vest, Pendant of Brawn, Band of Brawn, Band of Endurance, Stiletto, Militia Buckler | none |
| Basic Warrior | 4 | white | Novice's Chain Vest, Pendant of Brawn, Band of Brawn, Band of Endurance, Billy Club, Militia Buckler | none |
| Basic Warrior | 5 | white | Novice's Chain Vest, Pendant of Brawn, Band of Brawn, Band of Endurance, Stonesplinter Blade, Militia Buckler | none |
| Basic Warrior | 6 | white | Novice's Chain Vest, Pendant of Brawn, Band of Brawn, Band of Endurance, Trogg Club, Militia Buckler | none |
| Basic Warrior | 7 | white | Novice's Chain Vest, Pendant of Brawn, Band of Brawn, Band of Endurance, Trogg Club, Militia Buckler | none |
| Basic Warrior | 8 | white | Novice's Chain Vest, Pendant of Brawn, Band of Brawn, Band of Endurance, Trogg Club, Militia Buckler | none |
| Basic Warrior | 9 | white | Novice's Chain Vest, Pendant of Brawn, Band of Brawn, Band of Endurance, Trogg Club, Militia Buckler | none |
| Cutthroat | 10 | green | Trapper's Leather Jerkin, Poacher's Locket, Prowler's Signet, Echo Ridge Loop, Marshal's Broadsword, Padfoot's Shiv (OH) | none |
| Cutthroat | 11 | green | Stonefield Leather Jerkin, Brackwell Talisman, Brackwell Loop, Prowler's Signet, Brackwell Cutlass, Maclure Dirk (OH) | reflexive_strike 1 |
| Cutthroat | 12 | green | Stonefield Leather Jerkin, Brackwell Talisman, Brackwell Loop, Prowler's Signet, Brackwell Cutlass, Maclure Dirk (OH) | reflexive_strike 1, blood_frenzy 1, bloodletting 1 |
| Cutthroat | 13 | green | Stonefield Leather Jerkin, Brackwell Talisman, Brackwell Loop, Fencemender Loop, Brackwell Cutlass, Maclure Dirk (OH) | reflexive_strike 1, blood_frenzy 1, bloodletting 1, executioners_toll 1 |
| Cutthroat | 14 | green | Stonefield Leather Jerkin, Brackwell Talisman, Courier's Loop, Brackwell Loop, Morgan's Cutlass, Morgan's Dirk (OH) | reflexive_strike 2, blood_frenzy 2, bloodletting 1, executioners_toll 1 |
| Cutthroat | 15 | green | Fargodeep Leather Jerkin, Crystal Lake Talisman, Crystal Lake Loop, Courier's Loop, Crystal Lake Cutlass, Kobold Dirk (OH) | reflexive_strike 2, blood_frenzy 2, bloodletting 2, executioners_toll 1 |
| Cutthroat | 16 | green | Fargodeep Leather Jerkin, Crystal Lake Talisman, Crystal Lake Loop, Courier's Loop, Vigil Saber, Vigil Dagger (OH) | reflexive_strike 2, blood_frenzy 2, bloodletting 2, executioners_toll 2 |
| Cutthroat | 17 | green | Fargodeep Leather Jerkin, Crystal Lake Talisman, Crystal Lake Loop, Courier's Loop, Vigil Saber, Vigil Dagger (OH) | reflexive_strike 3, blood_frenzy 3, bloodletting 2, executioners_toll 2 |
| Cutthroat | 18 | green | Prospector's Chestpiece, Crystal Lake Talisman, Stonerite Loop, Crystal Lake Loop, Vigil Saber, Vigil Dagger (OH) | reflexive_strike 3, blood_frenzy 3, bloodletting 2, executioners_toll 2, phantom_guile 1 |
| Cutthroat | 19 | green | Azora Leather Jerkin, Stone Cairn Talisman, Stone Cairn Loop, Stonerite Loop, Stone Cairn Cutlass, Murloc Dirk (OH) | reflexive_strike 3, blood_frenzy 3, bloodletting 3, executioners_toll 2, phantom_guile 1, thirst 1 |
| Cutthroat | 20 | green | Azora Leather Jerkin, Stone Cairn Talisman, Stone Cairn Loop, Stonerite Loop, Stone Cairn Cutlass, Murloc Dirk (OH) | reflexive_strike 3, blood_frenzy 3, bloodletting 3, executioners_toll 3, phantom_guile 1, thirst 1 |
| Cutthroat | 21 | green | Azora Leather Jerkin, Stone Cairn Talisman, Stone Cairn Loop, Stonerite Loop, Timberman's Axe-Sword, Timberman's Dagger (OH) | reflexive_strike 4, blood_frenzy 3, bloodletting 3, executioners_toll 3, phantom_guile 1, thirst 1, scent_of_blood 1 |
| Cutthroat | 22 | green | Azora Leather Jerkin, Stone Cairn Talisman, Ridgewatch Loop, Stone Cairn Loop, Timberman's Axe-Sword, Timberman's Dagger (OH) | reflexive_strike 4, blood_frenzy 4, bloodletting 3, executioners_toll 3, phantom_guile 1, thirst 1, scent_of_blood 1 |
| Cutthroat | 23 | green | Ridgepoint Leather Jerkin, Eastvale Talisman, Eastvale Loop, Ridgewatch Loop, Eastvale Cutlass, Mirror Lake Dirk (OH) | reflexive_strike 4, blood_frenzy 4, bloodletting 3, executioners_toll 3, phantom_guile 2, thirst 2, scent_of_blood 1 |
| Cutthroat | 24 | green | Ridgepoint Leather Jerkin, Eastvale Talisman, Ledger Loop, Eastvale Loop, Eastvale Cutlass, Mirror Lake Dirk (OH) | reflexive_strike 4, blood_frenzy 4, bloodletting 3, executioners_toll 3, phantom_guile 2, thirst 2, scent_of_blood 1, flurry_of_blades 1 |
| Cutthroat | 25 | green | Ridgepoint Leather Jerkin, Eastvale Talisman, Ledger Loop, Eastvale Loop, Pit Sword, Pit Dirk (OH) | reflexive_strike 4, blood_frenzy 4, bloodletting 4, executioners_toll 3, phantom_guile 2, thirst 2, scent_of_blood 2, flurry_of_blades 1 |
| Cutthroat | 26 | green | Ridgepoint Leather Jerkin, Eastvale Talisman, Boatburner's Loop, Ledger Loop, Pit Sword, Pit Dirk (OH) | reflexive_strike 5, blood_frenzy 4, bloodletting 4, executioners_toll 3, phantom_guile 2, thirst 2, scent_of_blood 2, flurry_of_blades 1 |
| Cutthroat | 27 | green | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Cutlass, Forest's Edge Dirk (OH) | reflexive_strike 5, blood_frenzy 5, bloodletting 4, executioners_toll 3, phantom_guile 2, thirst 2, scent_of_blood 2, flurry_of_blades 1 |
| Cutthroat | 28 | green | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Cutlass, Forest's Edge Dirk (OH) | reflexive_strike 5, blood_frenzy 5, bloodletting 4, executioners_toll 3, phantom_guile 3, thirst 2, scent_of_blood 2, flurry_of_blades 2 |
| Cutthroat | 29 | green | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Cutlass, Forest's Edge Dirk (OH) | reflexive_strike 5, blood_frenzy 5, bloodletting 5, executioners_toll 3, phantom_guile 3, thirst 2, scent_of_blood 2, flurry_of_blades 2 |
| Cutthroat | 30 | green | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Cutlass, Forest's Edge Dirk (OH) | reflexive_strike 5, blood_frenzy 5, bloodletting 5, executioners_toll 3, phantom_guile 3, thirst 2, scent_of_blood 3, flurry_of_blades 2, ruthlessness 1 |
| Ranger | 10 | green | Trapper's Leather Jerkin, Poacher's Locket, Prowler's Signet, Echo Ridge Loop, Padfoot's Ill-Gotten Staff, Vineyard Poacher's Bow | none |
| Ranger | 11 | green | Stonefield Leather Jerkin, Brackwell Talisman, Brackwell Loop, Prowler's Signet, Brackwell Quarterstaff, Lion's Pride Shortbow | piercing_force 1 |
| Ranger | 12 | green | Stonefield Leather Jerkin, Brackwell Talisman, Brackwell Loop, Prowler's Signet, Brackwell Quarterstaff, Lion's Pride Shortbow | piercing_force 1, jagged_barbs 1, guerilla_tactics 1 |
| Ranger | 13 | green | Stonefield Leather Jerkin, Brackwell Talisman, Brackwell Loop, Fencemender Loop, Brackwell Quarterstaff, Lion's Pride Shortbow | piercing_force 1, jagged_barbs 1, guerilla_tactics 1, rhythmic_draw 1 |
| Ranger | 14 | green | Stonefield Leather Jerkin, Brackwell Talisman, Courier's Loop, Brackwell Loop, Narg's Crook, Lion's Pride Shortbow | piercing_force 2, jagged_barbs 2, guerilla_tactics 1, rhythmic_draw 1 |
| Ranger | 15 | green | Fargodeep Leather Jerkin, Crystal Lake Talisman, Crystal Lake Loop, Courier's Loop, Crystal Lake Quarterstaff, Grik's Shortbow | piercing_force 2, jagged_barbs 2, guerilla_tactics 2, rhythmic_draw 1 |
| Ranger | 16 | green | Fargodeep Leather Jerkin, Crystal Lake Talisman, Crystal Lake Loop, Courier's Loop, Vigil Staff, Grik's Shortbow | piercing_force 2, jagged_barbs 2, guerilla_tactics 2, rhythmic_draw 2 |
| Ranger | 17 | green | Fargodeep Leather Jerkin, Crystal Lake Talisman, Crystal Lake Loop, Courier's Loop, Vigil Staff, Kobold King's Longbow | piercing_force 3, jagged_barbs 3, guerilla_tactics 2, rhythmic_draw 2 |
| Ranger | 18 | green | Fargodeep Leather Jerkin, Crystal Lake Talisman, Stonerite Loop, Crystal Lake Loop, Vigil Staff, Kobold King's Longbow | piercing_force 3, jagged_barbs 3, guerilla_tactics 2, rhythmic_draw 2, steadfast_aim 1 |
| Ranger | 19 | green | Azora Leather Jerkin, Stone Cairn Talisman, Stone Cairn Loop, Stonerite Loop, Stone Cairn Quarterstaff, Cairn Watch Shortbow | piercing_force 3, jagged_barbs 3, guerilla_tactics 3, rhythmic_draw 3, steadfast_aim 1 |
| Ranger | 20 | green | Azora Leather Jerkin, Stone Cairn Talisman, Stone Cairn Loop, Stonerite Loop, Stone Cairn Quarterstaff, Cairn Watch Shortbow | piercing_force 3, jagged_barbs 3, guerilla_tactics 3, rhythmic_draw 3, steadfast_aim 1, twin_arrows 1 |
| Ranger | 21 | green | Azora Leather Jerkin, Stone Cairn Talisman, Stone Cairn Loop, Stonerite Loop, Timberman's Staff, Apprentice's Crossbow | piercing_force 4, jagged_barbs 4, guerilla_tactics 3, rhythmic_draw 3, steadfast_aim 1, twin_arrows 1 |
| Ranger | 22 | green | Azora Leather Jerkin, Stone Cairn Talisman, Ridgewatch Loop, Stone Cairn Loop, Timberman's Staff, Apprentice's Crossbow | piercing_force 4, jagged_barbs 4, guerilla_tactics 3, rhythmic_draw 3, steadfast_aim 2, twin_arrows 1 |
| Ranger | 23 | green | Ridgepoint Leather Jerkin, Eastvale Talisman, Eastvale Loop, Ridgewatch Loop, Eastvale Quarterstaff, Smuggler's Shortbow | piercing_force 4, jagged_barbs 4, guerilla_tactics 3, rhythmic_draw 3, steadfast_aim 2, twin_arrows 1, shattered_defenses 1, point_blank 1 |
| Ranger | 24 | green | Ridgepoint Leather Jerkin, Eastvale Talisman, Ledger Loop, Eastvale Loop, Eastvale Quarterstaff, Smuggler's Shortbow | piercing_force 4, jagged_barbs 4, guerilla_tactics 4, rhythmic_draw 3, steadfast_aim 2, twin_arrows 1, shattered_defenses 1, point_blank 1 |
| Ranger | 25 | green | Ridgepoint Leather Jerkin, Eastvale Talisman, Ledger Loop, Eastvale Loop, Eastvale Quarterstaff, Smuggler's Shortbow | piercing_force 4, jagged_barbs 4, guerilla_tactics 4, rhythmic_draw 4, steadfast_aim 2, twin_arrows 2, shattered_defenses 1, point_blank 1 |
| Ranger | 26 | green | Ridgepoint Leather Jerkin, Eastvale Talisman, Boatburner's Loop, Ledger Loop, Eastvale Quarterstaff, Smuggler's Shortbow | piercing_force 4, jagged_barbs 4, guerilla_tactics 4, rhythmic_draw 4, steadfast_aim 2, twin_arrows 2, shattered_defenses 1, point_blank 1, adrenaline_rush 1 |
| Ranger | 27 | green | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Quarterstaff, Deepwater Shortbow | piercing_force 5, jagged_barbs 4, guerilla_tactics 4, rhythmic_draw 4, steadfast_aim 2, twin_arrows 2, shattered_defenses 1, point_blank 1, adrenaline_rush 1 |
| Ranger | 28 | green | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Quarterstaff, Deepwater Shortbow | piercing_force 5, jagged_barbs 5, guerilla_tactics 4, rhythmic_draw 4, steadfast_aim 2, twin_arrows 2, shattered_defenses 2, point_blank 1, adrenaline_rush 1 |
| Ranger | 29 | green | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Quarterstaff, Deepwater Shortbow | piercing_force 5, jagged_barbs 5, guerilla_tactics 4, rhythmic_draw 4, steadfast_aim 2, twin_arrows 2, shattered_defenses 2, point_blank 2, adrenaline_rush 1 |
| Ranger | 30 | green | Riverpaw Leather Jerkin, Westbrook Talisman, Westbrook Loop, Boatburner's Loop, Westbrook Quarterstaff, Deepwater Shortbow | piercing_force 5, jagged_barbs 5, guerilla_tactics 5, rhythmic_draw 5, steadfast_aim 2, twin_arrows 2, shattered_defenses 2, point_blank 2, adrenaline_rush 1 |
| Initiate | 10 | green | Abbey Scholar's Robe, Scribe's Pendant, Echo Ridge Circle, Scribe's Band, Padfoot's Ill-Gotten Staff, Familiar's Spark | none |
| Initiate | 11 | green | Brackwell Wool Robe, Stonefield Pendant, Maclure Circle, Lion's Pride Seal, Stonefield Cudgel, Vigil Candle, Familiar's Spark | ley_line_tap 1 |
| Initiate | 12 | green | Brackwell Wool Robe, Stonefield Pendant, Maclure Circle, Lion's Pride Seal, Stonefield Cudgel, Vigil Candle, Familiar's Spark | ley_line_tap 1, volatile_embers 1, thermal_feedback 1 |
| Initiate | 13 | green | Brackwell Wool Robe, Stonefield Pendant, Maclure Circle, Fencepost Band, Stonefield Cudgel, Vigil Candle, Familiar's Spark | ley_line_tap 2, volatile_embers 1, thermal_feedback 1 |
| Initiate | 14 | green | Brackwell Wool Robe, Stonefield Pendant, Courier's Seal, Maclure Circle, Stonefield Cudgel, Vigil Candle, Familiar's Spark | ley_line_tap 2, volatile_embers 2, thermal_feedback 2 |
| Initiate | 15 | green | Crystal Lake Wool Robe, Fargodeep Pendant, Kobold Circle, Courier's Seal, Crystal Lake Quarterstaff, Familiar's Spark | ley_line_tap 3, volatile_embers 2, thermal_feedback 2 |
| Initiate | 16 | green | Seer's Robe, Fargodeep Pendant, Kobold Circle, Courier's Seal, Crystal Lake Quarterstaff, Familiar's Spark | ley_line_tap 3, volatile_embers 3, thermal_feedback 2 |
| Initiate | 17 | green | Seer's Robe, Fargodeep Pendant, Kobold Circle, Courier's Seal, Crystal Lake Quarterstaff, Familiar's Spark | ley_line_tap 3, volatile_embers 3, thermal_feedback 2, ley_line_attunement 1, frigid_conservation 1 |
| Initiate | 18 | green | Mystic's Wrap, Fargodeep Pendant, Rite Band, Kobold Circle, Crystal Lake Quarterstaff, Familiar's Spark | ley_line_tap 3, volatile_embers 3, thermal_feedback 3, ley_line_attunement 1, frigid_conservation 1 |
| Initiate | 19 | green | Stone Cairn Wool Robe, Azora Pendant, Murloc Circle, Rite Band, Azora Cudgel, Murloc Lantern, Familiar's Spark | ley_line_tap 4, volatile_embers 4, thermal_feedback 3, ley_line_attunement 1, frigid_conservation 1 |
| Initiate | 20 | green | Stone Cairn Wool Robe, Azora Pendant, Murloc Circle, Rite Band, Azora Cudgel, Murloc Lantern, Familiar's Spark | ley_line_tap 4, volatile_embers 4, thermal_feedback 3, ley_line_attunement 2, frigid_conservation 1 |
| Initiate | 21 | green | Stone Cairn Wool Robe, Azora Pendant, Murloc Circle, Rite Band, Timberman's Staff, Familiar's Spark | ley_line_tap 4, volatile_embers 4, thermal_feedback 3, ley_line_attunement 2, frigid_conservation 2, catalytic_resonance 1 |
| Initiate | 22 | green | Bright Armor, Azora Pendant, Ridgewatch Seal, Murloc Circle, Timberman's Staff, Familiar's Spark | ley_line_tap 4, volatile_embers 4, thermal_feedback 3, ley_line_attunement 2, frigid_conservation 2, catalytic_resonance 1, arcane_momentum 1 |
| Initiate | 23 | green | Eastvale Wool Robe, Ridgepoint Pendant, Mirror Lake Circle, Ridgewatch Seal, Ridgepoint Cudgel, Mirror Lake Lantern, Familiar's Spark | ley_line_tap 5, volatile_embers 4, thermal_feedback 4, ley_line_attunement 2, frigid_conservation 2, catalytic_resonance 1, arcane_momentum 1 |
| Initiate | 24 | green | Eastvale Wool Robe, Ridgepoint Pendant, Ledger Band, Mirror Lake Circle, Ridgepoint Cudgel, Mirror Lake Lantern, Familiar's Spark | ley_line_tap 5, volatile_embers 5, thermal_feedback 4, ley_line_attunement 2, frigid_conservation 2, catalytic_resonance 1, arcane_momentum 1 |
| Initiate | 25 | green | Eastvale Wool Robe, Ridgepoint Pendant, Ledger Band, Mirror Lake Circle, Pit Mace, Mirror Lake Lantern, Familiar's Spark | ley_line_tap 5, volatile_embers 5, thermal_feedback 4, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 2, arcane_momentum 1 |
| Initiate | 26 | green | Silver-thread Robe, Ridgepoint Pendant, Boatburner's Seal, Ledger Band, Pit Mace, Mirror Lake Lantern, Familiar's Spark | ley_line_tap 5, volatile_embers 5, thermal_feedback 4, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 2, arcane_momentum 2 |
| Initiate | 27 | green | Silver-thread Robe, Riverpaw Pendant, Boatburner's Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | ley_line_tap 5, volatile_embers 5, thermal_feedback 5, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 2, arcane_momentum 2 |
| Initiate | 28 | green | Silver-thread Robe, Riverpaw Pendant, Boatburner's Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | ley_line_tap 5, volatile_embers 5, thermal_feedback 5, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 3, arcane_momentum 3 |
| Initiate | 29 | green | Silver-thread Robe, Riverpaw Pendant, Boatburner's Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | ley_line_tap 5, volatile_embers 5, thermal_feedback 5, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 4, arcane_momentum 3 |
| Initiate | 30 | green | Silver-thread Robe, Riverpaw Pendant, Boatburner's Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | ley_line_tap 5, volatile_embers 5, thermal_feedback 5, ley_line_attunement 3, frigid_conservation 2, catalytic_resonance 5, arcane_momentum 3 |
| Vanguard | 10 | green | Stonefield Plate Vest, Vintner's Locket, Brawler's Signet, Echo Ridge Band, Marshal's Broadsword, Veteran Shield | none |
| Vanguard | 11 | green | Lion's Pride Plate Cuirass, Lion's Pride Torc, Maclure Band, Lion's Pride Signet, Brackwell Cutlass, Stonefield Kite Shield | divine_strength 1 |
| Vanguard | 12 | green | Lion's Pride Plate Cuirass, Lion's Pride Torc, Maclure Band, Lion's Pride Signet, Brackwell Cutlass, Stonefield Kite Shield | divine_strength 2, wrathful_iron 1 |
| Vanguard | 13 | green | Lion's Pride Plate Cuirass, Lion's Pride Torc, Maclure Band, Fencewright Ring, Brackwell Cutlass, Stonefield Kite Shield | divine_strength 2, wrathful_iron 2 |
| Vanguard | 14 | green | Lion's Pride Plate Cuirass, Lion's Pride Torc, Courier's Signet, Maclure Band, Morgan's Cutlass, Stonefield Kite Shield | divine_strength 3, wrathful_iron 3 |
| Vanguard | 15 | green | Grik's Plate Cuirass, Grik's Torc, Kobold Band, Courier's Signet, Crystal Lake Cutlass, Fargodeep Kite Shield | divine_strength 3, wrathful_iron 3, hallowed_ground 1 |
| Vanguard | 16 | green | Grik's Plate Cuirass, Grik's Torc, Kobold Band, Courier's Signet, Vigil Saber, Fargodeep Kite Shield | divine_strength 4, wrathful_iron 3, hallowed_ground 1 |
| Vanguard | 17 | green | Grik's Plate Cuirass, Grik's Torc, Kobold Band, Courier's Signet, Vigil Saber, Kobold King's Aegis | divine_strength 4, wrathful_iron 4, hallowed_ground 2 |
| Vanguard | 18 | green | Grik's Plate Cuirass, Grik's Torc, Standing Stone Ring, Kobold Band, Vigil Saber, Kobold King's Aegis | divine_strength 5, wrathful_iron 4, hallowed_ground 2 |
| Vanguard | 19 | green | Cairn Watch Plate Cuirass, Cairn Watch Torc, Murloc Band, Standing Stone Ring, Stone Cairn Cutlass, Azora Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 2, righteous_fury 1 |
| Vanguard | 20 | green | Cairn Watch Plate Cuirass, Cairn Watch Torc, Murloc Band, Standing Stone Ring, Stone Cairn Cutlass, Azora Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 2, righteous_fury 1, zealous_restoration 1 |
| Vanguard | 21 | green | Cairn Watch Plate Cuirass, Cairn Watch Torc, Murloc Band, Standing Stone Ring, Timberman's Axe-Sword, Apprentice's Bulwark | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 1, zealous_restoration 1, hammer_of_vanguard 1 |
| Vanguard | 22 | green | Cairn Watch Plate Cuirass, Cairn Watch Torc, Ridgewatch Signet, Murloc Band, Timberman's Axe-Sword, Apprentice's Bulwark | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 2, zealous_restoration 1, hammer_of_vanguard 1 |
| Vanguard | 23 | green | Smuggler's Plate Cuirass, Smuggler's Torc, Mirror Lake Band, Ridgewatch Signet, Eastvale Cutlass, Ridgepoint Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 3, zealous_restoration 2, hammer_of_vanguard 1 |
| Vanguard | 24 | green | Smuggler's Plate Cuirass, Smuggler's Torc, Ledger Signet, Mirror Lake Band, Eastvale Cutlass, Ridgepoint Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 3, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 25 | green | Smuggler's Plate Cuirass, Smuggler's Torc, Ledger Signet, Mirror Lake Band, Pit Sword, Ridgepoint Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 26 | green | Smuggler's Plate Cuirass, Smuggler's Torc, Boatburner's Signet, Ledger Signet, Pit Sword, Ridgepoint Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 27 | green | Deepwater Plate Cuirass, Deepwater Torc, Forest's Edge Band, Deepwater Signet, Westbrook Cutlass, Riverpaw Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 28 | green | Deepwater Plate Cuirass, Deepwater Torc, Forest's Edge Band, Deepwater Signet, Westbrook Cutlass, Riverpaw Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 29 | green | Deepwater Plate Cuirass, Deepwater Torc, Forest's Edge Band, Deepwater Signet, Westbrook Cutlass, Riverpaw Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Vanguard | 30 | green | Deepwater Plate Cuirass, Deepwater Torc, Forest's Edge Band, Deepwater Signet, Westbrook Cutlass, Riverpaw Kite Shield | divine_strength 5, wrathful_iron 5, hallowed_ground 3, righteous_fury 5, zealous_restoration 3, hammer_of_vanguard 1 |
| Acolyte | 10 | green | Abbey Scholar's Robe, Scribe's Pendant, Scribe's Signet, Echo Ridge Circle, Padfoot's Ill-Gotten Staff, Familiar's Spark | none |
| Acolyte | 11 | green | Brackwell Wool Robe, Stonefield Pendant, Lion's Pride Seal, Maclure Circle, Stonefield Cudgel, Vigil Candle, Familiar's Spark | virulence 1 |
| Acolyte | 12 | green | Brackwell Wool Robe, Stonefield Pendant, Lion's Pride Seal, Maclure Circle, Stonefield Cudgel, Vigil Candle, Familiar's Spark | virulence 1, vitality 1, masochism 1 |
| Acolyte | 13 | green | Brackwell Wool Robe, Stonefield Pendant, Lion's Pride Seal, Maclure Circle, Stonefield Cudgel, Vigil Candle, Familiar's Spark | virulence 2, vitality 1, masochism 1 |
| Acolyte | 14 | green | Brackwell Wool Robe, Stonefield Pendant, Lion's Pride Seal, Courier's Seal, Stonefield Cudgel, Vigil Candle, Familiar's Spark | virulence 2, vitality 2, masochism 2 |
| Acolyte | 15 | green | Crystal Lake Wool Robe, Fargodeep Pendant, Kobold Circle, Grik's Seal, Crystal Lake Quarterstaff, Familiar's Spark | virulence 3, vitality 2, masochism 2 |
| Acolyte | 16 | green | Seer's Robe, Fargodeep Pendant, Kobold Circle, Grik's Seal, Crystal Lake Quarterstaff, Familiar's Spark | virulence 3, vitality 3, masochism 2 |
| Acolyte | 17 | green | Seer's Robe, Fargodeep Pendant, Kobold Circle, Grik's Seal, Crystal Lake Quarterstaff, Familiar's Spark | virulence 3, vitality 3, masochism 3, crimson_surge 1 |
| Acolyte | 18 | green | Seer's Robe, Fargodeep Pendant, Rite Band, Kobold Circle, Crystal Lake Quarterstaff, Familiar's Spark | virulence 4, vitality 3, masochism 3, crimson_surge 1 |
| Acolyte | 19 | green | Stone Cairn Wool Robe, Azora Pendant, Murloc Circle, Cairn Watch Seal, Azora Cudgel, Murloc Lantern, Familiar's Spark | virulence 4, vitality 4, masochism 4, crimson_surge 1 |
| Acolyte | 20 | green | Stone Cairn Wool Robe, Azora Pendant, Murloc Circle, Cairn Watch Seal, Azora Cudgel, Murloc Lantern, Familiar's Spark | virulence 4, vitality 4, masochism 4, crimson_surge 2 |
| Acolyte | 21 | green | Stone Cairn Wool Robe, Azora Pendant, Murloc Circle, Cairn Watch Seal, Timberman's Staff, Familiar's Spark | virulence 5, vitality 4, masochism 4, crimson_surge 2, will_to_survive 1 |
| Acolyte | 22 | green | Bright Robe, Azora Pendant, Ridgewatch Seal, Murloc Circle, Timberman's Staff, Familiar's Spark | virulence 5, vitality 4, masochism 4, crimson_surge 2, will_to_survive 1, morbid_fascination 1 |
| Acolyte | 23 | green | Bright Robe, Ridgepoint Pendant, Mirror Lake Circle, Smuggler's Seal, Ridgepoint Cudgel, Mirror Lake Lantern, Familiar's Spark | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 1, morbid_fascination 1 |
| Acolyte | 24 | green | Bright Robe, Ridgepoint Pendant, Ledger Band, Mirror Lake Circle, Ridgepoint Cudgel, Mirror Lake Lantern, Familiar's Spark | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 2, morbid_fascination 1 |
| Acolyte | 25 | green | Bright Robe, Ridgepoint Pendant, Ledger Band, Mirror Lake Circle, Pit Mace, Mirror Lake Lantern, Familiar's Spark | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 2 |
| Acolyte | 26 | green | Silver-thread Robe, Ridgepoint Pendant, Boatburner's Seal, Ledger Band, Pit Mace, Mirror Lake Lantern, Familiar's Spark | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 3 |
| Acolyte | 27 | green | Silver-thread Robe, Riverpaw Pendant, Deepwater Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 3 |
| Acolyte | 28 | green | Silver-thread Robe, Riverpaw Pendant, Deepwater Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 3 |
| Acolyte | 29 | green | Silver-thread Robe, Riverpaw Pendant, Deepwater Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 3 |
| Acolyte | 30 | green | Silver-thread Robe, Riverpaw Pendant, Deepwater Seal, Forest's Edge Circle, Riverpaw Cudgel, Forest's Edge Lantern, Familiar's Spark | virulence 5, vitality 5, masochism 5, crimson_surge 2, will_to_survive 3, morbid_fascination 3 |
