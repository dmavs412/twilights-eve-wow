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

One row per skill, per talent rank, per aura, plus one `calling` and one `talent_tab` row per Calling
and tab that declares them. Columns, in groups:

| group | columns | meaning |
|---|---|---|
| identity | `kind`, `calling`, `key`, `name`, `rank` | `kind` is `skill`, `talent`, `calling`, `talent_tab` or `aura`; `key` is the save and content name; `rank` is 1 for a skill or aura |
| calling row | `tier`, `tier_cap`, `tier_bonus`, `shapeshift_form`, `armour`, `resource` | a Calling's Tier, Tier Cap, Tier Bonus (`strength=1;stamina=2`), form id, armour kinds joined by `\|`, and the resource every cost on its rows is an amount of |
| talent_tab row | `tab_id`, `background`, `tab_order` | the tab's id, art and order; `name` is the tab name that talent rows put in `tab`. Once a Calling has a `talent_tab` row, every talent's `tab` must name one |
| aura row | `spell_id`, `duration_ms`, `charges`, `effect1_kind`… | a buff granted by key; `spell_id` is a borrowed spell for its icon and name; a `proc` effect names auras in `apply_aura=key\|key` |
| resource | `resource` | on a skill row, overrides the Calling's resource for that row (a borrowed mana spell that costs rage needs none) |
| assumptions | `assumptions` | every guess beside the row it affects: placeholders, durations, borrowed ids, unmodelled parts. Every build prints the rows that carry one; the realm and client never see it |
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

`tools/twilights-eve/build_spells.py` reads the CSV, checks that every `spell_id`, `talent_id`,
`icon_id` and `source_tab_id` is a row in `Spell.dbc`, `Talent.dbc`, `SpellIcon.dbc` and
`TalentTab.dbc` through `tools/twilights-eve/wowdata.py`, and writes both outputs in one pass. It
exits non-zero, writing nothing, on an id the DBCs do not hold, an id that is not a whole number,
a duplicate rank within a Calling, a `realm_cost` below `client_cost`, an unknown effect kind or
malformed params on a modelled row, a half-filled spellmod, or a talent seat that is missing,
shared or has an unresolved prereq. The DBC check needs `--data` or `WOW_DATA`; `--no-dbc-check`
skips it (the tests use that). Run it by hand after a CSV edit and commit the CSV and both outputs
together, so a diff shows what a cell changed. Tests:
`~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests`.

```
~/.local/bin/python3.15 tools/twilights-eve/build_spells.py --data "$WOW_DATA"
```

## The outputs

`server_spells.json` is what the realm loads into its content resource at boot: a list of
Callings, each with its skills and talent tabs, each skill or rank carrying its realm columns and
its effects as `{kind, params, realm_only}`. The realm spawns one entity per ability with the
components `docs/bevy_guidelines.md` names for each `kind`.

`client_dbc_patch.json` is a list of row overrides, `{spell_id, fields: {cost, stances,
cooldown_ms, ..., borrowed_from_class, borrowed_family_bit, effects}, spellmods}`, plus the talent
seats (`talent_id`, grid position, prereq, one spell id per rank). Every row is emitted with its
`modelled` flag; an unmodelled row sends no `effects` key, so the client keeps the stock effects. Every
override with a resource also sends `power_type` (the row's `resource`, else its Calling's: mana 0, rage 1,
focus 2, energy 3, happiness 4) and, when the row has one, `description`; an empty description keeps the
borrowed text. Auras are not Spell.dbc overrides. The CSV uses the realm's vocabulary: `weapon=normal` or
`normalized`, boolean `unavoidable=yes`, `school_damage`, and `proc` (`on=cast|kill`, `skill=`, `yields_xp=yes`,
`chance=`, then one of `apply_aura=key|key` or `heal_pct=`). Seats are whole numbers. The client applies it
in memory when it loads its spell data (ADR 0003). When a real server plays to a stock client,
the same file feeds an MPQ writer.

## Where it stands

As of 2026-10-16 `build_spells.py` generates both outputs from the CSV, and they are committed
(3 Callings, 204 rows, 198 spell overrides, 49 talent seats). Basic Warrior is fully expressed in
the CSV (calling, tab, aura rows; spec #50); its rows whose effects the realm cannot yet do are
`modelled=no`. Nothing consumes the outputs yet: the game still runs on
`data/twilights-eve/callings.json` and the `content/*.toml` files generated from it, and the client
does not apply the patch. The patch carries no `family_name` field. Talents carry their
`description` and client spellmods, but their realm effects are not yet structured columns,
because `callings.json` never held them.
