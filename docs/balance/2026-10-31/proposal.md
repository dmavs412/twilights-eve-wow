# Balance check 4: proposed edits with Talent Rows set (model 2026-10-31)

Issue #530, the last ticket of #510. Rules: `docs/balance/rules.md`. Beside this file: `before.md` (the repo's data),
`root-497.md` (#497's 14 edits) and `after.md` (the edits below). Nothing here has landed in `data/` or `crates/`.
Davis picks.

Every Tier 1 Calling now fights with each open Talent Row set to its most damaging choice. On the repo's data 672
cells fail. #497's edits, built on a model with no talents, take that to 320. The edits below take it to 222. Tier
0, P-2, DPS, B-boss, B-named, G+5 and G+5-named hold almost everywhere. P-3, E-dmg, E-tank and B-2E do not, and
none of the four is a number away (see Still failing).

## How it was run

- Step 0 landed first, in its own commit: `model.build_stats` now keeps `callings.csv` `health_pct` and `mana_pct`
  percent of the base pools, as `scale_base_pools` does in the realm. Every Calling is at 100 in the repo, so
  `before.md` reads the same as it would have without the fix.
- `before.md`:
  `python3.15 tools/twilights-eve/balance/run.py --scenario all --levels 1-30 --fights 300 --callings all --jobs 4 --out docs/balance/2026-10-31/before.md`.
- Every candidate is a scratch data root in the session's scratchpad, built by `target/agent/build_root530.py`
  (not committed). It copies `data/twilights-eve/` (without `art/`) and `crates/twilight-realm/content/`, edits
  `mobs.toml` and `callings.csv` by text, edits `master.csv` through `tools/twilights-eve/sheet.py --csv <root>/...
  set`, and regenerates `server_spells.json` and `client_dbc_patch.json` with `build_spells.py --csv ... --callings
  ... --creature-spells ... --content ... --out-dir ... --no-dbc-check`. It stops when an edit's old value is not
  what the copy holds, so every old value below is the repo's.
- Each root ran the full command above with `--data-root <root>`, 300 fights a cell:

  | Root | Edits | Failing cells |
  |---|---|---|
  | repo | none | 672 |
  | root-497 | #497's 14 edits | 320 |
  | root-a | root-497; edit 10's Lance cut carried to Exsanguinate and Splintering Lance; Ranger 250, Acolyte 250 and Cutthroat 55 health; Snapshot, Twin Arrows and Adrenaline Rush bonus x2 | 268 |
  | root-b | root-a; Hogger 2.4/1.2; Initiate 500, Acolyte 350 and Ranger 350 health; Creeping Blight's tick x0.75 | 240 |
  | root-c | root-b without the Blight cut; the Snapshot bonus x2.5; Acolyte 500 and Cutthroat 45 health | 237 |
  | root-d | root-c without the Snapshot edits; Puncturing Shaft, Piercing Force and Killshot bonus x1.4; Acolyte 250 health and 300 mana; Initiate 200 mana | 222 |
  | root-e | root-d without the Acolyte's mana | 222 |

- What the roots showed. Blight's cut lowered the Acolyte's healing more than its DPS: its B-2E at 11 fell from 70%
  to 54%, so the cut came out. The Snapshot edits did nothing from 12 to 17, because there the Ranger spends its
  global cooldowns on Puncturing Shaft (70% of its damage at 14), so the Shaft family replaced them. The Acolyte
  missed B-2E at the same 16 levels at 250% health, at 500% and with 300% mana, so its mana edit came out. root-e is
  the proposal, and `after.md` is its run.
- The search stopped at root-e, before two hours of reruns. Every group still failing needs a mechanic, a rotation
  change in the model or a rule change (see Still failing).

## Before

The failures in `before.md`, one row per rule and Calling.

672 cells miss: T0-1 3, T0-2 5, P-2 97, P-3 105, DPS 36, E-dmg 56, E-tank 42, B-2E 90, B-boss 88, B-named 34, G+5 76, G+5-named 40.

| Rule | Calling | Levels or bosses that miss | Worst reading | Band | Worst gap |
|---|---|---|---|---|---|
| T0-1 | basic_warrior | 7-9 | level 9: win 3% | win >= 97% | 94 points |
| T0-2 | basic_warrior | 5-9 | level 6: win 0% | win >= 90% | 90 points |
| P-2 | ranger | 10-30 | level 10: win 0%, 2% left | win >= 80%, health left >= 20% | 80 points |
| P-2 | initiate | 10-30 | level 10: win 0% | win >= 80%, health left >= 20% | 80 points |
| P-2 | vanguard | 11-30 | level 12: win 0%, 3% left | win >= 80%, health left >= 20% | 80 points |
| P-2 | acolyte | 11-30 | level 11: win 0% | win >= 80%, health left >= 20% | 80 points |
| P-2 | cutthroat | 13-14, 16-22, 24-26, 28-30 | level 18: win 35%, 20% left | win >= 80%, health left >= 20% | 45 points |
| P-3 | cutthroat | 10-30 | level 10: win 70% | win 15% to 50% | 20 points |
| P-3 | ranger | 10-30 | level 10: win 0% | win 15% to 50% | 15 points |
| P-3 | initiate | 10-30 | level 10: win 0% | win 15% to 50% | 15 points |
| P-3 | vanguard | 10-30 | level 11: win 0% | win 15% to 50% | 15 points |
| P-3 | acolyte | 10-30 | level 10: win 0% | win 15% to 50% | 15 points |
| DPS | initiate | 10-30 | level 30: 103.0 DPS against 149.3 | above both tanks' DPS | 46.3 DPS |
| DPS | ranger | 10-16, 19-20, 26-30 | level 10: 37.4 DPS against 61.5 | above both tanks' DPS | 24.1 DPS |
| DPS | cutthroat | 10 | level 10: 55.0 DPS against 61.5 | above both tanks' DPS | 6.5 DPS |
| E-dmg | ranger | 10-30 | level 11: win 0% | win 35% to 65% | 35 points |
| E-dmg | initiate | 10-30 | level 11: win 0% | win 35% to 65% | 35 points |
| E-dmg | cutthroat | 10-13, 15, 18, 20-21, 23, 26-30 | level 10: win 100% | win 35% to 65% | 35 points |
| E-tank | vanguard | 10-30 | level 11: win 0% (damage best 87%) | win >= 70% and above every damage Calling | 88 points |
| E-tank | acolyte | 10-30 | level 23: win 0% (damage best 85%) | win >= 70% and above every damage Calling | 86 points |
| B-2E | ranger | 10-30 | level 11: win 0% | win >= 80% | 80 points |
| B-2E | initiate | 10-30 | level 10: win 0% | win >= 80% | 80 points |
| B-2E | vanguard | 10-30 | level 11: win 0% | win >= 80% | 80 points |
| B-2E | acolyte | 10-30 | level 10: win 0% | win >= 80% | 80 points |
| B-2E | cutthroat | 13-14, 24-26, 30 | level 14: win 14% | win >= 80% | 66 points |
| B-boss | ranger | 10-30 | level 11: win 0% | win >= 80% | 80 points |
| B-boss | initiate | 10-30 | level 10: win 0% | win >= 80% | 80 points |
| B-boss | vanguard | 11-30 | level 11: win 0% | win >= 80% | 80 points |
| B-boss | acolyte | 11-30 | level 11: win 0% | win >= 80% | 80 points |
| B-boss | cutthroat | 13-14, 24-26, 30 | level 26: win 40% | win >= 80% | 40 points |
| B-named | initiate | Ghost Howl (12); The Rake (10); Garrick Padfoot (10); Abbot Ashworth (10); Hogger (30); Princess (12); Morgan the Collector (14); Narg the Taskmaster (14); Speaker Thane (21); The Kept (22); Shadow of Morganth (22); The Eye Below (18); Ritualist Serane (18) | Hogger: win 0% | win >= 80% | 80 points |
| B-named | ranger | Hogger (30); Princess (12); Morgan the Collector (14); Narg the Taskmaster (14); The Kept (22); Shadow of Morganth (22); The Eye Below (18) | Hogger: win 0% | win >= 80% | 80 points |
| B-named | acolyte | Ghost Howl (12); Hogger (30); Princess (12); Morgan the Collector (14); Narg the Taskmaster (14); The Eye Below (18) | Hogger: win 0% | win >= 80% | 80 points |
| B-named | vanguard | Hogger (30); Princess (12); Morgan the Collector (14); Narg the Taskmaster (14); The Eye Below (18) | Hogger: win 0% | win >= 80% | 80 points |
| B-named | cutthroat | Hogger (30); Morgan the Collector (14); Narg the Taskmaster (14) | Hogger: win 2% | win >= 80% | 78 points |
| G+5 | initiate | 15-30 | level 16: win 0% | win >= 75% | 75 points |
| G+5 | cutthroat | 16-30 | level 30: win 0% | win >= 75% | 75 points |
| G+5 | ranger | 16-30 | level 16: win 0% | win >= 75% | 75 points |
| G+5 | vanguard | 16-30 | level 17: win 0% | win >= 75% | 75 points |
| G+5 | acolyte | 16-30 | level 16: win 0% | win >= 75% | 75 points |
| G+5-named | initiate | Ghost Howl (17); Garrick Padfoot (15); Hogger (35); Princess (17); Morgan the Collector (19); Narg the Taskmaster (19); Gllrg the Tidecaller (20); Gresh the Drowned (25); Speaker Thane (26); The Kept (27); Shadow of Morganth (27); The Eye Below (23); Ritualist Serane (23); Speaker Vael (29); Acolyte Marren (31) | Ghost Howl: win 0% | win >= 75% | 75 points |
| G+5-named | ranger | Hogger (35); Princess (17); Morgan the Collector (19); Narg the Taskmaster (19); Gresh the Drowned (25); Speaker Thane (26); The Kept (27); Shadow of Morganth (27); The Eye Below (23) | Hogger: win 0% | win >= 75% | 75 points |
| G+5-named | cutthroat | Hogger (35); Princess (17); Morgan the Collector (19); Narg the Taskmaster (19); Shadow of Morganth (27); The Eye Below (23) | Hogger: win 0% | win >= 75% | 75 points |
| G+5-named | acolyte | Ghost Howl (17); Hogger (35); Princess (17); Morgan the Collector (19); Narg the Taskmaster (19) | Hogger: win 0% | win >= 75% | 75 points |
| G+5-named | vanguard | Hogger (35); Princess (17); Morgan the Collector (19); Narg the Taskmaster (19); The Eye Below (23) | Hogger: win 0% | win >= 75% | 75 points |

## Edits

Fourteen of the 22 are #497's edits. Ten keep #497's value. Four change it: the Initiate's, the Acolyte's, the
Ranger's and the Cutthroat's `health_pct`. Eight are new.

1. `crates/twilight-realm/content/mobs.toml` `[difficulty]` `damage`: 3.0 to 1.35 (#497 edit 1). Mob side, for P-2,
   B-2E, B-boss, G+5 and the named bosses. One factor moves every level at once (ADR 0012).
2. `mobs.toml` `[difficulty]` `damage_at_1`: 1.5 to 0.5 (#497 edit 2). Mob side, for T0-1 and T0-2.
3. `mobs.toml` Hogger (96010) `health_multiplier`: 3.0 to 2.4. Mob side, for B-named and G+5-named. On root-a
   the Ranger beat him in blues at 30 in 38% of fights, the Acolyte 18% and the Initiate 0%.
4. `mobs.toml` Hogger (96010) `damage_multiplier`: 1.5 to 1.2. Mob side, for B-named and G+5-named.
5. `data/twilights-eve/callings.csv` `basic_warrior` `health_pct`: 100 to 350 (#497 edit 3). Character side, for
   T0-1 and T0-2.
6. `master.csv` `basic_warrior/fire_blast` `effect1_params`: `amount.base=20;amount.per_level=2.5` to
   `amount.base=50;amount.per_level=6.25` (#497 edit 4). Character side, for T0-1 and T0-2.
7. `master.csv` `basic_warrior/gouge` `effect1_params`: `amount.base=10;amount.per_level=1.5` to
   `amount.base=25;amount.per_level=3.75` (#497 edit 5). Character side, for T0-1 and T0-2.
8. `callings.csv` `initiate` `health_pct`: 100 to 500 (#497 edit 6 had 350). Character side, for B-2E. At 350 the
   Initiate missed B-2E at 19 levels and at 500 it misses at 9.
9. `callings.csv` `initiate` `mana_pct`: 100 to 200. Character side, for DPS and B-2E. Ley Ward absorbs 10% of the
   maximum mana. The edit lifts the Initiate's DPS at 29 from 142 to 146, above the Vanguard's 143, and its win
   against Hogger at 35.
10. `master.csv` `initiate/ley_spark` `effect1_params`: `amount.base=31;amount.per_level=2.5;crit_pct_at_full_mana=15`
    to `amount.base=62;amount.per_level=5;crit_pct_at_full_mana=15` (#497 edit 7). Character side, for DPS.
11. `master.csv` `initiate/searing_ember` `effect1_params`: `amount.base=21.5;amount.per_level=1.5` to
    `amount.base=43;amount.per_level=3` (#497 edit 8). Character side, for DPS.
12. `master.csv` `initiate/aura_searing_ember` `effect1_params`: `tick_ms=3000;tick.base=2;tick.per_level=0.3` to
    `tick_ms=3000;tick.base=4;tick.per_level=0.6` (#497 edit 9). Character side, for DPS.
13. `master.csv` `acolyte/blood_lance` `effect1_params`: `amount.base=36;amount.per_level=2` to
    `amount.base=25;amount.per_level=1.4` (#497 edit 10). Character side, for DPS.
14. `master.csv` `acolyte/exsanguinate` `effect1_params`: `amount.base=36;amount.per_level=2;...` to
    `amount.base=25;amount.per_level=1.4;...`, the `vs_aura` cells kept. Character side, for DPS. Exsanguinate is
    the Lance row's pick at most levels from 15, and it carries its own copy of Blood Lance's damage, so #497's
    edit 10 missed every level where it is set. On #497's root the Acolyte's DPS at 18 to 20 was above every
    damage Calling's.
15. `master.csv` `acolyte/splintering_lance` `effect1_params`: `amount.base=36;amount.per_level=2;...` to
    `amount.base=25;amount.per_level=1.4;...`, the splash cells kept. Character side, for DPS. It is the row's
    third choice. Left at 36/2 it would out-damage the other two and become the pick.
16. `callings.csv` `acolyte` `health_pct`: 100 to 250 (#497 edit 11 had 150). Character side, for P-2 and B-2E, to
    carry edits 14 and 15. The Lance heals the Acolyte for a share of its hit.
17. `callings.csv` `vanguard` `health_pct`: 100 to 130 (#497 edit 12). Character side, for B-2E and E-tank.
18. `callings.csv` `ranger` `health_pct`: 100 to 350 (#497 edit 13 had 150). Character side, for P-2 and B-2E. At
    150 the Ranger missed P-2 at 17 levels and B-2E at 16. At 250 it missed 3 and 9, and at 350 it misses 1 and 1.
19. `master.csv` `ranger/puncturing_shaft` `effect1_params`: `bonus.base=45;bonus.per_level=5` to
    `bonus.base=63;bonus.per_level=7` (x1.4), the other cells kept. Character side, for DPS. On root-c the
    Ranger was 0 to 5 DPS below the Acolyte from 12 to 17 and at 20.
20. `master.csv` `ranger/piercing_force` `effect1_params`: the same bonus change, the other cells kept. Character
    side, for DPS. It is a copy of the Shaft in the Shaft row.
21. `master.csv` `ranger/killshot` `effect1_params`: the same bonus change, the other cells kept. Character side,
    for DPS. It is the Shaft row's pick from 23.
22. `callings.csv` `cutthroat` `health_pct`: 100 to 45 (#497 edit 14 had 70). Character side, for P-3 and E-dmg. At
    70 and at 55 the Cutthroat missed P-3 at 14 levels and at 45 it misses at 12. It costs G+5 at 30 (78% to 73%).

`diff -u` of root-e against the repo (`master.csv` cut to the changed cells, the generated JSON left out):

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
@@ -906,4 +906,4 @@
 level_max = 30
-health_multiplier = 3.0
-damage_multiplier = 1.5
+health_multiplier = 2.4
+damage_multiplier = 1.2
 armor_multiplier = 1.0
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
+cutthroat,Cutthroat,Interface\Icons\Spell_Shadow_RitualOfSacrifice,energy,rogue,warrior,45,100,25,aura_blood_droplet,50,cloth|leather,one_hand_axe|two_hand_axe|bow|gun|one_hand_mace|two_hand_mace|polearm|one_hand_sword|two_hand_sword|staff|fist|misc|dagger|thrown|crossbow|fishing_pole|dual_wield,Cutthroat,"Strikes from the shadows and opens wounds that bleed. Stepping in the spilled blood drives them into a faster, harder Bloodlust."
+initiate,Initiate,Interface\Icons\Spell_Arcane_StarFire,mana,mage,warrior,500,200,,,,cloth,one_hand_axe|two_hand_axe|bow|gun|one_hand_mace|two_hand_mace|polearm|one_hand_sword|two_hand_sword|staff|fist|misc|dagger|thrown|crossbow|fishing_pole,Initiate,"Strikes from afar with fire and frost, then shatters what is chilled. Fragile up close."
+ranger,Ranger,Interface\Icons\Ability_Whirlwind,mana,hunter,warrior,350,100,,,,leather|mail,one_hand_axe|two_hand_axe|bow|gun|one_hand_mace|two_hand_mace|polearm|one_hand_sword|two_hand_sword|staff|fist|misc|dagger|thrown|crossbow|fishing_pole,Ranger,Fights with the bow and keeps the foe at a distance. Every step away is safety.
+acolyte,Acolyte,Interface\Icons\Spell_Shadow_ShadowWordPain,mana,priest,warrior,250,100,,,,cloth,one_hand_axe|two_hand_axe|bow|gun|one_hand_mace|two_hand_mace|polearm|one_hand_sword|two_hand_sword|staff|fist|misc|dagger|thrown|crossbow|fishing_pole,Acolyte,Rots foes and drinks their life to mend. Spends their own blood when mana runs dry.
--- a/data/twilights-eve/master.csv (changed cells only)
+++ b/data/twilights-eve/master.csv (changed cells only)
@@ basic_warrior/fire_blast effect1_params @@
-amount.base=20;amount.per_level=2.5
+amount.base=50;amount.per_level=6.25
@@ basic_warrior/gouge effect1_params @@
-amount.base=10;amount.per_level=1.5
+amount.base=25;amount.per_level=3.75
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
@@ acolyte/exsanguinate effect1_params @@
-amount.base=36;amount.per_level=2;vs_aura=aura_creeping_blight;vs_aura_pct=5;vs_aura_per_stack=yes;vs_aura_max_pct=50
+amount.base=25;amount.per_level=1.4;vs_aura=aura_creeping_blight;vs_aura_pct=5;vs_aura_per_stack=yes;vs_aura_max_pct=50
@@ acolyte/splintering_lance effect1_params @@
-amount.base=36;amount.per_level=2;splash_pct=30;splash_yd=8
+amount.base=25;amount.per_level=1.4;splash_pct=30;splash_yd=8
@@ ranger/puncturing_shaft effect1_params @@
-weapon=ranged;bonus.base=45;bonus.per_level=5;bonus.per_ap=0;armor_ignore.base=100;armor_ignore.per_level=10
+weapon=ranged;bonus.base=63;bonus.per_level=7;bonus.per_ap=0;armor_ignore.base=100;armor_ignore.per_level=10
@@ ranger/piercing_force effect1_params @@
-weapon=ranged;bonus.base=45;bonus.per_level=5;bonus.per_ap=0;armor_ignore.base=350;armor_ignore.per_level=10;crit_damage_pct=25
+weapon=ranged;bonus.base=63;bonus.per_level=7;bonus.per_ap=0;armor_ignore.base=350;armor_ignore.per_level=10;crit_damage_pct=25
@@ ranger/killshot effect1_params @@
-weapon=ranged;bonus.base=45;bonus.per_level=5;bonus.per_ap=0;armor_ignore.base=100;armor_ignore.per_level=10;below_health_pct=30;below_instant=yes
+weapon=ranged;bonus.base=63;bonus.per_level=7;bonus.per_ap=0;armor_ignore.base=100;armor_ignore.per_level=10;below_health_pct=30;below_instant=yes
```

## After

`after.md`, each rule's table as one line: cells that hold, cells that miss, the worst miss.

- T0-1: 9 hold, 0 miss.
- T0-2: 9 hold, 0 miss.
- P-2: 104 hold, 1 miss; worst ranger level 11, win 73%, 17% left.
- P-3: 25 hold, 80 miss; worst cutthroat level 10, win 100%.
- DPS: 61 hold, 2 miss; worst ranger level 11, 33.6 DPS against 55.6.
- E-dmg: 0 hold, 63 miss; worst cutthroat level 10, win 100%.
- E-tank: 0 hold, 42 miss; worst vanguard level 14, win 81% (damage best 100%).
- B-2E: 76 hold, 29 miss; worst acolyte level 14, win 1%.
- B-boss: 104 hold, 1 miss; worst vanguard level 14, win 39%.
- B-named: 100 hold, 0 miss.
- G+5: 79 hold, 1 miss; worst cutthroat level 30, win 73%.
- G+5-named: 97 hold, 3 miss; worst cutthroat Hogger at 35, win 0%.

Total: 222 cells fail, down from 672 on the repo and 320 on #497's root. The DPS and E-tank tables also show 105
comparison rows with no verdict of their own, which is why `run.py` counts 991 cells.

## Still failing

- E-dmg and E-tank (105 cells). One elite in greens is too easy: the Cutthroat wins 93-100%, and the Ranger and the
  Initiate win 91-100% at every level. Tightening it would break B-2E. On #497's root the Ranger won 82-100% against
  one elite in greens and 2-93% against two in blues, so the fight E-dmg wants lost a third of the time is already
  easier than the fight B-2E wants won 80% of the time. An elite that brings a damage Calling down to 35-65% costs
  about as much as three normals (P-3's band), and then two of them in blues kill every Calling. Blues add about
  double the green stat budget, and two elites cost about three times one. E-tank's first half holds (the Vanguard
  wins 81-100%, the Acolyte 96-100%). Its second half, above every damage Calling, cannot hold while those win
  every fight. This needs Davis: move E-dmg's band, B-2E's floor, or the gear either rule fights in.
- P-3 (80 cells). Against three normals the Acolyte wins 0-11% from 14 up, the Initiate 0-18% from 16 and the
  Ranger 0-54%, while the Vanguard wins 55-100% from 22. The Cutthroat swings between 13% and 100%. The model's casters fight a pull one
  target at a time, and every Calling's reading moves 30 points or more between levels with its gear set (the
  Vanguard 5% at 13, 69% at 15). The levers are the gear curve's shape per level (`build_gear.py` `BUDGET`) and
  area skills in the model's rotations, not a factor.
- B-2E (29 cells). The Acolyte misses at 11-25 and 30. On root-c its fights against two elites at 13 to 17 ended
  in its death after about 170 s at 42-57 DPS, and neither 500% health nor 300% mana turned a miss into a hold. It holds from 26, when Miasma's row
  opens, so the lever is Miasma or another area skill earlier in its kit, or in the model's rotation. The
  Initiate misses at 9 levels for the same reason. The Vanguard misses at 12-14 (56%, 18%, 2%) and holds at 15
  (100%), and B-boss misses at 14 (39%), so its blues there are a step behind the mobs, a gear-curve point.
- The Ranger at 10 and 11 (DPS 2 cells, P-2 1 cell). Point Blank's row opens at 12. Before it the Ranger cannot
  shoot inside 8 yd and fights the pull with its staff (on root-c white swings were 52% of its damage at 11). The
  fix is a mechanic: shots in melee from 10, or Point Blank's row at 10.
- Hogger at 35 (G+5-named, 3 cells). The Cutthroat, the Ranger and the Vanguard still miss there (the Cutthroat
  wins none), at their level-30 kits and talents on the provisional level table above 30. B-named Hogger at 30 holds for all five.
- G+5 Cutthroat at 30 (1 cell, 73% against 75%): edit 22's cost.

## What changes for the player

- A level-20 Cutthroat in greens against one Elwynn mob: the kill still takes 19 s, and it ends the fight with 75%
  health instead of 59%.
- A level-20 Vanguard in greens against one elite: before, it died every fight. After, it wins 95% in a 57 s fight
  and ends at 26% health.
- Abbot Ashworth (10) in blues: every Calling wins every fight. Before, the Initiate won 3%. The Vanguard ends at
  95% health instead of 63%.
- Hogger (30) in blues: before, every Calling lost (the Cutthroat won 2%). After, all five win 98-100%, the
  Initiate ends at 38% health and the Cutthroat at 70%. His health falls from 17,107 to 13,686.
- A level-9 Basic Warrior against one Northshire mob wins every fight (3% before) and ends at 80% health.
- A level-14 Ranger against one mob deals 70 DPS instead of 56 and kills in 21 s instead of 25.
- A level-20 Initiate against one mob wins every fight (51% before) at 102 DPS (65 before) and ends at 96% health.
  It has five times its base health, twice its base mana and hits twice as hard. The Cutthroat keeps 45% of its
  base health and still wins every elite. Davis should play both before the numbers land.

## Builds

`after.md`'s Builds section.


| Calling | Level | Build |
|---|---|---|
| basic_warrior | 1 | none |
| cutthroat | 10 | none |
| cutthroat | 12 | sinister_strike: practiced_strike |
| cutthroat | 15 | sinister_strike: practiced_strike, evasion: second_wind |
| cutthroat | 16 | sinister_strike: practiced_strike, evasion: phantom_guile |
| cutthroat | 18 | sinister_strike: practiced_strike, evasion: phantom_guile, eviscerate: open_wounds |
| cutthroat | 19 | sinister_strike: practiced_strike, evasion: phantom_guile, eviscerate: kidney_shot |
| cutthroat | 20 | sinister_strike: practiced_strike, evasion: second_wind, eviscerate: open_wounds |
| cutthroat | 21 | sinister_strike: practiced_strike, evasion: phantom_guile, eviscerate: executioners_toll, rupture: bloodletting |
| cutthroat | 22 | sinister_strike: practiced_strike, evasion: second_wind, eviscerate: open_wounds, rupture: bloodletting |
| cutthroat | 23 | sinister_strike: practiced_strike, evasion: second_wind, eviscerate: kidney_shot, rupture: bloodletting, passives: blood_rush |
| cutthroat | 26 | sinister_strike: practiced_strike, evasion: phantom_guile, eviscerate: open_wounds, rupture: bloodletting, passives: thirst, garrote: initiative |
| cutthroat | 27 | sinister_strike: practiced_strike, evasion: phantom_guile, eviscerate: open_wounds, rupture: bloodletting, passives: blood_rush, garrote: initiative |
| cutthroat | 29 | sinister_strike: practiced_strike, evasion: second_wind, eviscerate: open_wounds, rupture: bloodletting, passives: blood_rush, garrote: initiative |
| cutthroat | 31 | sinister_strike: practiced_strike, evasion: phantom_guile, eviscerate: open_wounds, rupture: bloodletting, passives: blood_rush, garrote: initiative |
| ranger | 10 | none |
| ranger | 12 | talents: point_blank |
| ranger | 15 | talents: point_blank, snapshot: twin_arrows |
| ranger | 18 | talents: point_blank, snapshot: twin_arrows, vault: parting_shot |
| ranger | 21 | talents: point_blank, snapshot: twin_arrows, vault: parting_shot, barbed_arrow: jagged_barbs |
| ranger | 23 | talents: point_blank, snapshot: twin_arrows, vault: parting_shot, barbed_arrow: jagged_barbs, puncturing_shaft: killshot |
| ranger | 26 | talents: point_blank, snapshot: twin_arrows, vault: parting_shot, barbed_arrow: jagged_barbs, puncturing_shaft: killshot, rapid_fire: flurry |
| initiate | 10 | none |
| initiate | 12 | ley_spark: arcane_substrate |
| initiate | 15 | ley_spark: arcane_substrate, ley_ward: volatile_ward |
| initiate | 18 | ley_spark: arcane_substrate, ley_ward: volatile_ward, searing_ember: thermal_feedback |
| initiate | 21 | ley_spark: arcane_substrate, ley_ward: volatile_ward, searing_ember: kindling_ember, biting_cold: combustion_engine |
| initiate | 22 | ley_spark: arcane_substrate, ley_ward: volatile_ward, searing_ember: thermal_feedback, biting_cold: combustion_engine |
| initiate | 23 | ley_spark: arcane_substrate, ley_ward: volatile_ward, searing_ember: kindling_ember, biting_cold: combustion_engine, passives: catalytic_resonance |
| initiate | 25 | ley_spark: arcane_substrate, ley_ward: volatile_ward, searing_ember: thermal_feedback, biting_cold: combustion_engine, passives: catalytic_resonance |
| initiate | 26 | ley_spark: arcane_substrate, ley_ward: volatile_ward, searing_ember: smoldering_ember, biting_cold: combustion_engine, passives: catalytic_resonance, catalyst: thermal_shock |
| initiate | 30 | ley_spark: arcane_substrate, ley_ward: volatile_ward, searing_ember: kindling_ember, biting_cold: combustion_engine, passives: catalytic_resonance, catalyst: thermal_shock |
| initiate | 31 | ley_spark: arcane_substrate, ley_ward: volatile_ward, searing_ember: smoldering_ember, biting_cold: combustion_engine, passives: catalytic_resonance, catalyst: thermal_shock |
| vanguard | 10 | none |
| vanguard | 12 | righteous_smash: iron_juggernaut |
| vanguard | 15 | righteous_smash: iron_juggernaut, shield_of_penitent: searing_penance |
| vanguard | 18 | righteous_smash: iron_juggernaut, shield_of_penitent: searing_penance, zealous_cleave: zealous_restoration |
| vanguard | 20 | righteous_smash: iron_juggernaut, shield_of_penitent: searing_penance, zealous_cleave: tithe_of_zeal |
| vanguard | 21 | righteous_smash: iron_juggernaut, shield_of_penitent: searing_penance, zealous_cleave: tithe_of_zeal, bulwark_charge: hammer_of_vanguard |
| vanguard | 22 | righteous_smash: vengeful_conviction, shield_of_penitent: searing_penance, zealous_cleave: zealous_restoration, bulwark_charge: hammer_of_vanguard |
| vanguard | 23 | righteous_smash: iron_juggernaut, shield_of_penitent: searing_penance, zealous_cleave: zealous_restoration, bulwark_charge: hammer_of_vanguard, passives: avenging_light |
| vanguard | 24 | righteous_smash: iron_juggernaut, shield_of_penitent: searing_penance, zealous_cleave: tithe_of_zeal, bulwark_charge: hammer_of_vanguard, passives: avenging_light |
| vanguard | 25 | righteous_smash: iron_juggernaut, shield_of_penitent: searing_penance, zealous_cleave: zealous_restoration, bulwark_charge: hammer_of_vanguard, passives: avenging_light |
| vanguard | 26 | righteous_smash: iron_juggernaut, shield_of_penitent: searing_penance, zealous_cleave: zealous_restoration, bulwark_charge: hammer_of_vanguard, passives: avenging_light, seal_of_iron: wrathful_iron |
| vanguard | 27 | righteous_smash: iron_juggernaut, shield_of_penitent: searing_penance, zealous_cleave: tithe_of_zeal, bulwark_charge: hammer_of_vanguard, passives: avenging_light, seal_of_iron: wrathful_iron |
| vanguard | 29 | righteous_smash: vengeful_conviction, shield_of_penitent: searing_penance, zealous_cleave: zealous_restoration, bulwark_charge: hammer_of_vanguard, passives: avenging_light, seal_of_iron: wrathful_iron |
| vanguard | 30 | righteous_smash: iron_juggernaut, shield_of_penitent: searing_penance, zealous_cleave: zealous_restoration, bulwark_charge: hammer_of_vanguard, passives: avenging_light, seal_of_iron: wrathful_iron |
| vanguard | 31 | righteous_smash: iron_juggernaut, shield_of_penitent: searing_penance, zealous_cleave: tithe_of_zeal, bulwark_charge: hammer_of_vanguard, passives: avenging_light, seal_of_iron: wrathful_iron |
| vanguard | 35 | righteous_smash: vengeful_conviction, shield_of_penitent: searing_penance, zealous_cleave: tithe_of_zeal, bulwark_charge: hammer_of_vanguard, passives: avenging_light, seal_of_iron: wrathful_iron |
| acolyte | 10 | none |
| acolyte | 12 | creeping_blight: withering_blight |
| acolyte | 15 | creeping_blight: withering_blight, blood_lance: exsanguinate |
| acolyte | 18 | creeping_blight: withering_blight, blood_lance: exsanguinate, talents: necrotic_detonation |
| acolyte | 21 | creeping_blight: withering_blight, blood_lance: exsanguinate, talents: necrotic_detonation, siphon_life: feeding_siphon |
| acolyte | 23 | creeping_blight: withering_blight, blood_lance: exsanguinate, talents: necrotic_detonation, siphon_life: feeding_siphon, dark_mending: sanguine_flow |
| acolyte | 24 | creeping_blight: withering_blight, blood_lance: exsanguinate, talents: necrotic_detonation, siphon_life: feeding_siphon, dark_mending: coagulation |
| acolyte | 25 | creeping_blight: withering_blight, blood_lance: exsanguinate, talents: necrotic_detonation, siphon_life: feeding_siphon, dark_mending: dark_surplus |
| acolyte | 26 | creeping_blight: withering_blight, blood_lance: exsanguinate, talents: necrotic_detonation, siphon_life: feeding_siphon, dark_mending: coagulation, miasma: breeding_ground |
| acolyte | 30 | creeping_blight: withering_blight, blood_lance: exsanguinate, talents: necrotic_detonation, siphon_life: feeding_siphon, dark_mending: sanguine_flow, miasma: breeding_ground |
| acolyte | 31 | creeping_blight: withering_blight, blood_lance: sanguine_barrier, talents: necrotic_detonation, siphon_life: gorging_siphon, dark_mending: coagulation, miasma: breeding_ground |
| acolyte | 35 | creeping_blight: withering_blight, blood_lance: sanguine_barrier, talents: necrotic_detonation, siphon_life: feeding_siphon, dark_mending: coagulation, miasma: breeding_ground |

Picks that changed from `before.md` to `after.md`:

- cutthroat 19: eviscerate open_wounds to kidney_shot.
- cutthroat 20: evasion phantom_guile to second_wind; eviscerate executioners_toll to open_wounds.
- cutthroat 21: eviscerate open_wounds to executioners_toll.
- cutthroat 24: evasion phantom_guile to second_wind; eviscerate open_wounds to kidney_shot.
- cutthroat 25: evasion phantom_guile to second_wind; eviscerate open_wounds to kidney_shot.
- cutthroat 26: eviscerate executioners_toll to open_wounds; rupture open_veins to bloodletting; passives blood_rush to thirst.
- cutthroat 27: evasion second_wind to phantom_guile.
- cutthroat 28: passives blood_frenzy to blood_rush.
- cutthroat 29: evasion phantom_guile to second_wind.
- cutthroat 30: evasion phantom_guile to second_wind; eviscerate executioners_toll to open_wounds; garrote brazen_garrote to initiative.
- cutthroat 31: garrote brazen_garrote to initiative.
- cutthroat 32: garrote brazen_garrote to initiative.
- cutthroat 33: garrote brazen_garrote to initiative.
- cutthroat 34: garrote brazen_garrote to initiative.
- cutthroat 35: garrote brazen_garrote to initiative.
- initiate 13: ley_spark warding_spark to arcane_substrate.
- initiate 19: searing_ember smoldering_ember to thermal_feedback.
- initiate 20: ley_spark warding_spark to arcane_substrate; searing_ember smoldering_ember to thermal_feedback.
- initiate 24: searing_ember thermal_feedback to kindling_ember.
- initiate 26: searing_ember thermal_feedback to smoldering_ember.
- initiate 27: searing_ember thermal_feedback to smoldering_ember.
- initiate 28: searing_ember thermal_feedback to smoldering_ember.
- initiate 29: searing_ember thermal_feedback to smoldering_ember.
- initiate 30: searing_ember thermal_feedback to kindling_ember.
- initiate 31: searing_ember thermal_feedback to smoldering_ember.
- initiate 32: searing_ember thermal_feedback to smoldering_ember.
- initiate 33: searing_ember thermal_feedback to smoldering_ember.
- initiate 34: searing_ember thermal_feedback to smoldering_ember.
- initiate 35: ley_spark ley_line_tap to arcane_substrate; searing_ember thermal_feedback to smoldering_ember.
- vanguard 13: righteous_smash vengeful_conviction to iron_juggernaut.
- vanguard 18: zealous_cleave tithe_of_zeal to zealous_restoration.
- vanguard 19: righteous_smash vengeful_conviction to iron_juggernaut.
- vanguard 20: zealous_cleave zealous_restoration to tithe_of_zeal.
- vanguard 21: righteous_smash vengeful_conviction to iron_juggernaut; zealous_cleave zealous_restoration to tithe_of_zeal.
- vanguard 23: righteous_smash vengeful_conviction to iron_juggernaut.
- vanguard 24: righteous_smash vengeful_conviction to iron_juggernaut.
- vanguard 25: righteous_smash vengeful_conviction to iron_juggernaut; zealous_cleave tithe_of_zeal to zealous_restoration.
- vanguard 26: righteous_smash vengeful_conviction to iron_juggernaut; zealous_cleave tithe_of_zeal to zealous_restoration.
- vanguard 27: zealous_cleave zealous_restoration to tithe_of_zeal.
- vanguard 30: righteous_smash vengeful_conviction to iron_juggernaut; zealous_cleave tithe_of_zeal to zealous_restoration.
- vanguard 31: zealous_cleave sweeping_zeal to tithe_of_zeal.
- vanguard 32: zealous_cleave sweeping_zeal to tithe_of_zeal.
- vanguard 33: zealous_cleave sweeping_zeal to tithe_of_zeal.
- vanguard 34: zealous_cleave sweeping_zeal to tithe_of_zeal.
- vanguard 35: righteous_smash iron_juggernaut to vengeful_conviction; zealous_cleave sweeping_zeal to tithe_of_zeal.
- acolyte 20: blood_lance sanguine_barrier to exsanguinate.
- acolyte 22: creeping_blight epidemic to withering_blight.
- acolyte 23: creeping_blight epidemic to withering_blight; dark_mending coagulation to sanguine_flow.
- acolyte 24: creeping_blight virulence to withering_blight.
- acolyte 25: blood_lance sanguine_barrier to exsanguinate; dark_mending coagulation to dark_surplus.
- acolyte 26: blood_lance sanguine_barrier to exsanguinate.
- acolyte 27: blood_lance sanguine_barrier to exsanguinate.
- acolyte 28: blood_lance sanguine_barrier to exsanguinate.
- acolyte 29: creeping_blight virulence to withering_blight.
- acolyte 30: dark_mending coagulation to sanguine_flow.
- acolyte 31: creeping_blight virulence to withering_blight; blood_lance exsanguinate to sanguine_barrier; talents masochism to necrotic_detonation; siphon_life feeding_siphon to gorging_siphon; dark_mending sanguine_flow to coagulation.
- acolyte 32: creeping_blight virulence to withering_blight; blood_lance exsanguinate to sanguine_barrier; talents masochism to necrotic_detonation; siphon_life feeding_siphon to gorging_siphon; dark_mending sanguine_flow to coagulation.
- acolyte 33: creeping_blight virulence to withering_blight; blood_lance exsanguinate to sanguine_barrier; talents masochism to necrotic_detonation; siphon_life feeding_siphon to gorging_siphon; dark_mending sanguine_flow to coagulation.
- acolyte 34: creeping_blight virulence to withering_blight; blood_lance exsanguinate to sanguine_barrier; talents masochism to necrotic_detonation; siphon_life feeding_siphon to gorging_siphon; dark_mending sanguine_flow to coagulation.
- acolyte 35: blood_lance exsanguinate to sanguine_barrier; talents masochism to necrotic_detonation; dark_mending dark_surplus to coagulation.

## Model gaps

- A Talent Row choice that replaces its skill's effect carries its own copy of the skill's formula. Exsanguinate and
  Splintering Lance repeat Blood Lance's damage. Twin Arrows and Adrenaline Rush repeat Snapshot's. Piercing Force
  and Killshot repeat Puncturing Shaft's, and Combustion Engine repeats Biting Cold's. A formula edit has to name
  every copy, or the copy left behind becomes the pick. That is how #497's edit 10 lost most of its effect once
  rows were set.
- The Acolyte's pools are not a lever in the model. At 250% health, at 500% and with 300% mana it missed B-2E at
  the same 16 levels.
  The readings don't say whether the model never runs it dry in those fights or its blood costs grow with the pool.
- Boss spells the model skips: Geomancer Vurx's frost_armor_vurx and frost_chill_vurx, The Eye Below's and Speaker
  Thane's eye_mind_flay, Gresh the Drowned's drowning_grasp, the Shadow of Morganth's terrify_creature, The Kept's
  void_shield_creature, and Hogger's eat and rushing_charge. Edits 3 and 4 were proven without Hogger's two.
- The level table above 30 is provisional (ADR 0012), which makes Acolyte Marren at 31 and Hogger at 35 in
  G+5-named suspect.
- The Evasion constants the 2026-10-28 list named are gone: the model reads Evasion from its aura now.
