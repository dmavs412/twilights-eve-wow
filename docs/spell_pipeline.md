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
| identity | `kind`, `calling`, `class`, `key`, `name`, `rank` | `kind` is `skill`, `talent`, `calling`, `talent_tab` or `aura`; `class` is the Calling's 1.12.1 class id (1 Warrior, 4 Rogue, 8 Mage ...), the same on every row of a Calling, and decides its power kind (ADR 0004); `key` is the save and content name; `rank` is 1 for a skill or aura |
| calling row | `tier`, `tier_cap`, `tier_bonus`, `shapeshift_form`, `armour` | a Calling's Tier, Tier Cap, Tier Bonus (`strength=1;stamina=2`), form id and armour kinds joined by `\|`; a row without `tier` leaves the rest of the Calling to the TOML. Its power is its `class`'s |
| progress | `status` | `placeholder`, `wip` or `done`, required on every row; `done` means Davis played it. The build prints a count per status and lists the `wip` rows |
| talent_tab row | `tab_id`, `background`, `tab_order` | the tab's id, art and order; `key` is what talent rows put in `tab` (not the display `name`). Once a Calling has a `talent_tab` row, every talent's `tab` must be one of its keys; a Calling with none (Vanguard, Zealot) keeps the bare tab name |
| aura row | `client_spell_id`, `duration_ms`, `charges`, `effect1_kind`… | a buff granted by key; `client_spell_id` is a borrowed spell for its icon and name; a `proc` effect names auras in `apply_aura=key\|key` |
| hidden row | `client_visible`, `realm_spell_id` | `client_visible=no` (empty means yes) emits nothing to the client patch and may set no `client_*` cell. It needs a `realm_spell_id`: a whole number of at least 1,000,000, unique across the sheet, written to `server_spells.json` as `spell_id` with `client_visible: false`. A visible row may not set `realm_spell_id` |
| assumptions | `assumptions` | every guess beside the row it affects: placeholders, durations, borrowed ids, unmodelled parts. Every build prints the rows that carry one; the realm and client never see it |
| ids | `client_spell_id`, `talent_id`, `source_tab_id`, `icon_id` | real 1.12.1 records; the build fails on an id the DBCs do not hold. `client_spell_id` reaches both JSON files as `spell_id` |
| talent seat | `tab`, `tab_row`, `tab_column`, `prereq_key`, `prereq_rank`, `exceptional`, `max_rank` | where the talent sits on the Calling's grid; `exceptional` marks a one-rank talent that still counts toward the row; `max_rank` (1 to 5, equal across a talent's ranks, every rank 1..`max_rank` present) is required on every talent rank row and empty on a skill |
| borrowed | `borrowed_from_class`, `borrowed_family_bit` | set when a Calling casts another class's spell (ADR 0003) |
| client | `client_cost`, `client_cooldown_ms`, `client_gcd_ms`, `client_cast_time_ms`, `client_range_min`, `client_range_max`, `client_duration_ms`, `client_stances`, `client_on_next_swing`, `client_not_in_combat` | what the stock client shows and enforces before it sends a cast, as Spell.dbc stores it |
| realm | `realm_cost`, `realm_cooldown_ms`, `realm_gcd_ms`, `realm_range_min`, `realm_range_max`, `realm_radius_yd`, `target`, `requires_out_of_combat` | what the realm does; `realm_cost` must be at least `client_cost`, or the client refuses casts the realm would allow |
| effects | `effect1_kind` to `effect5_kind`, `effect1_params` to `effect5_params` | the realm effects, in order; params are `key=value` pairs joined by `;`, nested keys dotted (`bonus.base=9.5;bonus.per_level=1.4`), lists joined by `\|` |
| spellmods | `spellmodN_type`, `spellmodN_op`, `spellmodN_target`, `spellmodN_value` for N = 1 to 5 | a rank's modifier: `flat` or `pct`, a 1.12.1 client op name such as `COST`, the skill keys it hits joined by `\|`, and the value at this rank. The build resolves the family mask (a borrowed skill's `borrowed_family_bit`, a stock skill's Spell.dbc family flags, which need `--data` or `--spell-flags FILE`) and fails when the mask also reaches a skill of the Calling the cell does not name. Both outputs carry the mods: one skill as `family_bit`, several as `family_mask`; `server_spells.json` adds `target`, the keys |
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

