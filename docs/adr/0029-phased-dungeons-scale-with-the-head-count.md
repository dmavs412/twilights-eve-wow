# Phased Dungeon creatures scale with the Head Count

The friends server (ADR 0009) lets up to five players share one Phased Dungeon Run (ADR 0006), but
every creature is tuned for one player, so a group walks through content built to be hostile.
Davis wants a dungeon's difficulty to follow the number of players in it. Grilled 2026-10-30.

## Decision

- **Head Count.** The living players standing in a Phased Dungeon's Phase. Each player counts
  one at any level; pets and summons do not count. A creature with an Audience (ADR 0014) counts
  only its Audience's living members in the Phase. The count is never below 1.
- **Two multipliers.** A creature's health is multiplied by 1 + 0.5 x (Head Count - 1) and its
  damage by 1 + 0.1 x (Head Count - 1). Both stack on top of the Power Curve, Difficulty, Tier
  Step, rank, Zone Factor and template. Five players face 3.0x health and 1.4x damage. Armour does
  not change. One pair of rates in `content/group.toml` covers every creature in every Phased
  Dungeon, trash, elites and bosses alike, with no per-dungeon or per-creature override.
- **When the count applies.** An idle creature follows the live Head Count every tick and keeps
  its share of health when it rescales. A creature fixes its scale when it enters combat and keeps
  it until it dies or evades; an evaded creature is idle again and follows the count. A creature
  that spawns mid-Run takes the Head Count of the moment it spawns.
- **Creature Spells follow.** Flat spell damage (`periodic_damage`, `gaze`, `nova`,
  `ground_patch` ticks) is multiplied by the damage multiplier, and an `absorb` shield's amount by
  the health multiplier, since a shield is extra health. `hit` is a share of the swing and `heal`
  a share of max health, so both already follow.
- **Rewards follow health.** A kill's XP and money are multiplied by the creature's health
  multiplier before the vanilla group split (`rules::xp`'s group rate, then an even share). Per
  player per kill that gives 0.75x solo XP with two players, about 0.78x with three, about 0.81x
  with four and 0.84x with five. A group also kills faster than one player, so a full group earns
  a little more XP an hour than a solo player. Items do not change: every credited player already
  gets their own loot.
- **Players are told.** When the Head Count changes during a Run, everyone in the Phase gets a
  system line, "Northshire Abbey is now tuned for 3 players." ("1 player" for one). The first
  player entering an empty dungeon gets no line.
- **Scope.** Phased Dungeons only. Wailing Caverns, the one instance map, stays per session and
  solo until shared instances get a grill of their own. Open-world creatures, Rumours outside a
  dungeon, the Noise meter and timed events do not change.

## Considered Options

- The count fixed at Run start: rejected, a friend who joins late would face mobs tuned for fewer.
- The count live even mid-fight: rejected, a death or a step through the Portal would change a
  fight in progress.
- The highest count seen in the Run: rejected, nothing here needs guarding against.
- Health only: rejected, a group with a healer would take no more danger than one player.
- More creatures instead of stronger ones: rejected, it needs spawn spots for the extras in every
  dungeon.
- +75% health a player: rejected after the XP sums; with rewards following health, +50% keeps a
  group's XP an hour near a solo player's.
- Counting only on-level players, or weighting by Power Level: rejected for one rule everyone can
  read.
- One shared Wailing Caverns with a Run: deferred, not rejected.
- The vanilla XP split unchanged: rejected, five players would level about 3.5 times slower than
  one.

## Consequences

Solo play does not change: a Head Count of 1 multiplies by 1. The pull's seed threat (a share of
max health, `content/group.toml`) and the Boss Bar read the scaled health with no change of their
own. Tuning from play edits the two rates in `content/group.toml`.
