# Balance rules: what the model must show

Davis set these on 2026-10-27 for the third Tier 1 balance check. The balance model
(`tools/twilights-eve/balance/`) judges every level 1 to 30 against them, and the Tier 2 Callings
at Power Levels 30 to 50 against the "Tier 2" section's (#695). A proposal that
changes mobs or characters is accepted when every rule still holds in the model. The numbers are
the model's readings, not play; play notes override them.

## The character

- Levels 1 to 9 are Tier 0: the Basic Warrior in its starting outfit and the best white (Common)
  rows it can wear at its level. Its four Talent Rows stay empty, so the Tier 0 rules measure
  the bare kit (#510).
- Levels 10 to 30 are the five Tier 1 Callings: Cutthroat, Ranger, Initiate, Vanguard, Acolyte.
  Each fights with every Talent Row its level has opened set to the choice that adds the most
  damage (ADR 0028, amended 2026-10-29). Rows open by the real level, not the Power Level. The
  old trees and their points are gone; nothing makes up their passive power.
- The model picks each Calling's build once per level (#510). It takes the open rows in level
  order and fights each row's three choices against 1 normal on level in greens, on one seed,
  with the earlier rows already set. The highest DPS wins. Choices within 1% of the best DPS
  tie, and a tie goes to the most health left, then to the first choice in the sheet. Every
  cell at that level fights with that one build.
- A choice counts as no change when the sim can't express it: a root, a movement slow, run
  speed, a taunt, a range change, more targets than a pull holds, or a proc that needs movement.
  The report lists those choices under "Ignored choices" with the reason.
- "Greens" means the best Uncommon row the Calling can wear in every slot at its level, from
  `data/twilights-eve/gear.csv`. "Blues" means the best Rare row where one exists at that level,
  else the green.
- The character uses its kicks, its utility skill and its cooldowns; it does not kite or reset.

## The mobs

- "On level" means a creature of the character's level, warrior unit class, in the Elwynn Forest
  Zone Factor (1.5 health, 1.3 damage) from level 11 to 30, Northshire's (1.0, 1.0) at 10 and
  below, and the Redridge Mountains' above 30 (Duskwood's is the same). Its numbers come from
  `mobs.toml` `[difficulty]`, the Power Curve and its rank factors, as `Difficulty::muls` in
  `crates/twilight-realm/src/mobs.rs` computes them. From level 31 the creature also takes the
  Tier Step (`[difficulty.tier_step]`).
- A "normal" is rank 0. An "elite" is rank 1 with template multipliers 1.0/1.0. A "generic boss"
  is rank 1 with `health_multiplier` 2.0 and `damage_multiplier` 1.1 and no spells (the middle
  of the hand-written bosses). The named bosses are every `boss = true` template in
  `crates/twilight-realm/content/*.toml`, at their own level, class, multipliers and spells.
- A pull of N mobs means all N reach melee within the first second.

## The rules

A fight is won when every mob dies and the character lives. Each cell is 300 fights; "win" is
the share won, "health left" the mean share of max health at the end of the won fights.

| Rule | Who | Gear | Against | Must show |
|---|---|---|---|---|
| T0-1 | Basic Warrior, levels 1 to 9 | starter + whites | 1 normal on level | win >= 97% |
| T0-2 | Basic Warrior, levels 1 to 9 | starter + whites | 2 normals on level | win >= 90% |
| P-2 | each Calling, levels 10 to 30 | greens | 2 normals on level | win >= 80%, health left >= 20% |
| P-3 | each Calling, levels 10 to 30 | greens | 3 normals on level | win between 15% and 50% |
| DPS | Cutthroat, Ranger, Initiate, levels 10 to 30 | greens | 1 normal on level | each one's DPS above both the Vanguard's and the Acolyte's |
| E-dmg | Cutthroat, Ranger, Initiate, levels 10 to 30 | greens | 1 elite on level | win between 35% and 65% |
| E-tank | Vanguard, Acolyte, levels 10 to 30 | greens | 1 elite on level | win >= 70%, and above every damage Calling's win at that level |
| B-2E | each Calling, levels 10 to 30 | blues | 2 elites on level | win >= 80% |
| B-boss | each Calling, levels 10 to 30 | blues | generic boss on level | win >= 80% |
| B-named | each Calling | blues at the boss's level | each named boss | win >= 80% |
| G+5 | each Calling, levels 15 to 30 | greens | generic boss 5 levels below the character | win >= 75% |
| G+5-named | each Calling | greens, 5 levels above the boss | each named boss | win >= 75% |

