# The master CSV names every vanilla edge case a Calling needs

The master CSV gives each rule a Calling's abilities depend on one named, validated column or
cell, and `build_spells.py` checks it. Three scripted attempts at the Cutthroat Calling failed the
build because `spell_id` read as "pick a number", a talent's `tab` held a display name, and a
spellmod could only reach a raw family bit. The full column list, values and defaults are in
issue #63 (the spec); this record keeps the decisions that are hard to see from the columns.

- **The borrowed row is `client_spell_id`.** The JSON key stays `spell_id`, so realm and client
  code do not change. A hidden ability (a proc's cleave, a talent's bleed) has `client_visible=no`
  and needs no client row. It carries a hand-picked `realm_spell_id` of at least 1,000,000,
  unique across the sheet, so saves and procs keep one number when rows move. A hidden row emits
  nothing to the client patch and may set no `client_*` cell. Rows refer to a hidden ability by
  `key`; the build translates.
- **Names, not bits.** A talent's `tab` is a `talent_tab` key. A spellmod's `spellmodN_target`
  lists skill keys joined by `|`; the build resolves the family mask and fails when it would also
  reach a spell the cell does not name. Five spellmods per row; `op` must be a 1.12.1 client op
  name. Spellmods go to both outputs.
- **Progress is data.** Every row has `status` (`placeholder`, `wip`, `done`); `done` means
  Davis played it. Every talent rank row has `max_rank`, and ranks 1..`max_rank` must exist.
- **Two kinds of gate.** Gates the stock client enforces are columns that map to Spell.dbc
  (`requires_weapon`, `cooldown_category`, `school`, `prevention`, `requires_caster_state`,
  `requires_target_state`, `cost_kind`, `cast_kind`, `interrupt_on`). Gates only the realm can
  judge (position, stealth, combat, target or caster state with a window, off-hand weapon, line
  of sight, usable while impaired) live in one `requires` cell of `key=value` params. The build
  fails when `client_not_in_combat=yes` lacks `requires: combat=out`, so the realm is never looser
  than the client.
- **Empty means the documented default.** Prevention follows school, defense type follows school
  and target, interrupts follow cast kind. `outcomes` holds exceptions (`dodge=no;crit=no`).
- **Every lasting effect is an `aura` row**, applied by `apply_aura=key`. Aura-only columns
  (stacking, exclusive group, dispel type, mechanic, visibility, `break_on`, `grants_form`) fail
  the build on any other row kind.
- **Every new field reaches `server_spells.json`;** every field with a Spell.dbc home also
  reaches `client_dbc_patch.json`. The realm enforcing the realm-only rules is later work, one
  ticket per family. A row stays `modelled=no` until its rules are enforced.

- **Details settled while building the identity and progress columns (#64).** A hidden row's
  `realm_spell_id` is written to `server_spells.json` as `spell_id`, beside `client_visible: false`,
  so the realm keeps one id key. `status` is required on every row, `calling` and `talent_tab` rows
  included. `max_rank` on a skill, aura or other non-talent row fails the build. A `talent_tab`'s key
  is emitted as `key` on the tab. `is_hidden` stays in the CSV, unread, until it is dropped.

## Considered Options

- **Adopt `is_hidden`.** Rejected. `client_visible` says what is withheld and is checked against
  the `client_*` cells.
- **A column per realm-only gate.** Rejected. Nine gate families would add mostly empty columns;
  one `requires` cell holds them.
- **Raw family bits in spellmods.** Rejected. Designers handle skill keys, and the mask check
  catches a cost cut that would also hit a stock spell.
- **Diminishing returns, maximum durations, reagents.** Left out; no Calling needs them yet.

## Consequences

Existing rows migrate with no change in behaviour: `modelled=yes` Basic Warrior rows become
`wip`, the rest `placeholder`, and inline lasting effects become aura rows. Adding a column still
goes through `/grill-me`; this schema is that grilling's outcome.
