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

One row per skill, per Skill Choice, per Talent and per aura, plus one `calling` row per Calling
that declares them. Columns, in groups:

| group | columns | meaning |
|---|---|---|
| identity | `kind`, `calling`, `key`, `name`, `rank` | `kind` is `skill`, `talent`, `calling` or `aura`; `key` is the save and content name; `rank` is 1 for a skill, a `talent` row or an aura. The `class` column is gone (ADR 0011): a Calling's power and stats are its `callings.csv` row (below) |
| calling row | `tier`, `tier_cap`, `row_levels`, `tier_bonus`, `shapeshift_form` | a Calling's Tier, Tier Cap, Row Levels, Tier Bonus (`strength=1;stamina=2`) and form id; a row without `tier` leaves the rest of the Calling to the TOML. `row_levels` (ADR 0028) is the levels its Talent Rows open at, joined by `\|` (`2\|4\|6\|8`): whole numbers, strictly rising, each from 1 to `tier_cap`, and it needs `tier` and `tier_cap` on the row. `server_spells.json` gets it as a list; a blank cell adds no key. Its `name` must be its `callings.csv` row's. The `armour` column moved to `callings.csv` as `armor` |
| progress | `status` | `placeholder`, `wip` or `done`, required on every row; `done` means Davis played it. The build prints a count per status and lists the `wip` rows |
| aura row | `spell_id`, `duration_ms`, `charges`, `effect1_kind`… | every lasting effect (a buff, a stun, a bleed, a slow) is its own aura row, granted by key; `spell_id` is the spell the client shows it as (its own row, or its skill's when they share the id). A `proc` effect names auras in `apply_aura=key\|key`; any other effect (`stun`, `incapacitate`, …) names exactly one in `apply_aura=key`, and the aura must hold an effect of the same kind. The realm reads that effect's params from the aura row, plus its `duration_ms` and `break_on` (so `apply_aura` replaces the inline `duration_ms` and `break_on_damage`); params the skill's effect adds (`per_combo`) win. Basic Warrior's Charge stun and Gouge incapacitate are aura rows (`charge_stun`, `gouge_incapacitate`); Zealot's rows are still inline, since its tables are the TOML's |
| aura-only | `max_stacks`, `stack_rule`, `unique_per`, `exclusive_group`, `group_scope`, `dispel_type`, `mechanic`, `aura_positive`, `aura_hidden`, `cancelable`, `persist_death`, `aura_description`, `break_on`, `grants_form` | aura rows only; any other row kind that sets one fails the build. `max_stacks` (whole number above 0, default 1); `stack_rule` `refresh` (default), `replace`, `ignore`, `independent` or `stack` (a `periodic_damage` named by `apply_aura` adds a stack per application up to `max_stacks`, starts its duration over, and ticks once per stack); `unique_per` `caster` (default) or `target`; `exclusive_group` a name, with `group_scope` `caster` (default) or `target` (a scope without a group fails); `dispel_type` `magic`, `curse`, `disease`, `poison` or `none` (default); `mechanic` one of the client's names (`charm`, `disoriented`, `disarm`, `distract`, `fear`, `fumble`, `root`, `pacify`, `silence`, `sleep`, `snare`, `stun`, `freeze`, `incapacitate` (= `knockout`, 14), `bleed`, `bandage`, `polymorph`, `banish`, `shield`, `shackle`, `mount`, `infected`, `turn`, `horror`, `invulnerability`, `interrupt`, `daze`, `discovery`, `immune_shield`, `sapped`, `enraged`), empty for none. `aura_positive` `yes`/`no`, empty = no when the row's `target` is `enemy`, yes otherwise; `cancelable` empty = the same as positive; `aura_hidden` and `persist_death` default no; `aura_description` is free text; `grants_form` a form id above 0 (Stealth is 30). `break_on` is `key=yes` pairs joined by `;` from `damage`, `attack`, `move`, `stealth_break`, `leave_combat`, or `damage_threshold=N` (exclusive with `damage`); empty = lasts its full duration. `incapacitate`'s old `break_on_damage` param is now `break_on: damage=yes` on its aura row. The server JSON carries every column resolved. The patch gets `dispel_type` (none 0, magic 1, curse 2, disease 3, poison 4), `mechanic` (the number) and `aura_description` as Spell.dbc fields, only when the cell is set, in the patch entry of the aura's `spell_id`; when a skill has that spell id too (Gouge's aura shows as Gouge) the fields join the skill's entry, the aura's look and targets giving way to the skill's, and two rows setting one other field to different values fail |
| hidden row | `client_visible` | `client_visible=no` (empty means yes) emits nothing to the client patch and may set no `client_*`, `visual`, `icon` or `needs_facing` cell. It needs a `spell_id`, written to `server_spells.json` with `client_visible: false` |
| learned by level | `learn_level` | The level a skill is learned at, a whole number from 1 to 60; empty means 1. Levelling up grants the skill with no trainer. The build emits it on a skill only above 1, and fails on any other row kind, on 0 and above 60. The realm shows the skill in the spell book, accepts its cast and puts it on the default bar only from that level (#335) |
| off the spell book | `spellbook` | `spellbook=no` (empty means yes) on a visible skill row keeps it off the spell book, the bar and the trainer: it is cast only as another's `becomes=`, and the realm loads it among `hidden_skills`, found by key. The client still has its row, so its `SpellGo` and damage log name a real spell (Explosive Shot on 13812, #154). Skill rows only; a `client_visible=no` row may not set it |
| assumptions | `assumptions` | every guess beside the row it affects: placeholders, durations, unmodelled parts. Every build prints the rows that carry one; the realm and client never see it |
| ids | `icon_id` | a real 1.12.1 `SpellIcon.dbc` record; the build fails on one the DBC does not hold |
| spell | `spell_id`, `visual`, `icon`, `needs_facing` | every skill, Skill Choice, Talent and aura row with a spell is its own Spell.dbc row, never a borrowed stock one (ADR 0011): `spell_id` is a whole number of at least 1,000,000, unique across the sheet (an aura may share its Calling's skill's or talent's), the one id both JSON files carry and the save stores. The borrowing columns `client_spell_id`, `realm_spell_id`, `borrowed_from_class` and `borrowed_family_bit` are gone; the build refuses a sheet that has one. `tools/twilights-eve/migrate_from_scratch.py` moved the sheet once (#204) and wrote `spell_id_remap.json`, the old-to-new table an older save is read through. `visual` is a stock spell id whose look the row takes: the build reads that spell's SpellVisual and missile speed from Spell.dbc (it needs `--data`). Every castable row fills it: a client-visible skill, or an active talent (one with a `cast_kind`); an empty cell fails the build. `visual=none` says the row shows no spell visual on purpose, as stock weapon shots do (Auto Shot and Shoot have SpellVisual 0, and the arrow comes from the ammo); the patch then carries no visual. A ground patch is drawn from the row it is named after (its `ground_spell`, else the casting row), so that row needs a real visual, not `none` (#320). `icon` is a PNG name in `data/twilights-eve/art/` (a missing file fails the build) or a stock icon path (`Interface\Icons\...`). `needs_facing` `yes`/`no`. A row also reads `school`, `cast_kind`, `dispel_type` and `mechanic` (the last two shown on a skill or talent too), `client_cost` and the other `client_*` cells, and every effect it has: the patch's `dbc_effects` lists them all, as `{effect, aura, base, per_level, amplitude_ms}` with the Spell.dbc effect number of the kind (an unlisted kind is a dummy effect, 3); the client holds those past three as extra effects, so `$s4` to `$s9` read them. The build also derives `implicit_targets` from `target` and `area_center` (enemy 6, ally 21, self or none 1, ground 16/28 with the `Targets` ground flag, a caster-centred area 22/15), `passive` for a Talent with no `target`, `ranged` for an auto-repeat or ranged-weapon skill, and each skill's own `SpellFamilyFlags` bit, from 33 up in sheet order (at most 31 skills a Calling), with family 4, so the spellmods that name it reach it alone |
| retired tree columns | `talent_id`, `source_tab_id`, `tab`, `tab_row`, `tab_column`, `prereq_key`, `prereq_rank`, `max_rank`, `tab_id`, `background`, `tab_order`, `exceptional` | the old Talent Tree's columns, retired by ADR 0028. They stay in the header and must stay empty on every row. The build refuses a filled one and names the column and ADR 0028 |
| rows | `variant_of`, `row_key`, `row_level` | Talent Rows (ADR 0028, spec #498). A skill row's `row_level` is the level its Skill Choice row opens: a whole number from the skill's `learn_level` (default 1) to the Calling's `tier_cap` (a Calling with no `tier_cap` row refuses any). A Skill Choice is a `talent` row with `variant_of=<skill key>` of the same Calling, `rank` 1 and its own `spell_id` (never its skill's), `name`, `icon`, `visual`, `status`, `description`; `kind`, `key`, `rank`, `assumptions`, `notes`, `modelled` and `variant_of` are its own too, every other cell is the skill's unless the choice sets it, and a cell of `-` blanks the inherited one. A set `effectN_kind` replaces slot N whole (kind and params, never merged); `effectN_kind=-` drops the slot; a `spellmodN_*` group follows the same rule. The merged row builds exactly as a skill (its own Spell.dbc row, the skill's family bit, so a spell mod aimed at the skill reaches it) and may not be `client_visible=no` or `spellbook=no`. A Talent is a `talent` row with `row_key=<key>`, `rank` 1 and `row_level`, built as a talent rank (`target` or `cast_kind` marks an active one). The build refuses a `talent` row with neither `variant_of` nor `row_key`, `variant_of` with `row_key`, `row_level` on a Skill Choice, a skill with `row_level` and other than three choices, a choice whose skill has no `row_level`, a `row_key` with other than three rows, and a Talent row's three with unequal `row_level`. Then a Calling's row levels must fill its `row_levels` exactly, one row a level: a level twice, a level missing, a level outside the list, and rows on a Calling with a blank `row_levels` all fail. Every Calling of one `tier` that sets `row_levels` must set the same list; a blank one is not compared. `server_spells.json` gets `rows` per Calling, sorted by level then sheet order: `{key, level, kind: "skill_choice" \| "talent", skill: <key or null>, choices: [built row x3]}`, a Skill Choice row's `key` its skill's |
| client | `client_cost`, `client_cooldown_ms`, `client_gcd_ms`, `client_cast_time_ms`, `client_range_min`, `client_range_max`, `client_duration_ms`, `client_stances`, `client_not_in_combat`, `client_radius_index` | what the client shows and enforces before it sends a cast, as Spell.dbc stores it; an empty cell is the from-scratch default (instant, a 1.5 s global cooldown, no range, cooldown or stance). `client_radius_index` is a SpellRadius.dbc id the build writes as `radius_index` on the row's first DBC effect, and the client sizes a ground cast's targeting circle from it. A `target=dest` row must set it. With `--data` the build fails when the index's yards differ from `realm_radius_yd` or an effect's `radius_yd` (#319). The old `client_on_next_swing` column is gone: it is `cast_kind=next_swing` (the build refuses a sheet that still has the column) |
| cost | `cost_kind`, `cost_extra_max` | `cost_kind` is `flat` (empty), `pct_base`, `all_remaining`, `per_sec`, `health` or `health_current` (that percent of the health left, which never kills); the realm charges `pct_base` as that percent of the caster's base mana for their level, and `health` as that percent of their max health when the cast goes off (no power), refusing a cast whose health cost is at least the health left. `cost_extra_max` (a whole number above 0) only with `all_remaining`. The patch moves `client_cost` to `cost_percent` for `pct_base` and to `cost_per_second` for `per_sec`, and sets `power_type` 4294967294 (health, -2) for `health`; `flat` and `all_remaining` stay `cost`. The server JSON carries `cost_kind` (always) and `cost_extra_max` |
| cast shape | `cast_kind`, `channel_tick_ms`, `interrupt_on`, `on_cast`, `defense_type`, `outcomes`, `weapon_hand`, `max_targets`, `area_shape`, `area_center` | skill and talent rows only; an aura row that sets any of these (or the cost columns) fails. `cast_kind`: `instant` (empty), `cast` (needs `client_cast_time_ms` above 0; the server JSON carries it as `cast_time_ms`, and the realm runs the same bar: `SMSG_SPELL_START` with that time, the global cooldown at once, the cost, cooldown and effects when the bar fills, after the cast is checked again; the player's step interrupts it, damage does not, and no other cast starts while it runs), `channel` (needs `client_duration_ms` above 0 and `channel_tick_ms`; the server JSON carries the duration as `channel_ms`, and the realm pays at the start, sends `MSG_CHANNEL_START`, and runs the row's effects at the target once per `channel_tick_ms`, each rolled afresh, until it runs out (`MSG_CHANNEL_UPDATE` 0); a step, a new cast, or the target dying or leaving range ends it early; pushback is not modelled), `next_swing`, `auto_repeat`; a hidden row skips the client-cell needs. The patch gets `on_next_swing`, `channeled` or `auto_repeat`. `interrupt_on`: `move\|damage\|pushback\|kick` or `none`; empty = `move\|pushback\|kick` for `cast` and `channel`, nothing otherwise; the server JSON always carries the resolved list, the patch gets it as `interrupt_flags` (`channel_interrupt_flags` for a channel; move 1, pushback 2, kick 8, damage 16). `on_cast`: `breaks_stealth` (default yes), `starts_auto_attack` (default yes for a `melee` row on an enemy), `resets_swing` (default no), written `key=yes\|no` joined by `;`; the server JSON carries all three resolved. `defense_type`: `melee\|ranged\|magic\|none`; empty = melee for a physical row on an `enemy`, magic for another school on an `enemy`, none otherwise; the patch gets the resolved one as `dmg_class` (none 0, magic 1, melee 2, ranged 3). `outcomes`: `miss`, `dodge`, `parry`, `block`, `crit` = `yes\|no` joined by `;`, exceptions only (`dodge=no;crit=no`); it replaces the old `unavoidable` param (a `weapon_strike` with `unavoidable=` now fails). `dodge=no;parry=no` together still write `unavoidable: true` into the strike's params, which the realm reads. `weapon_hand`: `main` (default), `off`, `both`. `max_targets`: a whole number above 0; empty = 1, or unlimited (`null`) when the row has an `area_shape` or a `realm_radius_yd`; above 1 needs one of those. `area_shape`: `circle\|cone\|chain`; `area_center`: `caster\|target\|ground`, needs `area_shape`. `weapon_hand` and the area's size are server-only; the area's centre picks the implicit targets |
| realm | `realm_cost`, `realm_cooldown_ms`, `realm_gcd_ms`, `realm_range_min`, `realm_range_max`, `realm_radius_yd`, `target` | what the realm does; `realm_cost` must be at least `client_cost`, or the client refuses casts the realm would allow |
| gates (client-mirrored) | `requires_weapon`, `cooldown_category`, `category_cooldown_ms`, `school`, `prevention`, `requires_caster_state`, `requires_target_state` | `requires_weapon`: weapon names joined by `\|` (`dagger`, `one_hand_sword`, `two_hand_sword`, `one_hand_axe`, `two_hand_axe`, `one_hand_mace`, `two_hand_mace`, `fist`, `polearm`, `staff`, `bow`, `gun`, `crossbow`, `thrown`, `wand`), `shield`, or the aliases `any_melee`, `any_ranged`, `one_hand`, `two_hand`; the patch gets `equipped_item_class`, `equipped_subclass_mask`, `equipped_inventory_mask` (a shield cannot mix with weapons). `cooldown_category` is a name, numbered from 10,000 in order of first appearance (no stock Category is that high) into the patch's `category`; it needs `category_cooldown_ms`, equal on every row sharing the name. `school` is `physical\|holy\|fire\|nature\|frost\|shadow\|arcane`, empty = physical, and `school_damage` takes it from the row (a `school=` param fails the build); `prevention` is `silence\|pacify\|none`, empty = pacify for physical and silence otherwise, patched as `prevention_type`. `requires_caster_state` / `requires_target_state` take `defense`, `health_below_20`, `berserking`, `frozen`, `judgement` or `hunter_parry` (patched as `caster_aura_state` / `target_aura_state`). Skill and talent rows only, except `school` (and the `prevention` it implies), which an aura row may set; the others fail on an aura row. Server JSON carries the names and resolved `school` / `prevention`, the patch the numbers |
| gates (realm-only) | `requires` | `key=value` pairs joined by `;`. Keys: `position` (`behind`, `front`), `facing` (`yes`, `no`), `stealth` (`yes`), `combat` (`in`, `out`), `target_state` (`bleeding`, `stunned`, `rooted`, `feared`, `incapacitated`, `poisoned`), `caster_state` (`after_crit`, `after_dodge`, `after_parry`, `after_block`, `after_kill`) which must come with `window_ms` (a whole number above 0), `offhand` (`weapon`), `los` (`yes`, `no`), `usable_while` (`stunned`, `feared`, `rooted`, `charmed`, `silenced`, joined by `\|`). `combat=out` also sets the realm's `requires_out_of_combat` flag. `client_not_in_combat=yes` fails the build unless `requires` has `combat=out`. The old `requires_out_of_combat` column is gone |
| effects | `effect1_kind` to `effect5_kind`, `effect1_params` to `effect5_params` | the realm effects, in order; params are `key=value` pairs joined by `;`, nested keys dotted (`bonus.base=9.5;bonus.per_level=1.4`), lists joined by `\|` |
| spellmods | `spellmodN_type`, `spellmodN_op`, `spellmodN_target`, `spellmodN_value` for N = 1 to 5 | a rank's modifier: `flat` or `pct`, a 1.12.1 client op name such as `COST`, the skill keys it hits joined by `\|`, and the value at this rank. The build resolves the family mask from each named skill's own bit (above) and fails when the mask also reaches a skill of the Calling the cell does not name. Both outputs carry the mods: one skill as `family_bit`, several as `family_mask`; `server_spells.json` adds `target`, the keys |
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
them all. `periodic_damage` also takes `duration_per_combo` (a formula in milliseconds, added to
`duration_ms` once per point; it too makes the row a finisher). Talent Rows' params (#500 reads
them, the build accepts them now): `school_damage` and `weapon_strike` take `controlled_pct=N` (N%
more against a stunned or incapacitated target); `weapon_strike` takes `cleave_pct=N` (a second
enemy in melee reach in front of the caster takes N% of the hit) and the pair
`below_health_pct=H;below_bonus_pct=N` (N% more against a target under H% health; one without
the other fails); `skill_crit_pct` takes `skill=any` for the caster's next hit with any skill.
Vanguard's rows (#504) add four, each a whole number of at least 1: `weapon_strike` takes
`block_value_pct=N` (N% of the caster's shield Block Value added to the strike, skill or
next-swing, on top of a talent's; nothing without a shield) and `heal_pct=N` (the caster heals
N% of what the strike dealt, up to their maximum health); `consume_aura_stacks` takes
`heal_pct=N` (the caster heals N% of the bonus damage the stacks dealt, summed over every
target) and `refund_per_stack=N` (the caster gains N points of their Calling's power per stack
taken, landed or not, scaled as every rage gain is). A `proc`'s `apply_aura` naming a stacking
aura row with no effects of its own (Conviction) adds one stack of it, as an `add_aura_stacks`
of one does, instead of putting up a buff.
Cutthroat's rows (#505) add more, each a whole number of at least 1. A target is bleeding while
it wears a Physical damage over time the caster put on it (Rupture, Garrote).
`add_combo_points` takes `bleeding_bonus=N` (N points more on a bleeding target). `school_damage`
and `weapon_strike` take `bleeding_pct=N` (N% more on a bleeding target, summed with
`controlled_pct` and `below_bonus_pct`), and `kill_heal_pct=N` and `kill_energize=N`, either alone. After a hit that
kills a mob whose kill yields experience, the caster heals N% of their maximum health and gains N
points of their own power; a kill that yields none gives nothing. `periodic_damage` takes
`drop_chance=N`, from 1 to 100: each Physical tick spills the Calling's blood droplet at N%
instead of the Calling's `bleed_drop_chance`, and a Calling with no `bleed_drop` spills none. A
`proc` takes `add_combo_points=N` on an aura row as on a talent: N points on the event's target,
which for a dodge is the mob the character is attacking. The talent kind `pickup_energize` takes
`amount=N`: each droplet picked up gives N of the character's own power, as `pickup_heal_pct`
heals.
Talent kinds that modify a skill:
`skill_crit_damage_pct` (`pct=`, with `builders=yes` only for skills that add combo points, or `skill=key` only for that skill's crits), `skill_armor_ignore` (`skill=`, `amount=` more armour that skill's strikes skip), `no_min_range` (`skill=key`, or `skill=ranged` for every `defense_type=ranged` skill: the realm drops the minimum range; the stock client still refuses inside the row's minimum, its range spell mod moving only the maximum),
`skill_duration_ms` (`skill=`, `amount=` ms added to the skill's auras) and `skill_cost`
(`skill=`, `amount=` flat power off the cost, or `pct=` that percent more), `skill_amount_pct`
(`skill=`, `pct=`; `applies_to=hit|tick|both`, default both, picks the direct hit or the
damage-over-time ticks, and `requires_target_aura=key` counts it only while the caster's aura of
that row is on the target as the blow lands, and `max_distance_yd=N` counts it only on a target within N yards; on a `weapon_strike` a talent's raises the bonus, an aura row's the whole strike, and an aura row with `stack_rule=stack` adds `pct` per stack up to `max_stacks`, its `charges` spent by the skill landing), `mod_armor` (an aura row, `amount=-N`: the target's armour N lower a stack, with `stack_rule=stack`), `extra_strike` (an aura row, `skill=;weapon=;weapon_pct=`: each cast of that skill also strikes its targets for that share of a weapon roll), `damage_pct_vs_aura` (a talent, `aura=key|key;school=;pct=`: that school's damage that much higher on a target wearing one of the auras), `mod_ranged_haste_pct` and `mod_ranged_crit_pct` (aura rows, `pct=`: the ranged weapon shoots that much faster, and a `defense_type=ranged` cast crits that many points more often), `skill_extra_target` (`skill=`, `pct=` chance that
the skill also strikes the nearest enemy in melee reach in front of the caster, for no combo
point) and `skill_bonus_combo` (`skill=`, `pct=` chance of one more combo point, with
`target_state=bleeding` only on a target the caster has a physical bleed on) and `crit_bleed`
(`aura_spell_id=`, `tick_ms=`, `duration_ms=`, `pct=`: a melee crit leaves a bleed of `pct`% of
the average swing, or with `of=crit_damage` of the crit's own damage, a new crit replacing it).
There is no `finisher` kind or flag. A cost is paid in the Calling's power, read from
its `class`; `realm_cost` and `client_cost` stay bare numbers.

**Adding a column** changes both generated schemas and the realm loader, so it goes through
`/grill-me` first (`CLAUDE.md`).

## callings.csv

`data/twilights-eve/callings.csv` holds one row per Calling (ADR 0011, #206); every Calling with
rows in `master.csv` needs exactly one, and the file names no other. `migrate_callings_csv.py`
wrote it once from the old `class` and `armour` cells.

| column | meaning |
|---|---|
| `calling`, `name` | the Calling's key and name (the `calling` row's `name` must match) |
| `icon` | the Calling's icon, a stock path (the first skill's icon to begin with) |
| `power` | `rage`, `energy`, `focus` or `mana`: the bar it runs on, the patch's `power_type` (mana 0, rage 1, focus 2, energy 3) on each of its spells. Focus fills 10 a 2 s tick to 100, like a vanilla pet's |
| `stat_profile` | the vanilla class table its per-level base stats, health and mana, crit and dodge from agility, attack power and mana regen follow: `warrior`, `paladin`, `hunter`, `rogue`, `priest`, `shaman`, `mage`, `warlock`, `druid`. The client's class-keyed tables (skill-line gates, reputation bases, the relic slot, the pet book, the combo-point refusal, the ranged-weapon tutorial) read it too; the wire still carries its class byte, which the client ignores |
| `item_class` | the vanilla class whose `AllowableClass` bit an item it equips must carry, realm and client alike; empty = `warrior`, as every Calling is today |
| `health_pct`, `mana_pct` | percent of the profile's base health and base mana (empty = 100); stamina's and intellect's parts stay whole |
| `energy_per_tick` | energy gained every 2 s regen tick, on an `energy` Calling only (Cutthroat 25); empty = the vanilla 20. Set on any other power, the build fails |
| `bleed_drop`, `bleed_drop_chance` | set together or not at all. `bleed_drop` is an aura row of the Calling with a `pickup` effect (Cutthroat `aura_blood_droplet`); `bleed_drop_chance` is a whole number from 1 to 100, the percent chance that a tick of the Calling's own physical damage over time on a mob spills that droplet near it |
| `armor` | armour kinds joined by `\|`: `cloth`, `leather`, `mail`, `plate`, `shield` |
| `weapons` | weapon kinds joined by `\|` (`one_hand_axe`, `two_hand_axe`, `bow`, `gun`, `one_hand_mace`, `two_hand_mace`, `polearm`, `one_hand_sword`, `two_hand_sword`, `staff`, `fist`, `misc`, `dagger`, `thrown`, `crossbow`, `wand`, `fishing_pole`), and `dual_wield` for a one-hander in the off hand too |
| `spellbook` | the spell book tab's name |
| `blurb` | a sentence or two the Calling Choice popup shows; empty for a Calling the popup never lists |

Both JSON files' Callings carry these as `power`, `stat_profile`, `item_class`, `health_pct`,
`mana_pct`, `energy_per_tick`, `bleed_drop` and `bleed_drop_chance` and `blurb` (each only when set), `armour`, `weapons`, `icon` and `spellbook`. The client patch also has a `callings`
table, `{key, name, icon, spellbook, power_type, class, item_class, spell_family, weapon_mask,
armor_mask, dual_wield}`, which the client (`benilla_formats::twilight_calling`) reads in place of
every class byte (#207): the realm names each character's Calling by key, its own at the roster
and every other player's as they come into view. `UnitClass` answers the Calling's name and its
profile's token, the power bar is its `power`, the spell book files the sheet's spells on a tab
named by `spellbook` with its `icon`, and items are judged on `item_class` and the two masks. A TOML Calling writes `power` and `stat_profile` (a
test Calling may still write `class = N` for both, the vanilla class's table and power).

## The script

`tools/twilights-eve/build_spells.py` reads the CSV, checks that every `icon_id` is a row in
`SpellIcon.dbc` through `tools/twilights-eve/wowdata.py`, and writes both outputs in one pass. It
exits non-zero, writing nothing, on an id the DBCs do not hold, an id that is not a whole number,
a duplicate rank within a Calling, a `realm_cost` below `client_cost`, a visible skill row whose
`client_cooldown_ms` differs from its `realm_cooldown_ms` (an empty cell is 0) unless its `notes`
say `cooldowns differ` and why, since the client draws the sweep from its own value, a missing or unknown
`class` or rows of one Calling that disagree on it, an unknown effect kind or malformed params on
a modelled row (an `energize` without a known `power`, an `add_combo_points` without a positive
whole `amount`, a `per_combo` that is not a formula block), a half-filled spellmod, a `talent`
row with neither `variant_of` nor `row_key`, or a filled tree column (ADR 0028). The DBC check needs `--data` or `WOW_DATA`; `--no-dbc-check`
skips it (the tests use that). Run it by hand after a CSV edit and commit the CSV and both outputs
together, so a diff shows what a cell changed. `--check` writes nothing and exits 1 when the
committed outputs differ from a fresh build; CI runs it with `--no-dbc-check`, which compares the
patch without the fields the DBCs resolve (visual, range, cast time and duration indexes). Tests:
`~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests`.

```
~/.local/bin/python3.15 tools/twilights-eve/build_spells.py --data "$WOW_DATA"
```

`tools/twilights-eve/sheet.py` reads and edits the CSV by `calling/key`. `get` prints a row's
non-empty cells, `set column=value` changes cells, `add-row` writes a new row after another, and
`add-column` extends the header. It rewrites only the records it touches, so the diff shows only
the cells you changed. `--kind` and `--rank` narrow a key shared by several rows (an aura
sharing a skill's key). Its docstring has the full usage.

```
~/.local/bin/python3.15 tools/twilights-eve/sheet.py get acolyte/miasma visual spell_id
~/.local/bin/python3.15 tools/twilights-eve/sheet.py set acolyte/blood_lance visual=24328
```

## The outputs

`server_spells.json` is what the realm loads into its content resource at boot: a list of
Callings, each with its `class`, its skills, its auras and its Talent Rows (`rows`, above), each
skill or choice carrying its realm columns and its effects as `{kind, params, realm_only}`. The realm spawns one entity per ability with the
components `docs/bevy_guidelines.md` names for each `kind`.

`client_dbc_patch.json` is a list of complete spell rows, `{spell_id, fields: {power_type, cost,
stances, cooldown_ms, ..., implicit_targets, spell_family, family_flags, icon, visual, dbc_effects},
spellmods}`. A Skill Choice or a Talent is a spell row like any skill. The patch has no
`talent_tabs` or `talents` key, and the client patches neither `Talent.dbc` nor `TalentTab.dbc`
(ADR 0028). The client's Talents panel (`assets/ui/Talents.xml`) reads each choice's name, icon
and description from its own patched Spell.dbc row.
Every row sends `power_type` (from its Calling's `class`: Warrior rage 1, Rogue energy 3, every
other class mana 0), the row's `name` and, when the row has one, `description`. An aura row's
entry carries its name, look, targets, effects and its `dispel_type`, `mechanic` and
`aura_description` (above). The CSV uses the realm's vocabulary: `weapon=normal` or
`normalized` or `ranged` (a `weapon_strike`'s `armor_ignore.base=;armor_ignore.per_level=` skips that much of the target's armour for the strike, on top of the caster's own, never below none) (the ranged slot's bow, gun, crossbow or thrown weapon, ranged attack power = agility; with `defense_type=ranged` it is never parried and dodged only from the front), the `outcomes` column in place of an `unavoidable` param, a `weapon_strike`'s `weapon_pct=` (the percent of the roll, default 100) and `crit_if_target_in=ground_effect` (a guaranteed crit on a target inside the caster's patch; it takes the row's `school`, so a magic school ignores armour), a one-rank talent row with `target` set as an active talent (its effects are the skill it teaches under its spell id, with the row's `realm_*` columns), `mod_stat` (`stat=strength|agility|stamina|intellect|spirit` with `pct=` of the total, or flat `amount=`; max-health changes move current health with them), `mod_health_regen_out_of_combat` (`pct=`; natural regeneration is a Spirit-based tick every 2 s out of combat), `incapacitate` (`duration_ms`, `break_on_damage`, `drop_combat`: a stun that damage after the cast ends and that drops the target out of combat; on waking, aggro rules apply), `next_cast` (an aura row: `skill=key|key` and any of `instant=yes`, `cast_time_ms=`, `cost_pct=`, `duration_ms=` (added to the aura the skill applies), `becomes=key` (the next shot of the named auto-repeat skill goes off as that skill of the Calling, a hidden one too: Explosive Payload); with `stack_rule=stack` each stack adds the amounts again, up to `max_stacks`, and a cast bar never goes below instant (Rhythmic Draw); an aura row's `proc` also takes `skill=` and `energize_pct_of_base=`: read when the next cast of a named skill starts, used up when it goes off, one of the row's `charges` a cast, or for its whole duration with none; the auras of one `exclusive_group` go together when any is used; the build adds each skill's lowest unshared `SpellFamilyFlags` bit as `family_bits`, and the realm sends the client the casting-time (op 10) and cost (op 14) spell mods while it lasts), `at_max_stacks` (an aura row with `stack_rule=stack`: `becomes=key`; each new application adds a stack, and at `max_stacks` the row `becomes` names replaces it), `ground_area` (on a `target=dest` row, cast with the client's ground point (`TARGET_FLAG_DEST_LOCATION`) within `realm_range_max`: `radius_yd=`, `duration_ms=`, `enemy_aura=key`, `ally_aura=key`; every enemy inside holds the enemy aura row and every friendly player the ally one, put on as they step in and taken off as they leave or the area ends, checked each tick; a new one from the same caster replaces the old; the realm shows it as a DynamicObject named after the row, created as it lands and destroyed as it ends or is replaced, sent to every client in view of it, ADR 0017), `ground_effect` (`radius_yd=`, `tick_ms=`, `duration_ms=`, `amount=` formula, the row's `school`: a patch where the cast ends, the caster's spot after a Charge, striking every enemy inside each tick; one a caster, a new one replacing the old; `ground_spell=key` names a client-visible skill row of the Calling, usually `spellbook=no`, that the build resolves to `ground_spell_id`: the patch's DynamicObject and its tick logs carry that id, and with none they carry the casting row's, ADR 0017; `caster_aura=key` names an aura row of the Calling that the build resolves to `caster_aura_spell_id`, which the caster alone wears while they stand in their own patch, by flat distance to its centre checked each tick: on as they step in or the patch lands under them, off as they step out, the patch ends or a new one replaces it, as Consecrated Ground), aura-row kind `pickup` (a blood droplet: `grants=key` is the aura row of the Calling that a player gets by walking within `radius_yd=` of it, flat; it lands `scatter_min_yd=` to `scatter_max_yd=` from the bleeding mob, lasts the row's `duration_ms`, and the client draws the row's `spell_id` as a DynamicObject; never worn; a Calling's `bleed_drop` column names it), aura-row kinds `periodic_damage` (`tick_ms=`, a `tick` formula, in the row's school), `mod_hit_chance` (`pct=`, negative for more of the unit's swings to miss), `mod_damage_taken_pct` (`pct=`, a whole number from -100 up and not 0, and an optional `school=`, with none meaning every school: more damage taken from others than the aura's caster, or less with a negative `pct`; on the player it changes every blow they take, before an absorb, and on a mob it changes nothing yet, as no other player deals damage) and `mod_healing_taken_pct` (`pct=`), `drain` (`amount=` formula in the row's school at the target, healing the caster `heal_pct=` percent of what it dealt), `absorb` (`pct=`, `pct_of=hit|heal|health_cost`, `duration_ms=`, `radius_yd=`, optional `school=`: a shield of that share of what the cast dealt, healed or cost in health on every friendly player within the radius of the caster, soaking only that school when set; a new one replaces the old), talent kinds `max_health_pct` (`pct=`, after the stat talents) and `healing_taken_pct` (`pct=`, with `below_health_pct=N` only under N% health), aura-row kinds `periodic_heal` (`tick_ms=`, a `tick` formula and/or `pct_of=hit|heal|health_cost;pct=N` of the cast that put it on, over the row's duration) and `mod_cast_speed_pct` (`pct=`: cast bars run 100/(100+pct) as long; the build adds every skill's lowest unshared family bit so the client's bars follow), `heal` (`amount=` formula: heals the target up to its max health; overheal is dropped), `periodic_heal` (`tick_ms=`, `duration_ms=`, a `tick` formula and/or `pct_of=hit|heal;pct=N`, N% of what this cast dealt or healed spread over the ticks; `ally=lowest_health;radius_yd=N;center=target|caster` puts it on the friendly player with the least health share within N yards instead of the cast's target; friendly players are the caster alone until other players exist), `periodic_damage` also takes `leech_pct=N` (each tick heals the caster N% of what it dealt) and `pct_of=hit|heal;pct=N` (and logs its ticks in the row's `school`, or for `apply_aura` the aura row's; the caster's own on themselves never takes them below 1 health), `reset_cooldowns` (`skill=key|key`, up to four: the build resolves them to spell ids; the caster's cooldowns on them end and the client is told), `energize`'s `pct_of_max=` (that percent of the bar's maximum, on top of `amount`), `blink` (`distance_yd=`: the caster sheds their stuns and roots, then is teleported that far along their facing on the next tick, stepping over the ground and stopping short where there is none or it climbs or drops more than 1.5 yd in a 1 yd step; `backward=yes` goes straight back from the facing and keeps it, as Vault), `root` (an aura row: the unit cannot move, still attacks and casts, and damage does not break it), `consume_aura` (with `per_stack=yes` its damage is `amount` once per stack the aura held, and `heal_allies_pct=N;radius_yd=M` then splits N% of the damage dealt among the friendly players within M yards of the target; `amount_pct` defaults to 100; `aura=key` takes that aura row off the target, then pays its outcome: `amount_of=skill;amount_pct=N`, damage of N% of that skill's `school_damage` hit in its school, which the build copies in as `amount` and `school`, and `energize_pct_of_max=N`, N% of the caster's power bar; nothing when the aura is absent; two consumes on one skill run in slot order), `consume_amount_pct` (a talent: `skill=` and `pct=` raise that skill's consume outcomes), `mod_spell_damage_pct` (an aura row: `pct=` more damage from the player's non-physical skills), `skill_amount_pct` on an aura row (`skill=`, `pct=`: the amounts that skill deals rise while it lasts), `periodic_damage` or `mod_run_speed` named by `apply_aura` go on under the aura row's spell id, so another skill (Elemental Convergence) applies the same aura the first does; `mod_run_speed` on a skill (`pct=`, negative for a snare on the enemy target, positive for a sprint, with `apply_aura=key` for the aura row's `pct` and duration; two snares on one unit, the strongest wins), `school_damage` (its school is the row's `school` column, default physical; any non-physical school ignores armour, rolls only the level-based miss and crits for 150%; `crit_pct_at_full_mana=` adds that many crit points at a full mana bar, scaled by its fill after the cost), and `proc` (`on=cast|kill`, `skill=`, `yields_xp=yes`,
`chance=`, then at least one of `apply_aura=key|key` or `heal_pct=` (not both), `energize=` or `add_combo_points=`; `on` is `cast`, `kill`, `dodge`, `finisher`, `block`, `parry`, `crit_taken`, `melee_hit`, `damage_done` (the character's cast or damage-over-time tick damaged a target, once a cast or tick), `periodic_tick` (with `skill=`: a damage over time the character put on with that skill ticks) `moving_for;ms=N` (the character has moved N ms without stopping, and again each further N ms; moving = a step reported in the last 0.75 s), `still_for;ms=N` (N ms with no step; once, until the next step), `crit` (with `skill=`: the character's cast landed a critical strike, once a cast) or `consume` (a `consume_aura` took its aura off a target; `spread=<yd>` puts a fresh copy of that aura, one stack and its row's full duration, on the nearest other living enemy within that many yards of the target), the filter `cost_kind=health` (only a cast that cost health), the shield's `absorb_of=heal|hit|health_cost` (what `absorb_cost_pct` is a share of, the cast's cost by default) and `absorb_school=school` (it soaks only that school), the filter `damage_school=school` (only damage in that school, for `damage_done` and `periodic_tick`), the actions `spread_aura=N` (a copy of the ticking aura, its time left and one stack, on the nearest other enemy within N yards that lacks it), `heal_allies_pct=N;radius_yd=M` (N% of the damage done, per rank, split among the friendly players within M yards of the character), `energize_pct_of_base=N` (N% of the character's base mana, per rank), `apply_aura_target=key` (that aura row on the event's target, per rank), `refund_cost_pct=N` (N% of the triggering cast's cost back, per rank), `repeat_cast=yes` (the triggering skill goes off again at once at the same target, free and off the global cooldown; a repeat never repeats), `cooldown_skill=key;cooldown_ms=-N` (that skill's cooldown N ms shorter, per rank), `splash_pct=N;radius_yd=M` (N% of the damage done, per rank, to every other enemy within M yards of the target) and `absorb_cost_pct=N;absorb_duration_ms=M` (a shield on the caster soaking N% of that cost for M ms, sized when it goes up, a new one replacing the old), the param `at_full_power=absorb` (when the bar was full before the cast paid, the shield goes up and nothing else; otherwise everything else and no shield), the filter `requires_target_aura=key|slowed` (the target wore that aura row's aura, or any snare, as it stood before the cast landed; for a cast's `damage_done` too, so a killing blow still counts), and the action `apply_aura_area=key;radius_yd=N;center=origin` puts that aura row (per rank) on every living enemy within N yards of where the caster stood as they cast, a landed white swing or weapon strike whose target is the mob hit). `while_aura=key` fires a talent's proc only while that aura row is on the player, and `weapon_pct=` deals that percent of the average weapon swing in `school=`. A defensive event's target is the attacker; `damage_attacker.base`/`.per_level` (a formula) or `block_value_pct=` (of the shield's Block Value, per rank) deals damage to it in `school=` (default physical), raised by the `skill_amount_pct` talents of `amount_skill=key`. An `aura` row may carry a `proc` too (`on`, `chance`, `heal_pct`, `energize`, one `apply_aura`, the damage params): it fires only while that aura is on the player. A skill's `grant_aura` (`aura=key`) puts up every effect of that aura row for the row's duration. Aura rows also take `skill_cost` (`skill=`, `amount=`: the skill costs that much less) and `skill_stacks` (`skill=`, `stacks=`: the skill's `add_aura_stacks` adds that many), each used up one `charges` at a time when the skill lands. The defensive stat kinds `parry_pct`, `block_pct` (only with a shield), `block_value_pct` (of the shield's Block Value) and `armor_items_pct` (of the armour from items, not agility's) take `pct=` on a talent, on an aura row, and on a skill with `apply_aura=key` that puts the aura up; block and parry add to the base and are capped at 100%. A talent's `skill_block_value_pct` (`skill=`, `pct=`) adds that percent of the shield's Block Value to the skill's strike, before armour. An `aura` row's effects are `skill_crit_pct` (`skill=`, `pct=`: that skill's crit chance, used up one `charges` at a time when the skill lands) and `mod_run_speed` (`pct=`: the realm sends the new run speed through `SMSG_FORCE_RUN_SPEED_CHANGE` and sends the old one back when it fades). The client applies it
in memory when it loads its spell data (ADR 0011). The build resolves each override's range, cast time and
duration to the stock `SpellRange.dbc`, `SpellCastTimes.dbc` and `SpellDuration.dbc` row that holds
the value (the lowest id on a tie; no match fails the build, and `--no-dbc-check` leaves the raw
values), so the patch carries `range_index`, `cast_time_index` and `duration_index`, which the
client writes over the row's own. Cost is an amount of the row's power (rage is stored in tenths).
A patch entry whose `spell_id` the catalog lacks and is at least 1,000,000 is inserted as a
complete row (name, icon, look, cost, school, cast shape, targets, family, effects); a stock id
the catalog lacks is skipped. The script also reads the realm's
`crates/twilight-realm/content/creature_spells.toml` (`--creature-spells`) and adds one such row
per Creature Spell, `{spell_id, creature, fields}`: its own id (1,900,000 up; one any master.csv
row or another creature spell claims fails the build), `name`, `icon`, `school`, `visual`, cast
time, range, duration, `aura_description` and its effect (a curse, a heal or a snare) as Spell.dbc
numbers (ADR 0008, 0011). The PNGs the `icon` cells name are written,
base64 by name, to `client_art.json`, which the client embeds like the patch: an icon at
`Interface\Icons\Twilight\<name>` is served from there before the MPQs, and the BLP decoders
accept PNG bytes. When a real server plays to a stock client,
the same file feeds an MPQ writer.

## Closed zones

`data/twilights-eve/closed_zones.toml` holds the map (`map = 0`, Azeroth) and the AreaTable zone
ids cut out of the world (`zones = [40]`, Westfall). `tools/twilights-eve/closed_zones.py` reads
it, rolls each ADT chunk's area id up through AreaTable's parent column to its top-level zone,
and writes `data/twilights-eve/closed_chunks.json`: `{"map", "zones", "chunks": [[gx, gy], ...]}`
with `gx = tile_x * 16 + chunk_ix` and `gy = tile_y * 16 + chunk_iy`, sorted, one pair per line.
It needs the game data (`--data` or `WOW_DATA`) and prints the closed count per zone and per
area name. After a TOML edit, run it and commit the TOML and the JSON together:

```
~/.local/bin/python3.15 tools/twilights-eve/closed_zones.py --data "$WOW_DATA"
```

Both the client and the realm compile the JSON in with `include_str!` (ADR 0015), so neither
needs game data to know which chunks are closed. Westfall comes to 12,674 chunks, 8,873 of them
The Great Sea, whose parent area is Westfall.

## Where it stands

As of 2026-10-16 `build_spells.py` generates both outputs from the CSV, and they are committed
(3 Callings, 199 rows). Basic Warrior is fully expressed in the CSV (calling and aura rows; spec
#50); its rows whose effects the realm cannot yet do are `modelled=no`. The realm reads Basic
Warrior from `server_spells.json` (`twilight-realm`'s `generated.rs`, compiled in): any Calling
whose file entry carries a `tier`, which only a `calling` row gives it. The loader reads every
schema #63 field into the content, loaded and not enforced: a skill's `meta` (gates, cost kind,
cast shape, spellmods), an aura's `row` (the aura-only columns), a talent's `rank_spellmods`, and
`hidden_skills` for `client_visible=no` rows, found by `CallingDef::hidden_skill(spell_id)`, and for `spellbook=no` rows, found by key.
Vanguard is fully expressed in the CSV too
(2026-10-17, from Davis's design sheet; its unmodelled mechanics are tickets). Zealot has a
`calling` row that names only its `class`; its tier and skills are still the tables
in `content/callings.toml`, which uses the same vocabulary (`class`, `power_cost`, `energize` with
`power`, `add_combo_points`, `per_combo`). A skill with no effects, or an effect kind the realm
lacks, is left out of the bar and spell book until its effects land. The client applies the patch at load
(`SpellCatalog::apply_dbc_patch`, compiled in), `power_type` included, and writes each gate,
cost-kind, cast-shape and aura field into its Spell.dbc column (ADR 0005, ticket #70). The patch carries no
`family_name` field; adding one is a schema change and goes through `/grill-me`. Talents carry
their `description` and client spellmods, but their realm effects are not yet structured columns,
because `callings.json` never held them.

Since #502, Talent Rows (ADR 0028) have replaced the Talent Trees for every Calling. The sheet has
no `talent_tab` rows and no tree talents, its tree columns are empty, and no Calling has tabs,
seats, prerequisites or talent points. Zealot has no rows yet.
