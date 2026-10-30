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
| Cutthroat | Sinister Strike, Evasion, Eviscerate, Rupture, Garrote, and one Talent row | Stealth gives its row to the blood droplets, which have no button (grill 2026-10-29) |
| Initiate | Ley-Spark, Ley Ward, Searing Ember, Biting Cold, Catalyst, and one Talent row | Blink gives its row to passives, two of which upgrade Blink (grill 2026-10-29) |
| Ranger | Snapshot, Vault, Barbed Arrow, Puncturing Shaft, Rapid Fire, and one Talent row | Explosive Shot has no button; its row went to a Talent row that holds Explosive Payload. Field Dressing has no row. Auto Shot never has one (grill 2026-10-29) |
| Acolyte | Creeping Blight, Blood Lance, Dark Mending, Miasma, Siphon Life, and one Talent row | the Talent row holds Necrotic Detonation, an active, and two passives (grill 2026-10-29) |

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
| Zealous Cleave | Sweeping Zeal | hits up to 5 enemies instead of 3 | `Ability_Warrior_DecisiveStrike` |
| Zealous Cleave | Zealous Restoration | heals the Vanguard for 75% of the Conviction Holy damage it deals | `Spell_Holy_HolyBolt` |
| Zealous Cleave | Tithe of Zeal | refunds 2 Rage per Conviction stack spent | `Spell_Holy_MindVision` |
| Bulwark Charge | Hammer of the Vanguard | the charge ends in a Holy strike for 150% weapon damage + 10 + 1/level | `Ability_ThunderBolt` |
| Bulwark Charge | Bulwark Slam | stuns the target 2 s on arrival | `Ability_Warrior_DefensiveStance` |
| Bulwark Charge | Fervent Charge | generates 30 Rage instead of 15 | `Spell_Holy_SealOfFury` |
| Talent row | Avenging Light | every landed melee hit, white swing or skill, also deals 25% of weapon damage as Holy | `Spell_Holy_SealOfRighteousness` |
| Talent row | Bulwark of Faith | max health +25% | `Spell_Holy_Restoration` |
| Talent row | Ceaseless Zeal | every block or parry gives 5 Rage and 1 Conviction | `Spell_Holy_SealOfWisdom` |
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

