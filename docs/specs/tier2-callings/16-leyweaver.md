Title: Leyweaver: the kit, ten Talent Rows, Teodric Lumen at a ley spring on the Darkened Bank, and his trial (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 4, 5, 8, 12 and 14. Blocked by 06-mana-kinds and 15-pyromancer (the
Calling tickets run one after another: they share `master.csv`, both generated JSON files,
`callings.csv`, `quests.toml`, `quest_items.toml` and `hub.toml`).

The Leyweaver is arcane: it hits hardest on a full mana bar (Ley-Spark's theme) and spends its
mana for burst, then taps the ley to fill up again. Same role as the Initiate, ranged spell
damage.

## How to write the rows (the same in every Calling ticket of this spec)

- Edit the sheet only with `~/.local/bin/python3.15 tools/twilights-eve/sheet.py` (`add-row`,
  `set`). `docs/agents/calling-script-guide.md` has the column rules; `docs/spell_pipeline.md`
  the vocabulary (tickets 03 to 08 add to it). Every row `status=placeholder`; `modelled=yes`
  where every kind on the row runs. Each skill, Skill Choice, Talent and aura row has its own
  `spell_id` above the sheet's highest (an aura may share its skill's). `visual` is the stock
  spell suggested below, or a close one (record it).
- The `calling` row: `tier=2`, `tier_cap=50`, `row_levels=5|10|15|20|25|30|35|40|45|50`, the
  placeholder `tier_bonus` below, and `shapeshift_form` copied from the Tier 1 Calling's row.
- A carried skill is a copy of the Tier 1 row under this Calling, with the aura rows it names
  copied too. "Starts stronger": every amount block's `base` becomes `round(1.1 * (base + 30 *
  per_level))`, keeping `per_level` and `per_ap`. A carried skill with no amount block takes 10%
  more on the number this ticket names. A carried effect that names a skill this Calling lacks is
  replaced as this ticket says.
- Every skill gets a Skill Choice row (`row_level` on the skill row, three `talent` rows with
  `variant_of`), the Kick and the utility included (ADR 0028 as ticket 01 amended it). Each
  choice fills only the cells that differ, writes a `change` line whose numbers are tokens, a full
  `description`, and no `icon`. A Talent row is three `talent` rows sharing `row_key` and
  `row_level`, each with a stock icon of its own. No Tier 1 choice is copied.
- `learn_level`: blank (1) for the two carried skills, the Kick and the first two new skills; 10,
  20 and 30 as the kit table says.
- Numbers are placeholders; tickets 21 to 24 fit them. Amounts below are `base + per_level * L`.
- Run `~/.local/bin/python3.15 tools/twilights-eve/build_spells.py --data "$WOW_DATA"` and commit
  the CSV with both outputs.
- Content: the trainer goes at the end of `crates/twilight-realm/content/hub.toml` under
  `# ── Tier 2 trainers (#PARENT) ──`; quests in `content/quests.toml` after 64013 under
  `# ── Tier 2 Advancement Quests and trials (#PARENT) ──`, in the form of the rows above them;
  quest items at the end of `content/quest_items.toml`'s `item` list. A placed unit's Z is `0.0`
  until `python3.15 -I tools/twilights-eve/ground_check.py --fix` writes it. Pick quest item
  displays whose icon no other item uses; `icon_check.py` must report no new clash.
- `docs/art-assets.md`: list the trainer's borrowed look and every placeholder icon, under a
  "Tier 2 Callings" heading (make it if missing).

## The Calling

- `data/twilights-eve/callings.csv`: `leyweaver,Leyweaver,Interface\Icons\Spell_Nature_StarFall,mana,mage,warrior,100,100,,,,cloth,<the initiate row's weapons>,Leyweaver,<blurb>`.
  Blurb: "Weaves the ley itself. Hits hardest on a full bar, pours it all into one blow, then
  taps the ley to fill up again."
- `calling` row: `tier_bonus=strength=0;agility=0;stamina=10;intellect=30;spirit=20`.

## The kit

