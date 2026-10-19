# Twilight spell-mod ops past vanilla's

Some talents need the client to change a number that vanilla's 29 `SpellModOp`s cannot reach.
Those talents send their cell through the same pipe as every other talent's client effect,
`SMSG_SET_FLAT_SPELL_MODIFIER` or `SMSG_SET_PCT_SPELL_MODIFIER`, on an op number past vanilla's.
These ops are Twilight extensions. Only benilla reads them, and a stock 1.12 client is not
supported: its unchecked write would overrun the table.

benilla's `SpellModifiers` (`crates/benilla-app/src/spell/mods.rs`) widens each table row by one
slot per Twilight op, so every reader keeps its one code path. In the master CSV a Twilight op is
a `spellmodN_op` name like any other. The realm maps names to numbers in
`callings::spell_mod_op`.

## The ops

| op | name | value | read by |
|---|---|---|---|
| 29 | `MIN_RANGE` | a percent of the minimum range: -100 removes it, -50 halves it | `SpellModifiers::min_max_range`, so the cast check and the action bar's range greying both see it |

Point Blank (#153) is the first user: its cell sets `MIN_RANGE` to -100 on every ranged skill
of the Ranger.
