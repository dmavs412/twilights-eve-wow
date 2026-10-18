# Writing a script that adds a Calling to master.csv

This guide is for an LLM that is asked to write a Python script appending one Calling (a class, such as Cutthroat) to
`data/twilights-eve/master.csv`. Read all of it before writing code. The schema is spec #63; the build
(`tools/twilights-eve/build_spells.py`) checks every rule below and rejects the sheet on the first error.
The design inputs for each Tier 1 Calling (class, tabs, Tier Bonus, kit) are in `docs/agents/tier1-callings-sheet.md`.

## What we want from you

One Python script, plus a short summary. The script appends that Calling's rows: its `calling` row, its `talent_tab` rows,
skills, auras, hidden helper abilities and every talent rank. The summary lists every guess you made and every effect
the game cannot run yet.

## Rules for the script

1. **Only add rows.** Never rewrite existing rows. Do not round-trip the file through
   `pandas.read_csv` and `to_csv`: that turns `78` into `78.0` and changes line endings on every row. Use the `csv` module:
   read the header line, build each new row as a dict, and open the file in append mode
   (`newline=''`, `lineterminator='\n'`).
2. **Write by column name, taken from the file's header.** Do not hard-code column order. If a row dict has a key that is not in the
   header, raise an error. Do not add the column. A new column is a schema change and only a person makes one.
3. **Empty means default.** Leave a cell empty unless you mean something other than its default.
   Write whole numbers without `.0` (`1757`, not `1757.0`).
4. **Refuse to run twice.** If any row already has this Calling's key in `calling`, stop with an error.
5. **Plain values only.** Write no formulas, no arithmetic in cells, and no new effect kinds on a `modelled=yes` row (see below).

## Row kinds

`kind` is one of these:

| kind | one per | purpose |
|---|---|---|
| `calling` | Calling | names the Calling. `calling` and `key` are both the Calling's key. |
| `talent_tab` | tab | `key`, `name`, `tab_id`, `background`, `tab_order` |
| `skill` | ability | something the player casts, or a hidden ability something else triggers |
| `aura` | lasting effect | every buff, debuff, DoT, stun or form. Skills apply it by key. |
| `talent` | talent **rank** | one row for each rank, 1 to `max_rank` |

Every row carries the following:
- `calling`: the Calling's key.
- `class`: the 1.12.1 class id the Calling stands on (1 Warrior, 2 Paladin, 3 Hunter, 4 Rogue, 5 Priest, 7 Shaman,
  8 Mage, 9 Warlock, 11 Druid). It is the same on every row and decides the power bar (Warrior rage, Rogue energy, others
  mana).
- `key`: snake_case and unique within the Calling.
- `name`
- `status=placeholder`: always, for rows you write.
- `modelled`: see below.

## Ids: never invent one

- **`client_spell_id`** is a **real 1.12.1 spell** whose client row the ability borrows: its icon, animation and
  button. Pick the closest real spell of the Calling's `class` (e.g. Sinister Strike Rank 1 is 1752, Eviscerate Rank 1 is 2098,
  Rupture Rank 1 is 1943, Evasion is 5277, Stealth Rank 1 is 1784). The build checks every id against the game's
  DBC files and fails on one that does not exist. If you are unsure of an id, still write your best real one and say so in
  `assumptions`.
- **Hidden abilities** never appear in the spell book; a proc or talent triggers them (an extra cleave
  hit, a bleed proc). They have `client_visible=no`, **no** `client_spell_id`, no `client_*` cells, and a
  `realm_spell_id` you pick: a whole number of at least 1,000,000, unique across the sheet. Before choosing, read the file to find
  the highest one already used.
- **Talents:** `talent_id` is a real `Talent.dbc` id. Each rank's `client_spell_id` is that talent's real spell for
  the rank. Use the closest real talent of the class.
