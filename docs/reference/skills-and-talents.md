# Twilight's Eve: skills, talents and Advancement data (tickets #8, #14, #15)

The data is in `data/twilights-eve/callings.json`. Every spell, talent and tab id in it is a
real 1.12.1 record. `tools/twilights-eve/extract_spells.py verify` checks them against the
client's DBCs, read straight off the MPQ patch chain (Spell.dbc comes from patch-2.MPQ, and
the talent tables, SpellIcon and SpellRange come from patch.MPQ). It also writes the full field
dump to `data/twilights-eve/spells-extracted.json` and prints level tables. The checks cover
names, ranks, icons, family flags, the client-side fields (cost, range, stances, aura states,
cooldowns, duration, GCD, on-next-swing, not-in-combat), whether talent rank spells form a
prefix of the real ranks, seats, prerequisite arrows, tier reachability, the point schedule,
and the spell-mod op, value and family bit of each `client_spellmod`. The script uses only the
standard library and runs on any Python 3.9+ (`--data` defaults to `$WOW_DATA`).

```
python tools/twilights-eve/extract_spells.py verify
python tools/twilights-eve/extract_spells.py search "Heroic Strike" --family 4 --brief
python tools/twilights-eve/extract_spells.py talents --class 1
python tools/twilights-eve/extract_spells.py spell 78 12294
python tools/twilights-eve/extract_spells.py selftest
python tools/twilights-eve/simulate_fights.py --calling warrior --mobs ... --stats ...
```

## The client enforces some things before it sends a cast

These findings come from reading `crates/benilla-app/src/spell/{usable,cast_send}.rs` and
`benilla-formats/src/spells/display.rs`. They decide which IDs we can use.

- **Stances.** The client refuses a cast whose Spell.dbc `Stances` mask leaves out the current
  shapeshift form, and it does this before sending (`form_refusal`, `0x612480`). Charge,
  Thunder Clap and Overpower are Battle Stance only, and Rend and Hamstring need Battle or
  another stance. So the realm **cannot ignore stances**. It has to keep `UNIT_FIELD_BYTES_1`
  byte 2 = 17 (Battle Stance) for every Warrior Calling. Side effect: `GetBonusBarOffset()`
  returns 1, so the main action bar pages to slots 73..84, and the default bar layout must
  use those slots. This is why Tier 1 has no Berserker-only skills (Whirlwind, Intercept,
  Pummel) and no Defensive-only ones (Revenge, Shield Block).
- **Power cost.** The client refuses a cast it cannot afford, using the Spell.dbc cost minus
  any spell mods the server pushed. The realm's cost must therefore be at least the DBC cost.
  The extractor enforces that. Talents that reduce cost (Improved Heroic Strike, Improved
  Thunder Clap, Improved Execute) need an `SMSG_SET_FLAT_SPELL_MODIFIER` with op COST on the
  family bit, or the client keeps refusing at the old cost.
- **Overpower** has AttributesEx `0x00100000`, which requires combo points. Following vmangos,
  the realm grants 1 combo point on the dodging target and clears it on cast or after 5 s.
- **Execute** has TargetAuraState 2. The realm must set `UNIT_FIELD_AURASTATE` bit `0x2` on a
  unit at or below 20% health.
- **Charge** is flagged not-in-combat on the client side.
- **Ranks.** The spellbook shows the Spell.dbc rank subtext (`ui_spellbook.rs:373`). The only
  player-rank skill without a rank is Bloodrage (2687). The other rankless rows (25710 Heroic
  Strike, 14895 Overpower, 9080 Hamstring, 9128 Battle Shout, the NPC Charges) cost 0 and
  carry the wrong stances or effects. So we keep the real player ranks, and **the ticket's "no
  ranks" criterion needs a Twilight-mode glue override that blanks `SpellDisplay.rank` for
  Calling skills**. That is a decision for #8.
