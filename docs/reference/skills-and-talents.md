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

## Tier 0, Basic Warrior: the eight skills (levels 1–10)

`L` is the level within the Tier. `W` is a white-swing roll, and `Wn` is a normalized roll.
Each linear term is `round(base + per_level*L + per_ap*AP)`.

| skill | id | rank | icon | rage | cd | client gate | realm formula | value at L1/5/10 |
|---|---|---|---|---|---|---|---|---|
| Heroic Strike | 78 | Rank 1 | 856 Ability_Rogue_Ambush | 15 | – (next swing) | – | W + 9.5+1.4L | +11/17/24 |
| Charge | 100 | Rank 1 | 457 Ability_Warrior_Charge | 0 | 15 s | Battle, 8–25 yd, out of combat | rage 8.5+0.14L, 1 s stun (7922) | 9/9/10 |
| Rend | 772 | Rank 1 | 245 Ability_Gouge | 10 | – | Battle/Def | 3 ticks/9 s of 4+0.24L+0.01AP | 5/6/7 per tick |
| Thunder Clap | 6343 | Rank 1 | 199 Spell_Nature_ThunderClap | 20 | 4 s | Battle | 8 yd, 4 targets, 2.6+1.23L+0.05AP; 10% attack slow 10 s | 5/11/19 |
| Battle Shout | 6673 | Rank 1 | 456 Ability_Warrior_BattleShout | 10 | – | – | +AP 14.5+0.5L, 2 min (the client's own scaling) | 15/17/20 |
| Hamstring | 1715 | Rank 1 | 23 Ability_ShockWave | 10 | – | Battle/Berserker | 0.7+0.54L+0.02AP; 40% slow 15 s | 2/4/8 |
| Overpower | 7384 | Rank 1 | 26 Ability_MeleeDamage | 5 | 5 s | Battle, combo point | Wn + 2+0.5L, cannot be dodged, parried or blocked | +3/5/7 |
| Bloodrage | 2687 | (none) | 86 Ability_Racial_BloodRage | 20% base hp | 60 s | – | 9.6+0.25L rage now + 10 over 10 s (29131) | 10/11/12 |

The vanilla rank progressions behind each fit are in the JSON's `vanilla_reference` fields.

## Tier 1, Warrior: eight skills (levels 1–30, Tier Bonus applies)

Mortal Strike 12294 (R1), Overpower 7887 (R2), Charge 6178 (R2), Rend 6546 (R2), Thunder Clap
8198 (R2), Cleave 845 (R1), Execute 5308 (R1) and Battle Shout 6192 (R3). All of them work in
Battle Stance. Values at L1/10/20/30: MS Wn+22/40/60/80; Overpower +7/12/18/24; Charge
11–14 rage; Rend 6–14 per tick over 4 ticks; Thunder Clap 12/26/41/57; Cleave +4/8/12/16 on 2
targets; Execute 54/95/140/185 plus 2–5 per extra rage; Battle Shout 22/40/60/80 AP. Mortal
Strike is a level-40 talent in vanilla. Here it is the Tier 1 signature, scaled down at low
levels. It is left out of the Tier 1 tree, which stops at tier 5. Heroic Strike, Hamstring and
Bloodrage drop out of the Tier 1 set.

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

**Basic Warrior** has one tab, "Basic Warrior" (id 900, WarriorArms art), and 9 points.
Capacity is 22 ranks over 2 tiers, and 9 points cannot reach tier 3. Tier 1: Improved Heroic
Strike 124 ×3, Cruelty 157 ×3 (cut from 5), Improved Rend 127 ×3, Improved Battle Shout 154 ×3
(cut). Tier 2: Improved Charge 126 ×2, Improved Overpower 131 ×2, Deep Wounds 121 ×3 (requires
Improved Rend 3/3, arrow straight down), Improved Thunder Clap 128 ×3.

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

## Status / TODO

The work stopped early on the token budget. What is still open:
- Rank blanking for "no ranks" (#8) is only proposed. Nobody has checked whether passive
  talent rank spells show up in the stock spellbook.
- The tooltip-sync spell-mod idea is untested. Improved Charge's mod has a family mask of 0,
  so it needs a class script on the realm.
- Enrage's swing cap (charges on 12880–14204) and the Deep Wounds tick math (12721, 4 ticks
  over 12 s) still need checking against vmangos.
- The realm cost check is in place, but cooldowns that come only from the category cooldown
  (Charge, Thunder Clap, Overpower, Mortal Strike, Shield Bash) are recorded as prose
  (`cooldown_note`), not cross-checked.
- The balance numbers are first guesses. The level tables assume a naked warrior's AP. Nobody
  has simulated time-to-kill against Barrens mobs.
- The Tier 2 and Tier 3 Tier Bonus values are placeholders.
- `extract_spells.py` has no tests of its own. The `verify` exit code is the gate.
