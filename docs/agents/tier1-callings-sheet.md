# What master.csv needs for each Tier 1 Calling

This guide lists the information needed before a Tier 1 Calling can go into
`data/twilights-eve/master.csv`. It says what to decide for each Calling, and what is already
fixed. Writing the rows is covered by `docs/agents/calling-script-guide.md`, which has the column
rules, the effect vocabulary and the checklist. Column meanings are in `docs/spell_pipeline.md`.
Use this guide to collect the inputs, then hand them to an LLM along with the script guide.

## The four Tier 1 Callings

Each one starts a Line (`GLOSSARY.md`). All five branch from Basic Warrior at level 10.

| Calling key | Name | `class` | Power | Advancement Quest | Talent tabs it borrows from (`source_tab_id` · art) | Where it stands |
|---|---|---|---|---|---|---|
| `vanguard` | Vanguard | 1 Warrior | rage | 64001 | Arms 161 `WarriorArms`, Fury 164 `WarriorFury`, Protection 163 `WarriorProtection` | kit is in `content/callings.toml`; the CSV has only its `calling` row |
| `cutthroat` | Cutthroat | 4 Rogue | energy | 64005 | Assassination 182 `RogueAssassination`, Combat 181 `RogueCombat`, Subtlety 183 `RogueSubtlety` | no kit; its trainer and quest exist |
| `initiate` | Initiate | 8 Mage | mana | 64006 | Arcane 81 `MageArcane`, Fire 41 `MageFire`, Frost 61 `MageFrost` | no kit; its trainer and quest exist |
| `ranger` | Ranger | 3 Hunter | mana | 64007 | Beast Mastery 361, Marksmanship 363 `HunterMarksmanship`, Survival 362 `HunterSurvival` | sheet in the CSV; plays once ranged combat lands |
| `acolyte` | Acolyte | 5 Priest | mana | 64008 | Discipline 201, Holy 202 `PriestHoly`, Shadow 203 `PriestShadow` | sheet in the CSV; Creeping Blight plays |

The tab ids and art names above come from the game's `TalentTab.dbc`. Every row of a Calling
carries the same `class`, and that `class` decides the power bar. A skill taken from another
class keeps the Calling's `class`, sets `borrowed_from_class`, and needs a `borrowed_family_bit`
(ADR 0003).

## Fixed for every Tier 1 Calling

- `tier=1`, `tier_cap=30`. `L` in every formula is the level within the Tier, 1 to 30.
- At most **8 skills** (`GLOSSARY.md`, Calling). Vanguard has five.
- A Talent Tree of **2 or 3 tabs**, sized so about 29 points reach the middle rows
  (first-slice spec, story 33).
- The Tier Bonus is added on top of Basic Warrior's (zero) and the race table. Vanguard's is
  `strength=12;agility=6;stamina=12;intellect=0;spirit=3`; use it as the size reference.
- `status=placeholder` on every new row. Claude moves a row to `wip`, and to `done` only after
  Davis has played it.

## What to decide for each Calling

These are the inputs to collect. Anything not supplied becomes a guess in `assumptions`.

### 1. The `calling` row

| Input | Column | Notes |
|---|---|---|
| Tier Bonus | `tier_bonus` | all five stats, `stat=N` joined by `;` |
| Armour | `armour` | from `cloth`, `leather`, `mail`, `plate`, `shield`, joined by `\|` |
| Form | `shapeshift_form` | the stance the client needs for the kit's stance-gated spells (Battle Stance is 17). Leave it empty if no skill is gated by a stance |
| Concept line | `notes` | one sentence on how the Calling plays |

### 2. Each skill (up to 8)

| Input | Columns |
|---|---|
| What it does, in one line, hidden parts included | `description` |
| The real spell it borrows its button from | `client_spell_id` (and `icon_id` if the icon should differ) |
| Cost, cooldown, GCD, cast time, range | `client_*` and `realm_*` pairs, with `realm_cost` ≥ `client_cost` |
| How it casts | `cast_kind`, plus `channel_tick_ms` for a channel |
| Who it hits | `target`; for an area also `area_shape`, `area_center`, `realm_radius_yd`, `max_targets` |
| Its damage school | `school` (spells must set this; physical is the default) |
| Gates | `requires_weapon`, `requires`, `client_stances`, `requires_*_state` |
| Its numbers | an `effectN_kind`/`effectN_params` per effect, each amount a `base`/`per_level`/`per_ap` formula |
| Anything that lasts | a separate `aura` row: duration, stacking, dispel type, mechanic, what breaks it |

### 3. Each talent tab (2 or 3)

`key`, display `name`, `tab_id` (a new id, since the build does not check it; Basic Warrior uses
900), `background` (the art name from the table above) and `tab_order`.

### 4. Each talent

| Input | Columns |
|---|---|
| Seat | `tab`, `tab_row`, `tab_column`, `prereq_key`/`prereq_rank` |
| Rank count | `max_rank`, plus one row per rank |
| The real talent it borrows | `talent_id`, `source_tab_id`, each rank's `client_spell_id` |
| What each rank does | effects, or `spellmodN_*` groups aimed at the Calling's skill keys |
| Tooltip per rank | `description` |

## What each Calling will need that the game does not have yet

The game runs only the effect kinds listed in the script guide. A row that needs anything else is
written `modelled=no` with a descriptive kind, and it shows as WIP in the game. Expect these gaps:

- **Cutthroat.** Combo points and finishers work (`add_combo_points`, `per_combo`). Stealth is an
  aura with `grants_form=30`; stealth openers set `client_stances` to the stealth form and
  `requires: stealth=yes`. Poisons, dodge-triggered procs and energy regeneration talents have no
  kind yet.
- **Initiate.** Spell damage is `school_damage` with `school` set; casts use `cast_kind=cast`.
  Mana costs often use `cost_kind=pct_base`. Snares, roots, freezes, absorb shields, Polymorph and
  Blink have no kind yet. Spell power scaling does not exist, so formulas use `base` and
  `per_level` only.
- **Ranger.** Ranged attacks need `cast_kind=auto_repeat` and `requires_weapon=any_ranged`. The
  realm has no Auto Shot or ranged weapon damage yet. Pets are out of scope.
- **Acolyte.** Health costs use `cost_kind=health` (a share of max health, refused if it would
  kill). Heals, drains, channels and ground areas have no kind yet. Ally effects reach friendly
  players in range, the caster included.
- **Vanguard.** Its kit already exists in `content/callings.toml`. Moving it into the CSV means
  copying the TOML's numbers row for row, which is a migration, not a design task. The TOML's
  inline params map to the CSV vocabulary: `unavoidable` becomes `outcomes=dodge=no;parry=no`,
  and lasting effects become aura rows.

List each new kind in the script's summary. Each one becomes a realm ticket.

## Order of work

1. Collect the inputs above for one Calling. Cutthroat comes first, because it is the most
   complete in the vocabulary.
2. Hand them to an LLM with `docs/agents/calling-script-guide.md`. It returns an append script and
   a summary of its guesses.
3. Run the script, then `~/.local/bin/python3.15 tools/twilights-eve/build_spells.py --data
   "$WOW_DATA"`. Fix any rows the build rejects, and commit the CSV with both outputs.
4. Davis plays the Calling. Rows that work move to `done`.
