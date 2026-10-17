# Power and tables follow the Calling's Class

A character's resource is decided by the Class of its current Calling, and nothing else: a
Calling on the Warrior Class runs on rage, one on the Rogue Class on energy, one on any other
Class on mana. The per-level tables (base health and mana, naked base stats by race, the
agility divisors) and the attack-power formula are read for that Class too. The Local Realm
keeps every Class's tables, extracted from the vmangos world DB, so putting a future Calling on
another Class is a content edit.

The Class is a per-row column in the master CSV, `class`, repeated on every row of a Calling;
the build fails when a Calling's rows disagree. It flows into `server_spells.json` once per
Calling and into every row of the client patch as `power_type`, so a borrowed spell gates
against the right bar. Nothing new is saved: the power kind, the unit's class byte and the bar's
maximum are derived again at Advancement and on load. Advancing onto another Class gives a
fresh bar: rage empty, energy and mana full.

A finisher is implied, not declared. Any effect that carries a `per_combo` formula block makes
its row a finisher: the cast fails with no combo points on the target and a cast that lands
spends them all. There is no `finisher` kind and no flag column. Builders use one
`add_combo_points` kind, and one `energize` kind names the power it gives; a gain of a power the
bar does not hold does nothing.

## Considered Options

- **A `power_type` column in the CSV.** Rejected. It would be a second cell that must agree with
  the Class on every row, and nothing in vanilla lets a Class use another bar (Druid forms, which
  are not modelled).
- **A power type on the character, saved.** Rejected. It would let a save disagree with its
  Calling; deriving it leaves one truth.
- **A `finisher` kind or column.** Rejected. A finisher is exactly a row whose amount or
  duration scales with combo points, so the `per_combo` block already says it.
- **One `energize_<power>` kind per power.** Rejected. The kind list would grow by one per power
  and the realm would hold three arms doing the same thing.

## Consequences

- Every shipped Calling sits on the Warrior Class and plays as before; a Rogue-Class or
  mana-Class Calling can be authored by setting `class` and writing costs as bare numbers.
- The client's class byte follows the Calling, so the stock talent frame, spellbook tabs and
  bar colour change with it. The client applying `power_type` from the patch is separate work.
- Talent cost modifiers stay bare numbers in the caster's power, as the client's COST spellmods
  are.
- Druid forms, percent-of-base-mana costs, spirit regeneration while casting and the energy and
  mana talents are out of scope until a Calling needs them.