| Key | Name | Learn | Row | What it does | Kinds | Visual |
|---|---|---|---|---|---|---|
| `ley_spark` | Ley-Spark | 1 | 5 | carried: the Initiate's, starts stronger (117 + 2.5L arcane, 15 crit points more at a full bar) | school_damage | as the Initiate's |
| `arcane_missiles` | Arcane Missiles | 1 | 10 | filler. Channel 3 s, a tick a second, 35 yd, 8% base mana: 35 + 1.5L arcane a tick, up to 30% more by the bar's fill at each tick | school_damage (`pct_at_full_mana=30`, ticket 06), `cast_kind=channel` | 5143 |
| `ley_ward` | Ley Ward | 1 | 15 | carried: the Initiate's; the shield is 11% of max mana instead of 10% | absorb | as the Initiate's |
| `arcane_surge` | Arcane Surge | 1 | 20 | instant, 35 yd, 20 s cooldown; costs 100 mana and up to 600 more of what is left: 60 + 3L arcane plus 60% of the mana spent | school_damage (`per_mana_pct=60`, ticket 06), `cost_kind=all_remaining`, `cost_extra_max=600` | 1449 |
| `ley_tap` | Ley Tap | 10 | 30 | channel 6 s, a tick every 2 s, no cost, 60 s cooldown: each tick gives 12% of max mana | energize (`pct_of_max=12`), `cast_kind=channel` | 12051 |
| `counterspell` | Counterspell | 1 | 35 | carried Kick: the Initiate's, lockout 6.5 s, a stopped cast gives 10% of max mana | interrupt, energize | as the Initiate's |
| `attunement` | Attunement | 20 | 40 | instant, no cost, 90 s cooldown: for 8 s Ley-Spark and Arcane Missiles cost nothing | grant_aura (aura: `next_cast skill=ley_spark|arcane_missiles;cost_pct=-100`, no charges) | 12043 |
| `ley_bind` | Ley Bind | 30 | 45 | utility. 1.5 s cast, 30 yd, 12% base mana: the foe is bound for 20 s; damage breaks it | incapacitate (aura row, `break_on: damage=yes`) | 118 |

Aura rows: a copy of the Initiate's Ley Ward shield if it has one, `aura_attunement`,
`aura_ley_bind`.

## The Talent Rows

| Level | Row | Choice A | Choice B | Choice C |
|---|---|---|---|---|
| 5 | Ley-Spark | Brimming Spark: 30 crit points at a full bar | Quick Spark: 1 s cast | Sharp Spark: 20% more damage |
| 10 | Arcane Missiles | Brimming Missiles: up to 50% more at a full bar | Extra Missile: 4 s, four ticks | Mana Missiles: each tick gives back 1% of max mana |
| 15 | Ley Ward | Deep Ward: 15% of max mana | Bursting Ward: a ward soaked whole hits enemies within 8 yd for its amount (`burst_radius_yd=8`) | Returning Ward: each soak gives back 30% of what it soaked as mana (`energize_pct=30`) |
| 20 | Arcane Surge | Overflow: 80% of the mana spent | Wide Surge: 50% of the hit to enemies within 6 yd of the target (`splash_pct`, `splash_yd`) | Quick Surge: 12 s cooldown |
| 25 | Talent, `row_key=leyweaver_wellspring` | Arcane Mind: 10% more intellect (`mod_stat`) | Deep Reserves: 200 more max mana (`max_power`) | Arcane Focus: Ley-Spark and Arcane Missiles cost 20% less (`skill_cost`, two effects) |
| 30 | Ley Tap | Deep Tap: 15% a tick | Quick Tap: 40 s cooldown | Walking Tap: moving does not end it (`interrupt_on=pushback|kick`) |
| 35 | Counterspell | Long Counter: lockout 8.5 s | Siphoning Counter: 20% of max mana | Spark Counter: a stopped cast resets Arcane Surge (`reset_cooldowns`) |
| 40 | Attunement | Long Attunement: 12 s | Quick Attunement: 60 s cooldown | Attuned Haste: also 20% faster casts while up (`mod_cast_speed_pct`) |
| 45 | Ley Bind | Long Bind: 30 s | Quick Bind: instant | Far Bind: 40 yd |
| 50 | Talent, `row_key=leyweaver_ley_master` | Arcane Potency: Arcane Surge crits 15% more often | Overcharge: a Ley-Spark crit gives 5% of base mana (`proc on=crit;skill=ley_spark;energize_pct_of_base=5`) | Deep Well: Ley Tap's cooldown 30 s shorter (spellmod `COOLDOWN`) |

