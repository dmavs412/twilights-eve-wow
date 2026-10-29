# Talent Rows, as the prototype plays them (spec #498, ADR 0028)

Grilled 2026-10-28. The prototype `skill-variants-prototype.html` (published at
https://claude.ai/artifact/X1UdqAcVZfhdQ7CL4SgECV) is the reference for the rules: its `reduce`
function is the row state machine (level, set, innkeeper), and its `SKILLS` table holds Basic
Warrior's twelve choices with the numbers the data ticket uses. Its dummy fight is a placeholder and
nothing reads it.

## Words

- **Talent Row.** A row of three on the Talents panel, opened at its `row_level`. One choice set
  per row. Two kinds: a Skill Choice row and a Talent row.
- **Skill Choice.** One of the three variants of a skill in its row. A strict upgrade of the plain
  skill, with its own spell id, name, icon and description. While set it replaces the plain skill
  in the spell book and on the bar. In the sheet, a `talent` row with `variant_of`.
- **Talent.** One of the three choices of a Talent row: a passive (one-rank effects) or an active
  (its own skill). In the sheet, a `talent` row with `row_key`. Acolyte's sixth row is the first.

## Rules

1. A row is locked below its `row_level`, open at and above it. Row levels are the Tier's Row
   Levels (amended 2026-10-29): 2, 4, 6, 8 at Tier 0; 4, 8, 12, 16, 20, 24 at Tier 1; none yet at
   Tier 2 and 3. A Calling has exactly one row at each of its Tier's Row Levels.
2. An open, empty row can be set anywhere, free.
3. A set row changes only while an innkeeper is within service range, free. The innkeeper's gossip
   line "I want to rethink my talents" opens the panel with every open row live. The realm checks
   the range on every set, not the client.
4. A set row never clears to empty.
5. Advancement clears every row. Rows are the new Calling's, all empty.
6. Setting a Skill Choice removes the plain skill (or the previous choice) from the spell book,
   adds the choice, and rewrites every action bar slot that held the old spell to the new one.
7. Setting a Talent adds its effects (passive) or its skill (active); changing it removes them.

## The sheet

Four columns join the master CSV: `variant_of`, `row_key`, `row_level` (#499) and `row_levels`
(amended 2026-10-29).

- A calling row sets `row_levels`, its Tier's Row Levels joined by `|`: `2|4|6|8` for Basic
  Warrior, `4|8|12|16|20|24` for each Tier 1 Calling. Blank means the Calling has no rows. The build
  refuses a Calling whose rows' levels are not exactly that list (a missing level, a level twice,
  a level not in it), and two calling rows of one `tier` with different lists.
- A skill row sets `row_level` when it has a Skill Choice row. Blank means no row. The build refuses
  a `row_level` below the skill's `learn_level` or above the Calling's `tier_cap`, and a Skill
  Choice whose skill has no `row_level`.
- A Skill Choice is `kind=talent`, `variant_of=<skill key>` of the same Calling, `rank=1`, its own
  `spell_id`, `name`, `icon`, `status`, `description`. Every other blank cell inherits the skill
  row's. A cell of `-` blanks the inherited cell. An `effectN_kind` cell replaces slot N whole
  (kind and params); `effectN_kind=-` drops slot N. Spellmod cells follow the same rule. The
  choice gets the skill's family bit. Exactly three choices per skill.
- A Talent is `kind=talent`, `row_key=<key>`, `row_level` (equal across the three), `rank=1`, no
  `variant_of`, the old talent effect vocabulary with one rank. An active one has `target` or
  `cast_kind` and is built as a skill. Exactly three rows per `row_key`.
- The tree columns stay in the header and must stay empty; the `talent_tab` kind is gone.

`server_spells.json` gets `rows` per Calling, in level order:
`{ key, level, kind: "skill_choice" | "talent", skill: <key or null>, choices: [built row x3] }`,
where a Skill Choice's built row is a full skill (merged) and a Talent's is a talent rank.

## The panel

`assets/ui/Talents.xml`, opened by the Talents micro button (shown from level 1) and by the
innkeeper's line. Realm-fed (ADR 0013): `TalentRows` carries the rows, `SetTalentRow` carries a
click.

- A 384 x 512 frame at the stock talent frame's place, the Journal's frame art. Title "Talents",
  under it the Calling's name.
- One strip per row, top to bottom in level order, 48 high. At the left a 40-wide label with the
  row's level: gold when open, grey when locked. Then three cells of 104 x 40, each a 28 px icon at
  the left and the choice's name to its right. A row of a Skill Choice shows the skill's name above
  its cells in small grey text.
- Cell states: locked (greyed, desaturated icon, no click); open and unset (normal, hover
  highlight, click sets); set (gold border, lit); open with another choice set (dimmed, no click,
  unless `atInnkeeper`, then normal and click changes).
- A line at the bottom: "Set rows change at an innkeeper." or, with `atInnkeeper`, "At the
  innkeeper: any open row can be changed."
- Hovering a cell shows the choice's spell tooltip (name, description) from its Spell.dbc row.
- Ten rows fit; past that the strips scroll.
- A Calling with no rows (Zealot) shows "No talents yet." where the strips would be.

## Basic Warrior's rows

Rows open at Charge 2, Fire Blast 4, Gouge 6, Heroic Strike 8. The plain rows are the real ones
in `master.csv`. Every choice's numbers are the prototype's; Davis's first play note tunes them.

| Row | Choice | Changes | Icon |
|---|---|---|---|
| Charge | Bull Rush | stun 3 s instead of 1 s | `Ability_BullRush` |
| Charge | Rallying Charge | generates 20 rage instead of 8.5 + 0.14 L | `Ability_Warrior_BattleShout` |
| Charge | Reckless Charge | usable in combat | `Ability_Rogue_Sprint` |
| Fire Blast | Searing Blast | the target burns for 60% of the hit over 6 s | `Spell_Fire_Immolation` |
| Fire Blast | Flash Burn | cooldown 4 s instead of 8 s | `Spell_Fire_FlameBolt` |
| Fire Blast | Backdraft | double damage to a stunned or incapacitated target | `Spell_Fire_SoulBurn` |
| Gouge | Deep Gouge | three times the damage | `Ability_BackStab` |
| Gouge | Blindside | incapacitates 6 s instead of 4 s; the caster's next hit is a critical strike | `Ability_Rogue_KidneyShot` |
| Gouge | Quick Gouge | 5 rage instead of 15, cooldown 6 s instead of 10 s | `Ability_Rogue_SliceDice` |
| Heroic Strike | Cleave | a second enemy in reach takes 50% of the hit | `Ability_Warrior_Cleave` |
| Heroic Strike | Finisher | 50% more damage to an enemy below 30% health | `Ability_Rogue_Eviscerate` |
| Heroic Strike | Frenzy | each strike within 5 s of the last adds 25% to the bonus, three stacks | `Spell_Shadow_UnholyFrenzy` |

Readings taken in the grill: Blindside's crit is the caster's next hit on any target within 10 s
(the prototype says "on it"); a Searing Blast burn tick still ends a Gouge, as the prototype shows.

## Tier 1 rows (amended 2026-10-29)

Six rows per Tier 1 Calling, at 4, 8, 12, 16, 20 and 24, one per skill the Calling learns at level
1. The Kick and the utility get none.

| Calling | Skills with a row | Note |
|---|---|---|
| Vanguard | Righteous Smash, Shield of the Penitent, Zealous Cleave, Bulwark Charge, Seal of Iron, and one Talent row | Consecration is Bulwark Charge's patch, not a button, so it has no row (grill 2026-10-29) |
| Cutthroat | Sinister Strike, Eviscerate, Rupture, Garrote, Evasion, Stealth | |
| Initiate | Ley-Spark, Searing Ember, Biting Cold, Catalyst, Blink, Ley Ward | |
| Ranger | six of Snapshot, Puncturing Shaft, Barbed Arrow, Vault, Rapid Fire, Field Dressing, Explosive Shot | the Build lane picks the one left out; Auto Shot never has a row |
| Acolyte | Creeping Blight, Blood Lance, Dark Mending, Miasma, Siphon Life, and one Talent row | the Talent row's three are passives or actives |

Which skill opens at which level, and the 18 choices, are placeholders the Build lane writes, one
ticket per Calling. Each choice may start from an old tree talent that touched the skill
(Iron Juggernaut, Executioner's Toll, Bloodletting, Explosive Payload), keeping its name, or be a
fresh design. Each is a strict upgrade, as above. A choice may need a new generic mechanic; the
ticket adds it with its tests. Each Calling's rows get their own grill later, and Davis's play
notes tune them. The Build lane fills in a table per Calling below, in the form of Basic
Warrior's.

## Vanguard's rows

Grilled with Davis 2026-10-29 (#504). Rows open at Righteous Smash 4, Shield of the Penitent 8,
Zealous Cleave 12, Bulwark Charge 16, the Talent row 20 and Seal of Iron 24. Shield of the
Penitent comes early so a tanking Vanguard gets a survival row soon. The Talent row's three are
passives (row key `passives`). The numbers are the grill's; Davis's first play note tunes them.

| Row | Choice | Changes | Icon |
|---|---|---|---|
| Righteous Smash | Iron Juggernaut | also deals Physical damage equal to 150% of the shield's Block Value | `Ability_Warrior_Sunder` |
| Righteous Smash | Vengeful Conviction | each hit adds 2 Conviction instead of 1 | `Ability_Warrior_Revenge` |
| Righteous Smash | Penitent's Smash | heals the Vanguard for 25% of the damage it deals | `Spell_Holy_SealOfSacrifice` |
| Shield of the Penitent | Searing Penance | each block deals double Holy: 24 + 2.4/level | `Spell_Holy_SearingLight` |
| Shield of the Penitent | Shield Specialization | each block while it is up gives 3 Rage | `INV_Shield_06` |
| Shield of the Penitent | Steadfast Penitent | lasts 15 s instead of 10, as long as its cooldown | `Spell_Holy_SealOfProtection` |
| Zealous Cleave | Sweeping Zeal | hits up to 5 enemies instead of 3 | `Ability_Warrior_Rampage` |
| Zealous Cleave | Zealous Restoration | heals the Vanguard for 75% of the Conviction Holy damage it deals | `Spell_Holy_HolyBolt` |
| Zealous Cleave | Tithe of Zeal | refunds 2 Rage per Conviction stack spent | `Spell_Holy_MindVision` |
| Bulwark Charge | Hammer of the Vanguard | the charge ends in a Holy strike for 150% weapon damage + 10 + 1/level | `Ability_ThunderBolt` |
| Bulwark Charge | Bulwark Slam | stuns the target 2 s on arrival | `Ability_Warrior_DefensiveStance` |
| Bulwark Charge | Fervent Charge | generates 30 Rage instead of 15 | `Spell_Holy_SealOfFury` |
| Talent row | Avenging Light | every landed melee hit, white swing or skill, also deals 25% of weapon damage as Holy | `Spell_Holy_SealOfRighteousness` |
| Talent row | Bulwark of Faith | max health +25% | `Spell_Holy_Restoration` |
| Talent row | Ceaseless Zeal | every block or parry gives 5 Rage and 1 Conviction | `Spell_Holy_SealOfBlood` |
| Seal of Iron | Wrathful Iron | hits also have a 50% chance to sear for 50% of weapon damage as Holy | `Spell_Holy_SealOfWrath` |
| Seal of Iron | Seal of Mending | both procs fire at 50% instead of 25% | `Spell_Holy_GreaterHeal` |
| Seal of Iron | Seal of Fervor | hits also have a 25% chance to give 5 Rage | `Spell_Holy_SealOfValor` |

Readings taken in the grill:

- Consecration's row became the Talent row because the patch has no button to swap. Davis asked
  for passives stronger than numbers on a patch.
- The plain patch itself changes: a Vanguard standing in their own Consecration takes 10% less
  damage (#509). That cut was first offered as a passive and Davis made it part of the patch.
- On one target Sweeping Zeal adds nothing, so the balance model (`docs/balance/rules.md`) reads
  the Cleave row as adding no damage when it fights a single enemy.
- Avenging Light, Wrathful Iron and Seal of Fervor fire on every landed melee hit the realm's
  `melee_hit` sees: white swings, next-swing strikes and skill strikes, once per target a Cleave
  hits.

## Tickets

In order, each blocked by the one before: data and build (#499), realm (#500), client panel
(#501), tree purge and docs (#502), then Row Levels and the Tier 1 Callings' rows (listed on #498).