- **Tooltips** use the DBC `$s1` with RealPointsPerLevel up to MaxLevel. Only Battle Shout
  scales with level. Every other skill's tooltip shows its rank's numbers. One option is to
  have the realm push `SET_FLAT_SPELL_MODIFIER` op ALL_EFFECTS on each skill's family bit
  every level, so the tooltips track its formulas. That is untested.

## Tier 0, Basic Warrior: two skills (levels 1–10)

`L` is the level within the Tier. `W` is a white-swing roll, and `Wn` is a normalized roll.
Each linear term is `round(base + per_level*L + per_ap*AP)`.

| skill | id | rank | icon | rage | cd | client gate | realm formula | value at L1/5/10 |
|---|---|---|---|---|---|---|---|---|
| Heroic Strike | 78 | Rank 1 | 856 Ability_Rogue_Ambush | 15 | – (next swing) | – | W + 9.5+1.4L | +11/17/24 |
| Charge | 100 | Rank 1 | 457 Ability_Warrior_Charge | 0 | 15 s | Battle, 8–25 yd, out of combat | rage 8.5+0.14L, 1 s stun (7922) | 9/9/10 |

The Callings handoff starts the Basic Warrior small (#18): Rend, Thunder Clap, Battle Shout,
Hamstring, Overpower and Bloodrage are gone from the kit, and the spellbook shows the two skills
with no rank text.

The vanilla rank progressions behind each fit are in the JSON's `vanilla_reference` fields.

## Tier 1, Warrior: eight skills (levels 1–30, Tier Bonus applies)

Mortal Strike 12294 (R1), Overpower 7887 (R2), Charge 6178 (R2), Rend 6546 (R2), Thunder Clap
8198 (R2), Cleave 845 (R1), Execute 5308 (R1) and Battle Shout 6192 (R3). All of them work in
Battle Stance. Values at L1/10/20/30: MS Wn+28/50/75/100; Overpower +7/12/18/24; Charge
11–14 rage; Rend 6–14 per tick over 4 ticks; Thunder Clap 12/26/41/57; Cleave +4/8/12/16 on 2
targets; Execute 30/77/129/181 plus 2–5 per extra rage; Battle Shout 22/40/60/80 AP. (The
simulation below retuned MS and Execute.) Mortal
Strike is a level-40 talent in vanilla. Here it is the Tier 1 signature, scaled down at low
levels. It is left out of the Tier 1 tree, which stops at tier 5. Heroic Strike drops out of the
Tier 1 set.

## Talent trees

The client's catalog (`benilla-formats/src/talents.rs`, `benilla-app/src/ui_talent.rs`, read
only) works like this:
- `TalentCatalog { tabs: Vec<TalentTabInfo{id,name,race_mask,class_mask,background}>,
  by_tab: HashMap<tab_id, Vec<Talent{id,tab,row,col,ranks[5],prereq_talent,prereq_rank(0-based),required_spell,exceptional}>> }`.
  Tabs are filtered by class and race bit, in file order. Talents are listed in native order,
  which is the index that `GetTalentInfo(tab, i)` and `LearnTalent` use. Its fields are
  private and only `load_talent_catalog` builds it, so the replacement needs a new
  constructor (a carry, or a child module).
- `ui_talent.rs` keeps it in `Res<Talents>`. `feed_talents` rebuilds when that resource
  changes, so the Twilight glue can `insert_resource` the current Calling's catalog on login
  and on Advancement. Names and icons come from Spell.dbc for the rank spell. A talent's rank
  is the highest rank spell the player knows, so learning a talent must send the rank spells
  as learned, and a wipe must remove them. The tier gate is `row*5 <= points spent in tab`.
  Points come from `PLAYER_CHARACTER_POINTS1`. A click sends `CMSG_LEARN_TALENT{talent_id,
  rank}`, which carries the real Talent.dbc id.