## The trainer

Teodric Lumen keeps a ley spring on the Darkened Bank, Duskwood's long strip along the river
below Elwynn (band 30 to 32), among the spiders and the wolves, far from any road. Find: hard.

```
guid = 9300115, entry = 190115, name = "Teodric Lumen", subname = "Leyweaver", level = 50,
display_id = 1470 (Larimaine Purdue's look, a placeholder), faction_template = 35,
role = "giver", npc_flags = 0x3, map = 0, position = [-10060.0, -560.0, 0.0], orientation = 1.57,
hidden = true, text_id = 64185,
greeting = "Can you feel it? Under your feet, like a river under ice. No? You will."
```

Stock spiders and wolves stand on dry ground near this spot (Venom Web Spiders about -10031,
-650; Green Recluses about -10086, -650). Keep him on dry land within 30 yd of the point.

## The Advancement Quest, 64404

`kind = "advancement"`, giver 190115, level 30, `zone_or_sort = -161`,
`advancement = { from = "initiate", to = "leyweaver", trial = 64437 }`,
`calling_choice = true`, money 0. Title "The Ley Beneath".

- Objectives: "Find the Leyweaver who listens to the ley, and pass his trial."
- Details: "There is a kind of mage who does not burn or freeze. He listens, and pours, and listens again.$B$BThe ley runs under the land like water under ice, and where it wells up beside a dark river, someone has been drinking from it for years. Elsa Brightwood will tell you who in Goldshire still studies the old lines. Ask her."
- Progress: "You found the spring. Did you hear it?"
- Completion: "Then pour. All of it, and fill again. You start over, Leyweaver. The ley does not."

Hints (`hint` rows on 64404):

| NPC | Option | Reply |
|---|---|---|
| Elsa Brightwood (190052) | "Who still studies the old ley lines?" | "Zaldimar Wefhellt, in Goldshire. He maps them for Azora, though Azora stopped asking." |
| Zaldimar Wefhellt (328) | "Where does the ley rise near here?" | "South, across the river, on Duskwood's bank. The Night Watch has a post down there. Their watcher sees blue lights on the water and pretends he doesn't. Ask him." |
| Watcher Callahan (2142) | "Have you seen lights on the river?" | "West of my post, along the bank, past the spiders. Blue lights at night, and a man who sits by them. He doesn't sleep. I don't ask." |

## The trial

Both: giver and ender 190115, level 30, `min_tier = 1`, `zone_or_sort = 10`. Easy: the search
was hard.

- **64436 Quiet the Bank.** `after_taken = 64404`. Kill 8 Venom Web Spiders (217) on the Darkened
  Bank. Money 2500. Objectives: "Kill 8 Venom Web Spiders on the Darkened Bank." Details: "The
  spiders spin over the spring and the ley tastes of them. Eight, and pour your bar into each
  one." Progress: "Quieter?" Completion: "Much. Listen."
- **64437 The Recluses.** `after = 64436`. Kill 6 Green Recluses (569) on the Darkened Bank.
  Money 5000. Objectives: "Kill 6 Green Recluses on the Darkened Bank." Details: "Last. The
  recluses are slower and bigger. Strike from a full bar, empty it, and fill it again before the
  next." Progress: "Full again?" Completion: "Full again. That is the whole of it. Sit."

## Failing tests first

New file `crates/twilight-realm/tests/tier2_leyweaver.rs` (setup as `tests/calling_choice.rs`):

- A level 30 Initiate gets the Calling Choice listing 64403 and 64404.
- With 64404 taken, Elsa, Zaldimar and Watcher Callahan show their hint lines; Teodric makes no
  map mark until met.
- The trial chain runs in order and gates 64404's hand-in; after it, the character is a level 1
  Leyweaver who knows Ley-Spark, Arcane Missiles, Ley Ward, Arcane Surge and Counterspell, and not
  Ley Tap until level 10.
- Arcane Surge from a full bar empties it up to the extra and hits harder than from a near-empty
  bar.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `build_spells.py --check` is clean; `ground_check.py` flags nothing; `icon_check.py` reports no
  new clash.

Assumptions allowed: every number, `visual` and icon; the Calling icon path; the trainer's exact
spot on the bank.
