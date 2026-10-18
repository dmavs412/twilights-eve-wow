# Creature Spells

Creatures can cast spells. A Creature Spell is a row in the hand-written
`crates/twilight-realm/content/creature_spells.toml`: a key, the stock client spell id the client
shows (cast bar, icon, debuff), a school, a cast time, a cooldown, a maximum range, a chance, and
the effect it applies, written in the same effect terms as a Calling's skills
(`callings::Effect`). A creature template names its spells by key with `spells = [...]`; a
template without them fights exactly as before.

A generic system casts them (`world/creature_cast.rs`). Once a second, a fighting creature that is
not already casting takes its first spell that is off cooldown and in range, rolls its chance,
checks line of sight, stops where it stands and casts: `SMSG_SPELL_START` with the cast time, then
`SMSG_SPELL_GO`, then the effect through the same aura store, slot fields and periodic ticks
(`SMSG_PERIODICAURALOG`) the player's skills use. While it casts it neither swings nor chases. The
cast is dropped, with nothing applied, if the creature dies, is stunned or evades, or if its
target dies or leaves the range. The cooldown runs from the cast's end.

The rows are not in `master.csv` and have no build script. They are few, they belong to creatures
rather than Callings, they never reach the DBC patch (the client already has the stock row it
shows), and Davis tunes their numbers by hand while testing.

## Considered Options

- Rows in `master.csv` beside the Callings' spells: rejected. The CSV's columns, its generator and
  its DBC patch exist for player spells, and a creature spell needs none of them.
- A creature AI script per creature: rejected. A system or match arm named after one creature or
  spell is a bug (`CLAUDE.md`).
- vmangos `creature_ai_scripts` or `creature_spells` imported as they are: rejected for now. The
  realm has no event AI, and a hand-written handful is easier to tune.

## Consequences

- Only `periodic_damage` is cast by creatures so far; the loader refuses any other effect until
  the system learns it. Direct damage, heals and buffs come as rows need them.
- The aura store's periodic damage carries its caster and school, so a tick on the player names
  the creature that cast it, in the right colour.
- Death clears the player's auras, a creature's curse with them.
- A player cannot interrupt a creature's cast yet; the realm has no interrupt effect.
- Tests can give a creature spells it lacks with `Realm::with_creature_spells`, held per realm.
