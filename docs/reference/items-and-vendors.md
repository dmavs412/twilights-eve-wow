# Items and vendors: reference data for loot, goods and Tokens

Offline data for tickets #9 (Loot and gear), #10 (Goods vendor, durability and death) and #12
(Token vendor). Everything here is real 1.12 data. The JSON files are inputs for the realm crate;
they are not loaded by anything yet.

| file | what |
|---|---|
| `data/twilights-eve/loot_items.json` | world-drop pools for levels 1-32 by level and quality, random-enchant greens, Wailing Caverns blues |
| `data/twilights-eve/token_vendor.json` | proposed Token-vendor greens for bands 1-10, 11-20 and 21-30, with a Token cost |
| `data/twilights-eve/goods_vendor.json` | food and drink for levels 1-30, the Crossroads stock, durability loss and the repair tables |
| `data/twilights-eve/warrior_proficiency.json` | what a Warrior can equip, and from what level |
| `tools/twilights-eve/import_items.py` | the generator (stdlib Python 3.10+) |
| `tools/twilights-eve/mpq_dbc.py` | a minimal MPQ and DBC reader, a port of `crates/benilla-mpq` |

## Source

- **vmangos world DB**: the `db-sqlite-<sha>.zip` asset of the `db_latest` release on
  [vmangos/core](https://github.com/vmangos/core/releases/tag/db_latest). It holds `mangos.sqlite`.
  The current files were made from `db-sqlite-4641790.zip` (the 2026-09-28 snapshot).
- **Patch gating**: vmangos keeps one row per patch. The script takes the highest
  `patch <= 10` row of `item_template`, `creature_template` and `map_template`, the highest
  `build <= 5875` row of `spell_template`, loot rows with `patch_min <= 10 <= patch_max`, and
  drops items that `forbidden_items` bans at patch 10. This is the same rule vmangos uses for a
  1.12 world (`LootMgr.cpp`, `ObjectMgr::LoadVendors`).
- **Client DBCs** from the 1.12.1 MPQs (`patch-2.MPQ`, then `patch.MPQ`, then `dbc.MPQ`):
  `DurabilityCosts.dbc`, `DurabilityQuality.dbc`, `SpellDuration.dbc`,
  `ItemRandomProperties.dbc` and `SpellItemEnchantment.dbc`.
- **Rules** (repair, durability loss, food ticks, proficiency) were read from vmangos source:
  `Player::DurabilityRepair`, `Player::DurabilityLossAll`, `Unit::DealDamage`,
  `Aura::HandleModRegen`, `mangosd.conf.dist.in`, `playercreateinfo_spell` and `npc_trainer`.

## Re-running

```bash
# embeddable or system Python 3.10+; no packages needed
python -I tools/twilights-eve/import_items.py \
    --db   "$LOCALAPPDATA/Temp/claude/twe-cache/vmangos/sqlite-dump/mangos.sqlite" \
    --data "C:/Users/davis/wow-data/Data"
# or let it download and unpack the snapshot into --cache when --db is missing:
python -I tools/twilights-eve/import_items.py --fetch
```

`--out` defaults to `data/twilights-eve`. The output is deterministic: the same DB and MPQs give
byte-identical files. A newer `db_latest` snapshot can shift counts slightly, since vmangos keeps
fixing loot.

## Loot pools (`loot_items.json`)

### What counts as a candidate

An item is a candidate when all of these hold:

1. **Kind**: weapon (not misc tools or fishing poles), armour (not shirts or tabards), a bag, or
   grey vendor trash (class 15 or 12, quality 0, no spell). Quality is grey, white, green or blue.
2. **Level**: item level 1-32 and required level 32 or lower. Vendor trash has item level 1 in
   the DB, so its level is the median level of the mobs that drop it, which must be 32 or lower.
3. **Real world drop**: at least one creature that is spawned in 1.12 on Eastern Kingdoms or
   Kalimdor (maps 0 and 1), and is not a boss (`rank` 3), drops it as a normal drop (not a quest
   drop). References in `reference_loot_template` are expanded.

### Exclusions (counts from the current DB)

| reason | count | notes |
|---|---:|---|
| `no_open_world_drop` | 1383 | vendor-only, crafted, quest rewards, dungeon-only |
| `not_obtainable` | 931 | `extra_flags & 0x04`, vmangos's "never obtainable in vanilla" (test and placeholder items live here) |
| `random_enchant` | 446 | stats rolled at drop; listed separately, see below |
| `drops_only_above_level_32` | 114 | only mobs above level 37 (or, for trash, a median above 32) drop it |
| `spell_effect_unsupported` | 35 | a use, chance-on-hit or unsupported equip spell (e.g. Shadowfang, Night Reaver, Skeletal Club, casting gloves) |
| `quest_item` | 4 | starts a quest or is bound to one (Smotts' Cutlass, Demon Scarred Cloak, Shadow Hunter Knife, Torch of Retribution) |
| `deprecated_flag` | 2 | `ITEM_FLAG_DEPRECATED` (Easter Dress, Brushwood Blade) |
| `bind_on_pickup` | 1 | BoP greens and blues (Pumpkin Bag) |
| `forbidden_in_1_12`, `test_or_placeholder_name` | 0 | already removed by the rules above, kept as a guard |

The small buckets are listed by name in `excluded`; the big ones are counts only.

**Equip spells the realm can honour** stay in. An equip spell is kept when every effect is
`APPLY_AURA` with one of: attack power, ranged attack power, crit, hit, dodge, parry, block
chance, block value, a skill (Defense, weapon skills), a stat, armour or a resistance, and health
regen. Each kept item lists them under `effects[].mods` (e.g. Gauntlets of Ogre Strength:
`attack_power 16`; Ring of Defense: `skill 95 +4`, where 95 is Defense). Use, chance-on-hit, and
caster auras (spell damage, healing, mana regen) are not honoured, so items with them are out.

**Not excluded on purpose**: cloth, leather and caster stats. A Warrior can wear cloth in
vanilla and vendor it otherwise; drops do not need to fit the looter. Each item carries
`warrior.usable`, `warrior.from_level` and, where its fixed stats are only mana, intellect or
spirit, `caster_stats_only`. Unique-equipped world-drop rings are kept (they are BoE).

### Counts by band (fixed-stat pools)

Band is `pool_level` (item level for gear and bags). "Warrior" counts weapons and armour a
Warrior can use that are not `caster_stats_only`.

| band | grey | white | green | blue | Warrior grey / white / green / blue |
|---|---:|---:|---:|---:|---|
| 1-5 | 32 | 24 | 0 | 0 | 23 / 18 / 0 / 0 |
| 6-10 | 56 | 128 | 8 | 0 | 33 / 128 / 6 / 0 |
| 11-15 | 45 | 87 | 63 | 0 | 33 / 82 / 52 / 0 |
| 16-20 | 56 | 3 | 99 | 6 | 37 / 3 / 78 / 6 |
| 21-25 | 61 | 27 | 95 | 33 | 37 / 21 / 87 / 30 |
| 26-30 | 57 | 2 | 97 | 28 | 37 / 2 / 84 / 25 |
| 31-32 | 8 | 0 | 40 | 11 | 5 / 0 / 34 / 11 |

1066 items in all: 693 armour, 248 weapons, 109 vendor trash, 16 bags.

Two gaps come from the data, not the filters. There are no greens below item level 7, because
vanilla has none. White world-drop gear clusters at item levels 6-15 and 21-25, so a level 16-20
or 26-30 mob should borrow whites from a neighbouring band. The realm's roll should take items
from `pools_by_level` at L-2..L+2 for a mob of level L and widen the window when a quality's list
is empty.

### Random-enchant greens ("of the Bear")

446 world-drop greens (4 in 6-10, 54 in 11-15, 96 in 16-20, 97 in 21-25, 132 in 26-30, 63 in
31-32) have no fixed stats. vmangos rolls `item_enchantment_template(entry =
item.random_property)` at drop time to pick an `ItemRandomProperties.dbc` id, whose enchantments
carry the stats. They are kept out of the fixed pools and listed under `random_enchant`, with the
suffix tables already resolved (`suffix_tables[random_property]`: suffix name, chance, stat
mods). 1.12 suffix stats are equip spells (`MOD_STAT`, `MOD_RESISTANCE`, attack power), and those
are decoded. Caster suffixes ("of Healing", "of Sorcery", "of the Owl" mana regen) are marked
`supported: false`.

To use them, the realm must roll a suffix, send its id in `ITEM_FIELD_RANDOM_PROPERTIES_ID` so
that the stock client renders the name and stat lines from its own DBC, apply the mods
server-side, and save the id with the item. Without that, they show and behave as stat-less
greens. This doubles the green pool from 11-15 up, so it is worth doing in #9 if it fits.

### Dungeon blues (`dungeon_blues`)

- `wailing_caverns`: all 44 blues any Wailing Caverns creature can drop in 1.12. Mutanus,
  Trigore, Boahn and Mad Magglish have no static spawn row (they are summoned or come from
  scripts and pools), so the script counts them as WC creatures. 12 are the instance's own named
  drops (`wailing_caverns_only`): Kresh's Back, Glowing Lizardscale Cloak, Leggings of the Fang,
  Cobrahn's Grasp, Venomstrike, Stinging Viper, Seedcloud Buckler, Living Root, Deep Fathom Ring,
  Slime-encrusted Pads, Mutant Scale Breastplate and Firebelcher. The rest are BoE world-drop
  blues that WC trash shares with the world.
- `level_17_25_warrior_blues`: 59 other blue drops with required level 17-25, from dungeons
  (Deadmines, Shadowfang Keep, Blackfathom Deeps, Stockade, Gnomeregan, Razorfen Kraul) or the
  world. Each has fixed stats, no unsupported spell, no caster-only stats, no cloth body armour,
  and is usable by a Warrior at its required level.
- `proposed_boss_pool` (61 entries): the WC blues a Warrior can sensibly use (29), plus the
  required-level 18-22 extras. Venomstrike and Stinging Viper are out because of their poison
  procs, Seedcloud Buckler because its stats are caster-only, and Firebelcher because it is a
  wand.

## Token vendor (`token_vendor.json`)

For each band (by required level: 1-10, 11-20, 21-30) and each slot, the stock has the best
Warrior green near the top of the band and the best around its middle. Only fixed-stat greens
from the loot pools that a Warrior can use at their required level are picked. Cloth body
armour is left out, and so are items with no Warrior value. The Warrior score is
`str*1 + sta*0.8 + agi*0.7 + armor/20 + dps/2`, plus equip auras (AP/2, Defense/2).
14 + 25 + 29 items.

**Proposed cost**: `max(1, round(item_level / 4 * slot_mod))`, where `slot_mod` is 1.0 for two-handers,
0.9 for chest, legs and head, 0.8 for one-handers, 0.7 for shield, shoulders, hands and feet,
0.6 for waist, neck and finger, and 0.5 for wrist, back and ranged. This gives 1-4 Tokens in 1-10,
2-6 in 11-20 and 3-7 in 21-30, so a full band costs 35, 97 and 156 Tokens. Ticket #11 makes
Tokens an occasional quest reward. If that is about one Token per two or three quests, a few
greens per Tier is the right pace. Tune `slot_mod` or the divisor once #11's rate is fixed.

**Caveat for #12: the 1.12 merchant window cannot show a Token price.** `SMSG_LIST_INVENTORY`
in 1.12 carries only a copper price per item. Extended costs (item or currency prices) came in
2.0. The ways to make it work:

- (a) Make Tokens a real bag item and send each entry's Token cost as its copper price,
  so the window reads "5c" for 5 Tokens. The realm takes Tokens instead of money on buy, and
  refuses with `BUY_ERR_NOT_ENOUGHT_MONEY` when they run short. The money display then misleads.
- (b) Sell through a gossip menu that names the cost, and give the item on pick.
- (c) Put the cost into the item name sent in the vendor's item query.

(a) is the most "stock"; ticket #12 should decide.

## Goods vendor (`goods_vendor.json`)

### Food and drink

There are 35 food and drink items with required level 30 or lower, sold by vendors with a
permanent (non-event) 1.12 spawn. Each lists `buy_price` per `buy_count` (5 per purchase),
`sell_price`, the spell, and the total health or mana restored with its duration. A
`MOD_REGEN` / `MOD_POWER_REGEN` aura gives its amount every 5 s, so the total is
`amount * duration / 5000` (Tough Jerky: 17 * 18 s / 5 = 61 health over 18 s).
`proposed_crossroads_stock` is what the Crossroads really sells:

| tier (req. level) | food (health) | drink (mana) | price per 5 |
|---|---|---|---|
| 1 | Tough Jerky (Zargh), Shiny Red Apple (innkeeper), Tough Hunk of Bread (Moorane): 61 / 18 s | Refreshing Spring Water: 151 / 18 s | 25c |
| 5 | Haunch of Meat, Tel'Abim Banana, Freshly Baked Bread: 243 / 21 s | Ice Cold Milk: 436 / 21 s | 1s 25c |
| 15 | Mutton Chop, Snapvine Watermelon, Moist Cornbread: 552 / 24 s | Melon Juice: 835 / 24 s | 5s |
| 25 | Wild Hog Shank, Goldenbark Apple, Mulgore Spice Bread: 874 / 27 s | Sweet Nectar: 1344 / 27 s | 10s |

Food and drink work only out of combat, and the aura breaks when you stand up or are damaged.
Warriors have no mana, so drink is there only for the stock list.

### Selling and buyback

A vendor pays `sell_price * count`, and items with `sell_price` 0 cannot be sold. Buyback (12
slots) costs the same amount. Every item in the JSON files carries `sell_price` and `buy_price`.

### Repair (vmangos `Player::DurabilityRepair`)

```
lost   = max_durability - durability
mult   = DurabilityCosts.dbc[item_level].multiplier[idx]
         idx = item subclass (weapons) | subclass + 21 (armour) | 0 (anything else)
qmod   = DurabilityQuality.dbc[(quality + 1) * 2]
         poor 0.6, common 0.8, uncommon 1.0, rare 1.25, epic 2.5
cost   = uint(lost * mult * qmod)
cost   = uint(cost * discount + 0.5)     # discount 1.0 (reputation discounts not modelled)
if cost == 0: cost = 1                   # per item, only when lost > 0
```

Both tables are copied into `goods_vendor.json` (`repair.durability_costs` for item levels 1-60,
29 multipliers each; `repair.durability_quality`). `DurabilityCosts.dbc` comes from
`patch-2.MPQ`. Worked example: a green mail chest (subclass 3, so idx 24) of item level 20 has a
multiplier of 5. Twenty lost points cost 20 * 5 * 1.0 = 100c, i.e. 1s.

### Durability loss

- **Death** (killed by a creature): every equipped item loses `floor(max * 0.10)` points,
  at least 1. The spec's "10% durability" matches vanilla exactly.
- **Spirit healer** (not used in the slice): 25% on every item, bags included.
- **Combat**: each damaging hit dealt or taken has a 0.5% chance
  (`DurabilityLossChance.Damage = 0.5`) to take 1 point from a random equipment slot; an empty
  slot loses nothing.
- **Broken**: at 0 durability the item's stats are removed (`_ApplyItemMods(..., false)`) and
  come back when it is repaired.

## Warrior proficiency (`warrior_proficiency.json`)

Vanilla 1.12, from `playercreateinfo_spell` (class 1) and `npc_trainer`:

- Armour: cloth, leather, mail, shields (and bucklers) and Block are known at creation. **Plate**
  is trained at **level 40** (trainer spell 7109 teaches 750).
- Weapons at creation differ by race:
  - human: 1H axe, 1H mace, 1H sword
  - orc: 1H axe, 2H axe, 1H sword
  - dwarf: 1H axe, 2H axe, 1H mace
  - night elf: dagger, 1H mace, 1H sword
  - undead: dagger, 1H sword, 2H sword
  - tauren: 1H axe, 1H mace, 2H mace
  - gnome: dagger, 1H mace, 1H sword
  - troll: 1H axe, dagger, thrown
- Every other Warrior weapon skill (2H axes, maces and swords, staves, fist weapons, daggers,
  bows, guns, crossbows, thrown) costs 10s at a Weapon Master from level 1. **Polearms need
  level 20** (trainer spell 15991).
- Never: wands, librams, idols, totems.

**Rule for the realm**:

```
can_equip(item, level) =
    item.required_level <= level
    and (item.allowable_class == -1 or item.allowable_class & 1)
    and level >= gate(item.class, item.subclass)

gate: weapons 1, except polearm 20; wand never
      armour 1, except plate 40; libram, idol and totem never
```

The slice has no Weapon Master, so every weapon type is open from level 1 (polearms from 20)
whatever the race. Plate is out of reach under the level-30 cap. A refusal uses the stock
`EQUIP_ERR_NO_REQUIRED_PROFICIENCY` (or `EQUIP_ERR_CANT_EQUIP_LEVEL_I` for required level).
Every item record carries `warrior.usable`, `warrior.from_level` and `warrior.blocked_by`.

## Item record fields

`entry, name, quality, item_level, required_level, class, subclass, inventory_type` (plus
`*_name` labels), `allowable_class, allowable_race, display_id, icon` (from vmangos
`item_display_info`), `stats[]` (stat and value), `armor, block, resistances, damage[]` (min, max
and school), `speed_ms, dps, durability, sell_price, buy_price, buy_count, bonding, max_count,
stackable, container_slots, sheath, material, effects[]`, `warrior{usable, from_level,
blocked_by}`, and, for pool items, `kind, pool_level` and `drop{open_world_creatures,
creature_level_min, creature_level_max}`. Prices are in copper.

## Caveats

- vmangos is a community reconstruction. Drop sources and some qualities (e.g. Spellbinder Boots
  and Defender Spaulders are white with no stats) are what vmangos ships, not sniffed truth.
- "World drop" means some open-world creature drops it. The pools do not tie items to creature
  types, so a plainstrider can drop a Gnoll Spittle. Per-creature tables can be built later from
  the same DB, since `creature_loot_template` is there.
- The token costs and the Warrior score are proposals, not vanilla data.
- `tools/` is in this clone's `.git/info/exclude`, so the scripts were added with `git add -f`.

## Status / TODO

Stopped early because the token budget ran out. What exists is complete and re-runnable. Still open:

- No realm code reads these files yet. Tickets #9, #10 and #12 will have to load them, or turn
  them into Rust data.
- The `random_enchant` greens are resolved but sit outside the fixed pools. Whether #9 supports
  random suffixes still has to be decided.
- The Token price display is still open (see the caveat above), and so is the Token cost
  pacing, which depends on #11.
- There are no per-creature loot tables yet. The pools are by level only.
- The proposed WC boss pool (61) is broad. It could be trimmed to the 12 WC-only blues plus a
  handful of others.
