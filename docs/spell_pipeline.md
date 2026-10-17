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
| aura row | `client_spell_id`, `duration_ms`, `charges`, `effect1_kind`… | every lasting effect (a buff, a stun, a bleed, a slow) is its own aura row, granted by key; `client_spell_id` is the spell the client shows it as. A `proc` effect names auras in `apply_aura=key\|key`; any other effect (`stun`, `incapacitate`, …) names exactly one in `apply_aura=key`, and the aura must hold an effect of the same kind. The realm reads that effect's params from the aura row, plus its `duration_ms` and `break_on` (so `apply_aura` replaces the inline `duration_ms` and `break_on_damage`); params the skill's effect adds (`per_combo`) win. Basic Warrior's Charge stun and Gouge incapacitate are aura rows (`charge_stun`, `gouge_incapacitate`); Vanguard's and Zealot's rows are still inline, since their tables are the TOML's |
| aura-only | `max_stacks`, `stack_rule`, `unique_per`, `exclusive_group`, `group_scope`, `dispel_type`, `mechanic`, `aura_positive`, `aura_hidden`, `cancelable`, `persist_death`, `aura_description`, `break_on`, `grants_form` | aura rows only; any other row kind that sets one fails the build. `max_stacks` (whole number above 0, default 1); `stack_rule` `refresh` (default), `replace`, `ignore` or `independent`; `unique_per` `caster` (default) or `target`; `exclusive_group` a name, with `group_scope` `caster` (default) or `target` (a scope without a group fails); `dispel_type` `magic`, `curse`, `disease`, `poison` or `none` (default); `mechanic` one of the client's names (`charm`, `disoriented`, `disarm`, `distract`, `fear`, `fumble`, `root`, `pacify`, `silence`, `sleep`, `snare`, `stun`, `freeze`, `incapacitate` (= `knockout`, 14), `bleed`, `bandage`, `polymorph`, `banish`, `shield`, `shackle`, `mount`, `infected`, `turn`, `horror`, `invulnerability`, `interrupt`, `daze`, `discovery`, `immune_shield`, `sapped`, `enraged`), empty for none. `aura_positive` `yes`/`no`, empty = no when the row's `target` is `enemy`, yes otherwise; `cancelable` empty = the same as positive; `aura_hidden` and `persist_death` default no; `aura_description` is free text; `grants_form` a form id above 0 (Stealth is 30). `break_on` is `key=yes` pairs joined by `;` from `damage`, `attack`, `move`, `stealth_break`, `leave_combat`, or `damage_threshold=N` (exclusive with `damage`); empty = lasts its full duration. `incapacitate`'s old `break_on_damage` param is now `break_on: damage=yes` on its aura row. The server JSON carries every column resolved. The patch gets `dispel_type` (none 0, magic 1, curse 2, disease 3, poison 4), `mechanic` (the number) and `aura_description` as Spell.dbc fields, only when the cell is set, in an override of the aura's `client_spell_id`; when a skill already overrides that spell id (Gouge's aura shows as spell 1776) the fields join the skill's override, and two rows setting one field to different values fail |
| hidden row | `client_visible`, `realm_spell_id` | `client_visible=no` (empty means yes) emits nothing to the client patch and may set no `client_*` cell. It needs a `realm_spell_id`: a whole number of at least 1,000,000, unique across the sheet, written to `server_spells.json` as `spell_id` with `client_visible: false`. A visible row may not set `realm_spell_id` |
| assumptions | `assumptions` | every guess beside the row it affects: placeholders, durations, borrowed ids, unmodelled parts. Every build prints the rows that carry one; the realm and client never see it |
| ids | `client_spell_id`, `talent_id`, `source_tab_id`, `icon_id` | real 1.12.1 records; the build fails on an id the DBCs do not hold. `client_spell_id` reaches both JSON files as `spell_id` |
| talent seat | `tab`, `tab_row`, `tab_column`, `prereq_key`, `prereq_rank`, `exceptional`, `max_rank` | where the talent sits on the Calling's grid; `exceptional` marks a one-rank talent that still counts toward the row; `max_rank` (1 to 5, equal across a talent's ranks, every rank 1..`max_rank` present) is required on every talent rank row and empty on a skill |
| borrowed | `borrowed_from_class`, `borrowed_family_bit` | set when a Calling casts another class's spell (ADR 0003) |
| client | `client_cost`, `client_cooldown_ms`, `client_gcd_ms`, `client_cast_time_ms`, `client_range_min`, `client_range_max`, `client_duration_ms`, `client_stances`, `client_not_in_combat` | what the stock client shows and enforces before it sends a cast, as Spell.dbc stores it. The old `client_on_next_swing` column is gone: it is `cast_kind=next_swing` (the build refuses a sheet that still has the column) |
| cost | `cost_kind`, `cost_extra_max` | `cost_kind` is `flat` (empty), `pct_base`, `all_remaining`, `per_sec` or `health`. `cost_extra_max` (a whole number above 0) only with `all_remaining`. The patch moves `client_cost` to `cost_percent` for `pct_base` and to `cost_per_second` for `per_sec`, and sets `power_type` 4294967294 (health, -2) for `health`; `flat` and `all_remaining` stay `cost`. The server JSON carries `cost_kind` (always) and `cost_extra_max` |
| cast shape | `cast_kind`, `channel_tick_ms`, `interrupt_on`, `on_cast`, `defense_type`, `outcomes`, `weapon_hand`, `max_targets`, `area_shape`, `area_center` | skill and talent rows only; an aura row that sets any of these (or the cost columns) fails. `cast_kind`: `instant` (empty), `cast` (needs `client_cast_time_ms` above 0), `channel` (needs `client_duration_ms` above 0 and `channel_tick_ms`), `next_swing`, `auto_repeat`; a hidden row skips the client-cell needs. The patch gets `on_next_swing`, `channeled` or `auto_repeat`. `interrupt_on`: `move\|damage\|pushback\|kick` or `none`; empty = `move\|pushback\|kick` for `cast` and `channel`, nothing otherwise; the server JSON always carries the resolved list, the patch gets `interrupt_flags` (`channel_interrupt_flags` for a channel; move 1, pushback 2, kick 8, damage 16) only when the cell is set. `on_cast`: `breaks_stealth` (default yes), `starts_auto_attack` (default yes for a `melee` row on an enemy), `resets_swing` (default no), written `key=yes\|no` joined by `;`; the server JSON carries all three resolved. `defense_type`: `melee\|ranged\|magic\|none`; empty = melee for a physical row on an `enemy`, magic for another school on an `enemy`, none otherwise; the patch gets `dmg_class` (none 0, magic 1, melee 2, ranged 3) only when the cell is set. `outcomes`: `miss`, `dodge`, `parry`, `block`, `crit` = `yes\|no` joined by `;`, exceptions only (`dodge=no;crit=no`); it replaces the old `unavoidable` param (a `weapon_strike` with `unavoidable=` now fails). `dodge=no;parry=no` together still write `unavoidable: true` into the strike's params, which the realm reads. `weapon_hand`: `main` (default), `off`, `both`. `max_targets`: a whole number above 0; empty = 1, or unlimited (`null`) when the row has an `area_shape` or a `realm_radius_yd`; above 1 needs one of those. `area_shape`: `circle\|cone\|chain`; `area_center`: `caster\|target\|ground`, needs `area_shape`. The area and `weapon_hand` columns are server-only until the patch ticket maps implicit targets |
| realm | `realm_cost`, `realm_cooldown_ms`, `realm_gcd_ms`, `realm_range_min`, `realm_range_max`, `realm_radius_yd`, `target` | what the realm does; `realm_cost` must be at least `client_cost`, or the client refuses casts the realm would allow |
| gates (client-mirrored) | `requires_weapon`, `cooldown_category`, `category_cooldown_ms`, `school`, `prevention`, `requires_caster_state`, `requires_target_state` | `requires_weapon`: weapon names joined by `\|` (`dagger`, `one_hand_sword`, `two_hand_sword`, `one_hand_axe`, `two_hand_axe`, `one_hand_mace`, `two_hand_mace`, `fist`, `polearm`, `staff`, `bow`, `gun`, `crossbow`, `thrown`, `wand`), `shield`, or the aliases `any_melee`, `any_ranged`, `one_hand`, `two_hand`; the patch gets `equipped_item_class`, `equipped_subclass_mask`, `equipped_inventory_mask` (a shield cannot mix with weapons). `cooldown_category` is a name, numbered from 10,000 in order of first appearance (no stock Category is that high) into the patch's `category`; it needs `category_cooldown_ms`, equal on every row sharing the name. `school` is `physical\|holy\|fire\|nature\|frost\|shadow\|arcane`, empty = physical, and `school_damage` takes it from the row (a `school=` param fails the build); `prevention` is `silence\|pacify\|none`, empty = pacify for physical and silence otherwise, patched as `prevention_type`. `requires_caster_state` / `requires_target_state` take `defense`, `health_below_20`, `berserking`, `frozen`, `judgement` or `hunter_parry` (patched as `caster_aura_state` / `target_aura_state`). Skill and talent rows only, except `school` (and the `prevention` it implies), which an aura row may set; the others fail on an aura row. Server JSON carries the names and resolved `school` / `prevention`, the patch the numbers |
| gates (realm-only) | `requires` | `key=value` pairs joined by `;`. Keys: `position` (`behind`, `front`), `facing` (`yes`, `no`), `stealth` (`yes`), `combat` (`in`, `out`), `target_state` (`bleeding`, `stunned`, `rooted`, `feared`, `incapacitated`, `poisoned`), `caster_state` (`after_crit`, `after_dodge`, `after_parry`, `after_block`, `after_kill`) which must come with `window_ms` (a whole number above 0), `offhand` (`weapon`), `los` (`yes`, `no`), `usable_while` (`stunned`, `feared`, `rooted`, `charmed`, `silenced`, joined by `\|`). `combat=out` also sets the realm's `requires_out_of_combat` flag. `client_not_in_combat=yes` fails the build unless `requires` has `combat=out`. The old `requires_out_of_combat` column is gone |
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
borrowed text. An aura row is an override only for its `dispel_type`, `mechanic` and `aura_description` (above). The CSV uses the realm's vocabulary: `weapon=normal` or
`normalized`, the `outcomes` column in place of an `unavoidable` param, `mod_stat` (`stat=strength|agility|stamina|intellect|spirit` with `pct=` of the total, or flat `amount=`; max-health changes move current health with them), `mod_health_regen_out_of_combat` (`pct=`; natural regeneration is a Spirit-based tick every 2 s out of combat), `incapacitate` (`duration_ms`, `break_on_damage`, `drop_combat`: a stun that damage after the cast ends and that drops the target out of combat; on waking, aggro rules apply), `school_damage` (its school is the row's `school` column, default physical; any non-physical school ignores armour, rolls only the level-based miss and crits for 150%), and `proc` (`on=cast|kill`, `skill=`, `yields_xp=yes`,
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
whose file entry carries a `tier`, which only a `calling` row gives it. The loader reads every
schema #63 field into the content, loaded and not enforced: a skill's `meta` (gates, cost kind,
cast shape, spellmods), an aura's `row` (the aura-only columns), a talent's `rank_spellmods`, and
`hidden_skills` for `client_visible=no` rows, found by `CallingDef::hidden_skill(realm_spell_id)`.
Vanguard and Zealot have a
`calling` row that names only their `class`; their tiers, skills and talents are still the tables
in `content/callings.toml`, which uses the same vocabulary (`class`, `power_cost`, `energize` with
`power`, `add_combo_points`, `per_combo`). A skill with no effects, or an effect kind the realm
lacks, is left out of the bar and spell book until its effects land; a talent in that state is
learnable but does nothing and shows as WIP. The client applies the patch at load
(`SpellCatalog::apply_dbc_patch`, compiled in), `power_type` included, and writes each gate,
cost-kind, cast-shape and aura field into its Spell.dbc column (ADR 0005, ticket #70). The patch carries no
`family_name` field; adding one is a schema change and goes through `/grill-me`. Talents carry
their `description` and client spellmods, but their realm effects are not yet structured columns,
because `callings.json` never held them.