Readings taken in the build (#504): `Ability_Warrior_Rampage` and `Spell_Holy_SealOfBlood` are not
in the 1.12 archives, so Sweeping Zeal and Ceaseless Zeal take the free stock icons above.

## Cutthroat's rows

Grilled with Davis 2026-10-29 (#505). Rows open at Sinister Strike 4, Evasion 8, Eviscerate 12,
Rupture 16, the Talent row 20 and Garrote 24. Evasion comes early so a struggling Cutthroat gets a
survival row soon. Stealth has no row. Its place went to a Talent row of blood droplet passives
(row key `passives`). The numbers are the grill's; Davis's first play note tunes them.

| Row | Choice | Changes | Icon |
|---|---|---|---|
| Sinister Strike | Serrated Strikes | awards 2 combo points instead of 1 against a bleeding target | `INV_Sword_17` |
| Sinister Strike | Flurry of Blades | a second enemy in reach takes 50% of the strike | `Ability_Warrior_PunishingBlow` |
| Sinister Strike | Practiced Strike | costs 25 Energy instead of 35 | `Ability_Warrior_DecisiveStrike` |
| Evasion | Phantom Guile | each dodge while it is up gives 1 combo point and 10 Energy | `Spell_Shadow_Charm` |
| Evasion | Elusiveness | cooldown 60 s instead of 120 | `Spell_Nature_Invisibilty` |
| Evasion | Second Wind | also heals 30% of max health over its 15 s | `Spell_Nature_Rejuvenation` |
| Eviscerate | Open Wounds | 50% more damage to a bleeding target | `Spell_Shadow_DeathScream` |
| Eviscerate | Executioner's Toll | a killing blow that yields experience heals 20% of max health and gives 80 Energy | `Spell_Shadow_Fumble` |
| Eviscerate | Kidney Shot | also stuns the target 1 s per combo point spent | `INV_Gauntlets_04` |
| Rupture | Bloodletting | the bleed deals 50% more damage | `INV_Weapon_Crossbow_11` |
| Rupture | Open Veins | every tick spills a blood droplet, instead of half of them | `Spell_Shadow_LifeDrain` |
| Rupture | Sanguine Rupture | heals 80% of its damage instead of 40% | `Ability_Hunter_Pet_Bat` |
| Talent row | Blood Frenzy | Bloodlust stacks to 10 instead of 5 | `Ability_Racial_BloodRage` |
| Talent row | Thirst | each droplet picked up heals 10% of max health | `Spell_Shadow_LifeDrain02` |
| Talent row | Blood Rush | each droplet picked up gives 10 Energy | `Spell_Shadow_ShadowWordDominate` |
| Garrote | Brazen Garrote | needs no Stealth | `Spell_Shadow_Curse` |
| Garrote | Choking Garrote | also stuns the target 3 s | `Ability_CheapShot` |
| Garrote | Initiative | awards 3 combo points instead of 1 | `Spell_Shadow_Possession` |

Davis chose the rows, their order and the 18 choices. The readings below are mine unless marked.

- Stealth and Garrote are both pressed once per fight, at the opener. Stealth gave its row to the
  droplets, as Consecration's row went to passives (Davis).
- No choice in the Sinister Strike or Evasion row adds damage on one target by itself. Serrated
  Strikes, Practiced Strike and Phantom Guile add it through more combo points or Energy, so the
  balance model (`docs/balance/rules.md`) sets each of those rows to whichever choice its sim finds
  most damaging. Flurry of Blades adds nothing on one target, as Sweeping Zeal.
- A target is bleeding while it wears a Physical damage over time the Cutthroat put on it
  (Rupture, Garrote). The old Serrated Strikes used the same test.
- Brazen Garrote has no cooldown, as plain Garrote has none, and a recast refreshes the bleed. Cast
  from Stealth it still opens with a critical strike.
- Kidney Shot's and Choking Garrote's stuns take diminishing returns as Bull Rush's do.
- Phantom Guile's combo point goes on the Cutthroat's current target, as the old talent's did.

Readings taken in the build (#505): Phantom Guile grants its buff with `grant_aura`, as Searing
Penance does, because a `dodge_pct` effect takes only the dodge from its aura row and not the
proc. Every mob's kill pays experience today (ADR 0012), so Executioner's Toll pays out on every
killing blow. A kill that paid none would give nothing, but no mob pays none.

## Initiate's rows

Grilled with Davis 2026-10-29 (#506). Rows open at Ley-Spark 4, Ley Ward 8, Searing Ember 12,
Biting Cold 16, the Talent row 20 and Catalyst 24. Ley Ward comes early because Initiate dies more
than any other Tier 1 Calling in the balance sims. Catalyst, the payoff, closes the rows. Blink has
no row. Its place went to a Talent row of passives (row key `passives`), and two of the three
upgrade Blink. The numbers are the grill's; Davis's first play note tunes them.

| Row | Choice | Changes | Icon |
|---|---|---|---|
| Ley-Spark | Ley-Line Tap | 50% chance to refund its mana; at full mana, a shield of twice the cost for 10 s instead | `Spell_Shadow_ManaBurn` |
| Ley-Spark | Arcane Substrate | each cast cuts the next Biting Cold's cast by 0.5 s and lengthens the next Searing Ember's burn by 5 s | `Spell_Fire_MeteorStorm` |
| Ley-Spark | Warding Spark | a critical hit wards the Initiate for 50% of its damage for 8 s | `Spell_Nature_AstralRecalGroup` |
| Ley Ward | Siphoning Ward | half of the damage the ward absorbs comes back as mana | `Spell_Nature_Purge` |
| Ley Ward | Frost Ward | an enemy whose damage the ward absorbs is slowed 50% for 4 s | `Spell_Frost_FrostWard` |
| Ley Ward | Volatile Ward | when the ward breaks, it bursts for the amount it absorbed as Arcane to enemies within 8 yd | `Spell_Nature_WispSplode` |
| Searing Ember | Thermal Feedback | each burn tick has a 10% chance to make the next Biting Cold instant and free | `Spell_Fire_Flare` |
| Searing Ember | Smoldering Ember | a burning enemy deals 15% less damage to the Initiate | `Spell_Fire_Incinerate` |
| Searing Ember | Kindling Ember | each burn tick returns 3% of max mana | `Spell_Fire_MasterOfElements` |
| Biting Cold | Combustion Engine | 45% more damage to a target the Initiate's Searing Ember burns | `Spell_Fire_FlameShock` |
| Biting Cold | Splintering Cold | the slow also lands on every enemy within 8 yd of the target | `Spell_Frost_IceShard` |
| Biting Cold | Frostbite | roots a target that is already slowed for 2 s | `Spell_Frost_FrostArmor` |
| Talent row | Shattered Blink | Blink leaves a ring of frost where the Initiate stood that roots enemies within 8 yd for 4 s | `Spell_Frost_FreezingBreath` |
| Talent row | Arcane Momentum | after a Blink, Ley-Spark deals 45% more damage for 5 s | `Spell_Shadow_Teleport` |
| Talent row | Catalytic Resonance | each Catalyst that consumes an Ember or a Cold gives 10% more spell damage for 8 s | `Spell_Nature_Lightning` |
| Catalyst | Thermal Shock | the shatter deals 30% more and the Ember refund gives 30% more mana | `Spell_Frost_FrostShock` |
| Catalyst | Deep Freeze | consuming a Biting Cold also stuns the target 3 s | `Spell_Frost_Glacier` |
| Catalyst | Catalytic Chain | each Ember or Cold it consumes is copied onto another enemy within 10 yd | `Spell_Fire_Fire` |

Davis chose the rows, their order, the 18 choices, Thermal Feedback's 10% (the old talent had 20%),
Kindling Ember's 3% and Frostbite's condition. The readings below are mine, and Davis accepted
them.

- Old talents keep their name and their full-rank numbers. Four old icons clashed and were
  swapped: Thermal Feedback's was Searing Ember's, Catalytic Resonance's was Catalyst's (it takes
  its own buff's), Combustion Engine's was Backdraft's, and Thermal Shock's was Fire Blast's.
- Davis skipped Overcharged Spark and Wildfire, the plain damage choices offered for Ley-Spark and
  Searing Ember, as he twice did for Cutthroat. He also skipped Greater Ley Ward, the one plain
  number offered for Ley Ward. The Ley-Spark and Searing Ember rows add damage only through mana
  and tempo, so the balance model (`docs/balance/rules.md`) sets each to whichever choice its sim
  finds most damaging.
- Arcane Substrate gives both buffs on each cast. Each goes with its own skill's next cast.
- Frostbite's "already slowed" means any snare on the target before the cast lands, the test the
  old Frigid Conservation used. Biting Cold's own slow in the same cast does not count.
- Kindling Ember's 3% is per tick, whatever the stacks.
- Smoldering Ember's 15% cut is to the Initiate's damage taken from that enemy only.
- Volatile Ward bursts only when it breaks; a ward that times out does nothing.
- A new Warding Spark ward replaces the old one only when it is larger.
- Frost Ward's slow is its own aura, so Catalyst does not shatter it, but it counts as a slow for
  Frostbite.
- Splintering Cold's slow is Biting Cold's own aura, so Catalyst shatters it on any of those
  enemies.

Readings taken in the build (#506): the two Arcane Substrate rider rows lost their shared
`exclusive_group`, which is what made the realm spend both on the first cast; each now goes with
its own skill's next cast. Ley-Line Tap's `refund_cost` and the old talent proc's
`refund_cost_pct` and `at_full_power=absorb` call one realm helper; the proc's half stays for a
later Talent. A proc's `skill=` and a `next_cast` buff's skills answer to a Skill Choice of that
skill, so Catalytic Resonance follows Thermal Shock and Arcane Substrate's Cold rider shortens
Frostbite. Frostbite's `root` shows as a root aura in the client patch, as a stun does. No icon was
swapped: all 18 are in the 1.12 archives.

## Ranger's rows

Grilled with Davis 2026-10-29 (#507). Rows open at the Talent row 4, Snapshot 8, Vault 12, Barbed
Arrow 16, Puncturing Shaft 20 and Rapid Fire 24. Davis put the Talent row first. Snapshot is
pressed most. Vault is the only survival row, so it opens next. Rapid Fire's 60 s cooldown closes
the rows. Explosive Shot has no button, and only the old Explosive Payload talent fired it, so its
row became a Talent row (row key `talents`) that holds Explosive Payload. Field Dressing has no
row. The numbers are the grill's; Davis's first play note tunes them.

| Row | Choice | Changes | Icon |
|---|---|---|---|
| Talent row | Explosive Payload | active, 15 s cooldown: the next Auto Shot becomes Explosive Shot, 150% weapon damage as Fire to the target and every enemy within 8 yd | `INV_Misc_Bomb_04` |
| Talent row | Guerilla Tactics | each 2 s of moving without stopping gives a stack, up to 3; the next Snapshot deals 50% more per stack and spends them all | `Ability_Parry` |
| Talent row | Point Blank | shots have no 8 yd minimum range; Puncturing Shaft deals 20% more to a target within 8 yd | `Ability_EyeOfTheOwl` |
| Snapshot | Twin Arrows | 30% chance to fire a second, free Snapshot at the same target; the second never repeats | `Spell_Holy_PrayerOfHealing` |
| Snapshot | Rhythmic Draw | each Snapshot cuts the next Puncturing Shaft's cast by 0.5 s, stacking | `Spell_Nature_StrangleVines` |
| Snapshot | Adrenaline Rush | a critical hit takes 6 s off Vault's cooldown | `Ability_Hunter_SwiftStrike` |
| Vault | Parting Gift | leaves a trap where the Ranger stood that roots the first enemy to step on it for 4 s | `Spell_Nature_TimeStop` |
| Vault | Parting Shot | fires a free Snapshot at the Ranger's target on landing | `Ability_Hunter_CriticalShot` |
| Vault | Unbound Leap | also sheds snares, and gives 40% run speed for 3 s | `Ability_Mount_WhiteTiger` |
| Barbed Arrow | Jagged Barbs | each bleed tick has a 20% chance to sunder the target's armor by 250 for 10 s, stacking 3 times | `Ability_PierceDamage` |
| Barbed Arrow | Pinning Barb | also roots the target for 2 s | `Ability_Rogue_Trip` |
| Barbed Arrow | Bloodscent | each bleed tick returns 2% of base mana | `Spell_Frost_WizardMark` |
| Puncturing Shaft | Piercing Force | a critical hit deals 250% instead of 200%, and the shot ignores 250 more armor | `Ability_SearingArrow` |
| Puncturing Shaft | Staggering Shaft | also stuns the target for 2 s | `Ability_Hunter_AimedShot` |
| Puncturing Shaft | Killshot | instant against a target below 30% health | `Ability_Hunter_SniperShot` |
| Rapid Fire | Flurry | the next 2 Snapshots after it cost nothing and always crit | `Ability_Ensnare` |
| Rapid Fire | Rapid Assault | while it is up, each Snapshot fires an extra free arrow for 50% weapon damage | `Ability_UpgradeMoonGlaive` |
| Rapid Fire | Running Fire | while it is up, Auto Shot keeps firing while the Ranger moves | `Ability_Marksmanship` |

Davis chose the rows, their order and the 18 choices. The readings below are mine, and Davis
accepted them.

- Old talents keep their name and their full-rank numbers. Parting Gift's 4 s is its two ranks of
  2 s. Four old icons clashed with a plain skill and were swapped: Explosive Payload's was Silencing
  Shot's, Adrenaline Rush's was Cutthroat's Kick, Jagged Barbs' was Barbed Arrow's and Rapid
  Assault's was Auto Shot's. The swapped icon also goes on the choice's buff or debuff aura.
- Davis skipped Steadfast Aim, the plain-numbers passive. He also skipped one new choice in every
  Skill Choice row (Lingering Barbs, Rolling Recovery, Contagious Barbs, Restless Draw) and the old
  Shrapnel Payload. He picks mechanics over numbers, as with the other Callings.
- Explosive Payload keeps its 15 s cooldown. Explosive Shot stays hidden (`spellbook=no`), and
  Explosive Payload is the only thing that fires it.
- Point Blank keeps its client spell modifier for the minimum range (ADR 0010), so setting it sends
  the modifier and changing it takes the modifier away.
- Guerilla Tactics gives 50% per stack, so a Snapshot can deal up to 150% more.
- Plain Vault already sheds stuns and roots. Unbound Leap adds snares to that list.
- Parting Gift's trap arms at once and lasts 20 s. It is not Snare Trap's trap, and both can be
  down at once.
- Parting Shot's Snapshot is the Ranger's current Snapshot (a Snapshot Skill Choice when one is
  set), and it counts as a Snapshot for Twin Arrows, Adrenaline Rush and Guerilla Tactics. With no
  target in range or sight, it fires nothing.
- Killshot reads the target's health when the cast starts.
- Rapid Assault loses its own 3 min button. It works only while Rapid Fire is up.
- Every row has a choice that adds damage, so the balance model (`docs/balance/rules.md`) can set
  every row to a damage choice.

Readings taken in the build (#507): Vault is a self cast and carries no unit target, so Parting
Shot fires at the unit the Ranger last selected; the realm now remembers the selection. Rhythmic
Draw's stack and Twin Arrows' repeat come from a Snapshot that landed: a miss gives neither.
Adrenaline Rush cuts the cooldown of the Vault choice set on its row, if one is. Rapid Assault's
extra arrow now follows a Snapshot choice too; before, a buff's `extra_strike` answered only to
its skill's own key. Running Fire's buff keeps Rapid Fire's name and icon, as Phantom Guile's buff
keeps Evasion's. No icon was swapped: all 18 are in the 1.12 archives.

## Acolyte's rows

Grilled with Davis 2026-10-29 (#508). Rows open at Creeping Blight 4, Blood Lance 8, the Talent row
12, Siphon Life 16, Dark Mending 20 and Miasma 24. Davis put Creeping Blight, the Calling's core,
first, and the Talent row in the middle, once Blight and Lance both have rows. Miasma's 15 s cooldown
closes the rows. The Talent row (row key `talents`) holds Necrotic Detonation, an active, and two
passives: Davis said on 2026-10-23 that Detonation stays a talent. Hush (the Kick) and Grasping
Shadows (the utility) get no row. The numbers are the grill's; Davis's first play note tunes them.

| Row | Choice | Changes | Icon |
|---|---|---|---|
| Creeping Blight | Epidemic | each tick has a 30% chance to copy Blight, with its stacks and time left, onto another enemy within 10 yd that does not wear it | `Spell_Shadow_Contagion` |
| Creeping Blight | Virulence | each tick has a 2.5% chance per stack to make the next Blood Lance cast 0.5 s faster | `Spell_Shadow_Requiem` |
| Creeping Blight | Withering Blight | the target deals 2% less damage per stack, up to 20% | `Spell_Shadow_CurseOfMannoroth` |
| Blood Lance | Exsanguinate | 5% more damage per Creeping Blight stack on the target, up to 50% | `Spell_Shadow_BloodBoil` |
| Blood Lance | Splintering Lance | enemies within 8 yd of the target take 30% of the hit, and each one wearing Blight gains a stack | `Spell_Shadow_CorpseExplode` |
| Blood Lance | Sanguine Barrier | the ally who gets the HoT also gets a shield of 20% of the hit for 6 s | `Spell_Shadow_SacrificialShield` |
| Talent row | Necrotic Detonation | active, 20 s cooldown: consumes the target's Creeping Blight for 12 + 1 per level a stack as Shadow, and heals allies within 10 yd for 150% of it | `Spell_Shadow_UnsummonBuilding` |
| Talent row | Masochism | Blood Lance and Dark Mending give back 30% of the health they cost over 5 s | `Spell_Nature_MoonGlow` |
| Talent row | Will to Survive | below 50% health, the Acolyte takes 30% more healing | `Spell_Holy_Renew` |
| Siphon Life | Feeding Siphon | each tick has a 30% chance to add a stack to the target's Creeping Blight | `Spell_Shadow_LifeDrain` |
| Siphon Life | Gorging Siphon | healing past max health becomes a shield, up to 20% of max health, for 10 s | `Spell_Shadow_VampiricAura` |
| Siphon Life | Unbroken Siphon | can be channeled while moving | `Spell_Shadow_Haunting` |
| Dark Mending | Coagulation | also leaves a shield on the ally of 30% of the heal that soaks Physical damage for 10 s | `Spell_Holy_Heal02` |
| Dark Mending | Dark Surplus | healing past the ally's max health bursts as Shadow damage on enemies within 8 yd of the ally | `Spell_Shadow_AbominationExplosion` |
| Dark Mending | Sanguine Flow | the next Blood Lance within 6 s is instant | `Spell_Shadow_DeathPact` |
| Miasma | Noxious Vapors | enemies inside have 10% less chance to hit with melee and ranged attacks | `Spell_Shadow_PlagueCloud` |
| Miasma | Breeding Ground | every 2 s, each enemy inside that wears the Acolyte's Creeping Blight gains a stack | `Spell_Shadow_CreepingPlague` |
| Miasma | Shroud of Rot | the cloud sits on the Acolyte and moves with them | `Spell_Shadow_AuraOfDarkness` |

Davis chose the rows, their order, the 18 choices, Feeding Siphon's 30% chance per tick, and Dark
Mending's two new choices after he turned down the first four offered. The readings below are mine,
and Davis accepted them.

- Old talents keep their name and their full-rank numbers. Two old icons clashed and were swapped:
  Epidemic's was Cutthroat's Evasion's, and Noxious Vapors' was Creeping Blight's. The swapped icon
  also goes on the choice's aura. The new icons were unused in the sheet at a466eb1a.
- Davis skipped every control choice offered (Crippling Blight, Clutching Siphon, Binding Mend), the
  plain cast speed of Crimson Surge, Symbiotic Rot and Corpse Bloom for the Talent row, Echoing Mend,
  and Morbid Fascination. For Dark Mending he turned down Transference, Rotting Renewal, Binding Mend
  and then Borrowed Life.
- Necrotic Detonation keeps its cells: 20 s cooldown, 12 + 1 per level a stack, the 150% heal to
  allies within 10 yd.
- Epidemic's copies roll too, so Blight can spread through a pack.
- Virulence's buff holds one charge. A new proc refreshes it.
- Withering Blight cuts the target's damage to everyone, not only to the Acolyte.
- Exsanguinate counts the stacks before the Lance's own stack lands.
- Splintering Lance has no cap on enemies. The splash does not feed the HoT or the shield.
- Sanguine Barrier's shield is 20% of the damage the Lance dealt. It goes on the HoT's ally, and a new
  one replaces the old. Today the realm shields only the caster, as every absorb does, so solo it is
  the Acolyte.
- Masochism counts Blood Lance and Dark Mending, the two health costs.
- Will to Survive checks the Acolyte's health as each heal lands, from any source.
- Feeding Siphon rolls only on a tick that hits, and adds only to the Acolyte's own Blight.
- Gorging Siphon's shield grows with each tick's overhealing up to 20% of max health, and lasts 10 s
  from its last growth.
- Unbroken Siphon drops only the rule that a step ends the channel. Range and the target's death
  still end it.
- Coagulation's shield is 30% of the full heal, so it shields even an ally at full health.
- Dark Surplus counts only overhealing.
- Sanguine Flow holds one charge, and the Lance still costs health.
- Noxious Vapors covers melee and ranged attacks, not spells.
- Breeding Ground never refreshes Blight's timer, which nothing refreshes.
- Shroud of Rot casts on the Acolyte with no ground target. A new Miasma replaces the old.
- Every row has a choice that adds damage (Epidemic, Exsanguinate, Necrotic Detonation, Feeding
  Siphon, Dark Surplus, Breeding Ground), so the balance model (`docs/balance/rules.md`) can set
  every row to a damage choice.

## Tickets

In order, each blocked by the one before: data and build (#499), realm (#500), client panel
(#501), tree purge and docs (#502), then Row Levels and the Tier 1 Callings' rows (listed on #498).
