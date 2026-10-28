# Balance check 3: proposed edits (model 2026-10-28)

Issue #489, the last of six for the third Tier 1 balance check. Rules: `docs/balance/rules.md`. Inputs: `pulls.md`,
`elites.md`, `blues.md` and `plus-five.md` beside this file. Nothing here has landed in `data/`, `crates/` or
`tools/`. Davis decides.

The proposal does not make every rule hold. It takes the model from 676 missing cells to 323. Tier 0 holds at every
level, and B-boss, B-named, G+5 and G+5-named hold for most Callings. P-3, E-dmg and E-tank still fail almost
everywhere. Two of the remaining groups need a mechanic or a rotation change, not a number (see Still failing).

## How it was run

- Every candidate is a scratch data root under the session's scratchpad, built by `target/agent/build_root489.py`
  (not committed): it copies `data/twilights-eve/` and `crates/twilight-realm/content/`, edits `mobs.toml` and
  `callings.csv` by text, edits `master.csv` through `tools/twilights-eve/sheet.py --csv <root>/... set`, and
  regenerates `server_spells.json` and `client_dbc_patch.json` with `build_spells.py --csv ... --callings ...
  --content ... --out-dir ... --no-dbc-check`.
- The model ran from a scratch copy of `tools/twilights-eve/balance/` with one fix: `model.build_stats` now keeps
  `callings.csv` `health_pct` percent of the base health, as `rules/stats.rs` `scale_base_pools` does in the realm.
  The repo's model ignores that column (see Model gaps), and with every Calling at 100 both copies print the same
  numbers. The baseline below is that copy on the repo's data: 676 failing cells, the same as the four reports
  added up (245 + 98 + 211 + 122).
- The levers were found with an in-memory probe (`target/agent/probe489.py`, 100 fights a cell at six levels) and
  then proven on full roots:

  | Root | Edits | Failing cells, 300 fights |
  |---|---|---|
  | repo | none | 676 of 991 |
  | root-a | the edits below | 323 of 991 |
  | root-b | root-a and `[difficulty.rank]` elite 2.0/1.5 to 3.2/1.25 | 335 of 991 |

  The probe also tried mob damage 1.5 and 1.6, elite 2.5/2.2, 2.6/1.8 and 4.0/1.4, Point Blank moved to talent row
  2, and Cutthroat at 90% health. Each scored worse than root-a or no better. root-a is the proposal.
- Command for `after.md`, run from the scratch model copy:
  `python3.15 <scratchpad>/model-489/run.py --scenario all --data-root <scratchpad>/root-a --fights 300 --jobs 4 --out docs/balance/2026-10-28/after.md`

## Before

The failures of the four reports, one row per rule and Calling, from the baseline run.

676 cells miss: T0-1 3, T0-2 5, P-2 97, P-3 105, DPS 35, E-dmg 56, E-tank 42, B-2E 87, B-boss 84, B-named 40, G+5 77, G+5-named 45.

