Title: Sharpshooter: the kit, ten Talent Rows, Wren Ashcombe on the hill above Three Corners, and her trial (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 4, 5, 9, 12 and 14. Blocked by 01-calling-choice-per-tier and
02-trainer-search. It is the first Calling ticket: tickets 11 to 19 follow it one by one, because
all ten edit `master.csv`, both generated JSON files, `callings.csv`, `quests.toml`,
`quest_items.toml` and `hub.toml`, and take `spell_id`s from the top of the sheet.

The Sharpshooter is the Ranger Line's planted branch: it is rewarded for standing still and has
tools to hold its ground. Same role as the Ranger, ranged weapon damage.

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

- `data/twilights-eve/callings.csv`: `sharpshooter,Sharpshooter,Interface\Icons\Ability_Hunter_SniperShot,mana,hunter,warrior,100,100,,,,leather|mail,<the ranger row's weapons>,Sharpshooter,<blurb>`.
  Blurb: "Plants its feet and drops the target before it arrives. Every second standing still
  makes the next shot hit harder."
- `calling` row: `tier_bonus=strength=0;agility=30;stamina=20;intellect=10;spirit=0` (twice the
  Ranger's, a placeholder for ticket 24).

## The kit

| Key | Name | Learn | Row | What it does | Kinds | Visual |
|---|---|---|---|---|---|---|
| `auto_shot` | Auto Shot | 1 | none | copy of the Ranger's, unchanged | weapon_strike | as the Ranger's |
| `puncturing_shaft` | Puncturing Shaft | 1 | 5 | carried: the Ranger's, starts stronger (bonus 215 + 5L, armour ignored 440 + 10L) | weapon_strike | as the Ranger's |
| `steady_shot` | Steady Shot | 1 | 10 | 1.5 s cast, 35 yd, 4% base mana: ranged weapon damage + 68 + 2L, and a stack of Steady Aim | weapon_strike, add_aura_stacks | 3044 |
| `rapid_fire` | Rapid Fire | 1 | 15 | carried: the Ranger's; its aura's haste 44% instead of 40% | grant_aura | as the Ranger's |
| `aimed_shot` | Aimed Shot | 1 | 20 | 3 s cast, 35 yd, 10% base mana, 10 s cooldown: ranged weapon damage + 230 + 6L; halves healing on the target for 10 s | weapon_strike, healing_reduction | 19434 |
| `sunder_arrow` | Sunder Arrow | 10 | 30 | instant, 35 yd, 6% base mana, 6 s cooldown: 80% ranged weapon damage + 20 + 1.5L, and a stack of Sundering (250 less armour a stack, up to 5, 15 s) | weapon_strike, mod_armor | 3043 |
| `silencing_shot` | Silencing Shot | 1 | 35 | carried Kick: the Ranger's, lockout 3.5 s; its bonus resets Pinning Shot's cooldown instead of Vault's (the Sharpshooter has no Vault) | interrupt, reset_cooldowns | as the Ranger's |
| `hold_your_ground` | Hold Your Ground | 20 | 40 | instant, no cost, 45 s cooldown: 12 s of 30% less damage taken; moving ends it | grant_aura (aura: mod_damage_taken_pct, `break_on: move=yes`) | 19263 |
| `pinning_shot` | Pinning Shot | 30 | 45 | utility. Instant, 35 yd, 5% base mana, 30 s cooldown: roots the target 6 s | root (aura row, as the Ranger's `aura_parting_gift`) | 5116 |

Aura rows: `aura_steady_aim` (mod_damage_done_pct 4 a stack, `max_stacks=5`, `stack_rule=stack`,
15 s, `break_on: move=yes`), `aura_sundering` (mod_armor -250, `max_stacks=5`,
`stack_rule=stack`, 15 s, `target=enemy`; apply it as the Ranger's Puncturing Shaft choice
applies `aura_sundered_5`), `aura_hold_your_ground`, `aura_pinning_shot`, and copies of
`aura_rapid_fire` (pct 44).

## The Talent Rows

| Level | Row | Choice A | Choice B | Choice C |
|---|---|---|---|---|
| 5 | Puncturing Shaft | Deep Shaft: armour ignored 50% higher | Quick Shaft: 2 s cast | Sundering Shaft: also adds a Sundering stack |
| 10 | Steady Shot | Steadier Shot: 2 Steady Aim stacks a cast | Quick Draw: 1 s cast | Heavy Draw: 30% more bonus damage |
| 15 | Rapid Fire | Long Volley: lasts 15 s (an aura row copy) | Planted Fire: also 15% more ranged crit while up (aura's second effect `mod_ranged_crit_pct`) | Quick Recovery: 40 s cooldown |
| 20 | Aimed Shot | Crippling Aim: also slows 50% for 6 s (`mod_run_speed`) | Finishing Aim: 50% more under 35% health (`below_health_pct`, `below_bonus_pct`) | Sure Aim: crits deal 30% more (`crit_damage_pct`) |
| 25 | Talent, `row_key=sharpshooter_planted` | Rooted Stance: after 2 s without a step, 15% ranged haste until you move (`proc on=still_for;ms=2000` putting up an aura with `mod_ranged_haste_pct`, `break_on: move=yes`) | Eagle Eye: every shot of the kit 5 yd more range (spellmod `flat RANGE 5`) | Iron Footing: Hold Your Ground's cooldown 15 s shorter (spellmod `flat COOLDOWN -15000`) |
| 30 | Sunder Arrow | Shred: 2 Sundering stacks a shot | Bleeding Arrow: also a 9 s bleed, 8 + 0.8L every 3 s | Long Sunder: Sundering lasts 25 s (an aura row copy) |
| 35 | Silencing Shot | Long Silence: lockout 5 s | Pinning Silence: a stopped cast also roots the target 3 s | Mana Return: a stopped cast gives 10% of max mana (`energize pct_of_max`) |
| 40 | Hold Your Ground | Unbowed: 45% less damage | Second Breath: also heals 10 + 1L a second while up (`periodic_heal` on the aura) | Stone Stance: also 20% ranged haste while up |
| 45 | Pinning Shot | Long Pin: 9 s | Barbed Pin: the target also bleeds 8 s | Double Pin: also roots one more enemy within 8 yd (`max_targets=2`, `area_shape=chain`) |
| 50 | Talent, `row_key=sharpshooter_deadeye` | Killing Calm: Aimed Shot crits 15% more often (`skill_crit_pct`) | Armour Breaker: Puncturing Shaft and Aimed Shot skip 200 more armour (`skill_armor_ignore`) | Unhurried: Steady Shot and Aimed Shot cost 30% less (`skill_cost pct=-30`) |

## The trainer

Wren Ashcombe on the hilltop east of Three Corners, looking down the road into Redridge. Find:
medium.

```
guid = 9300116, entry = 190116, name = "Wren Ashcombe", subname = "Sharpshooter", level = 50,
display_id = 2384 (Watcher Dodds's look, a placeholder), faction_template = 35, role = "giver",
npc_flags = 0x3, map = 0, position = [-9463.0, -1995.0, 0.0], orientation = 4.71, hidden = true,
text_id = 64186,
greeting = "Don't step on the line, $C. I've stood on this spot so long the gnolls think I'm a tree."
```

## The Advancement Quest, 64405

`kind = "advancement"`, giver 190116, level 30, `zone_or_sort = -261`,
`advancement = { from = "ranger", to = "sharpshooter", trial = 64443 }`, `calling_choice = true`,
money 0. Title "The Sharpshooter's Stillness".

- Objectives: "Find the Sharpshooter who watches the road into Redridge, and pass her trial."
- Details: "A bow that never moves watches the road where it climbs into Redridge, from high ground above the three roads. The orcs and gnolls who come that way never reach the post below.$B$BThalyssa Windbough trained beside her once. Ask Thalyssa who she means."
- Progress: "You climbed up here. That's the easy part. Have you done what I asked?"
- Completion: "You held still when it counted. Most never learn that. You start over now, $N, a Sharpshooter, and the first lesson is the one you just passed: the ground you stand on is a weapon."

Hints (`hint` rows on 64405):

| NPC | Option | Reply |
|---|---|---|
| Thalyssa Windbough (190053) | "Who shoots without moving?" | "Wren Ashcombe. She went east to Redridge and stopped writing. Guard Parker at Three Corners will know where she stands. His gnolls keep dying before they reach him." |
| Guard Parker (464) | "Who's been shooting the gnolls?" | "Somebody on the hill east of my post, above the road. One arrow each. Never two. I stopped asking who. You go up and ask." |

## The trial

All three: giver and ender 190116, level 30, `min_tier = 1`, `zone_or_sort = 44`.

- **64441 Hold the Hill.** `after_taken = 64405`. Kill 10 Redridge Thrashers (712) at Three
  Corners. Money 2500. Objectives: "Kill 10 Redridge Thrashers around Three Corners without
  leaving the hill more than you must." Details: "Don't chase them. Let them come up to you. Ten
  thrashers, and every one dies before it reaches the top." Progress: "Still standing?"
  Completion: "Good. You only moved when you had to."
- **64442 Spiders in the Grass.** `after = 64441`. Kill 8 Tarantulas (442) at Three Corners.
  Money 2500. Objectives: "Kill 8 Tarantulas in the grass around Three Corners." Details: "A
  shooter who can't see what's in the grass is a dead shooter. The tarantulas hide better than
  the gnolls. Find eight and kill them before they get close." Progress: "Found them?"
  Completion: "Eight. Your eyes are better than your feet, which is how it should be."
- **64443 The Far Shore.** `after = 64442`. Kill 6 Murloc Shorestrikers (1083) on Lake
  Everstill's shore. Money 5000. Objectives: "Kill 6 Murloc Shorestrikers on the shore of Lake
  Everstill." Details: "Last one. The shorestrikers on Lake Everstill duck into the water when
  they're hurt. Kill six before they reach it. Stand on the bank, and don't get your feet wet."
  Progress: "Dry boots?" Completion: "Six, and dry boots. Come back up the hill. I've something
  to tell you."

## Failing tests first

New file `crates/twilight-realm/tests/tier2_sharpshooter.rs`, using `tests/calling_choice.rs`'s
setup for a capped character and `tests/common/mod.rs` (`spell`, line 105; `walk_up_to`, line
354):

- A level 30 Ranger gets the Calling Choice listing 64405 among its options.
- With 64405 taken, `MapMarks` has no mark for Wren Ashcombe until the character opens her
  gossip; Thalyssa and Guard Parker show their hint lines and replies.
- Wren offers 64441 only once 64405 is taken; 64442 and 64443 follow in order; 64405 cannot be
  handed in before 64443 is; after it, the hand-in makes the character a level 1 Sharpshooter
  who knows Auto Shot, Puncturing Shaft, Steady Shot, Rapid Fire, Aimed Shot and Silencing Shot,
  and not Sunder Arrow until level 10.
- `server_spells.json` gives the Sharpshooter ten rows at 5 to 50, three choices each, and its
  Puncturing Shaft at L = 1 deals more bonus than the Ranger's at L = 30 (read both rows'
  formulas from the JSON).

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `build_spells.py --check` is clean; `ground_check.py` flags nothing; `icon_check.py` reports no
  new clash.

Assumptions allowed: every number, `visual` and icon; the Calling icon path; the trainer's look
and exact spot on the hilltop (stay on high, open ground within 15 yd).