Levels 1 to 10 are meant to be easy; the Tier 0 rules are the floor, not the target.

## Tier 2

The ten Tier 2 Callings are judged at Power Levels 30 to 50 by the same rules, with Tier 1's
thresholds. A Tier 2 Calling's real level 1 is Power Level 30 and its real level 50 is Power Level
50 (`rules/stats.rs` `power_level`); its Talent Rows open by the real level. At Power Level 30 a
capped Tier 1 Calling and a fresh Tier 2 Calling are both judged. Greens and blues come from
`gear.csv` as for Tier 1; a row stays usable at every level above its required level, and the
report's Kits table names any slot with no usable row at all. A Calling with no sim yet gets one
"no sim" row a rule and no verdict. `run.py --levels 30-50` runs them; the report prints their rows
under their own heading.

The damage pairs are Bloodletter, Nightblade, Pyromancer, Leyweaver, Sharpshooter and Trapper.
The sturdy pairs are Ironclad, Zealot, Blightcaller and Bloodmender. Blightcaller and Bloodmender
are the main healers.

| Rule | Who | Gear | Against | Must show |
|---|---|---|---|---|
| P-2 | each Tier 2 Calling, Power Levels 30 to 50 | greens | 2 normals on level | win >= 80%, health left >= 20% |
| P-3 | each Tier 2 Calling, Power Levels 30 to 50 | greens | 3 normals on level | win between 15% and 50% |
| DPS | the damage pairs, Power Levels 30 to 50 | greens | 1 normal on level | each one's DPS above every sturdy pair's |
| E-dmg | the damage pairs, Power Levels 30 to 50 | greens | 1 elite on level | win between 35% and 65% |
| E-tank | the sturdy pairs, Power Levels 30 to 50 | greens | 1 elite on level | win >= 70%, and above every damage pair's win at that level |
| B-2E | each Tier 2 Calling, Power Levels 30 to 50 | blues | 2 elites on level | win >= 80% |
| B-boss | each Tier 2 Calling, Power Levels 30 to 50 | blues | generic boss on level | win >= 80% |
| B-named | each Tier 2 Calling | blues at the boss's level | each named boss of level 30 to 50 | win >= 80% |
| G+5 | each Tier 2 Calling, Power Levels 30 to 50 | greens | generic boss 5 levels below the character | win >= 75% |
| G+5-named | each Tier 2 Calling | greens, 5 levels above the boss | each named boss of level 30 to 50 | win >= 75% |
| H-1 | Blightcaller, Bloodmender, Power Levels 30 to 50 | greens | 1 elite on level hitting a dummy friend for 60 s | the friend lives with health left >= 30% in win >= 80% |

H-1 measures healing. The elite swings only at the dummy friend, never at the healer, for 60 s; an
elite the healer kills is replaced at once by a fresh one. The friend is an on-level character in
greens: it has the healer's own maximum health and armour, and no dodge, parry or block. A swing
misses 5% of the time and crits 5%. The healer plays its kit on the mob as it would, and its heals
reach the friend as the friend is the most hurt ally (`ally=lowest_health`, `heal_allies_pct`,
ground areas, shields). A fight is won when the friend and the healer live the whole 60 s and the
friend keeps at least 30% of its health; "health left" is the friend's.

## Levers

A proposal may move either side. Mob-side levers: `mobs.toml` `[difficulty]` (health, damage,
damage_at_1, fade_to), `[difficulty.rank]`, a named template's multipliers, a creature spell's
row. Character-side levers: a Calling's Tier Bonus and `callings.csv` columns, a skill or
talent formula in `master.csv` (edited with `tools/twilights-eve/sheet.py`), the gear budgets
and weapon DPS tables in `tools/twilights-eve/build_gear.py`, shield armour and block in
`gear.csv`. Rank factors and named templates cannot fix an ordering rule (DPS, E-tank), so
those take character-side levers. The smallest set of edits that makes every rule hold wins.