`tools/twilights-eve/build_spells.py` reads the CSV, checks that every `client_spell_id`, `talent_id`,
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
`modelled` flag; an unmodelled row sends no `effects` key, so the client keeps the stock effects. Every
override also sends `power_type` (from its Calling's `class`: Warrior rage 1, Rogue energy 3, every other class mana 0) the row's `name` (the client shows the sheet's name, not the borrowed spell's, keeping a `(WIP)` mark) and, when the row has one, `description`; an empty description keeps the
borrowed text. Auras are not Spell.dbc overrides. The CSV uses the realm's vocabulary: `weapon=normal` or
`normalized`, boolean `unavoidable=yes`, `mod_stat` (`stat=strength|agility|stamina|intellect|spirit` with `pct=` of the total, or flat `amount=`; max-health changes move current health with them), `mod_health_regen_out_of_combat` (`pct=`; natural regeneration is a Spirit-based tick every 2 s out of combat), `incapacitate` (`duration_ms`, `break_on_damage`, `drop_combat`: a stun that damage after the cast ends and that drops the target out of combat; on waking, aggro rules apply), `school_damage` (`school=physical|holy|fire`, default physical; any non-physical school ignores armour, rolls only the level-based miss and crits for 150%), and `proc` (`on=cast|kill`, `skill=`, `yields_xp=yes`,
`chance=`, then one of `apply_aura=key|key` or `heal_pct=`). An `aura` row's effects are `skill_crit_pct` (`skill=`, `pct=`: that skill's crit chance, used up one `charges` at a time when the skill lands) and `mod_run_speed` (`pct=`: the realm sends the new run speed through `SMSG_FORCE_RUN_SPEED_CHANGE` and sends the old one back when it fades). Seats are whole numbers. The client applies it
in memory when it loads its spell data (ADR 0003). The build resolves each override's range, cast time and
duration to the stock `SpellRange.dbc`, `SpellCastTimes.dbc` and `SpellDuration.dbc` row that holds
the value (the lowest id on a tie; no match fails the build, and `--no-dbc-check` leaves the raw
values), so the patch carries `range_index`, `cast_time_index` and `duration_index`, which the
client writes over the row's own. The client keeps the borrowed spell's stock effects; a sheet
`school_damage` or `weapon_strike` only overwrites the numbers of the stock damage effect it lines
up with, so `$s1` and spell mods read the sheet's damage. Cost is an amount of the row's power
(rage is stored in tenths). When a real server plays to a stock client,
the same file feeds an MPQ writer.

## Where it stands

As of 2026-10-16 `build_spells.py` generates both outputs from the CSV, and they are committed
(3 Callings, 206 rows). Basic Warrior is fully expressed in the CSV (calling, tab, aura rows; spec
#50); its rows whose effects the realm cannot yet do are `modelled=no`. The realm reads Basic
Warrior from `server_spells.json` (`twilight-realm`'s `generated.rs`, compiled in): any Calling
whose file entry carries a `tier`, which only a `calling` row gives it. Vanguard and Zealot have a
`calling` row that names only their `class`; their tiers, skills and talents are still the tables
in `content/callings.toml`, which uses the same vocabulary (`class`, `power_cost`, `energize` with
`power`, `add_combo_points`, `per_combo`). A skill with no effects, or an effect kind the realm
lacks, is left out of the bar and spell book until its effects land; a talent in that state is
learnable but does nothing and shows as WIP. The client applies the patch at load
(`SpellCatalog::apply_dbc_patch`, compiled in), `power_type` included. The patch carries no
`family_name` field; adding one is a schema change and goes through `/grill-me`. Talents carry
their `description` and client spellmods, but their realm effects are not yet structured columns,
because `callings.json` never held them.