| Rule | Calling | Levels or bosses that miss | Worst reading | Band | Worst gap |
|---|---|---|---|---|---|
| T0-1 | basic_warrior | 7-9 | level 9: win 3% | win >= 97% | 94 points |
| T0-2 | basic_warrior | 5-9 | level 6: win 0% | win >= 90% | 90 points |
| P-2 | ranger | 10-30 | level 10: win 0%, 2% left | win >= 80%, health left >= 20% | 80 points |
| P-2 | initiate | 10-30 | level 10: win 0% | win >= 80%, health left >= 20% | 80 points |
| P-2 | acolyte | 10-30 | level 11: win 0% | win >= 80%, health left >= 20% | 80 points |
| P-2 | vanguard | 11-30 | level 12: win 0%, 10% left | win >= 80%, health left >= 20% | 80 points |
| P-2 | cutthroat | 12-22, 24-26 | level 18: win 12%, 18% left | win >= 80%, health left >= 20% | 68 points |
| P-3 | cutthroat | 10-30 | level 10: win 70% | win 15% to 50% | 20 points |
| P-3 | ranger | 10-30 | level 10: win 0% | win 15% to 50% | 15 points |
| P-3 | initiate | 10-30 | level 10: win 0% | win 15% to 50% | 15 points |
| P-3 | vanguard | 10-30 | level 11: win 0% | win 15% to 50% | 15 points |
| P-3 | acolyte | 10-30 | level 10: win 0% | win 15% to 50% | 15 points |
| DPS | cutthroat | 10 | level 10: 55.0 DPS against 61.5 | above both tanks' DPS | 6.5 DPS |
| DPS | ranger | 10-22 | level 21: 51.2 DPS against 87.9 | above both tanks' DPS | 36.7 DPS |
| DPS | initiate | 10-30 | level 28: 72.4 DPS against 113.0 | above both tanks' DPS | 40.6 DPS |
| E-dmg | cutthroat | 10-11, 16-18, 20, 23-30 | level 10: win 100% | win 35% to 65% | 35 points |
| E-dmg | ranger | 10-30 | level 11: win 0% | win 35% to 65% | 35 points |
| E-dmg | initiate | 10-30 | level 11: win 0% | win 35% to 65% | 35 points |
| E-tank | vanguard | 10-30 | level 11: win 0% | win >= 70% and above every damage Calling | 70 points |
| E-tank | acolyte | 10-30 | level 11: win 0% | win >= 70% and above every damage Calling | 70 points |
| B-2E | ranger | 10-30 | level 11: win 0% | win >= 80% | 80 points |
| B-2E | initiate | 10-30 | level 10: win 0% | win >= 80% | 80 points |
| B-2E | vanguard | 10-30 | level 11: win 0% | win >= 80% | 80 points |
| B-2E | acolyte | 10-30 | level 10: win 0% | win >= 80% | 80 points |
| B-2E | cutthroat | 13-14, 18 | level 14: win 12% | win >= 80% | 68 points |
| B-boss | ranger | 10-30 | level 11: win 0% | win >= 80% | 80 points |
| B-boss | initiate | 10-30 | level 10: win 0% | win >= 80% | 80 points |
| B-boss | vanguard | 11-30 | level 11: win 0% | win >= 80% | 80 points |
| B-boss | acolyte | 11-30 | level 11: win 0% | win >= 80% | 80 points |
| B-boss | cutthroat | 13-14 | level 14: win 29% | win >= 80% | 51 points |
| B-named | initiate | Ghost Howl (12); The Rake (10); Garrick Padfoot (10); Abbot Ashworth (10); Hogger (30); Princess (12); Morgan the Collector (14); Narg the Taskmaster (14); Gresh the Drowned (20); Speaker Thane (21); The Kept (22); Shadow of Morganth (22); The Eye Below (18); Ritualist Serane (18); Speaker Vael (24); Acolyte Marren (26) | Ghost Howl: win 0% | win >= 80% | 80 points |
| B-named | acolyte | Ghost Howl (12); Hogger (30); Princess (12); Morgan the Collector (14); Narg the Taskmaster (14); The Eye Below (18) | Hogger: win 0% | win >= 80% | 80 points |
| B-named | ranger | Hogger (30); Princess (12); Morgan the Collector (14); Narg the Taskmaster (14); Gresh the Drowned (20); Speaker Thane (21); The Kept (22); Shadow of Morganth (22); The Eye Below (18); Ritualist Serane (18) | Hogger: win 0% | win >= 80% | 80 points |
| B-named | vanguard | Hogger (30); Princess (12); Morgan the Collector (14); Narg the Taskmaster (14); Shadow of Morganth (22); The Eye Below (18) | Hogger: win 0% | win >= 80% | 80 points |
| B-named | cutthroat | Morgan the Collector (14); Narg the Taskmaster (14) | Narg the Taskmaster: win 60% | win >= 80% | 20 points |
| G+5 | ranger | 15-30 | level 16: win 0% | win >= 75% | 75 points |
| G+5 | initiate | 15-30 | level 15: win 0% | win >= 75% | 75 points |
| G+5 | cutthroat | 16-30 | level 18: win 3% | win >= 75% | 72 points |
| G+5 | vanguard | 16-30 | level 16: win 0% | win >= 75% | 75 points |
| G+5 | acolyte | 16-30 | level 16: win 0% | win >= 75% | 75 points |
| G+5-named | ranger | Ghost Howl (17); Garrick Padfoot (15); Abbot Ashworth (15); Hogger (35); Princess (17); Morgan the Collector (19); Narg the Taskmaster (19); Gllrg the Tidecaller (20); Gresh the Drowned (25); Speaker Thane (26); The Kept (27); Shadow of Morganth (27); The Eye Below (23) | Hogger: win 0% | win >= 75% | 75 points |
| G+5-named | initiate | Ghost Howl (17); Garrick Padfoot (15); Abbot Ashworth (15); Hogger (35); Princess (17); Morgan the Collector (19); Narg the Taskmaster (19); Gllrg the Tidecaller (20); Gresh the Drowned (25); Speaker Thane (26); The Kept (27); Shadow of Morganth (27); The Eye Below (23); Ritualist Serane (23); Speaker Vael (29); Acolyte Marren (31) | Ghost Howl: win 0% | win >= 75% | 75 points |
| G+5-named | acolyte | Ghost Howl (17); Hogger (35); Princess (17); Morgan the Collector (19); Narg the Taskmaster (19) | Hogger: win 0% | win >= 75% | 75 points |
| G+5-named | cutthroat | Hogger (35); Princess (17); Morgan the Collector (19); Narg the Taskmaster (19) | Hogger: win 0% | win >= 75% | 75 points |
| G+5-named | vanguard | Hogger (35); Princess (17); Morgan the Collector (19); Narg the Taskmaster (19); Speaker Thane (26); Shadow of Morganth (27); The Eye Below (23) | Hogger: win 0% | win >= 75% | 75 points |

