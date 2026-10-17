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

- **Details settled while building the spellmod columns (#65).** The op list is the 1.12.1 client's
  (`GLOBAL_COOLDOWN` and `HASTE` are later-expansion ops and are not accepted). A stock skill's
  mask comes from Spell.dbc, so a stock target needs `--data` or `--spell-flags`. "Reaches a spell
  the row did not name" is checked against the skills of the same Calling in the sheet, not every
  Spell.dbc row, since other ranks of a stock spell share its flags. One named skill is emitted as
  `family_bit` (the client patch is unchanged for existing talents); several as `family_mask`,
  which the client crate does not read yet. `server_spells.json` also carries `target`, the keys.

- **Details settled while building the gate columns (#66).** Aura states are `defense` 1,
  `health_below_20` 2, `berserking` 3, `frozen` 4, `judgement` 5, `hunter_parry` 7, from memory of
  vmangos's `AuraStateType` (no vmangos source on the build machine); only 2 is confirmed by
  `docs/reference/skills-and-talents.md`, so check the rest when a row first uses one. Weapon
  subclass and inventory-type masks follow vmangos's `ItemSubclassWeapon` and `InventoryType`;
  `any_ranged` includes `wand`; `shield` is item class 4 and cannot mix with weapons. Category
  numbers start at 10,000 and follow first appearance in the sheet. `caster_state` requires
  `window_ms`. Gate columns other than `school` fail on an aura row. The server JSON keeps
  `requires_out_of_combat: true` (set by `combat=out`) so the realm loader is untouched, and a
  `school_damage` effect's `school` param is written into the server JSON from the row's `school`
  when it is not physical. The patch writes `school` and `prevention_type` on every override.
  New columns sit at the end of the CSV header; rows shorter than the header read the missing
  cells as empty. `requires_out_of_combat=yes` migrated on the three Charge rows.

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

## Details settled while building (ticket #67, cost and cast shape)

The spec left these open; each took the reading closest to vanilla 1.12.1 and the existing code.

- `client_on_next_swing` was renamed in place to `cast_kind` (no row rewrite). Heroic Strike is
  `next_swing`; Charge's old `no` is now empty, so the patch no longer writes `on_next_swing: false`.
  The `on_next_swing=yes` param on Heroic Strike's `weapon_strike` effect stays, because the realm
  reads it.
- `outcomes` keys are `miss`, `dodge`, `parry`, `block`, `crit`, each `yes|no`. Overpower is
  `dodge=no;parry=no` (the realm's table has no block). The build writes `unavoidable: true` into a
  `weapon_strike` effect when both are `no`, so the realm and the patch read what they always did; a
  literal `unavoidable` param now fails.
- Server JSON carries the resolved defaults for `cost_kind`, `cast_kind`, `interrupt_on`, `on_cast`,
  `defense_type`, `weapon_hand` and `max_targets` (`null` is unlimited). The patch carries
  `interrupt_flags` and `dmg_class` only when the cell is set, so a default leaves the borrowed row.
- Interrupt bits (move 1, pushback 2, kick 8, damage 16) are vmangos's `InterruptFlags`; the channel
  flags reuse them. The client patch ticket must check the channel values against the DBC.
- `starts_auto_attack` defaults to yes when the resolved `defense_type` is `melee` and the target is
  an `enemy`. `max_targets` above 1 needs `area_shape` or `realm_radius_yd`.
- `area_shape`, `area_center` and `weapon_hand` have no patch field yet: implicit targets need the
  borrowed row to compare against, which belongs to the client patch ticket.