- **Tabs:** `tab_id` is a real `TalentTab.dbc` id and `background` its art name. Rogue's tabs are 182
  `RogueAssassination`, 181 `RogueCombat` and 183 `RogueSubtlety`. A talent's `source_tab_id` is the real tab its
  `talent_id` belongs to. Its `tab` is the **key** of one of this Calling's `talent_tab` rows (`skirmisher`, not
  `Skirmisher`).

## modelled and status

- `modelled=yes` only when every effect kind on the row is in the vocabulary below, with valid params.
  Otherwise use `modelled=no`. The game then shows the row as work in progress. That is expected and fine.
- On a `modelled=no` row you may use a new, descriptive effect kind (`proc_on_dodge`). List every such kind in your
  summary; each one is future work.
- `status` is always `placeholder`. Never write `wip` or `done`.

## Columns by group

### Talent seat
- `tab`, `tab_row` (1 up), `tab_column` (1 to 4), `max_rank` (1 to 5, the same on every rank row).
- `prereq_key` and `prereq_rank`: another talent this one requires.
- Every rank from 1 to `max_rank` must have a row. Ranks share the seat and differ in numbers and `description`.

### Client-facing cost and timing (`client_*`) and realm values (`realm_*`)
- `client_cost`, `client_cooldown_ms`, `client_gcd_ms`, `client_cast_time_ms`, `client_range_min`, `client_range_max` and
  `client_duration_ms` are what the button shows. The `realm_*` versions are what the game enforces.
- `realm_cost` must be at least `client_cost`.
- Fill both columns of a pair, usually with the same values.

### Cost
- `cost_kind`: `flat` (default), `pct_base` (percent of base mana), `all_remaining` (pay the cost, then the rest of the bar
  up to `cost_extra_max`), `per_sec` (channel drain) or `health`.

### Gates the client checks (it greys the button)
- `requires_weapon`: weapon names joined by `|`. The names are `dagger`, `one_hand_sword`, `two_hand_sword`, `one_hand_axe`,
  `two_hand_axe`, `one_hand_mace`, `two_hand_mace`, `fist`, `polearm`, `staff`, `bow`, `gun`, `crossbow`, `thrown`,
  `wand` and `shield`. Group names are `any_melee`, `any_ranged`, `one_hand` and `two_hand`.
- `cooldown_category` + `category_cooldown_ms`: give rows that share a cooldown the same name and the same time.
- `school`: `physical` (default), `holy`, `fire`, `nature`, `frost`, `shadow` or `arcane`.
- `prevention`: `silence`, `pacify` or `none`. Leave it empty to follow the school (physical rows are pacified, spells silenced).
- `requires_caster_state` / `requires_target_state`: only states the client knows, such as `defense` (just dodged,
  parried or blocked) and `health_below_20`. Anything else goes in `requires`.
- `client_stances`: a stealth-only ability lists the stealth form (30) here **and** sets `requires: stealth=yes`.
- `client_not_in_combat=yes` must come with `requires: combat=out`.

### Gates only the game checks: the `requires` cell
Write `key=value` pairs joined by `;`, and lists joined by `|`:

| key | values | example |
|---|---|---|
| `position` | `behind`, `front` | Backstab: `position=behind` |
| `facing` | `yes`/`no` (on by default for melee) | |
| `stealth` | `yes` | Ambush, Garrote |
| `combat` | `in`, `out` | Stealth: `combat=out` |
| `target_state` | `bleeding`, `stunned`, `rooted`, `feared`, `incapacitated`, `poisoned` | |
| `caster_state` | `after_crit`, `after_dodge`, `after_parry`, `after_block`, `after_kill`, with `window_ms=` | Riposte: `caster_state=after_parry;window_ms=5000` |
| `offhand` | `weapon` | needs dual wield |
| `los` | `yes`/`no` | |
| `usable_while` | `stunned`, `feared`, `rooted`, `charmed`, `silenced` | |

### How it casts
- `cast_kind`: `instant` (default), `cast` (needs `client_cast_time_ms`), `channel` (needs `client_duration_ms`
  and `channel_tick_ms`), `next_swing` (replaces the next melee swing), `auto_repeat` (Shoot).
