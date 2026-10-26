# Power Level, and combat and XP that ignore the level gap

Advancement resets a character's level, but the world it advances into is built for higher
levels: a Tier 1 character at level 1 hunts in Elwynn at 10 to 30. Vanilla's rules compare the
attacker's level with the defender's everywhere, so a level 1 in Elwynn misses, is dodged and is
crushed by every mob, earns at most +20% XP for a kill far above it, and gets no XP at all for
content it fell behind on. This ADR removes the level gap from those rules and gives the
character a second level for its body.

- **Power Level.** Each Tier's levels map onto its content band in a straight line: Tier 0 is
  1 to 10, Tier 1 is 10 to 30, Tier 2 is 30 to 50 and Tier 3 is 50 to 60. Base stats from the race
  and class tables, and every combat formula that reads the character's level (armour
  penetration, rage, spell scaling, a Bounty Hunter's level), use the Power Level. XP, Talent
  Points, the Tier Cap and the level the client shows use the real level.
- **The attack table ignores levels.** Melee and spell attacks miss 5% of the time in both
  directions. Mobs dodge 5% and parry 5% from the front, and crit 5%. There are no glancing or
  crushing blows. Weapon skill and defence skill no longer move the table. Dual wield still adds
  19% to a white swing's miss, and hit chance auras still subtract. The numbers live in content.
- **Kill XP comes from the mob's level.** `mob level × 5 + 45`, doubled for an elite. There are
  no gray mobs and no cap on the bonus for a mob above the character. A group splits the kill
  equally after vanilla's group bonus.
- **Quest XP comes from the quest's level, with no falloff.** The Tier XP Rate applies to kills,
  quests and discovery alike. Because the reward basis and the cost of a level are separate
  columns in `levels.toml`, the cost can be steepened without raising quest pay with it.
- **Quests gate by Tier.** A quest is offered from the Tier its level falls in, with no upper
  Tier and no level window. The Advancement Quest at the Tier Cap is the one level gate left.
  Advancement no longer forgets Hunting Quests: every quest is done once per character, except
  Bounties and Repeatable Quests.
- **Mob level is the danger.** Mob health and damage grow per mob level on top of vanilla's
  table, then by a factor for the area the mob stands in, then by a factor for its rank. All of
  it lives in `mobs.toml`.

## Considered options

- **Effective level everywhere.** Treat a Tier 1 level 1 as level 10 for XP, quest gates and the
  attack table, and keep vanilla's formulas. Rejected: it keeps gray mobs and level windows,
  which punish a character who falls behind, and the attack table's gap would still need a
  floor for content above the band.
- **A bigger flat Tier Bonus instead of Power Level.** Rejected: one number cannot fit both level 1
  and level 29 of a Tier.
- **Show the Power Level to the client.** Rejected: it hides the reset, and red quests and skull
  mobs right after Advancement are the intended signal that the new area is dangerous.
- **Scale mobs by the gap to their attacker.** Rejected: it contradicts the level-free attack
  table, and a group has no single gap.

## Consequences

- vmangos's formulas in `rules/melee.rs`, `rules/xp.rs` and `rules/quest.rs` stop being the
  reference for those rules. Its tables (`creature_classlevelstats`, the race and class stat
  tables) still are.
- The level table is no longer vanilla above level 9. Levels 30 to 60 are provisional until a
  Tier 2 band exists to calibrate them.
- Old Bounties can be farmed by a higher Tier. That is accepted: their pay is fixed by their own
  level.

## Amended 2026-10-26: mobs grow on the Power Curve

The 5% health and 4% damage a level compounded on the vanilla creature table, which itself grows
about 7% a level through the 20s, so mobs outran gear: a level 20 in level-19 greens lost to two
level-15 normals, and every Tier 1 Calling but the Ranger died to one level-20 Elwynn mob.

- **One Power Curve for both sides.** P(level) is level / 10 up to 30, then 3% compounding
  (`content/power_curve.toml`). A creature's health and damage are its unit class's level-10
  vanilla values x P(level), then `[difficulty]`, the Tier Step, its rank, its Zone Factor and
  its template. Armour stays the vanilla table's at its level. Gear budgets follow the same
  curve (ADR 0026's amendment). `health_per_level` and `damage_per_level` are gone.
- **One Difficulty, fitted at Power Level 20.** Health 3.2, damage 3.0, chosen by the balance
  model so a level 20 in level-20 greens kills a same-level Elwynn mob in about 22 s and loses
  about half its health; the same numbers land near that at 10 and 30. If a level drifts, the
  curve's shape changes, never a per-level patch.
- **Damage fades in over Tier 0.** The damage factor runs from 1.5 at level 1 to 3.0 at 10 in a
  straight line, so Tier 0 hits close to how it did and there is no cliff at 10.
- **The Tier Step.** Creatures of level 31 and up take another factor (1.5 health, 1.5 damage),
  the answer to Tier 2's Tier Bonus. It is a placeholder until Tier 2 Callings exist to fit it.
  Gear does not take it.

Northshire gets easier, about 8-14 s kills at level 10 in greens. That is accepted: Elwynn's
Zone Factor still makes the step out of it hostile.
