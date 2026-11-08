Title: Bloodmender, a main healer: the kit, ten Talent Rows, Sister Elowen Marr in Cathedral Square, and her trial (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 4, 5, 10, 12 and 14. Blocked by 08-resurrect-and-blood-heals and
18-blightcaller (the Calling tickets run one after another: they share `master.csv`, both
generated JSON files, `callings.csv`, `quests.toml`, `quest_items.toml` and `hub.toml`).

The Bloodmender is one of the game's two main healers. Health is its resource: it spends its own
blood for bigger heals and shields on its friends, then drinks the foe's to fill up again. Same
Line as the Acolyte, cloth.

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

- `data/twilights-eve/callings.csv`: `bloodmender,Bloodmender,Interface\Icons\Spell_Shadow_LifeDrain,mana,priest,warrior,100,100,,,,cloth,<the acolyte row's weapons>,Bloodmender,<blurb>`.
  Copy the Acolyte row's `health_pct` and `mana_pct` if they differ from 100.
  Blurb: "A healer who pays in blood. Spends its own health to mend and shield its friends, then
  drinks the foe's to fill up again."
- `calling` row: `tier_bonus=strength=0;agility=0;stamina=24;intellect=16;spirit=30`.

## The kit

| Key | Name | Learn | Row | What it does | Kinds | Visual |
|---|---|---|---|---|---|---|
| `blood_lance` | Blood Lance | 1 | 5 | carried: the Acolyte's (3% health, 1.5 s cast), starts stronger (106 + 2L shadow; the most hurt friend within 15 yd heals 20% of the hit over 6 s). Its `grow_aura` effect names the Blight, which the Bloodmender lacks: drop that slot | school_damage, periodic_heal | as the Acolyte's |
| `blood_feast` | Blood Feast | 1 | 10 | filler. Channel 3 s, a tick a second, 30 yd, 6% base mana: drains 25 + 1.2L shadow a tick and heals you twice what it drains | drain (`heal_pct=200`), `cast_kind=channel` | 689 |
| `dark_mending` | Dark Mending | 1 | 15 | carried: the Acolyte's heal on a friend paid in health, starts stronger (418 + 10L), with its `aura_dark_mending` copied | heal, periodic_damage | as the Acolyte's |
| `transfusion` | Transfusion | 1 | 20 | instant, 40 yd, a friend, costs 15% of your max health: the friend heals 150% of what it cost you over 6 s | periodic_heal (`apply_aura`; aura row `pct_of=health_cost;pct=150`, ticket 08), `cost_kind=health` | 755 |
| `blood_ward` | Blood Ward | 10 | 30 | instant, costs 12% of your max health, 20 s cooldown: every friend within 15 yd of you gets a shield of 60% of what it cost, for 10 s | absorb (`pct=60;pct_of=health_cost;radius_yd=15;duration_ms=10000`), `cost_kind=health` | 17 |
| `hush` | Hush | 1 | 35 | carried Kick: the Acolyte's, lockout 4.5 s. Its bonus grows the Blight, which the Bloodmender lacks: a stopped cast instead drains 20 + 1L from the target and heals you all of it | interrupt, drain (`heal_pct=100`) | as the Acolyte's |
| `circle_of_blood` | Circle of Blood | 20 | 40 | instant at a ground point within 30 yd, costs 10% of your max health, 30 s cooldown: for 10 s an 8 yd area heals friends inside 25 + 2.5L every 2 s | ground_area (`ally_aura`), `target=dest`, `client_radius_index` for 8 yd, `cost_kind=health` | 2060 |
| `last_rites` | Last Rites | 30 | 45 | utility. 10 s cast, 30 yd, costs 20% of your max health, out of combat: a dead friend returns with twice the health it cost you | resurrect (`pct_of=health_cost;pct=200`, ticket 08), `target=friend`, `requires=combat=out`, `cost_kind=health` | 2006 |

Aura rows: a copy of `aura_dark_mending`; `aura_transfusion` (periodic_heal, 6 s, a tick a
second, `pct_of=health_cost;pct=150`); `aura_circle_of_blood` (periodic_heal for friends inside).

## The Talent Rows

| Level | Row | Choice A | Choice B | Choice C |
|---|---|---|---|---|
| 5 | Blood Lance | Deep Lance: 25% more damage | Mending Lance: the friend heals 40% of the hit | Quick Lance: 1 s cast |
| 10 | Blood Feast | Gorging Feast: heals three times what it drains | Sated Feast: healing past your max grows a shield of up to 15% of max health for 8 s (`overheal_absorb_max_pct=15;overheal_absorb_ms=8000`) | Long Feast: 4 s, four ticks |
| 15 | Dark Mending | Deep Mending: heals 30% more | Quick Mending: 5 s cooldown | Gentle Mending: costs 10% health |
| 20 | Transfusion | Rich Transfusion: 200% of the cost | Quick Transfusion: over 4 s (an aura row copy) | Cheap Transfusion: costs 10% health |
| 25 | Talent, `row_key=bloodmender_blood_pact` | Masochism: health you spend heals you 30% of it over 5 s (as the Acolyte's `aura_masochism_5`: `proc` with `cost_kind=health`) | Thick Blood: 10% more max health (`max_health_pct`) | Sanguine Flow: a health-cost cast makes your next Blood Lance instant (as the Acolyte's `aura_sanguine_flow`) |
| 30 | Blood Ward | Thick Ward: 90% of the cost | Wide Ward: 25 yd | Long Ward: 15 s |
| 35 | Hush | Long Hush: lockout 6.5 s | Deep Drink: the stopped cast's drain is doubled | Mana Hush: a stopped cast also gives 10% of max mana |
| 40 | Circle of Blood | Deep Circle: heals 50% more (an ally aura copy) | Wide Circle: 12 yd | Long Circle: 14 s |
| 45 | Last Rites | Full Rites: three times the cost | Quick Rites: 5 s cast | Cheap Rites: costs 10% health |
| 50 | Talent, `row_key=bloodmender_hearts_blood` | Crimson Gift: Dark Mending and Transfusion heal 15% more (`skill_amount_pct`, two effects; if that kind does not raise heals, write the row `modelled=no` with a descriptive kind and record the gap) | Iron Veins: healing on you 30% stronger under 30% health | Bloodsurge: a health-cost cast gives 2% of base mana (`proc` with `cost_kind=health;energize_pct_of_base=2`) |

## The trainer

Sister Elowen Marr tends the orphans of Stormwind on the steps of Cathedral Square, outside the
doors of a cathedral that no longer opens (the Cathedral of Light is a Phased Dungeon for 48 to
50). Find: medium.

```
guid = 9300119, entry = 190119, name = "Sister Elowen Marr", subname = "Bloodmender", level = 50,
display_id = 7008 (Brother Sarno's look, a placeholder), faction_template = 35, role = "giver",
npc_flags = 0x3, map = 0, position = [-8630.0, 760.0, <the square's floor Z>], orientation = 0.8,
hidden = true, text_id = 64189,
greeting = "Sit with me, $C. The children are asleep. A little blood buys a lot of quiet."
```

Cathedral Square is part of the city's WMO: run `python3.15 tools/twilights-eve/wmo_floor.py
-8630 760` and write the square's floor (the stock Brother Kristoff, 1444, stands on it at
-8623.2, 766.7, z 97.4), then let `ground_check.py` check it. Stay outside
`cathedral_of_light.toml`'s footprint and 5 yd from Kristoff.

## The Advancement Quest, 64408

`kind = "advancement"`, giver 190119, level 30, `zone_or_sort = -262`,
`advancement = { from = "acolyte", to = "bloodmender", trial = 64458 }`,
`calling_choice = true`, money 0. Title "The Bloodmender's Gift".

- Objectives: "Find the Bloodmender who tends Stormwind's orphans, and pass her trial."
- Details: "You have spent your blood on yourself, $N. A Bloodmender spends it on everyone else, and drinks it back from those who deserve to lose it.$B$BA sister who heals with her own blood tends the orphans of Stormwind, outside the doors of a cathedral that no longer opens. Brother Corwin Ashgrave knows the brother who brings her bread. Ask him."
- Progress: "You found me. Did you bleed for them?"
- Completion: "Then you know the price, and you paid it gladly. You start over, Bloodmender. Your veins don't."

Hints (`hint` rows on 64408):

| NPC | Option | Reply |
|---|---|---|
| Brother Corwin Ashgrave (190054) | "Who heals with her own blood?" | "Sister Elowen Marr. Brother Kristoff brings her bread in Cathedral Square. He'll know where she sits today." |
| Brother Kristoff (1444) | "Where is Sister Marr?" | "On the steps by the orphans, where she always is. She bleeds for anyone who asks. Be kind to her." |

## The trial

All three: giver and ender 190119, level 30, `min_tier = 1`, `zone_or_sort = 10`. Medium.

- **64456 The Wounded Watch.** `after_taken = 64408`. Three `talk` objectives in Darkshire:
  Watcher Ladimore (576), option "Let me see that wound.", reply "It's nothing. It's been nothing
  for a week. ...Thank you."; Clerk Daltry (267), option "You're bleeding through your sleeve.",
  reply "Am I? I hadn't looked up from the ledger in days. Thank you."; Elaine Carevin (633),
  option "Hold still. This will sting.", reply "It does. It's better. Who sent you?" Money 2500.
  Objectives: "Tend the wounded of Darkshire: Watcher Ladimore, Clerk Daltry and Elaine Carevin."
  Details: "Go to Darkshire and mend three of them. Pay for it yourself. Don't tell them what it
  cost." Progress: "Did they thank you?" Completion: "They always do. They never know."
- **64457 What Bit Them.** `after = 64456`. Kill 8 Rabid Dire Wolves (565) on the Darkened Bank.
  Money 2500. Objectives: "Kill 8 Rabid Dire Wolves on the Darkened Bank." Details: "The bites
  came from the wolves along the river. Eight, and drink what they owe you." Progress: "Full
  again?" Completion: "Good. You took back what you gave."
- **64458 The Cursing Dead.** `after = 64457`. Kill 6 Skeletal Mages (203) at Tranquil Gardens
  Cemetery. Money 5000. Objectives: "Kill 6 Skeletal Mages at Tranquil Gardens Cemetery." Details:
  "Last. The skeleton mages curse the Watch from the gardens. Six. Spend, drink, spend again."
  Progress: "Still standing?" Completion: "Standing, and not a drop wasted. Kneel."

## Failing tests first

New file `crates/twilight-realm/tests/tier2_bloodmender.rs` (setup as `tests/calling_choice.rs`;
health costs as `tests/acolyte_blood.rs`; a second player as `tests/common/party.rs`):

- A level 30 Acolyte gets the Calling Choice listing 64407 and 64408.
- With 64408 taken, Corwin and Kristoff show their hint lines; Sister Marr makes no map mark
  until met.
- 64456 completes after the three talk lines; the chain gates 64408's hand-in; after it, the
  character is a level 1 Bloodmender who knows Blood Lance, Blood Feast, Dark Mending,
  Transfusion and Hush, and not Blood Ward until level 10.
- Healing with blood: Transfusion on a hurt second player costs the Bloodmender 15% of max health
  and heals the player 150% of that over 6 s; Blood Feast refills the Bloodmender.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `build_spells.py --check` is clean; `ground_check.py` flags nothing; `icon_check.py` reports no
  new clash.

Assumptions allowed: every number, `visual` and icon; the Calling icon path; the trainer's exact
spot on the square.