## Edits

1. `crates/twilight-realm/content/mobs.toml` `[difficulty]` `damage`: 3.0 to 1.35. Mob side, for P-2, B-2E, B-boss,
   G+5 and the named bosses: every Calling took about half its health from one on-level normal, and from 11 a
   second mob or a generic boss killed it. One factor moves every level at once, with no per-level patch
   (ADR 0012).
2. `mobs.toml` `[difficulty]` `damage_at_1`: 1.5 to 0.5. Mob side, for T0-1 and T0-2: Tier 0's mobs fade in from
   a third of the old hit.
3. `data/twilights-eve/callings.csv` `basic_warrior` `health_pct`: 100 to 350. Character side, for T0-1 and T0-2:
   the Basic Warrior's health grows from 220 to 332 while the mob's grows nine times.
4. `master.csv` `basic_warrior/fire_blast` `effect1_params`: `amount.base=20;amount.per_level=2.5` to
   `amount.base=50;amount.per_level=6.25` (x2.5). Character side, for T0-1 and T0-2.
5. `master.csv` `basic_warrior/gouge` `effect1_params`: `amount.base=10;amount.per_level=1.5` to
   `amount.base=25;amount.per_level=3.75` (x2.5). Character side, for T0-1 and T0-2.
6. `callings.csv` `initiate` `health_pct`: 100 to 350. Character side, for P-2, B-2E, B-boss and the named bosses:
   the Initiate had 357 to 753 health in greens and died in 6 to 8 s to one elite.
7. `master.csv` `initiate/ley_spark` `effect1_params`: `amount.base=31;amount.per_level=2.5` to
   `amount.base=62;amount.per_level=5` (x2, `crit_pct_at_full_mana=15` kept). Character side, for DPS: the
   Initiate's DPS was below both tanks' at every level.
8. `master.csv` `initiate/searing_ember` `effect1_params`: `amount.base=21.5;amount.per_level=1.5` to
   `amount.base=43;amount.per_level=3` (x2). Character side, for DPS.
9. `master.csv` `initiate/aura_searing_ember` `effect1_params`: `tick.base=2;tick.per_level=0.3` to
   `tick.base=4;tick.per_level=0.6` (x2, `tick_ms=3000` kept). Character side, for DPS.
