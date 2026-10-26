# Gear budgets compound with level, loot rolls by Loot Band, and a Line of gear shares one icon

Elwynn Forest from level 11 to 30 needs twenty levels of Gear where Northshire needed one. Spec
#448 (grilled 2026-10-25) hand-authors every row, as spec #272 did for Northshire, and this ADR
settles the three rules that outlive the spec: how a row's power grows with its level, how the
realm picks what a kill drops, and how the no-duplicate-icons rule survives 389 new rows.

- **A budget compounds 5% a level from its level 10 anchor.** `budget(quality, L) =
  round_half_up(base x 1.05^(L-10))` with base white 2, green 6, blue 13, epic 18 (#426's
  anchors). Weapon DPS compounds the same 5% from #426's green anchors (one-hander 7.9, staff
  10.5, bow 6.0), a blue x1.3, an epic x1.5, a white x0.8. Mob health grows 5% a level
  (ADR 0012), so a character who keeps up with the band's greens keeps the same fight length
  from 10 to 30 by construction. The flex stays at 2 points and 10% of DPS; `build_gear.py`
  refuses a row at or above entry 900000 outside it. Rows at level 10 and under keep #426's
  flat numbers.
- **The realm rolls by Loot Band.** A Loot Band is a range of mob levels with its own white
  and green lists in `content/loot_bands.toml`: Northshire 1-10 (the existing roll), then
  11-14, 15-18, 19-22, 23-26 and 27-30. A kill on map 0 rolls 8% a white and 4% a green of the
  mob's band; a mob above the last band takes the last band. The vanilla pool roll is gone.
  The Token vendor serves the buyer's band by the same table (amended by ADR 0027: the Token
  vendor is gone and the Source is called White; the roll is unchanged). A creature's own `rolls`
  (elites, bosses) run on top, first success wins.
- **A Line is one item for the icon rule.** ADR 0025 let the rows generated from one recipe
  share an icon. This ADR extends that to a Line: hand-authored rows that repeat across bands
  or dungeons with only their numbers changing (the five Pendants of Brawn, the five World
  Swift daggers, the three Dungeon Mind staffs). `gear.csv` names the Line in a `line` column,
  `build_gear.py` carries it into `gear.toml`, and `icon_check.py` groups rows by Line before
  it looks for clashes. A row with no Line is its own item. Quest greens, Capstone Chests and
  epics have no Line. The rule between Lines is unchanged: no two Lines share an icon.

## Consequences

- A new band of rows is five sets of twenty-ish rows and a table entry, not a new rule. The
  Barrens, when it returns, takes bands the same way.
- The tooltip's numbers tell a Line's rows apart, as a potion's steps are told apart.
- `gear_drops.csv` is empty and the `pool_level` field in `gear.toml` is always 0; a later
  cleanup can remove both.

## Rejected

- **Linear budgets (base x L/10).** Simpler numbers, but a level 30 green would carry 3x a level
  10 green against a mob with 2.65x the health; the fight gets shorter as the player levels.
- **Per-band budgets from a fight model.** Most exact, and the model becomes a prerequisite
  for every row. The model runs after the rows instead, as a check (#448 decision 18).
- **Icons unique per quality.** A blue ring that looks like a white one reads wrong.
- **Painting the overflow.** About 150 PNGs for rings, amulets, bows and shields.

## Amended 2026-10-26: budgets follow the Power Curve, and gear doubles

A level 20 in level-19 greens lost to two level-15 Elwynn normals: the vanilla creature table
grows faster than 5% a level through the 20s, so mobs outran gear. Gear and mobs now share one
**Power Curve** P(level) (`content/power_curve.toml`, ADR 0012's amendment): level / 10 up to 30,
then 3% compounding. `budget(quality, L) = round_half_up(base x P(L))` above item level 10, and
weapon DPS is its anchor x P(L) at every level, replacing 1.05^(L-10).

Every row also got stronger, the vanilla rows too: stats and spell power x2, weapon damage x1.6,
armour x1.5, rounded half up. The anchors double to white 4, green 12, blue 26, epic 36 with a
flex of 4, and the weapon anchors move x1.6 (green one-hander 12.6, staff 16.8, bow 9.6). The
"linear budgets" option rejected above failed only because mobs grew on their own curve; on one
shared curve it is the right shape up to 30.
