Title: Ironclad: the kit, ten Talent Rows, Sergeant Maera Holt at the Lakeshire bridge, and her trial (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 4, 5, 6, 12 and 14. Blocked by 03-holy-tank-kinds and
10-sharpshooter (the Calling tickets run one after another: they share `master.csv`, both
generated JSON files, `callings.csv`, `quests.toml`, `quest_items.toml` and `hub.toml`).

The Ironclad is the Vanguard Line's wall of steel: every block builds Conviction, and it spends
Conviction on shield blows. Same role as the Vanguard, tank.

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

- `data/twilights-eve/callings.csv`: `ironclad,Ironclad,Interface\Icons\INV_Shield_06,rage,warrior,warrior,100,100,,,,cloth|leather|mail|plate|shield,<the vanguard row's weapons>,Ironclad,<blurb>`.
  Blurb: "A wall of steel the pack breaks against. Every block builds Conviction, and every
  shield blow spends it."
- `calling` row: `tier_bonus=strength=24;agility=12;stamina=24;intellect=0;spirit=6`.

## The kit

| Key | Name | Learn | Row | What it does | Kinds | Visual |
|---|---|---|---|---|---|---|
| `righteous_smash` | Righteous Smash | 1 | 5 | carried: the Vanguard's, starts stronger (bonus 90 + 2L), a stack of Conviction | weapon_strike, add_aura_stacks | as the Vanguard's |
| `shield_slam` | Shield Slam | 1 | 10 | instant, melee, 15 rage, 6 s cooldown, needs a shield: weapon damage + 30 + 2L + 200% of Block Value, then spends every Conviction stack for 12 + 1.2L holy damage a stack | weapon_strike (`block_value_pct=200`), consume_aura_stacks | 23922 |
| `shield_of_penitent` | Shield of the Penitent | 1 | 15 | carried: the Vanguard's; its aura's block chance 55% instead of 50%, its proc damage by the amount rule | grant_aura | as the Vanguard's |
| `retort` | Retort | 1 | 20 | instant, melee, 5 rage, usable for 5 s after a block (`requires: caster_state=after_block;window_ms=5000`): a holy strike for 80% weapon damage + 20 + 1.5L, and 2 Conviction | weapon_strike (`school=holy`), add_aura_stacks | 6572 |
| `iron_wall` | Iron Wall | 10 | 30 | instant, no cost, 2 min cooldown: 8 s of 40% less damage taken | grant_aura (aura: mod_damage_taken_pct -40) | 871 |
| `rebuke` | Rebuke | 1 | 35 | carried Kick: the Vanguard's, lockout 4.5 s, a stopped cast gives a Conviction stack | interrupt, add_aura_stacks | as the Vanguard's |
| `steel_resolve` | Steel Resolve | 20 | 40 | instant, 10 rage, 45 s cooldown: for 15 s every block heals you 2% of max health | grant_aura (aura: `proc on=block;heal_pct=2`) | 12975 |
| `hold_the_line` | Hold the Line | 30 | 45 | utility. Instant, no cost, 20 s cooldown: taunts every enemy within 10 yd for 4 s (Bulwark Taunt's `self_area` form), and a stack of Hold the Line (5% block chance) per enemy taunted, up to 5, for 10 s | taunt, add_aura_stacks (`per_target=yes`, ticket 03) | 1161 |

Aura rows: `conviction` (copy of the Vanguard's), `aura_shield_of_penitent` (copy, block 55%),
`aura_iron_wall`, `aura_steel_resolve`, `aura_hold_the_line` (`block_pct` 5, `max_stacks=5`,
`stack_rule=stack`, 10 s).

## The Talent Rows

| Level | Row | Choice A | Choice B | Choice C |
|---|---|---|---|---|
| 5 | Righteous Smash | Smash of Conviction: 2 Conviction a hit | Shield Smash: adds 50% of Block Value (`block_value_pct=50`) | Hungry Smash: heals you 20% of the damage (`heal_pct=20`) |
| 10 | Shield Slam | Stunning Slam: also stuns 2 s (`stun`, an aura row) | Heavy Slam: 300% of Block Value | Refunding Slam: 3 rage back per Conviction spent (`refund_per_stack=3`) |
| 15 | Shield of the Penitent | Long Penance: lasts 15 s | Searing Penance: blocks deal twice the holy damage | Steadfast: 10 s cooldown |
| 20 | Retort | Quick Retort: usable for 8 s after a block | Righteous Retort: 3 Conviction | Crushing Retort: 120% weapon damage |
| 25 | Talent, `row_key=ironclad_bulwark` | Thick Plate: 10% more armour from items (`armor_items_pct`) | Shield Mastery: 10% more block chance (`block_pct`) | Bastion's Heart: 10% more max health (`max_health_pct`) |
| 30 | Iron Wall | Longer Wall: lasts 12 s | Quick Wall: 80 s cooldown | Wall of Thorns: while up, every block deals 20 + 2L holy damage to the attacker (aura `proc on=block`, `damage_attacker`) |
| 35 | Rebuke | Long Rebuke: lockout 6.5 s | Rebuke of Conviction: a stopped cast gives 2 Conviction | Shield Rebuke: a stopped cast also stuns the target 2 s |
| 40 | Steel Resolve | Iron Resolve: 3% a block | Long Resolve: lasts 25 s | Resolute Block: also 15% block chance while up |
| 45 | Hold the Line | Wider Line: 15 yd | Firm Line: 2 stacks per enemy taunted | Hard Line: the taunt lasts 6 s |
| 50 | Talent, `row_key=ironclad_last_bastion` | Last Bastion: healing on you 30% stronger under 30% health (`healing_taken_pct below_health_pct=30`) | Conviction's Edge: Shield Slam crits 10% more often (`skill_crit_pct`) | Vengeful Wall: each block has a 50% chance to give a Conviction stack (`proc on=block;chance=50;apply_aura=conviction`) |

## The trainer

Sergeant Maera Holt at the west end of the Lakeshire bridge, beside the stock Guard Howe (903, at
-9412.6, -2255.4), in plate older than most of the Watch. Find: easy.

```
guid = 9300110, entry = 190110, name = "Sergeant Maera Holt", subname = "Ironclad", level = 50,
display_id = 3287 (Ilsa Corbin's look, a placeholder), faction_template = 35, role = "giver",
npc_flags = 0x3, map = 0, position = [-9405.0, -2250.0, 0.0], faces = [-9250.0, -2200.0],
hidden = true, text_id = 64180,
greeting = "Nobody ordered us to hold this bridge, $C. That's why it's still standing."
```

## Commander Aldous Wray

Wray (`hub.toml`, guid 9300034, entry 190071, line 1739) stops giving 64002 and becomes the
Vanguard Line's first hint. Give him `npc_flags = 0x3` and `text_id = 64190` so hint lines show
in his gossip menu (he keeps 64004), and a new greeting: "Stormwind still has a barracks, $C, and
I still have warriors to test. The best of mine went east and south, where the court won't send
soldiers. Ask, and I'll tell you where."

## The Advancement Quest, 64002 (rewritten)

Keep the id. `giver = 190110` (was Wray), drop `completable = false`, add `trial = 64414` and
`calling_choice = true`; `advancement = { from = "vanguard", to = "ironclad", trial = 64414 }`,
level 30, `zone_or_sort = -81`, money 0. Title stays "The Ironclad's Oath".

- Objectives: "Find the Ironclads at the Lakeshire bridge in Redridge, and pass their sergeant's trial."
- Details: "Thirty battles' worth of scars, and still standing at the front. The Ironclads hold the line when every other blade has broken.$B$BThey're not in Stormwind. East of Elwynn, where the road crosses Lake Everstill into Lakeshire, someone still keeps the bridge in Ironclad plate. Commander Wray in the Stormwind barracks knows her name. Find her at the bridge."
- Progress: "You found the bridge. Now hold it. Have you done what I asked?"
- Completion: "Then swear it. You'll hold a line nobody ordered you to hold, and you'll hold it after everyone else has run. You start over, Ironclad. Your shield arm doesn't."

Hint (`hint` row on 64002):

| NPC | Option | Reply |
|---|---|---|
| Commander Aldous Wray (190071) | "Where do I find the Ironclads?" | "Lakeshire. They hold the bridge into town because nobody else will. Look for Sergeant Holt at the west end, in plate older than you are." |

## The trial

All four: giver and ender 190110, level 30, `min_tier = 1`, `zone_or_sort = 44`. Hard: the
search was easy.

- **64411 The Highway Holds.** `after_taken = 64002`. Kill 12 Blackrock Outrunners (485) on
  Lakeridge Highway, south of the lake. Money 2500. Objectives: "Kill 12 Blackrock Outrunners on
  Lakeridge Highway." Details: "The orcs run messages up and down the highway south of the lake.
  Every runner that dies is an order that never arrives. Twelve." Progress: "The highway?"
  Completion: "Twelve orders that never arrived. Good."
- **64412 Shields of the Blackrock.** `after = 64411`. Collect 8 Blackrock Shields (980811) from
  Blackrock Grunts (440) and Blackrock Renegades (437) at 40%. Money 2500. Objectives: "Bring 8
  Blackrock Shields from the grunts and renegades on Lakeridge Highway." Details: "You learn a
  shield by breaking it. Bring me eight of theirs, and we'll see whose held." Progress: "Their
  shields?" Completion: "Cracked, every one. Yours isn't. That's the lesson."
- **64413 The Lake Shore.** `after = 64412`. Kill 10 Murloc Flesheaters (422) on Lake
  Everstill's shore. Money 2500. Objectives: "Kill 10 Murloc Flesheaters on the shore of Lake
  Everstill." Details: "The murlocs come up out of the lake at night and take whoever's on the
  bridge. Make the shore a place they don't come back from." Progress: "The shore?" Completion:
  "Quiet water tonight. The Watch will thank you, and won't know why."
- **64414 Hold the Line.** `after = 64413`. Kill 10 Blackrock Renegades (437) on Lakeridge
  Highway. Money 5000. Objectives: "Kill 10 Blackrock Renegades on Lakeridge Highway." Details:
  "Last. The renegades are the ones who left the Blackrock and still raid for themselves. They
  fight dirty and they fight together. Ten, and you come back on your feet." Progress: "Standing?"
  Completion: "Standing. Then come and swear."

Quest item: `{ entry = 980811, name = "Blackrock Shield", quality = 1, item_level = 30, class =
12, display_id = <a shield-looking display with a free icon>, stackable = 20 }`; the collect row
on 64412 is `{ item = 980811, count = 8, chance = 40, from = [440, 437] }`.

## Failing tests first

New file `crates/twilight-realm/tests/tier2_ironclad.rs`, using `tests/calling_choice.rs`'s
setup for a capped character and `tests/common/mod.rs` (`spell`, line 105; `walk_up_to`, line
354):

- A level 30 Vanguard gets the Calling Choice listing 64002 and 64003.
- With 64002 taken, Wray shows the hint line; `MapMarks` has no mark for Sergeant Holt until her
  gossip is opened.
- Holt offers 64411 only once 64002 is taken; the chain runs in order; 64002 cannot be handed in
  before 64414 is; after it, the character is a level 1 Ironclad who knows Righteous Smash,
  Shield Slam, Shield of the Penitent, Retort and Rebuke, and not Iron Wall until level 10.
- Wray still offers 64004 to a level 50 Zealot and no longer offers 64002.
- `server_spells.json` gives the Ironclad ten rows at 5 to 50, three choices each, and its
  Righteous Smash at L = 1 has a larger bonus than the Vanguard's at L = 30.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `build_spells.py --check` is clean; `ground_check.py` flags nothing; `icon_check.py` reports no
  new clash.

Assumptions allowed: every number, `visual` and icon; the Calling icon path; the trainer's look
and exact spot (on the road at the bridge's west end, not on the bridge itself, within 15 yd).
