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
  (its own skill). In the sheet, a `talent` row with `row_key`. No shipped Calling has one yet.

## Rules

1. A row is locked below its `row_level`, open at and above it.
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

Three columns join the master CSV: `variant_of`, `row_key`, `row_level`.

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

## Tickets

In order, each blocked by the one before: data and build (#499), realm (#500), client panel
(#501), tree purge and docs (#502). Tier 1 Callings' rows are later ticket sets, one per Calling.