- `interrupt_on`: `move`, `damage`, `pushback` or `kick`, joined by `|`, or `none`. Leave it empty for the default.
- `on_cast`: `breaks_stealth=no` (Sap, Distract), `starts_auto_attack=no` (Gouge, Blind), `resets_swing=yes`.
- `defense_type`: `melee`, `ranged`, `magic` or `none`. Leave it empty; it follows from the school and the target.
- `outcomes`: exceptions only, such as `crit=no` or `dodge=no;parry=no;block=no`.
- `weapon_hand`: `main` (default), `off` or `both`.
- `target`: `enemy`, `self` or `friend`. For an area: `max_targets`, `area_shape` (`circle`, `cone` or `chain`), `area_center`
  (`caster`, `target` or `ground`) and `realm_radius_yd`.

### Auras (aura rows only)
- Anything that lasts is an `aura` row with `duration_ms`. The skill applies it with an effect whose params include
  `apply_aura=<aura key>`: one key, and the aura row must hold an effect of the same kind as the skill's effect
  (Charge: `effect3_kind=stun`, `effect3_params=apply_aura=charge_stun`; the aura row has `effect1_kind=stun`). Put
  the lasting effect's params and `duration_ms` on the aura row, not on the skill's effect. An incapacitate that
  damage ends is `break_on=damage=yes` on its aura row. Set `target=enemy` on a debuff row.