10. `master.csv` `acolyte/blood_lance` `effect1_params`: `amount.base=36;amount.per_level=2` to
    `amount.base=25;amount.per_level=1.4` (x0.7). Character side, for DPS: the Acolyte out-damaged every damage
    Calling at 10, and with edit 11 it lives long enough to pass the Cutthroat at 13 too. After this edit only
    the level-13 Cutthroat (58.3 DPS) is still below it.
11. `callings.csv` `acolyte` `health_pct`: 100 to 150. Character side, for P-2 and B-2E (the tanks' survival).
12. `callings.csv` `vanguard` `health_pct`: 100 to 130. Character side, for E-tank and B-2E.
13. `callings.csv` `ranger` `health_pct`: 100 to 150. Character side, for P-2 and B-2E after Point Blank at 23.
14. `callings.csv` `cutthroat` `health_pct`: 100 to 70. Character side, for E-dmg and P-3: with mob damage halved
    the Cutthroat won every elite and three-mob pull.

No named boss template changed. With mob damage at 1.35 the generic boss and almost every named boss fall inside
B-named and G+5-named, so a template multiplier would only move the few that still miss, and those miss for
Calling reasons (the Ranger before 23, the Initiate against Hogger).

`diff -u` of root-a against the repo (`master.csv` cut to the changed cells; the generated JSON left out):

```diff
--- a/crates/twilight-realm/content/mobs.toml
+++ b/crates/twilight-realm/content/mobs.toml
@@ -18,6 +18,6 @@
 health = 3.2
-damage = 3.0
+damage = 1.35
 # Damage fades in: `damage_at_1` at level 1, climbing in a straight line to `damage` at
 # `fade_to`, so Tier 0's mobs hit about as they did before the Power Curve.
-damage_at_1 = 1.5
+damage_at_1 = 0.5
 fade_to = 10
--- a/data/twilights-eve/callings.csv
+++ b/data/twilights-eve/callings.csv
@@ -1,8 +1,8 @@
 calling,name,icon,power,stat_profile,item_class,health_pct,mana_pct,energy_per_tick,bleed_drop,bleed_drop_chance,armor,weapons,spellbook,blurb
-basic_warrior,Basic Warrior,Interface\Icons\Ability_Rogue_Ambush,rage,warrior,warrior,100,100,,,,cloth|leather|mail|shield,one_hand_axe|two_hand_axe|bow|gun|one_hand_mace|two_hand_mace|polearm|one_hand_sword|two_hand_sword|staff|fist|misc|dagger|thrown|crossbow|fishing_pole,Basic Warrior,
-vanguard,Vanguard,Interface\Icons\Ability_Rogue_Ambush,rage,warrior,warrior,100,100,,,,cloth|leather|mail|plate|shield,one_hand_axe|two_hand_axe|bow|gun|one_hand_mace|two_hand_mace|polearm|one_hand_sword|two_hand_sword|staff|fist|misc|dagger|thrown|crossbow|fishing_pole,Vanguard,"Holds the front of the line in plate. Takes the hits, keeps foes on you, and hits back hard."
+basic_warrior,Basic Warrior,Interface\Icons\Ability_Rogue_Ambush,rage,warrior,warrior,350,100,,,,cloth|leather|mail|shield,one_hand_axe|two_hand_axe|bow|gun|one_hand_mace|two_hand_mace|polearm|one_hand_sword|two_hand_sword|staff|fist|misc|dagger|thrown|crossbow|fishing_pole,Basic Warrior,
+vanguard,Vanguard,Interface\Icons\Ability_Rogue_Ambush,rage,warrior,warrior,130,100,,,,cloth|leather|mail|plate|shield,one_hand_axe|two_hand_axe|bow|gun|one_hand_mace|two_hand_mace|polearm|one_hand_sword|two_hand_sword|staff|fist|misc|dagger|thrown|crossbow|fishing_pole,Vanguard,"Holds the front of the line in plate. Takes the hits, keeps foes on you, and hits back hard."
 zealot,Zealot,Interface\Icons\Ability_Warrior_InnerRage,rage,warrior,warrior,100,100,,,,,one_hand_axe|two_hand_axe|bow|gun|one_hand_mace|two_hand_mace|polearm|one_hand_sword|two_hand_sword|staff|fist|misc|dagger|thrown|crossbow|fishing_pole,Zealot,
-cutthroat,Cutthroat,Interface\Icons\Spell_Shadow_RitualOfSacrifice,energy,rogue,warrior,100,100,25,aura_blood_droplet,50,cloth|leather,one_hand_axe|two_hand_axe|bow|gun|one_hand_mace|two_hand_mace|polearm|one_hand_sword|two_hand_sword|staff|fist|misc|dagger|thrown|crossbow|fishing_pole|dual_wield,Cutthroat,"Strikes from the shadows and opens wounds that bleed. Stepping in the spilled blood drives them into a faster, harder Bloodlust."
-initiate,Initiate,Interface\Icons\Spell_Arcane_StarFire,mana,mage,warrior,100,100,,,,cloth,one_hand_axe|two_hand_axe|bow|gun|one_hand_mace|two_hand_mace|polearm|one_hand_sword|two_hand_sword|staff|fist|misc|dagger|thrown|crossbow|fishing_pole,Initiate,"Strikes from afar with fire and frost, then shatters what is chilled. Fragile up close."
-ranger,Ranger,Interface\Icons\Ability_Whirlwind,mana,hunter,warrior,100,100,,,,leather|mail,one_hand_axe|two_hand_axe|bow|gun|one_hand_mace|two_hand_mace|polearm|one_hand_sword|two_hand_sword|staff|fist|misc|dagger|thrown|crossbow|fishing_pole,Ranger,Fights with the bow and keeps the foe at a distance. Every step away is safety.
-acolyte,Acolyte,Interface\Icons\Spell_Shadow_ShadowWordPain,mana,priest,warrior,100,100,,,,cloth,one_hand_axe|two_hand_axe|bow|gun|one_hand_mace|two_hand_mace|polearm|one_hand_sword|two_hand_sword|staff|fist|misc|dagger|thrown|crossbow|fishing_pole,Acolyte,Rots foes and drinks their life to mend. Spends their own blood when mana runs dry.
+cutthroat,Cutthroat,Interface\Icons\Spell_Shadow_RitualOfSacrifice,energy,rogue,warrior,70,100,25,aura_blood_droplet,50,cloth|leather,one_hand_axe|two_hand_axe|bow|gun|one_hand_mace|two_hand_mace|polearm|one_hand_sword|two_hand_sword|staff|fist|misc|dagger|thrown|crossbow|fishing_pole|dual_wield,Cutthroat,"Strikes from the shadows and opens wounds that bleed. Stepping in the spilled blood drives them into a faster, harder Bloodlust."
+initiate,Initiate,Interface\Icons\Spell_Arcane_StarFire,mana,mage,warrior,350,100,,,,cloth,one_hand_axe|two_hand_axe|bow|gun|one_hand_mace|two_hand_mace|polearm|one_hand_sword|two_hand_sword|staff|fist|misc|dagger|thrown|crossbow|fishing_pole,Initiate,"Strikes from afar with fire and frost, then shatters what is chilled. Fragile up close."
+ranger,Ranger,Interface\Icons\Ability_Whirlwind,mana,hunter,warrior,150,100,,,,leather|mail,one_hand_axe|two_hand_axe|bow|gun|one_hand_mace|two_hand_mace|polearm|one_hand_sword|two_hand_sword|staff|fist|misc|dagger|thrown|crossbow|fishing_pole,Ranger,Fights with the bow and keeps the foe at a distance. Every step away is safety.
+acolyte,Acolyte,Interface\Icons\Spell_Shadow_ShadowWordPain,mana,priest,warrior,150,100,,,,cloth,one_hand_axe|two_hand_axe|bow|gun|one_hand_mace|two_hand_mace|polearm|one_hand_sword|two_hand_sword|staff|fist|misc|dagger|thrown|crossbow|fishing_pole,Acolyte,Rots foes and drinks their life to mend. Spends their own blood when mana runs dry.
--- a/data/twilights-eve/master.csv (changed cells only)
+++ b/data/twilights-eve/master.csv (changed cells only)
@@ basic_warrior/gouge effect1_params @@
-amount.base=10;amount.per_level=1.5
+amount.base=25;amount.per_level=3.75
@@ basic_warrior/fire_blast effect1_params @@
-amount.base=20;amount.per_level=2.5
+amount.base=50;amount.per_level=6.25
@@ initiate/ley_spark effect1_params @@
-amount.base=31;amount.per_level=2.5;crit_pct_at_full_mana=15
+amount.base=62;amount.per_level=5;crit_pct_at_full_mana=15
@@ initiate/searing_ember effect1_params @@
-amount.base=21.5;amount.per_level=1.5
+amount.base=43;amount.per_level=3
@@ initiate/aura_searing_ember effect1_params @@
-tick_ms=3000;tick.base=2;tick.per_level=0.3
+tick_ms=3000;tick.base=4;tick.per_level=0.6
@@ acolyte/blood_lance effect1_params @@
-amount.base=36;amount.per_level=2
+amount.base=25;amount.per_level=1.4
```

## After

`after.md`, each rule's table as one line: cells that hold, cells that miss, the worst miss.

- T0-1: 9 hold, 0 miss.
- T0-2: 9 hold, 0 miss.
- P-2: 82 hold, 23 miss; worst ranger level 12, win 0%.
- P-3: 17 hold, 88 miss; worst cutthroat level 10, win 100%.
- DPS: 49 hold, 14 miss; worst ranger level 22, 49.6 DPS against 79.3.
- E-dmg: 0 hold, 63 miss; worst cutthroat level 10, win 100%.
- E-tank: 2 hold, 40 miss; worst vanguard level 10, win 100%.
- B-2E: 51 hold, 54 miss; worst ranger level 12, win 0%.
- B-boss: 93 hold, 12 miss; worst ranger level 14, win 0%.
- B-named: 93 hold, 7 miss; worst initiate Hogger at 30, win 0%.
- G+5: 66 hold, 14 miss; worst ranger level 17, win 0%.
- G+5-named: 92 hold, 8 miss; worst ranger Hogger at 35, win 0%.

Total: 323 of 991 cells fail, down from 676.

## Still failing

- E-dmg and E-tank (103 cells). After edit 1 the elite is too easy for the Cutthroat (89-100% at every level) and
  the Initiate (97-100%), and the Ranger wins 0-19% from 11 to 22 and 98-100% from 23. The tanks win 83-100%,
  but the Initiate or the Cutthroat ties or beats them, so "above every damage Calling" fails. A harder elite
  does not fix it: root-b's 3.2/1.25 and the probe's 2.6/1.8 and 4.0/1.4 all drop the Vanguard faster than the
  Cutthroat (root-b: E-dmg 4 hold, E-tank 8, but B-boss and G+5 lose 20 cells). The ordering needs the tanks to
  gain more from a long single fight than the damage Callings do. That is a kit question (a defensive talent in
  the tanks' damage build, block that scales, or a Cutthroat Evasion that covers less), and Evasion is a constant
  in the model, not read from the data (Model gaps).
- P-3 (88 cells). Three normals stay at 0-10% for every Calling but the Cutthroat from 11 up, while two normals
  hold at 80% or more. The band needs one normal to cost about a sixth of the character's health (three mobs cost
  about six times one), so every Calling would have to keep about 84% health against one normal at every level.
  The Callings' health left against one normal moves 15 points between levels with the green set they wear (the
  Cutthroat 72% at 17 and 94% at 29), so one factor cannot put all five in a 35-point band. The lever is the gear
  curve's shape per level (`build_gear.py` `BUDGET`), not tried tonight, or an area-of-effect skill for the
  casters.
