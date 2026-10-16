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
| identity | `kind`, `calling`, `key`, `name`, `rank` | `kind` is `skill` or `talent`; `key` is the save and content name; `rank` is 1 for a skill |
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

**Adding a column** changes both generated schemas and the realm loader, so it goes through
`/grill-me` first (`CLAUDE.md`).

## The script

`tools/twilights-eve/build_spells.py` reads the CSV, checks every `spell_id`, `talent_id` and
`source_tab_id` against the real `Spell.dbc`, `Talent.dbc` and `TalentTab.dbc` through
`tools/twilights-eve/wowdata.py` (the stdlib MPQ and DBC reader `extract_spells.py` already
uses), and writes both outputs. It exits non-zero on an unknown id, a duplicate `key` within a
Calling, a `realm_cost` below `client_cost`, a client column set on effect 4 or 5, or a talent
rank whose `spell_id` is not that rank's spell in `Talent.dbc`. Run it by hand after a CSV edit
and commit the CSV and both outputs together, so a diff shows what a cell changed.

```
~/.local/bin/python3.15 tools/twilights-eve/build_spells.py --data "$WOW_DATA"
```

## The outputs

`server_spells.json` is what the realm loads into its content resource at boot: a list of
Callings, each with its skills and talent tabs, each skill or rank carrying its realm columns and
its effects as `{kind, params, realm_only}`. The realm spawns one entity per ability with the
components `docs/bevy_guidelines.md` names for each `kind`.

`client_dbc_patch.json` is a list of row overrides, `{spell_id, fields: {power_type, cost,
stances, family_name, family_flags, ...}}`, plus the talent catalog seats. The client applies it
in memory when it loads its spell data (ADR 0003). When a real server plays to a stock client,
the same file feeds an MPQ writer.

## Where it stands

The CSV was generated on 2026-10-16 from `data/twilights-eve/callings.json` by
`tools/twilights-eve/callings_to_csv.py`. Until `build_spells.py` exists and the realm reads
`server_spells.json`, the game still runs on `callings.json` and the `content/*.toml` files
generated from it. Talents in the CSV carry their `description` and client spellmods; their realm
effects are not yet structured columns, because `callings.json` never held them.