- Aura-only columns on any other row kind fail the build.
- `max_stacks` (default 1).
- `stack_rule`: `refresh` (default), `replace` (a finisher's new strength replaces the old), `ignore` or
  `independent`.
- `unique_per`: `caster` (default) or `target`.
- `exclusive_group` + `group_scope` (`caster` or `target`): only one aura of the group at a time. Examples: one poison per hand, one Seal.
- `dispel_type`: `magic`, `curse`, `disease`, `poison` or `none`. `mechanic`: `stun`, `root`, `snare`, `fear`,
  `polymorph`, `incapacitate`, `silence`, `disarm`, `bleed`, `charm`, `sleep` or `shield`.
- `aura_positive`, `aura_hidden`, `cancelable`, `persist_death`: `yes` or `no`. Leave them empty for the defaults.
- `aura_description`: the text shown on the buff itself.
- `break_on`: `damage=yes`, `damage_threshold=N`, `attack=yes`, `move=yes`, `stealth_break=yes` or `leave_combat=yes`.
- `grants_form`: a form id (Stealth is 30).

### Talent spellmods: changes to other skills' numbers
- Five groups, `spellmodN_type` (`flat` or `pct`), `spellmodN_op`, `spellmodN_target` and `spellmodN_value`, for N = 1 to 5.
- `target` is skill keys joined by `|`.
- `op` is one of `DAMAGE`, `DURATION`, `THREAT`, `EFFECT1`, `CHARGES`, `RANGE`, `RADIUS`, `CRITICAL_CHANCE`,
  `ALL_EFFECTS`, `NOT_LOSE_CASTING_TIME`, `CASTING_TIME`, `COOLDOWN`, `EFFECT2`, `COST`, `CRIT_DAMAGE_BONUS`,
  `RESIST_MISS_CHANCE`, `JUMP_TARGETS`, `CHANCE_OF_SUCCESS`, `ACTIVATION_TIME`, `EFFECT_PAST_FIRST`,
  `CASTING_TIME_OLD`, `DOT`, `EFFECT3` or `SPELL_BONUS_DAMAGE` (the 1.12.1 client's own list; there is no
  `GLOBAL_COOLDOWN` or `HASTE`).
- The build fails if the targets' combined family mask also reaches a skill of the Calling the cell did not name.
- Example, "Sinister Strike costs 6 less": `flat`, `COST`, `sinister_strike`, `-6`. Durations are in ms.

### Effects
- `effect1_kind` to `effect5_kind`, each with `effectN_params` (`key=value;key=value`, nested keys dotted, lists `|`).
- Effects 1 to 3 may show on the client; 4 and 5 are realm-only. `description` must still mention them.

### Text
- `description` is the tooltip. It must describe everything the row does, hidden parts included.
- `notes` is for designers.
- `assumptions` lists every guess on that row: an unsure id, a placeholder number, a borrowed icon.

## Effect vocabulary the game runs today (allowed on `modelled=yes`)

`add_combo_points`, `aura_attack_power`, `charge`, `energize`, `healing_reduction`, `incapacitate`, `judgement`,
`mod_crit_chance`, `mod_crit_damage`, `mod_health_regen_out_of_combat`, `mod_run_speed`, `mod_stat`,
`periodic_damage`, `proc`, `proc_on_cast`, `proc_on_kill`, `school_damage`, `seal`, `skill_amount_pct`,
`skill_crit_pct`, `skill_rage`, `stun`, `weapon_strike`.

**Formulas.** An amount is a block, `name.base`, `name.per_level` and `name.per_ap`, computed as `round(base + per_level × level +
per_ap × attack power)`. Never put arithmetic in a cell.

The params the build checks:
- **`weapon_strike`:** `weapon=normal` or `normalized` (never `W`). Bonus damage is a `bonus.*` formula block.
- **`energize`:** `power=rage|energy|mana` and an `amount.*` block.
- **`add_combo_points`:** `amount=` a positive whole number (at most five points are held).
- **Finishers:** there is no finisher kind. A damage or duration effect with a `per_combo.*` block (`per_combo.base`,
  `per_combo.per_level`, `per_combo.per_ap`) is a finisher. It needs combo points and spends them all.
- **`mod_stat`:** `stat=strength|agility|stamina|intellect|spirit` with `pct=` or `amount=`.
- **`proc`:** `on=cast|kill`, optional `skill=` and `chance=`, then exactly one of `apply_aura=key|key` or
  `heal_pct=`.

Any other trigger (on dodge, on crit, on a finisher) or proc rate is not in the vocabulary yet. Write it as a
descriptive kind on a `modelled=no` row and list it in the summary.

## A worked example (Rupture)

Two rows: a skill that spends combo points, and the bleed aura it applies.

```
kind=skill  calling=cutthroat class=4 key=rupture name=Rupture rank=1 client_spell_id=1943
            client_cost=25 realm_cost=25 client_range_max=5 realm_range_max=5 target=enemy
            effect1_kind=periodic_damage
            effect1_params=apply_aura=rupture_bleed;per_combo.base=2000;per_combo.per_level=0
            modelled=no status=placeholder
            description=Finishing move that makes the target bleed. Lasts longer per combo point.
            assumptions=Duration per point and bleed damage are placeholders.
kind=aura   calling=cutthroat class=4 key=rupture_bleed name=Rupture client_spell_id=1943
            duration_ms=8000 mechanic=bleed stack_rule=replace dispel_type=none
            effect1_kind=periodic_damage effect1_params=tick_ms=2000;amount.base=10;amount.per_level=1
            modelled=no status=placeholder
```

## Checklist before you hand back

- [ ] The script only appends, writes by header name, and refuses unknown columns and a second run.
- [ ] There is one `calling` row and one `talent_tab` row per tab, and talents' `tab` names a tab key.
- [ ] Every talent has every rank from 1 to `max_rank`, with the same seat on each rank.
- [ ] Every `client_spell_id`, `talent_id` and `tab_id` is a real 1.12.1 id. Every guess is in `assumptions`.
- [ ] Hidden abilities have `client_visible=no`, a unique `realm_spell_id` of 1,000,000 or more, and no `client_*` cells.
- [ ] Every lasting effect is an `aura` row applied by key.
- [ ] `modelled=yes` only where every effect kind is in the vocabulary. Every row has `status=placeholder`.
- [ ] No cell holds `1.0`-style whole numbers, arithmetic, or `W`.
- [ ] The summary lists the guesses and the new effect kinds.