- What the replacement needs per tab: an id (unique within the catalog), name, background art
  base, class_mask 1, race_mask 511. Per talent: the real talent id, the tab id, a 0-based
  row and col, rank spells as a prefix of the real ones (1–5), the prerequisite talent id and
  rank in the same tab, required_spell 0, and the exceptional flag. The JSON holds all of
  this with 1-based tier and column.

**Basic Warrior** has one tab, "Recruit" (id 900, WarriorArms art), and 9 points. Capacity
is 21 ranks over 2 rows on the vanilla grid: row 2 opens at 5 points. Row 1: Improved Heroic
Strike 124 ×3, Improved Charge 126 ×2, Toughness 140 ×5. Row 2: Cruelty 157 ×5, Unbridled Wrath
159 ×5, Anger Management 137 ×1. Anger Management (rage decays 30% slower out of combat) has no
effect kind in the realm, so it ships with none: it is learnable, counts toward the row, and
`tests/content_callings.rs` lists it as not modelled.

**Tier 1 Warrior** has 29 points across three tabs. Each tab keeps the real tabs' seats and
stops at tier 5, so 20 points reach Sweeping Strikes, Death Wish or Concussion Blow.
Arms 161: 41 ranks, ending in Axe/Sweeping Strikes/Mace/Sword. Fury 164: 40 ranks, up to
Death Wish. Protection 163: 27 ranks. In Protection, One-Handed Weapon Specialization moves
from tier 6 to tier 4, and Last Stand loses its Improved Bloodrage prerequisite. The dropped
talents and the reasons are in the JSON `catalog_note`. Five talents are exceptional (active)
and need realm cast handlers: Sweeping Strikes, Piercing Howl, Death Wish, Last Stand and
Concussion Blow. Each talent's `realm_effect` follows its DBC values, so the talent tooltips
are accurate.

## Tier Bonus, caps and points

The caps are 10/30/50/60. The Tier Bonus is cumulative: T1 +12 Str, +6 Agi, +12 Sta, +3 Spi.
T2 and T3 grow it to about 2.3× and 4×. Points come one per level from level 2, which gives
9/29/49/59 at the cap. Respec uses vanilla's escalating gold cost (proposed).

## Quests

Both quests should be **synthesized**. The 1.12 client receives quest text in server packets
and has no quest DBC, so borrowing an ID gains nothing and risks cache collisions. They use
ids 64001 and 64002 and `ZoneOrSort -81`, so they appear under the stock "Warrior" quest-log
header (QuestSort.dbc 81). The Advancement Quest is a speak-to quest, modelled on vanilla 1505
"Veteran Uzzek". Uzzek (creature 5810, the Barrens) is a candidate model for the Hub's
Warrior trainer. Champion's quest has an objective the realm never credits. The tone comes
from 1718, 1791 and 1712, and no text is copied.

## Checked against vmangos and the client code

- **Deep Wounds** (vmangos `SpellEffects.cpp`, the `EffectDummy` arm for 12162/12850/12868)
  takes the average of `UNIT_FIELD_MINDAMAGE` and `UNIT_FIELD_MAXDAMAGE`, so attack power is
  included. It multiplies that by 0.2/0.4/0.6 and casts 12721 with base points = total / 4.
  12721 is 4 ticks of 3 s over 12 s. The `realm_effect` text matches. vmangos uses the
  off-hand when that swing timer is further along. The realm can ignore that until dual wield
  exists.
- **Enrage.** 12880..14204 carry 12 proc charges and a 12 s duration. The bonus ends after
  12 s or 12 melee swings, whichever comes first.
- **Last Stand** (same file): vmangos casts 12976 with base points = 30% of max health.
- **Cooldowns.** `verify` now requires the realm's `cooldown_ms` to equal the longer of
  Spell.dbc RecoveryTime and CategoryRecoveryTime, and `gcd_ms` to equal StartRecoveryTime.
  The client runs both off `SMSG_SPELL_GO`. All 10 skills pass.
