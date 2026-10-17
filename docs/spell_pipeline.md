# The spell pipeline

Every spell, talent and ability number in the game comes from one hand-edited file,
`data/twilights-eve/master.csv`, and reaches the realm and the client only through generated
files. Nothing downstream is edited by hand, and no `.rs` file names a spell id or a talent's
numbers.

```
data/twilights-eve/master.csv          hand-edited, the truth
        |
        v  tools/twilights-eve/build_spells.py   (run by hand; verifies ids against the DBCs)
        +--> data/twilights-eve/server_spells.json     the realm loads this at boot
        +--> data/twilights-eve/client_dbc_patch.json  the client applies this to its DBC rows
```

## The CSV

One row per skill, one row per talent rank. Columns, in groups:

| group | columns | meaning |
|---|---|---|
| identity | `kind`, `calling`, `class`, `key`, `name`, `rank` | `kind` is `skill` or `talent`; `class` is the Calling's 1.12.1 class id (1 Warrior, 4 Rogue, 8 Mage ...), the same on every row of a Calling, and decides its power kind (ADR 0004); `key` is the save and content name; `rank` is 1 for a skill |
| ids | `spell_id`, `talent_id`, `source_tab_id`, `icon_id` | real 1.12.1 records; the build fails on an id the DBCs do not hold |
| talent seat | `tab`, `tab_row`, `tab_column`, `prereq_key`, `prereq_rank`, `exceptional` | where the talent sits on the Calling's grid; `exceptional` marks a one-rank talent that still counts toward the row |
| borrowed | `borrowed_from_class`, `borrowed_family_bit` | set when a Calling casts another class's spell (ADR 0003) |
| client | `client_cost`, `client_cooldown_ms`, `client_gcd_ms`, `client_cast_time_ms`, `client_range_min`, `client_range_max`, `client_duration_ms`, `client_stances`, `client_on_next_swing`, `client_not_in_combat` | what the stock client shows and enforces before it sends a cast, as Spell.dbc stores it |
| realm | `realm_cost`, `realm_cooldown_ms`, `realm_gcd_ms`, `realm_range_min`, `realm_range_max`, `realm_radius_yd`, `target`, `requires_out_of_combat` | what the realm does; `realm_cost` must be at least `client_cost`, or the client refuses casts the realm would allow |
| effects | `effect1_kind` to `effect5_kind`, `effect1_params` to `effect5_params` | the realm effects, in order; params are `key=value` pairs joined by `;`, nested keys dotted (`bonus.base=9.5;bonus.per_level=1.4`), lists joined by `\|` |
| spellmods | `spellmod1_type`, `spellmod1_op`, `spellmod1_family_bit`, `spellmod1_value`, and the same for `spellmod2` | a talent rank's client-side modifier: `flat` or `pct`, an op such as `COST`, the family bit it hits, and the value at this rank |
| text | `modelled`, `description`, `notes` | `modelled` is `no` while the realm has no component for it; `description` is what the tooltip must say, hidden effects included |

**The three-effect rule.** Effects 1 to 3 may be mirrored into the DBC patch. Effects 4 and 5 are
realm-only: `build_spells.py` writes them to `server_spells.json` with `realm_only: true` and
never to the patch. The `description` must still describe them, because the client builds the
tooltip from the row's text and has no other way to show them (`docs/dbc_structure.md`).

**Formulas.** An amount is `base`, `per_level` and `per_ap`: `round(base + per_level * L +
per_ap * AP)`, half away from zero, never below `min`. `L` is the level within the Tier, `AP` the
attack power at cast time. `W` is a white-swing roll, `Wn` a normalized one. Keep formulas as
these fields, never as arithmetic in a cell.

**Power and combo points.** `energize` gives `amount` (a formula) of `power=rage|energy|mana`;
a target whose bar is of another kind gains nothing. `add_combo_points` puts `amount` (a positive
whole number) on the target, five at most; points on another target are lost, and a target's
death takes them. A damage or duration effect (`weapon_strike`, `spell_damage`,
`periodic_damage`, `stun`) may carry a `per_combo` formula block (`per_combo.base`,
`per_combo.per_level`, `per_combo.per_ap`), added once per point held on the target; any row
with a `per_combo` block is a finisher: it fails with no points and a cast that lands spends
them all. There is no `finisher` kind or flag. A cost is paid in the Calling's power, read from
its `class`; `realm_cost` and `client_cost` stay bare numbers.

**Adding a column** changes both generated schemas and the realm loader, so it goes through
`/grill-me` first (`CLAUDE.md`).

## The script

`tools/twilights-eve/build_spells.py` reads the CSV, checks that every `spell_id`, `talent_id`,
`icon_id` and `source_tab_id` is a row in `Spell.dbc`, `Talent.dbc`, `SpellIcon.dbc` and
`TalentTab.dbc` through `tools/twilights-eve/wowdata.py`, and writes both outputs in one pass. It
exits non-zero, writing nothing, on an id the DBCs do not hold, an id that is not a whole number,
a duplicate rank within a Calling, a `realm_cost` below `client_cost`, a missing or unknown
`class` or rows of one Calling that disagree on it, an unknown effect kind or malformed params on
a modelled row (an `energize` without a known `power`, an `add_combo_points` without a positive
whole `amount`, a `per_combo` that is not a formula block), a half-filled spellmod, or a talent
seat that is missing, shared or has an unresolved prereq. The DBC check needs `--data` or `WOW_DATA`; `--no-dbc-check`
skips it (the tests use that). Run it by hand after a CSV edit and commit the CSV and both outputs
together, so a diff shows what a cell changed. Tests:
`~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests`.

```
~/.local/bin/python3.15 tools/twilights-eve/build_spells.py --data "$WOW_DATA"
```

## The outputs

`server_spells.json` is what the realm loads into its content resource at boot: a list of
Callings, each with its `class`, its skills and talent tabs, each skill or rank carrying its
realm columns and its effects as `{kind, params, realm_only}`. The realm spawns one entity per ability with the
components `docs/bevy_guidelines.md` names for each `kind`.

`client_dbc_patch.json` is a list of row overrides, `{spell_id, fields: {power_type, cost,
stances, cooldown_ms, ..., borrowed_from_class, borrowed_family_bit, effects}, spellmods}`, plus the talent
seats (`talent_id`, grid position, prereq, one spell id per rank). Every row is emitted with its
`modelled` flag; an unmodelled row sends no `effects` key, so the client keeps the stock effects. The client applies it
in memory when it loads its spell data (ADR 0003). When a real server plays to a stock client,
the same file feeds an MPQ writer.

## Where it stands

As of 2026-10-16 `build_spells.py` generates both outputs from the CSV, and they are committed
(3 Callings, 198 rows, 185 spell overrides, 49 talent seats). Nothing consumes them yet: the
realm runs on the `Effect` enum fed by `crates/twilight-realm/content/callings.toml`, which is
edited by hand in step with the CSV and uses the same vocabulary (`class`, `power_cost`,
`energize` with `power`, `add_combo_points`, `per_combo`); that enum path is the interim
consumer until the `bevy_ecs` port and a CSV-to-TOML bridge. The client does not apply the
patch yet, `power_type` included. The patch carries no `family_name` field; adding one is a
schema change and goes through `/grill-me`. Talents carry their `description` and client
spellmods, but their realm effects are not yet structured columns, because `callings.json`
never held them.
