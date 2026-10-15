# Vanilla 1.12 combat and XP rules for the Local Realm

Reference for ticket #4 (One mob, one kill) and the combat, XP, aggro and death rules of the
Twilight's Eve slice (#1, #7, #11, #15). Every rule cites the function in the reference servers
it was read from, and the worked test vectors can be copied into realm unit tests.

**Sources (read at these commits):**

- **vmangos** `vmangos/core` @ `53d24d5c` (2026-10-07), paths under `src/game/`. World data comes
  from the vmangos sqlite world DB (release `db-sqlite-4641790`, `mangos.sqlite`).
- **cmangos** `cmangos/mangos-classic` @ `8ec338a1` (2026-08-31), paths under `src/game/`.

**Which one to follow.** Use **vmangos** as the primary reference. It targets patch 1.12 by
default, its world DB is the one the spawn import (#7) reads, and it has the low-level-mob
reduction that matters for the level 1–6 Crossroads camps. Where cmangos disagrees, the
difference is listed in [§14](#14-where-vmangos-and-cmangos-disagree) and next to the rule.

**Data files** (generated from the vmangos world DB, committed next to this doc):

| file | what |
|---|---|
| `data/twilights-eve/xp_table.json` | XP to next level, levels 1–60 (`player_xp_for_level`) |
| `data/twilights-eve/warrior_level_stats.json` | Warrior base health/mana and agility→crit/dodge divisors per level (`player_classlevelstats`, `player_crit_per_agility`, `player_dodge_per_agility`), naked base stats per race per level 1–60 (`player_levelstats`), and derived check rows |
| `data/twilights-eve/mob_base_stats.json` | `creature_classlevelstats` for mob unit classes 1 (warrior), 2 (paladin), 8 (mage), levels 1–63 |

**Conventions used in the vectors.**

- Chances are in **basis points** (bp, 1/100 of a percent, so 500 = 5.00%), as vmangos computes
  them: `int32(chance_percent * 100)`, truncated toward zero. The roll is `urand(0, 9999)`, and an
  outcome happens when `roll < running_sum`. (cmangos rolls `urand(1, 10000)` and tests
  `roll <= running_sum`, which is the same thing shifted by one.)
- A Rust implementation should do the hit-table arithmetic in integer bp, or in `f64` and then
  truncate. The expected values below are exact-arithmetic results truncated to bp. vmangos's own
  `f32` maths can be one bp lower in a few cases; tests should not pin such cases.
- Skill values: a warrior's weapon skill and defence are always **5 × level** in this slice (no
  skill-up grind, see [§2](#2-weapon-skill-and-defence)). A mob's weapon skill and defence are
  **5 × its level**.
- "L10 human warrior" means the naked base stats from `player_levelstats`: str 33, agi 26, sta 31
  (see `warrior_level_stats.json`, `naked_checks`).

---

## 1. Melee attack table (white swings)

**Source:** vmangos `Objects/SpellCaster.cpp` `SpellCaster::RollMeleeOutcomeAgainst`,
`SpellCaster::GetMeleeMissChance`; `Objects/Unit.cpp` `Unit::GetUnitDodgeChance`,
`GetUnitParryChance`, `GetUnitBlockChance`, `GetUnitCriticalChance`,
`GetGlancingBlowDamageMultiplier`, `CalculateMeleeDamage`. cmangos `Entities/Unit.cpp`
`Unit::RollMeleeOutcomeAgainst`, `CalculateEffectiveMissChance`, `CalculateEffectiveDodgeChance`,
`CalculateEffectiveParryChance`, `CalculateEffectiveBlockChance`, `CalculateEffectiveGlanceChance`,
`CalculateEffectiveCritChance`, `CalculateEffectiveCrushChance`, `CalculateGlanceAmount`.

One roll, one table. Each outcome takes a slice of 0..9999 in this order, and the remainder is a
normal hit:

```
vmangos order:  miss → dodge → parry → glancing → block → crit → crushing → hit
cmangos order:  miss → dodge → parry → block → glancing → crit → crushing → hit
```

The order changes which outcome is pushed off the table when the slices add up to more than
10000. That does not happen at the level spreads in this slice.

Definitions used below:

```
A_skill   = attacker weapon skill            (player: 5*L_player, mob: 5*L_mob)
A_max     = 5 * attacker level
V_def     = victim defence skill            (player: 5*L_player, mob: 5*L_mob)
V_max     = 5 * victim level
skillDiff = A_skill - V_max                  (used for dodge / parry / block)
fullDiff  = A_skill - V_def                  (used for miss and crit)
capped    = min(A_max, A_skill) - V_max      (used for parry and crit vs mobs)
```

### 1.1 Player attacking a mob (vmangos)

| outcome | chance | notes |
|---|---|---|
| **miss** | `5% − fullDiff × 0.1%` if `fullDiff ≥ −10`; `5% − fullDiff × 0.2%` if `fullDiff < −10`. Then × `L_mob/10` if `L_mob < 10`. Then − hit% from gear/talents, but the first 1% of +hit is ignored when `fullDiff < −10`. Clamp 0..60%. | dual-wield +19% not needed |
| **dodge** | `500 − 10 × skillDiff` bp, then × `L_mob/10` if `L_mob < 10` | mob base dodge 5% (`GetUnitDodgeChance`) |
| **parry** | only if the player is in front of the mob (`HasInArc`). `500 − 20 × capped` bp, or `500 − 60 × capped` bp when `capped < −10`; then × `L_mob/10` if `L_mob < 10` | mob base parry 5%; `CREATURE_FLAG_EXTRA_NO_PARRY` disables |
| **glancing** | `(10 + (V_def − min(A_skill, A_max)) × 2)%`, clamped 0..40% | players (and their pets) vs non-player victims only, white swings and next-melee abilities only. No level reduction. |
| **block** | only from the front, and only a mob with a shield. `500 − 10 × skillDiff` bp, **capped at 500** for mobs, then × `L_mob/10` if `L_mob < 10` | mob base block 5% with a shield (`creature_equip_template`); `CREATURE_FLAG_EXTRA_NO_BLOCK` disables. The realm's mob data has no equipment, so a template blocks only when its content says `shield = true`. |
| **crit** | player sheet crit (§5.3, which already contains `(A_skill − A_max) × 0.04%`) plus `fullDiff × 0.04%` if `fullDiff > 0`, else `+ (min(A_max, A_skill) − V_def) × 0.2%`. Floor 0. | not reduced for low-level mobs |
| **crushing** | never (players cannot crush) | |
| **hit** | the rest | |

Damage per outcome (vmangos `Unit::CalculateMeleeDamage`):

- **crit**: × 2.0 (`(200 + SPELL_AURA_MOD_CRIT_PERCENT_VERSUS) / 100`).
- **glancing**: × a uniform random multiplier in `[low, high]`, where `d = V_def − A_skill`
  (uncapped skill), `low = clamp(1.3 − 0.05·d, 0.01, 0.91)`, `high = clamp(1.2 − 0.03·d, 0.2, 0.99)`
  (`Unit::GetGlancingBlowDamageMultiplier`). Warriors are not "wand users", so the caster penalty
  does not apply.
- **block**: damage − victim block value. A mob's block value is
  `L_mob / 2 + str / 20` (`Creature::GetShieldBlockValue`).
- **dodge, parry, miss**: 0 damage.
- Armour (§3) is applied to the raw swing **before** the outcome multipliers.

### 1.2 Mob attacking a player (vmangos)

| outcome | chance |
|---|---|
| **miss** | `5% − fullDiff × 0.04%`, clamped 0..60% (the victim is a player, so always the 0.04 factor) |
| **dodge** | player's `PLAYER_DODGE_PERCENTAGE` (§5.4) in bp `− 4 × skillDiff`. **Can't dodge from behind** against a player victim. |
| **parry** | player's `PLAYER_PARRY_PERCENTAGE` in bp `− 4 × skillDiff`, only if the player has the Parry skill (warriors train it; see the note under §5.4) and a weapon, and the attacker is in front |
| **block** | player's `PLAYER_BLOCK_PERCENTAGE` in bp `− 4 × skillDiff`, only with a shield equipped and an unbroken shield, the weapon not sheathed, and the attacker in front |
| **crit** | mob base 5% `+ fullDiff × 0.04%` (`GetUnitCriticalChance`). Always crit a sitting player. |
| **crushing** | `t = A_max − min(V_def, V_max)`; if `t ≥ 15`, chance = `(2t − 15)%`. Mobs only, not pets. |
| **hit** | the rest |

Crushing blow damage is × 1.5 (`totalDamage += totalDamage / 2`). Note that crushing starts at
**3 levels** above a player with full defence (t = 15 → 15%), not 4. At +4 it is 25%, at +5 35%.

The player's dodge, parry and block fields that the table reads already include the
`(def − 5L) × 0.04%` term, and that term is 0 in this slice.

**Parry haste** (vmangos `Unit::AttackerStateUpdate`, after the outcome): when the victim parries,
its own swing timer is cut. With `T` = the victim's full attack time and `t` = the time left on its
timer: if `0.2T < t ≤ 0.6T` the timer is set to `0.2T`; if `t > 0.6T` it drops by `0.4T`. This
applies to players and mobs alike.

### 1.3 Test vectors: player attacking a mob

Player = L10 human warrior, weapon skill 50, sheet crit = `26 / 5.20021 = 4.9998%` → 499 bp.
The mob is a normal mob attacked from the front. Each cell is the slice size in bp; the final
column is the `[start, end)` of the hit slice.

| case | miss | dodge | parry | glance | block | crit | hit slice |
|---|---|---|---|---|---|---|---|
| P10 vs M10 | 500 | 500 | 500 | 1000 | 500 | 499 | [3499, 10000) |
| P10 vs M11 | 550 | 550 | 600 | 2000 | 500 | 399 | [4599, 10000) |
| P10 vs M12 | 600 | 600 | 700 | 3000 | 500 | 299 | [5699, 10000) |
| P10 vs M13 | 800 | 650 | 1400 | 4000 | 500 | 199 | [7549, 10000) |
| P10 vs M14 | 900 | 700 | 1700 | 4000 | 500 | 99 | [7899, 10000) |
| P10 vs M7 (low-level mob, ×0.7) | 245 | 245 | 140 | 0 | 245 | 559 | [1434, 10000) |
| P10 vs M5 (×0.5) | 125 | 125 | 0 | 0 | 125 | 599 | [974, 10000) |

Level 1–5 cases (player sheet crit 5.00% = 500 bp: L1 human, agi 20 / 4.0):

| case | miss | dodge | parry | glance | block | crit | hit slice |
|---|---|---|---|---|---|---|---|
| P1 vs M1 (×0.1) | 50 | 50 | 50 | 1000 | 50 | 500 | [1700, 10000) |
| P3 vs M1 (×0.1) | 40 | 40 | 30 | 0 | 40 | 540 | [690, 10000) |
| P5 vs M5 (×0.5) | 250 | 250 | 250 | 1000 | 250 | 500 | [2500, 10000) |

Level 60 vs a boss-level mob (sheet crit 5.00%), for the vanilla "+3 levels" numbers people quote:

| case | miss | dodge | parry | glance | block | crit |
|---|---|---|---|---|---|---|
| P60 vs M63, +0% hit | 800 | 650 | 1400 | 4000 | 500 | 200 |
| P60 vs M63, +1% hit | 800 (the first 1% is ignored) | | | | | |
| P60 vs M63, +2% hit | 700 | | | | | |
| P60 (skill 305) vs M63 | 600 | 600 | 1400 | 4000 | 500 | 220 |

Worked example, P10 vs M13: `fullDiff = 50 − 65 = −15 < −10`, so miss = 5 + 15 × 0.2 = 8%.
`skillDiff = −15`, so dodge = 500 + 150 = 650. `capped = −15 < −10`, so parry = 500 + 900 = 1400.
Glance = 10 + 15 × 2 = 40% (at the cap). Block = 650, capped at 500. Crit = 4.9998 − 15 × 0.2 =
1.9998% → 199. The running sum is 800, 1450, 2850, 6850, 7350, 7549, so a roll of 7548 is a crit
and 7549 is a hit.

Roll-to-outcome vectors (P10 vs M10): roll 0 → miss, 499 → miss, 500 → dodge, 1499 → parry,
1500 → glancing, 2499 → glancing, 2500 → block, 3000 → crit, 3498 → crit, 3499 → hit, 9999 → hit.
From behind the mob (no parry, no block): miss [0,500), dodge [500,1000), glance [1000,2000),
crit [2000,2499), hit [2499,10000).

Glancing damage band by `d = V_def − A_skill`:

| d | multiplier range |
|---|---|
| 0 (same level) | 0.91 – 0.99 |
| 5 | 0.91 – 0.99 |
| 10 | 0.80 – 0.90 |
| 15 | 0.55 – 0.75 |

### 1.4 Test vectors: mob attacking a player

Player = L10 human warrior: defence 50, dodge 4.9998% (agi only) → 499 bp, parry 5% (if
trained), block 5% (with a shield). Mob crit is 5% base.

| case | miss | dodge | parry | block | crit | crush | hit slice |
|---|---|---|---|---|---|---|---|
| M8 vs P10 | 540 | 539 | 540 | 540 | 460 | 0 | [2619, 10000) |
| M10 vs P10 | 500 | 499 | 500 | 500 | 500 | 0 | [2499, 10000) |
| M12 vs P10 | 460 | 459 | 460 | 460 | 540 | 0 | [2379, 10000) |
| M13 vs P10 | 440 | 439 | 440 | 440 | 560 | 1500 | [3819, 10000) |
| M14 vs P10 | 420 | 419 | 420 | 420 | 580 | 2500 | [4759, 10000) |
| M15 vs P10 | 400 | 399 | 400 | 400 | 600 | 3500 | [5699, 10000) |
| M10 vs P10, no shield, no Parry | 500 | 499 | 0 | 0 | 500 | 0 | [1499, 10000) |

### 1.5 Yellow attacks (abilities)

**vmangos** (`SpellCaster::MeleeSpellHitResult` → the same `RollMeleeOutcomeAgainst` with a spell
entry): **one roll**, the same table, with these changes:

- After miss, a slice for **resist** by mechanic (`SPELL_AURA_MOD_MECHANIC_RESISTANCE`); 0 for us.
- Crit chance comes from `GetSpellCritChance`, whose melee branch uses the same melee crit.
- **No glancing**, except for next-melee abilities (`IsNextMeleeSwingSpell`: Heroic Strike,
  Cleave), which roll exactly like a white swing.
- **No crushing**, except next-melee abilities.
- **Block** only for spells with `SPELL_ATTR_EX3_COMPLETELY_BLOCKED`. Otherwise the block is a
  partial block in the damage step.
- Spells with `SPELL_ATTR_NO_ACTIVE_DEFENSE` can't be dodged, parried or blocked.
- Weapon skill is used only when the spell is melee range (`rangeIndex == SPELL_RANGE_IDX_COMBAT`)
  or needs a weapon. Otherwise the attacker's skill is `5 × level`.
- The miss chance has no dual-wield penalty.

**cmangos** (`Unit::MeleeSpellHitResult` then the crit roll in the spell damage step): **two rolls**.
The first roll is miss / resist / dodge / parry / deflect / block(full) / hit. A hit then rolls
crit on its own (`CalculateEffectiveCritChance` with the ability). This is the classic "two-roll
yellow" model.

**For the realm:** both give a level-60 player the same miss, dodge and parry for abilities, and
neither glances. The difference is the crit rate: under a single roll, crit is pushed down by the
slices above it; under two rolls, crit = `P(hit) × crit%`. Pick one and say which. The
recommendation is the **cmangos two-roll model**, which matches the spec's wording ("yellow
attacks: two-roll for crit"), keeps ability crit independent of the avoidance slices, and is
simpler to test.

Two-roll vector, P10 vs M12, ability, from the front: miss 600 + dodge 600 + parry 700 = 1900 bp
avoided. Ability crit = 299 bp of the 8100 bp that land.

---

## 2. Weapon skill and defence

- Skill max = **5 × level** for every weapon and for defence (`SpellCaster::GetSkillMaxForLevel`,
  `GetLevelForTarget(target) * 5`).
- A mob's weapon skill and defence = **5 × its level** (`SpellCaster::GetUnitMeleeSkill`). A
  creature with `CREATURE_STATIC_FLAG_NO_DEFENSE` has defence 0 (not needed).
- In vanilla a player's weapon skill grinds up toward the max (vmangos `Player::UpdateCombatSkills`,
  with a 100% → 50% chance curve up to 90% of max and lower after that). **The realm should keep
  weapon skill and defence at the max for the level** (vmangos config
  `AlwaysMaxSkillForLevel` does exactly this). There are no skill-up messages, and the hit table
  matches the character sheet.
- Defence and weapon skill above the max (from gear) do not reduce glancing or crushing (they are
  capped), but they do count for miss, crit and dodge (`fullDiff`).

Vectors: `skill_max(1) = 5`, `skill_max(10) = 50`, `skill_max(30) = 150`, `skill_max(60) = 300`.

---

## 3. Armour mitigation

**Source:** vmangos `Objects/SpellCaster.cpp` `SpellCaster::CalcArmorReducedDamage`; cmangos
`Entities/Unit.cpp` `Unit::CalcArmorReducedDamage`. These are identical.

```
k   = 0.1 * armor / (8.5 * attacker_level + 40)
dr  = k / (1 + k)               // same as armor / (armor + 85*L + 400)
dr  = clamp(dr, 0, 0.75)
dmg = max(1, damage - damage * dr)
```

- `attacker_level` is the **attacker's** level, so a mob's level when a mob hits the player.
- Armour from the target, minus armour penetration (none in this slice), floored at 0.
- vmangos stores the result in a `uint32`, which truncates, and then dithers
  (`rand_ditheru`). Use truncation in tests.
- Only physical (`SPELL_SCHOOL_MASK_NORMAL`) damage is reduced.
- 75% is reached at `armor = 3 × (85·L + 400)`: 3750 at L10, 6300 at L20, 8850 at L30, 16500 at L60.

| armour | attacker level | DR | 100 dmg → |
|---|---|---|---|
| 0 | 10 | 0% | 100 |
| 52 (L10 human, agi × 2) | 10 | 3.99% | 96 |
| 25 (L3 mob) | 3 | 3.68% | 96 |
| 100 | 1 | 17.09% | 82 |
| 492 (L10 mob) | 10 | 28.24% | 71 |
| 492 (L10 mob) | 12 | 25.73% | 74 |
| 822 (L20 mob) | 20 | 28.13% | 71 |
| 1000 | 10 | 44.44% | 55 |
| 3000 | 30 | 50.42% | 49 |
| 4000 | 60 | 42.11% | 57 |
| 17265 | 60 | 75% (cap) | 25 |

---

## 4. Rage

**Source:** vmangos `Objects/Player.cpp` `Player::RewardRage`, `Player::Regenerate` (POWER_RAGE),
`Player::RegenerateAll`; call sites in `Objects/Unit.cpp` `Unit::DealDamage`. cmangos
`Entities/Player.cpp` `Player::RewardRage` (identical formula), `Player::Regenerate`.

Rage is stored ×10 (0..1000 for 0..100 rage).

```
c(L)        = 0.0091107836 * L^2 + 3.225598133 * L + 4.2652911      // rage conversion value
dealt:  add = damage / c(L) * 7.5       (rage)
taken:  add = damage / c(L) * 2.5       (rage)
stored += uint32(add * 10)                                          // truncate to tenths
```

- **Dealt**: only `DIRECT_DAMAGE` (white swings), only actual damage after armour. Ability damage
  (`SPELL_DIRECT_DAMAGE`) gives no rage. Dodged or parried white swings give
  `RewardRage(would_be_damage * 0.75, attacker)` in vmangos. Misses give nothing.
- **Taken**: any damage from another unit, after mitigation.
- **Decay** out of combat: vmangos −2 rage per 2 s tick (−1 rage/s, `20 * Rate.Rage.Loss` every
  `REGEN_TIME_PLAYER_FULL = 2000` ms). cmangos −2.5 rage per 2 s (−1.25 rage/s). No decay in
  combat. No regeneration.
- `addRage` is also × `Rate.Rage.Income` (1.0).

| L | c(L) | deal 10 | deal 50 | deal 100 | take 10 | take 50 |
|---|---|---|---|---|---|---|
| 1 | 7.5000 | 99 (9.9 rage) | 499 | 999 | 33 | 166 |
| 10 | 37.4324 | 20 | 100 | 200 | 6 | 33 |
| 20 | 72.4216 | 10 | 51 | 103 | 3 | 17 |
| 30 | 109.2329 | 6 | 34 | 68 | 2 | 11 |
| 60 | 230.6000 | 3 | 16 | 32 | 1 | 5 |

(Values are in stored tenths. At L1 the polynomial gives c = 7.50000002, a hair over 7.5, so 10
damage dealt is 9.99999997 rage, which truncates to 99 tenths, not 100. Tests at L1 must expect 99.)

Decay vector (vmangos): 30 rage out of combat → 28 after 2 s → 20 after 10 s → 0 after 30 s.

---

## 5. Warrior stats

All of these are naked base stats plus gear plus Tier Bonus. The Tier Bonus is added to the base
stats (spec), so it flows through every formula below.

### 5.1 Base stats, health and mana per level

- Base health and base mana per level: `player_classlevelstats` (class 1); Warrior base mana is 0.
  Loaded by `ObjectMgr::LoadPlayerInfo`, used by `Player::GetCreateHealth`. Full table in
  `warrior_level_stats.json` → `per_level[].base_health`.
  Spot values: L1 20, L2 29, L10 101, L20 199, L30 374, L40 649, L50 1079, L60 1689.
- Naked str/agi/sta/int/spi per race per level: `player_levelstats` (class 1),
  `warrior_level_stats.json` → `race_base_stats`.

### 5.2 Health from stamina

vmangos `StatSystem.cpp` `Player::GetHealthBonusFromStamina`, `Player::UpdateMaxHealth`:

```
bonus      = min(sta, 20) + max(sta - 20, 0) * 10
max_health = base_health(L) + bonus             // then aura % mods, none here
```

Vectors: sta 0 → 0, sta 20 → 20, sta 22 → 40, sta 31 → 130. L1 human (sta 22) → 20 + 40 = **60**.
L10 human (sta 31) → 101 + 130 = **231**. L10 orc (sta 33) → **251**. L60 human (sta 110) → 1689 +
920 = **2609**.

### 5.3 Attack power, weapon damage, crit

`Unit::GetAttackPowerFromStrengthAndAgility` (vmangos) / `Player::UpdateAttackPowerAndDamage`
(cmangos):

```
melee AP (warrior) = 3*L + 2*str - 20
ranged AP (warrior) = L + agi - 10       // not needed
```

Weapon damage, `Player::CalculateMinMaxDamage`:

```
min = weapon_min + AP / 14 * weapon_speed_seconds
max = weapon_max + AP / 14 * weapon_speed_seconds
swing damage = urand(uint32(min), uint32(max))           // Unit::CalculateDamage
unarmed: weapon 1–2 damage (BASE_MINDAMAGE / BASE_MAXDAMAGE), speed 2.0 s (BASE_ATTACK_TIME)
```

A broken weapon (durability 0) cannot be used: it falls back to the unarmed 1–2 damage
(`!CanUseEquippedWeapon`).

Melee crit shown on the sheet (`Player::UpdateCritPercentage`, `Player::GetMeleeCritFromAgility`):

```
crit% = agi / agi_per_crit(L) + (weapon_skill - 5L) * 0.04     // warriors have no class base crit
```

`agi_per_crit(L)` is the divisor table in `warrior_level_stats.json` (4.0 at L1, 5.20021 at L10,
7.80031 at L20, 10.395 at L30, 20.0 at L60). vmangos reads it from `player_crit_per_agility`.
cmangos interpolates linearly from 3.9 at L1 to 20.0 at L60 instead (see §14).

Vectors (naked):

| | L | str | agi | AP | crit% |
|---|---|---|---|---|---|
| human | 1 | 23 | 20 | 29 | 5.0000 |
| human | 10 | 33 | 26 | 76 | 4.9998 |
| human | 20 | 47 | 35 | 134 | 4.4870 |
| human | 30 | 62 | 44 | 194 | 4.2328 |
| human | 60 | 120 | 80 | 400 | 4.0000 |
| orc | 1 | 26 | 17 | 35 | 4.2500 |
| tauren | 10 | 38 | 21 | 86 | 4.0383 |

Weapon vectors:

- L1 human unarmed: AP 29 → 29 / 14 × 2.0 = 4.143 → min 5.143, max 6.143 → urand(5, 6).
- L10 human, a 7–14 dmg weapon at 2.6 s: AP 76 → 76 / 14 × 2.6 = 14.114 → min 21.11, max 28.11 →
  urand(21, 28). Display DPS = (21.11 + 28.11) / 2 / 2.6 = 9.47.

### 5.4 Dodge, parry, block, armour

`Player::UpdateDodgePercentage`, `UpdateParryPercentage`, `UpdateBlockPercentage`, `UpdateArmor`:

```
dodge% = agi / agi_per_dodge(L) + (def - 5L) * 0.04           // warriors: no class base dodge
parry% = 5 + (def - 5L) * 0.04       if the character can parry, else 0
block% = 5 + (def - 5L) * 0.04       if the character can block (has Block and a shield), else 0
armor  = item armour + 2 * agi
```

For warriors `player_dodge_per_agility` equals `player_crit_per_agility` (it differs only at L19 in
the 6th digit), so naked dodge% = naked crit%. Vectors: L10 human dodge 4.9998%, armour 52; L60
human dodge 4.0%, armour 160; L1 orc dodge 4.25%, armour 34.

Note: a new warrior has Block and Dodge from creation (`playercreateinfo_spell`), but **Parry** is
a trained skill (vanilla warriors learn it from the class trainer at level 6). Whether a Basic
Warrior can parry is a design decision for the slice. The values above assume it can.

---

## 6. Mob base stats

**Source:** vmangos world DB `creature_classlevelstats`; applied in `Objects/Creature.cpp`
`Creature::SelectLevel`, `StatSystem.cpp` `Creature::UpdateDamagePhysical`.

```
max_health  = round(health(L, unit_class) * template.health_multiplier)
dmg_avg     = melee_damage(L, unit_class) * template.damage_multiplier
dmg_min/max = dmg_avg * (1 -/+ template.damage_variance)      // variance usually 0.14
armor       = armor(L, unit_class) * template.armor_multiplier
swing time  = template.base_attack_time                       // 2000 ms for almost all normal mobs
```

- `melee_damage` is the **average per swing** at the template's attack time. A mob with a faster
  swing has its `damage_multiplier` lowered in the DB to match (e.g. Savannah Huntress: 1500 ms,
  ×0.75).
- AP is already in `melee_damage`. Mob AP only matters when it changes from its DB value
  (`× (0.7 + 0.3 × AP_now / AP_db)`).
- Elites and bosses are made by template multipliers (e.g. Deviate Guardian, elite L18–19: health
  ×3, damage ×1.7; Lord Cobrahn: health ×5, damage ×3.45). The server rate config for each rank is
  1.0.
- A mob's level is rolled uniformly in `[level_min, level_max]` at spawn.

Normal mob base table, unit class 1 (warrior, which almost all beasts and melee humanoids use),
levels 1–35. Min/max damage is shown at the usual variance of 0.14. Classes 2 and 8 and levels
36–63 are in `mob_base_stats.json`.

| L | health | dmg avg | dmg min–max | armour | AP |
|---|---|---|---|---|---|
| 1 | 42 | 1.58 | 1.36–1.80 | 0 | 24 |
| 2 | 55 | 2.31 | 1.99–2.64 | 0 | 26 |
| 3 | 71 | 3.26 | 2.80–3.71 | 25 | 28 |
| 4 | 86 | 4.50 | 3.87–5.13 | 60 | 30 |
| 5 | 102 | 6.01 | 5.17–6.85 | 103 | 32 |
| 6 | 120 | 8.01 | 6.89–9.13 | 156 | 34 |
| 7 | 137 | 10.37 | 8.92–11.82 | 221 | 36 |
| 8 | 156 | 12.05 | 10.36–13.73 | 297 | 38 |
| 9 | 176 | 13.81 | 11.88–15.75 | 386 | 40 |
| 10 | 198 | 15.24 | 13.11–17.37 | 492 | 42 |
| 11 | 222 | 17.20 | 14.79–19.61 | 518 | 44 |
| 12 | 247 | 19.67 | 16.92–22.43 | 552 | 46 |
| 13 | 273 | 21.43 | 18.43–24.42 | 585 | 48 |
| 14 | 300 | 23.62 | 20.31–26.92 | 619 | 50 |
| 15 | 328 | 25.71 | 22.11–29.31 | 653 | 54 |
| 16 | 356 | 27.17 | 23.37–30.98 | 688 | 56 |
| 17 | 386 | 29.13 | 25.05–33.21 | 722 | 60 |
| 18 | 417 | 30.51 | 26.23–34.78 | 755 | 64 |
| 19 | 449 | 32.36 | 27.83–36.90 | 789 | 68 |
| 20 | 484 | 34.21 | 29.42–38.99 | 822 | 70 |
| 21 | 521 | 35.49 | 30.52–40.46 | 858 | 74 |
| 22 | 562 | 37.20 | 31.99–42.40 | 892 | 78 |
| 23 | 605 | 38.45 | 33.07–43.84 | 925 | 80 |
| 24 | 651 | 40.10 | 34.49–45.72 | 959 | 84 |
| 25 | 699 | 41.66 | 35.83–47.49 | 992 | 86 |
| 26 | 750 | 42.97 | 36.95–48.99 | 1026 | 90 |
| 27 | 800 | 44.70 | 38.44–50.95 | 1061 | 92 |
| 28 | 853 | 45.99 | 39.55–52.43 | 1093 | 96 |
| 29 | 905 | 47.64 | 40.97–54.31 | 1127 | 100 |
| 30 | 955 | 49.39 | 42.47–56.30 | 1160 | 102 |
| 31 | 1006 | 50.63 | 43.54–57.72 | 1194 | 106 |
| 32 | 1057 | 52.17 | 44.86–59.47 | 1229 | 108 |
| 33 | 1110 | 53.48 | 45.99–60.97 | 1261 | 112 |
| 34 | 1163 | 55.02 | 47.32–62.72 | 1296 | 114 |
| 35 | 1220 | 56.51 | 48.60–64.42 | 1328 | 118 |

Barrens templates (vmangos, latest patch row), to check the import and the hand-made camps:

| entry | name | levels | unit class | health × | dmg × | swing | detection | call-for-help |
|---|---|---|---|---|---|---|---|---|
| 2955 | Plainstrider | 1–2 | 1 | 1.0 | 1.0 | 2000 | 10 | 5 |
| 3098 | Mottled Boar | 1–2 | 1 | 1.0 | 1.0 | 2000 | 10 | 5 |
| 3124 | Scorpid Worker | 3 | 1 | 1.0 | 1.0 | 2000 | 10 | 5 |
| 3101 | Vile Familiar | 3–4 | 2 | 1.0 | 1.0 | 2000 | 10 | 5 |
| 3111 | Razormane Quilboar | 6–7 | 1 | 1.0 | 1.0 | 2000 | 18 | 5 |
| 3112 | Razormane Scout | 7–8 | 1 | 1.0 | 1.0 | 2000 | 18 | 5 |
| 3244 | Greater Plainstrider | 11–12 | 1 | 1.0 | 1.0 | 2000 | 18 | 5 |
| 3254 | Sunscale Lashtail | 11–13 | 1 | 0.8 | 1.0 | 2000 | 18 | 5 |
| 3415 | Savannah Huntress | 11–12 | 1 | 1.0 | 0.75 | 1500 | 18 | 5 |
| 3272 | Kolkar Wrangler | 12–13 | 1 | 1.0 | 1.0 | 2000 | 18 | 5 |
| 3242 | Zhevra Runner | 13–14 | 1 | 1.0 | 1.0 | 2000 | 18 | 5 |
| 3273 | Kolkar Stormer | 13–14 | 2 | 1.0 | 1.0 | 2000 | 18 | 5 |
| 3276 | Witchwing Harpy | 14–15 | 1 | 1.0 | 0.95 | 2000 | 18 | 5 |
| 3637 | Deviate Guardian (elite) | 18–19 | 1 | 3.0 | 1.7 | 2000 | 20 | 5 |
| 3669 | Lord Cobrahn (elite) | 20 | 2 | 5.0 | 3.45 | 2000 | 20 | 5 |

Vector: a L10 normal unit-class-1 mob with multipliers 1.0 has 198 health, 492 armour, and hits for
13.11–17.37 every 2.0 s before the player's armour. Against a L10 human warrior with 52 armour
(3.99% DR), a 15-damage swing lands for 14.

---

## 7. Swing timer

**Source:** vmangos `Objects/Player.cpp` `Player::SetRegularAttackTime`, `Objects/Unit.cpp`
`Unit::AttackerStateUpdate`, `Unit::CanReachWithMeleeAutoAttack`, `GetCombatReachToTarget`.

- Attack time = the weapon's `Delay` (ms), or 2000 ms unarmed (`BASE_ATTACK_TIME`). A mob uses
  `base_attack_time`.
- The timer counts down every update. When it reaches 0 and the target is in melee reach and in
  front, the unit swings and the timer is reset to the full attack time. If the target is out of
  reach or not in front, the swing waits at 0 (the client shows "out of range" / "facing") and goes
  off as soon as it can.
- Melee reach = `max(5.0, reach_attacker + reach_victim + 4/3)`, with each combat reach floored at
  1.5. Most units have reach 1.5, which gives 5.0 yd. When both units are moving faster than 70%
  of run speed there is an extra 2.66 yd of leeway (not needed).
- Starting auto-attack on a target whose swing timer is already at 0 swings at once.
- Parry haste on the victim: see §1.2.
- Dual wield and off-hand are not needed.

Vector: a 2.6 s weapon from t = 0 with the target always in reach swings at t = 0 (timer starts
ready), 2.6, 5.2, 7.8 s.

---

## 8. Kill XP

**Source:** vmangos `Formulas.h` `MaNGOS::XP::GetGrayLevel`, `GetZeroDifference`,
`BaseGainLevelFactor`, `BaseGain`, `Gain`. cmangos `Tools/Formulas.h` `MaNGOS::XP::BaseGain`,
`IsTrivialLevelDifference`, `Gain`.

```
gray_level(P) = 0                       if P <= 5
              = P - 5 - floor(P/10)     if P <= 39
              = P - 1 - floor(P/5)      if P <= 59 (and 60 in vmangos)

ZD(P) = 5 (P<8), 6 (P<10), 7 (P<12), 8 (P<16), 9 (P<20), 11 (P<30), 12 (P<40),
        13 (P<45), 14 (P<50), 15 (P<55), 16 (P<60), 17 (60)

base = P*5 + 45
if M >= P:          xp = base * (1 + 0.05 * min(M - P, 4))
elif M > gray(P):   xp = base * (ZD + M - P) / ZD          // = base * (1 - (P-M)/ZD)
else:               xp = 0                                 // gray mob
if elite (rank 1, 2 or 3):  xp *= 2.5 in a non-raid dungeon, else 2.0
xp *= template.xp_multiplier
xp = nearbyint(xp)          // round half to even
```

- Rank 4 (rare) is not elite. Pets, totems, critters and summoned creatures give 0.
- cmangos decides gray with `IsTrivialLevelDifference` instead of `GetGrayLevel`. For P = 1..60
  these give the same answer for every M (checked mechanically).
- The kill is shared only in a group, which this slice does not have. Rested XP is not needed.

Gray level table: P1–5 → 0, P6 → 1, P7 → 2, P8 → 3, P9 → 4, P10 → 4, P11 → 5, P12 → 6, P13 → 7,
P14 → 8, P15 → 9, P16 → 10, P17 → 11, P18 → 12, P19 → 13, P20 → 13, P21 → 14, P22 → 15, P23 → 16,
P24 → 17, P25 → 18, P26 → 19, P27 → 20, P28 → 21, P29 → 22, P30 → 22, P31 → 23 … P39 → 31,
P40 → 31, P50 → 39, P60 → 47.

Vectors:

| player | mob | xp | note |
|---|---|---|---|
| 1 | 1 | 50 | |
| 1 | 2 | 52 | 52.5 → 52 (and f32 maths gives 52.49) |
| 1 | 3 | 55 | |
| 1 | 5 | 60 | +4 cap |
| 1 | 6 | 60 | still capped at +4 |
| 5 | 3 | 42 | ZD 5: 70 × 3/5 |
| 5 | 1 | 14 | gray level 0, so still green |
| 6 | 1 | 0 | gray (≤ 1) |
| 6 | 2 | 15 | 75 × 1/5 |
| 10 | 10 | 95 | |
| 10 | 8 | 68 | ZD 7: 95 × 5/7 = 67.86 |
| 10 | 5 | 27 | 95 × 2/7 = 27.14 |
| 10 | 4 | 0 | gray |
| 10 | 14 | 114 | |
| 10 | 20 | 114 | capped at +4 |
| 20 | 20 | 145 | |
| 20 | 15 | 79 | ZD 11: 145 × 6/11 |
| 20 | 13 | 0 | gray |
| 20 | 22 elite, open world | 319 | 145 × 1.1 × 2 |
| 20 | 20 elite, dungeon | 362 | 145 × 2.5 = 362.5 → 362 |
| 25 | 19 | 77 | |
| 25 | 18 | 0 | gray |
| 30 | 30 | 195 | |
| 30 | 23 | 81 | ZD 12: 195 × 5/12 |
| 60 | 60 | 345 | |
| 60 | 48 | 101 | |
| 60 | 47 | 0 | gray |

---

## 9. XP to next level

`data/twilights-eve/xp_table.json`, from the vmangos world DB `player_xp_for_level`
(`Player::GetXPForNextLevel`). These are the 1.12 values. The sum for levels 1–59 is 4,084,700.

| L | xp | L | xp | L | xp | L | xp |
|---|---|---|---|---|---|---|---|
| 1 | 400 | 16 | 16000 | 31 | 50800 | 46 | 123200 |
| 2 | 900 | 17 | 17700 | 32 | 54500 | 47 | 129100 |
| 3 | 1400 | 18 | 19400 | 33 | 58600 | 48 | 135100 |
| 4 | 2100 | 19 | 21300 | 34 | 62800 | 49 | 141200 |
| 5 | 2800 | 20 | 23200 | 35 | 67100 | 50 | 147500 |
| 6 | 3600 | 21 | 25200 | 36 | 71600 | 51 | 153900 |
| 7 | 4500 | 22 | 27300 | 37 | 76100 | 52 | 160400 |
| 8 | 5400 | 23 | 29400 | 38 | 80800 | 53 | 167100 |
| 9 | 6500 | 24 | 31700 | 39 | 85700 | 54 | 173900 |
| 10 | 7600 | 25 | 34000 | 40 | 90700 | 55 | 180800 |
| 11 | 8800 | 26 | 36400 | 41 | 95800 | 56 | 187900 |
| 12 | 10100 | 27 | 38900 | 42 | 101000 | 57 | 195000 |
| 13 | 11400 | 28 | 41400 | 43 | 106300 | 58 | 202300 |
| 14 | 12900 | 29 | 44300 | 44 | 111800 | 59 | 209800 |
| 15 | 14400 | 30 | 47400 | 45 | 117500 | 60 | (cap) |

Level-up vectors: L1 with 350 XP + a 60 XP kill → L2 with 10 XP. L1 with 0 XP + 1400 XP in one
award → L3 with 100 XP (400 + 900 used; vmangos `Player::GiveXP` loops while
`xp >= next_level_xp`). At a Tier Cap the level does not rise and no XP is banked (#15).

---

## 10. Quest XP and falloff

**Source:** vmangos `QuestDef.cpp` `Quest::XPValue`; cmangos `Quests/QuestDef.cpp`
`Quest::XPValue`. The two are identical (cmangos derives `fullxp` from `RewMoneyMaxLevel`, which
we do not use).

```
d = player_level - quest_level
factor = 1.0 (d <= 5), 0.8 (d == 6), 0.6 (d == 7), 0.4 (d == 8), 0.2 (d == 9), 0.1 (d >= 10)
xp = ceil(full_xp * factor)
```

The vanilla falloff is confirmed: **full XP up to 5 levels above the quest**, then 80/60/40/20/10%.
A player **below** the quest's level gets full XP (no bonus).

Spec rule (#1, #11): `full_xp = xp_to_next(player_level)` (§9), so

```
quest_reward(P, Q) = ceil(xp_to_next(P) * factor(P - Q))
```

Do this in integer arithmetic (`(xp * pct + 99) / 100`) to avoid `f32` rounding. The `f32` form
gives the same values for the whole table.

| player | quest | xp_to_next(P) | factor | reward |
|---|---|---|---|---|
| 10 | 10 | 7600 | 1.0 | 7600 |
| 8 | 10 | 5400 | 1.0 | 5400 |
| 15 | 10 | 14400 | 1.0 | 14400 |
| 16 | 10 | 16000 | 0.8 | 12800 |
| 17 | 10 | 17700 | 0.6 | 10620 |
| 18 | 10 | 19400 | 0.4 | 7760 |
| 19 | 10 | 21300 | 0.2 | 4260 |
| 20 | 10 | 23200 | 0.1 | 2320 |
| 25 | 10 | 34000 | 0.1 | 3400 |

**Design note.** With the vanilla curve, a quest stays worth a full level for 5 levels above its
own level. The spec's "about one level at its own level" holds, but "shrinks once I outlevel it"
only starts at +6. If a Hunting Ground should be outgrown sooner, that is a deliberate deviation
from vanilla and should be written down as one.

---

## 11. Aggro, social aggro, leash, respawn

### 11.1 Aggro radius

**Source:** vmangos `Objects/Creature.cpp` `Creature::GetAttackDistance`; cmangos
`Entities/Unit.cpp` `Unit::GetAttackDistance`.

```
diff   = player_level - mob_level, floored at -25          (MAX_LEVEL_DIFF_FOR_AGGRO_RANGE)
radius = detection_range - diff                             // +1 yd per level the mob is above
radius = max(radius, min(detection_range, 5))               // floor 5 yd (cmangos: flat 5)
radius *= Rate.Creature.Aggro (1.0)
if detection_range < 1: no proximity aggro
```

- `detection_range` is per creature template. In the vmangos DB, 7217 of the ~7300 normal
  level 1–35 templates use **18 yd**. The level 1–3 starter beasts use 10 yd, and dungeon elites
  20 yd.
- Gray mobs still aggro in vanilla. Their radius just shrinks toward 5 yd.
- The check is 3D distance plus line of sight (`MoveInLineOfSight`), and the mob must be hostile
  (the faction template).

Vectors (detection 18): P10 vs M10 → 18; P10 vs M13 → 21; P10 vs M7 → 15; P10 vs M1 → 9;
P30 vs M10 → 5 (floor); P1 vs M30 → 18 + 25 = 43 (the diff is clamped at −25); P1 vs M3 at
detection 10 → 12.

### 11.2 Social aggro

**Source:** vmangos `Objects/Creature.cpp` `Creature::CallAssistance`, `Creature::CallForHelp`,
the call-for-help timer in `Creature::Update`; `World.cpp` defaults. cmangos
`Entities/Creature.cpp`, `World/World.cpp`.

1. **Assistance on pull.** When a mob enters combat, every assist-eligible creature (same faction
   group, alive, not in combat, able to attack the target) within **CreatureFamilyAssistanceRadius**
   of it joins after a **1.5 s** delay (`CreatureFamilyAssistanceDelay` = 1500 ms). The radius is
   **10 yd in vmangos** and **5 yd in cmangos**. It is called once per combat
   (`CSTATE_ALREADY_CALL_ASSIST`).
2. **Call for help while chasing** (vmangos): every 1 s, if the mob is more than 10 yd from its
   spawn point, it calls `CallForHelp(call_for_help_range)`, which is the template's
   `call_for_help_range`, **5 yd** for almost all mobs. Dragging a mob through another camp pulls
   that camp too.
3. Fleeing mobs call for help in 10 yd (vmangos `FleeingMovementGenerator`) and look for
   assistance in `CreatureFamilyFleeAssistanceRadius` (30 yd). Not needed for this slice.

For the realm: a pulled mob brings every same-camp mob within 10 yd of it, 1.5 s later.

### 11.3 Leash and evade

- **vmangos** `Creature::IsOutOfThreatArea`: open world only (no leash inside dungeons). The mob
  evades when **both** it and its target are farther than
  `max(ThreatRadius = 50 yd, 1.5 × aggro radius)` from where combat started, **and** 12 s have
  passed since the last "leash extension". The extension refreshes every 5 s while the mob is
  chasing and can reach its target, and while it is crowd-controlled. A per-template
  `leash_range` (0 = off) is a hard cap: past it the mob evades at once.
- **cmangos** `Combat/CombatManager.cpp`: after the combat timer runs out (refreshed by combat
  actions), the mob evades when its victim is more than `LeashRadius` = **30 yd** from the mob's
  last refresh position. There is also a per-template `Leash`.
- **Evade** (both): the mob drops its threat, runs home, ignores attacks on the way back
  (`MELEE_HIT_EVADE` / `SPELL_MISS_EVADE`, the client shows "Evade"), and resets to full health on
  arrival.
- Also both: a mob evades when its target becomes unreachable (no path).

The spec's "leash and evade at about 40–60 yd from home" fits the vmangos 50 yd threat radius.
Recommendation: evade when the mob is more than 50 yd from its combat-start point and no damage
has been exchanged for 12 s, or at once past 60 yd.

### 11.4 Respawn and corpses

- vmangos world `creature.spawntimesecsmin/max`. The default is 120 s. The Barrens normal mobs
  checked use 300–413 s. Corpse decay is `Corpse.Decay.NORMAL` = 300 s, and a looted corpse
  despawns at once.
- The spec overrides this: respawn about 60 s after the player leaves the area (#7). That is a
  design rule, not vanilla.

---

## 12. Durability

**Source:** vmangos `Objects/Unit.cpp` `Unit::DealDamage` (death and per-hit), `Objects/Player.cpp`
`Player::DurabilityLossAll`, `DurabilityLoss`, `DurabilityPointsLoss`,
`DurabilityPointLossForEquipSlot`; `Handlers/NPCHandler.cpp` `WorldSession::SendSpiritResurrect`.
cmangos has the same functions and the same defaults.

- **Death** (killed by a creature, not in a battleground): every **equipped** item loses
  `max(1, uint32(max_durability × 0.10))` points. Bags are not touched. The client gets
  `SMSG_DURABILITY_DAMAGE_DEATH`. This is the same as the spec's "10% durability".
- **Spirit healer resurrection**: 25% of max durability on equipped **and** bag items, plus
  resurrection sickness. Not used, because the spec respawns at the Hub.
- **On hit**: `DurabilityLossChance.Damage` = **0.5%** per hit taken and 0.5% per hit dealt. On
  success, one random equipment slot (0..18, an empty slot means nothing happens) loses
  **1 point**. cmangos also has `DurabilityLossChance.Parry` 0.05% and `.Absorb` 0.5%.
- At 0 durability an equipped item's stats are removed (`_ApplyItemMods(item, slot, false)`
  before durability is set to 0) and come back when repaired above 0. A broken weapon cannot be
  used for melee (§5.3), and a broken shield cannot block (`GetUnitBlockChance` checks
  `!IsBroken()`).

Vectors: an item with max 40 at 40 → death → 36; max 40 at 3 → 0 (and its stats are removed);
max 5 at 5 → 4 (the minimum loss is 1); max 0 → unchanged.

---

## 13. Order of a white swing (summary for the implementer)

1. Timer at 0, target in reach and in front (§7).
2. Raw damage = urand(min, max) (§5.3 or §6).
3. Armour reduction with the attacker's level (§3).
4. Roll the table (§1) and apply the outcome multiplier (crit ×2, glance ×[low, high],
   crush ×1.5, block − block value, avoid → 0).
5. Deal damage. Rage: dealt to the attacker if a warrior, taken to the victim if a warrior (§4).
   Durability rolls (§12).
6. Parry haste if parried (§1.2). Reset the swing timer.

---

## 14. Where vmangos and cmangos disagree

| rule | vmangos | cmangos | impact here |
|---|---|---|---|
| Miss vs mob, `d = V_def − A_skill > 10` | `5 + 0.2·d` (8% at +3 levels) | `6 + 0.6·(d − 10)` (9% at +3, 12% at +4) | pulls 3+ levels up |
| Mobs below level 10 | miss, dodge, parry and block all × `L_mob/10` | no reduction | **large for the level 1–6 camps** (P1 vs M1 miss 0.5% vs 5%) |
| Glancing vs mobs ≤ level 10 | yes (10% at same level) | only if the mob is level > 10 | levels 1–10 |
| Glancing cap | 40% | 100% (no effective cap; 50% at +4) | 4+ levels up |
| Mob dodge when the player is higher level (`d < 0`) | `−0.1%`/point (and ×L/10) | `−0.04%`/point | farming lower mobs |
| Mob parry, `0 < d ≤ 10` | `+0.2%`/point (7% at +2) | `+0.1%`/point (6% at +2) | +1..+2 level mobs |
| Mob block, player higher level | `−0.1%`/point | `−0.04%`/point | small |
| Player crit vs a lower mob | `+0.04%`/point of weapon skill over defence | `+0.2%`/point (L10 vs L7: 8% vs 5.6%) | farming lower mobs |
| Player crit vs a higher mob | `−0.2%`/point of capped skill vs defence | `−0.2%`/point of max skill vs defence | same with max skill |
| Table order | glance before block | block before glance | none unless the table overflows |
| Yellow attacks | one roll, crit in the same table | two rolls (avoid, then crit) | ability crit rate |
| Rage decay out of combat | 1 rage/s | 1.25 rage/s | small |
| Warrior agility → crit/dodge | DB table (`player_crit_per_agility`), 5.20021 at L10 | linear from 3.9 at L1 to 20 at L60 (6.356 at L10) | L10 human: 5.00% vs 4.09% crit |
| Kill-XP gray cutoff | `GetGrayLevel` | `IsTrivialLevelDifference` | same for 1–60; cmangos `GetGrayLevel(60)` = 51, but that is only used for the colour |
| Social assist radius | 10 yd | 5 yd | camp pulls |
| Leash | 50 yd threat radius from combat start + 12 s no extension | 30 yd from the last refresh point after the combat timer | evade feel |
| Armour, rage gain, kill XP, quest falloff, durability, AP, health from stamina, crushing | identical | identical | none |

Where they disagree, this doc's vectors are **vmangos**. A realm that follows cmangos instead must
change the vectors marked in §1.3 for M7, M5, P1–P5 and the +3/+4 rows.