- The Ranger before 23 (P-2, DPS, B-2E, B-boss, G+5, most of its 21 misses each). It fights in melee with a staff
  until Point Blank at 23 removes the minimum range (`world/cast.rs` `no_min_range`). Moving Point Blank to row 2
  in the probe did not help: the damage build spends its points on the other talents first, so it arrives at 21.
  The fix is a mechanic: shots usable in melee from 10, or a kiting rotation in the model.
- The Initiate and the Acolyte against two elites and against pulls (B-2E 20 and 16 cells, P-2 10 for the
  Initiate). Both fight a pull one target at a time with no area spell in the model's rotation, so the second mob
  swings for the whole fight. The Acolyte's 2-mob fights are bimodal (it wins with about 70% health left or
  dies). This is a rotation change, or Ember Blast and Miasma used in the model.
- Hogger (8 cells: 3 of B-named's 7 and 5 of G+5-named's 8). He is the one boss above his generic neighbour
  (3.0/1.5). After the edits the Cutthroat and the Acolyte beat him in blues at 30, the Vanguard wins 59%, the
  Ranger 48% and the Initiate 0%. At 35 in greens every Calling loses (0-21%), because the character keeps its
  level-30 kit and talents there. Lowering his template to 2.4/1.2 is the next thing to try, once Eat and Rushing
  Charge are modelled, and G+5-named at 35 waits on the level table above 30.