- **Spellbook visibility.** The stock gate is `SpellDisplay::in_spellbook`
  (`benilla-formats/src/spells/display.rs:453`, used by `ui_spellbook.rs` `build_book`). It
  hides a spell with Attributes `0x80` (do-not-display), `0x20`, or a nonzero castUI. Passive
  alone does not hide a spell. `extract_spells.py` now dumps `in_spellbook`:
  - All 106 passive talent rank spells in the trees carry `0x80` and stay out of the book.
  - Only the five active talents appear, under their skill line: Sweeping Strikes, Piercing
    Howl, Death Wish, Last Stand and Concussion Blow.
  - All 10 Calling skills are bookable, and `verify` now checks that.

## Implementation pointers (read-only survey, nothing changed)

**Blanking the rank text (#8, "no ranks").** Every reader of the rank text goes through
`SpellDisplay.rank` in the `Spells` resource (`crates/benilla-app/src/ui_action/mod.rs`,
`load_spells`, `commands.insert_resource(Spells { catalog, … })`):
- spellbook slot `ui_spellbook.rs:373` (`rank: d.and_then(|d| d.rank.clone())`) and its sort
  key at `:401`;
- the tooltip's rank column, the `rank` field of `benilla-ui/src/script/tooltip_spell.rs`,
  which is built from the catalog;
- the learn line "You have learned a new ability: %s" (`spell/net.rs:510-511`, `ranked_name()`);
- the craft and pet readers (`ui_craft.rs:205`, `ui_pet/bar.rs:134`, `ui_pet_book.rs:156`),
  which don't matter here.

`SpellCatalog.spells` is private (`benilla-formats/src/spells/mod.rs:282`), and there is only
`get`, no `get_mut`. The smallest carry has two parts:
1. A child module `spells/twilight.rs`. Its `mod` line in `spells/mod.rs` is the one upstream
   line touched. It adds `impl SpellCatalog { pub fn clear_ranks(&mut self, ids: impl
   IntoIterator<Item = u32>) }`, which sets `rank = None`.
2. A Twilight-mode system in `crates/benilla-app/src/twilight/`. It runs once
   `Res<Spells>` exists (`resource_added::<Spells>`) and calls `clear_ranks` on every Calling
   skill id. Clearing all spells works too, since Twilight's Eve has no ranks.

`Spells` is a change-detected resource, so the spellbook, action and tooltip feeds re-push by
themselves.

**Battle Stance form.** Nothing in the client needs changing. The realm sets the form byte in
`UNIT_FIELD_BYTES_1` (`ObjectFields::unit_shapeshift_form`). The client already reads it in:
- `spell/usable.rs:387` (the grey-out walk) and `spell/cast_send.rs:638` (press-time refusal),
  through `SpellDisplay::form_refusal` (`display.rs:583`);
- `ui_action/feed.rs:323-333`, which maps form 17 through `SpellShapeshiftForm.dbc`'s
  `bonus_bar` to `set_bonus_bar_offset(1)` and fires `UPDATE_BONUS_ACTIONBAR`.

The stance bar (`ui_shapeshift.rs`) lists only known shapeshift spells. With Battle Stance
(2457) unlearned it stays empty. If the realm also applies the 2457 aura, the buff frame
shows the Battle Stance icon the way real servers do. That is optional.

## Balance simulation

`tools/twilights-eve/simulate_fights.py` is a stdlib Monte Carlo. It pits a human Warrior
(naked base stats + Tier Bonus + an assumed weapon and armour) against a same-level
`creature_classlevelstats` warrior-class mob. It uses `vanilla-combat-rules.md` (hit table,
armour, rage, two-roll yellow attacks) and the formulas in `callings.json`. Its inputs are
`mob_base_stats.json` and `warrior_level_stats.json` from the combat-reference branch
(`worktree-agent-a37ae08df1dab3a30`); pass `--mobs` and `--stats` until that branch lands.
Results, 2000 fights per level, median (p10–p90). The Basic Warrior rows were run on the old
eight-skill kit:

| Calling | L | player hp | mob hp | fight | hp lost |
|---|---|---|---|---|---|
| Basic Warrior | 1 | 60 | 42 | 7.7 s (6.2–9.2) | 12% |
| Basic Warrior | 5 | 136 | 102 | 14.4 s (12.0–16.7) | 29% |
| Basic Warrior | 10 | 231 | 198 | 20.6 s (18.1–25.6) | 59% |
| Warrior (T1) | 1 | 180 | 42 | 4.2 s | 1% |
| Warrior (T1) | 10 | 351 | 198 | 13.1 s (10.5–18.2) | 21% |
| Warrior (T1) | 20 | 569 | 484 | 18.9 s (16.2–24.0) | 39% |
| Warrior (T1) | 30 | 884 | 955 | 24.9 s (22.1–32.9) | 49% |

What the simulation shows:
- Basic Warrior's L5–10 fights land in the 15–25 s band as written.
- Level 1 is short because a level-1 mob has 42 hp. White swings do over 70% of the damage,
  so no skill formula can stretch it. If that matters, raise low-level mob health on the mob
  side.
- In Tier 1, Mortal Strike was starved of rage. I raised it to +25 + 2.5L (+100 at L30), and
  the simulated rotation now pools rage for it. Execute was overtuned at low levels; I lowered
  it to 15 + 4.2L + 0.2AP.
- Tier 1 at L30 reaches about 25 s only with the green-weapon assumption from L15 (about
  11–17 DPS). With white weapons it takes about 30 s, so Token greens are part of the curve.
- The Tier Bonus makes a fresh Tier 1 clearly stronger: about 3× Basic Warrior's L1 health
  and half the fight time at L10.
- Rend barely appears in the Tier 1 rotation while pooling for Mortal Strike. That is a
  rotation choice, not a bug.

## Status / TODO

Done since the first pass:
- vmangos checks for Deep Wounds and Enrage.
- Category cooldown and GCD cross-checks in `verify`.
- Spellbook visibility checked and enforced.
- Rank-blanking and stance pointers.
- `extract_spells.py selftest` (MPQ hash keys, formula rounding, and DBC facts when the data
  is present).
- The balance simulation, with Mortal Strike and Execute retuned.

Built in #14 (talents):
- The trees are Calling data: `[[calling.talent_tab]]` in `crates/twilight-realm/content/callings.toml`,
  with one `effects` entry per talent that the realm applies (`src/talents.rs`), and the
  `client_spellmod` cells sent at login, on a learn and on a respec. The glue rebuilds the
  client's talent catalog from the current Calling (`benilla-app/src/twilight/talents.rs`).
- The stock UI opens the talent frame only from level 10 (`UIParent.lua` `ToggleTalentFrame`,
  `MainMenuBarMicroButtons.lua` `UpdateTalentButton`). Twilight mode redefines both without the
  level gate, since points start at level 2.
- The respec price follows vmangos `Player::resetTalentsCost` (1 g, 5 g, 10 g, then 5 g more a
  time up to 50 g), not the JSON's proposed `first`/`step`, and has no monthly discount. The
  Warrior trainer is Uzzek (creature 5810, display 4391), a `content/hub.toml` NPC beside the
  Crossroads start point; his gossip lives in `src/trainer.rs`.

Still open:
- Pushing spell mods so tooltips follow the realm's formulas is untested. Improved Charge's
  mod has family mask 0, so the realm needs a class script for it.
- The simulation leaves out talents, multi-mob pulls (Thunder Clap, Cleave, Sweeping
  Strikes), the Wailing Caverns elites, and mob types other than warrior-class. Its gear
  table is an assumption.
- The Tier 2 and Tier 3 Tier Bonus values are placeholders.
- The simulator has no tests of its own. Its numbers are a tuning aid, and the realm's tests
  remain the authority.
