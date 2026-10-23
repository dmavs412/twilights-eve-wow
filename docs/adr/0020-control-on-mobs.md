# Control on mobs: Kicks, Lockouts, boss immunity and diminishing returns

The Tier 1 Callings had no way to stop a mob's cast except a stun or Gouge, and no control
for packs. Spec #<spec> gives each one a Kick and a utility skill. The rules below cover every
player control on a mob, not just those skills. Davis decided them in the grill of 2026-10-23.

- **A Kick stops the cast under way and locks its school.** The mob can't start any Creature
  Spell of that school, instant ones included, until the Lockout ends. Spells of other schools
  still go off, and the mob keeps swinging. A Kick on a mob that isn't casting does nothing.
- **A landed Kick pays a bonus.** In a skill row, the effects listed after `interrupt` run only
  on a target whose cast it stopped. The bonus is data, so each Calling's differs: Conviction,
  a combo point, mana, a cooldown reset or a DoT.
- **Bosses**, the mobs whose `boss` flag raises the Boss Bar, take Kicks, Lockouts, stuns and
  taunts. Incapacitates and knockbacks don't affect them. A slow on a boss is halved. An
  interrupt can be marked `bosses=no` in its row, as Ember Blast's is.
- **Diminishing returns.** Stuns and incapacitates each have their own clock per mob. A
  second control of the same kind within 15 seconds of the last one lasts half as long. A third
  lasts a quarter. A fourth finds the mob immune until 15 seconds have passed since the last one
  landed. Lockouts, slows, roots, knockbacks and taunts never diminish. A control that is
  resisted by immunity does nothing, and that includes not interrupting.
- **Duels are out of scope.** A duel carries damage only today (#180), so none of this
  reaches another player.

## Considered options

- **Lock all casting, or only the spell that was kicked.** Rejected. Locking all casting is
  too strong when most mobs know one or two spells. Locking one spell is too weak to read as a
  Kick.
- **No Lockout, with the bonus as the whole reward.** Rejected. The mob would start the same
  spell again at once.
- **Bosses immune to every control, or a per-mob immunity list in content.** Rejected. Kicking
  should matter on bosses, and a list per mob is data nobody would keep up.
- **One shared diminishing clock, a clock per skill, or diminishing Lockouts.** Rejected for
  vanilla's groups, so Kicks stay reliable and mixing a stun with an incapacitate isn't
  punished.