## What changes for the player

- A level-20 Cutthroat in greens against one Elwynn mob: the kill still takes 22 s, and it ends the fight with
  80% health instead of 59%.
- A level-20 Vanguard in greens against one elite: before, it died every fight; after, it wins 93% in a 57 s
  fight and ends at 32% health.
- Abbot Ashworth (10) in blues: every Calling wins every fight. Before, the Initiate won 3%. The Vanguard ends at
  95% health instead of 62%.
- Hogger (30) in blues: before, only the Cutthroat beat him (96%). After, the Cutthroat and the Acolyte win
  every fight, the Vanguard 59% and the Ranger 48%; the Initiate still loses every fight.
- A level-9 Basic Warrior against one Northshire mob wins every fight (3% before) and against two wins 98% (0%).
- The Initiate has three and a half times its base health and hits twice as hard. In play it would feel like a
  different Calling, and Davis should judge that before the numbers land.

## Model gaps

- `model.build_stats` ignores `callings.csv` `health_pct` and `mana_pct`; the realm applies both
  (`equipment.rs` calls `scale_base_pools`). Edits 3 and 6 and 11 to 14 depend on the one-line fix the scratch copy
  carries. That fix should land in `tools/twilights-eve/balance/model.py` before any of these edits do.
- Evasion's 50% dodge and 15 s are constants in `model.py` (`evasion_until`, `return 50`), not read from
  `aura_evasion`, so a data edit to Evasion would not show in the model.
- The probe's first skill scaling missed Heroic Strike: its damage sits in `bonus.base` and `bonus.per_level`, not
  `amount`. Edits 4 and 5 carry Tier 0 without it.
- Boss spells the model skips: Geomancer Vurx's frost_armor_vurx and frost_chill_vurx, the Eye Below's and Speaker
  Thane's eye_mind_flay, Gresh's drowning_grasp, the Shadow of Morganth's terrify_creature, the Kept's
  void_shield_creature, and Hogger's eat and rushing_charge.
- Scythemaw, the Rake and Ghost Howl spawn only in `barrens.toml`, where the realm applies no Zone Factor; the
  model fights them at Elwynn's.
- The kit picker gives the casters stock vanilla chests with almost no armour (Bright Armor, Bright Robe,
  Silver-thread Robe) over the Loot Band robes, because their weights rank intellect far above armour and stamina
  (#485). Every caster reading from 22 up carries that.
- The Ranger and the Initiate's talent picks come from a fixed wish list in the model, so a talent moved to an
  earlier row still arrives late (the Point Blank test above).
- The level table above 30 is provisional (ADR 0012), which makes Acolyte Marren at 31 and Hogger at 35 in
  G+5-named suspect.
