Title: Pyromancer: the kit, ten Talent Rows, Magus Ilyra Vance in the Mage Quarter, and her trial (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 4, 5, 8, 12 and 14. Blocked by 05-aura-copies and 14-nightblade (the
Calling tickets run one after another: they share `master.csv`, both generated JSON files,
`callings.csv`, `quests.toml`, `quest_items.toml` and `hub.toml`).

The Pyromancer sets one fire and lets it spread through the pack: its Embers stack higher, last
longer and jump to nearby foes. Same role as the Initiate, ranged spell damage.

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

- `data/twilights-eve/callings.csv`: `pyromancer,Pyromancer,Interface\Icons\Spell_Fire_FlameBolt,mana,mage,warrior,100,100,,,,cloth,<the initiate row's weapons>,Pyromancer,<blurb>`.
  Blurb: "Sets one fire and lets it spread. Embers stack, last and jump from foe to foe until the
  pack burns down."
- `calling` row: `tier_bonus=strength=0;agility=0;stamina=10;intellect=30;spirit=20`.

## The kit

Ember below means this Calling's `aura_searing_ember` (a copy of the Initiate's: fire damage over
time, 3 stacks, `stack_rule=stack`, 12 s; tick 12 + 0.3L by the amount rule). Every skill that
grows or copies Embers names that one row, so no choice may swap it for another aura row.

| Key | Name | Learn | Row | What it does | Kinds | Visual |
|---|---|---|---|---|---|---|
| `searing_ember` | Searing Ember | 1 | 5 | carried: the Initiate's, starts stronger (hit 73 + 1.5L, Ember tick 12 + 0.3L) | school_damage, periodic_damage | as the Initiate's |
| `fireball` | Fireball | 1 | 10 | filler. 2.5 s cast, 35 yd, 9% base mana: 90 + 4L fire, and the Ember on the target grows a stack | school_damage, grow_aura | 133 |
| `catalyst` | Catalyst | 1 | 15 | carried, the Ember half only (the Pyromancer has no Biting Cold): instant, 12 s cooldown, takes the Ember off the target for 30 + 2.5L fire a stack, and gives 11% of max mana | consume_aura (`per_stack=yes`, `energize_pct_of_max=11`) | as the Initiate's |
| `pyroblast` | Pyroblast | 1 | 20 | 4 s cast, 35 yd, 15% base mana, 10 s cooldown: 200 + 8L fire and a 12 s burn of 15 + 1L every 3 s | school_damage, periodic_damage | 11366 |
| `kindle` | Kindle | 10 | 30 | instant, 6% base mana, 10 s cooldown: the Ember on the target, with its stacks and time left, is copied to the two nearest foes within 10 yd | copy_aura (`targets=2;radius_yd=10`, ticket 05) | 11129 |
| `counterspell` | Counterspell | 1 | 35 | carried Kick: the Initiate's, lockout 6.5 s, a stopped cast gives 10% of max mana | interrupt, energize | as the Initiate's |
| `flamestrike` | Flamestrike | 20 | 40 | 2 s cast at a ground point within 30 yd, 14% base mana, 12 s cooldown: 70 + 3L fire to every enemy within 8 yd, each also taking an Ember | school_damage, periodic_damage (`target=dest`, `area_center=ground`, `area_shape=circle`, `realm_radius_yd=8`, `client_radius_index` for 8 yd) | 2120 |
| `blast_wave` | Blast Wave | 30 | 45 | utility. Instant, 10% base mana, 25 s cooldown: 45 + 2L fire to every enemy within 8 yd, knocks them back and slows them 50% for 6 s | school_damage, knockback, mod_run_speed (`target=self_area`) | 11113 |

Aura rows: `aura_searing_ember` (copy), `aura_pyroblast` (fire, 12 s), `aura_blast_wave` (slow).

## The Talent Rows

| Level | Row | Choice A | Choice B | Choice C |
|---|---|---|---|---|
| 5 | Searing Ember | Hot Ember: 40% more on the hit | Cheap Ember: 30% less mana | Twin Ember: also hits one more enemy within 8 yd (`max_targets=2`, `area_shape=chain`) |
| 10 | Fireball | Hot Fireball: 2 s cast | Stoking Fireball: the Ember grows 2 stacks | Fireball Splash: 30% of the hit to enemies within 5 yd (`splash_pct`, `splash_yd`) |
| 15 | Catalyst | Bright Catalyst: 50% more a stack | Refunding Catalyst: 20% of max mana | Spreading Catalyst: a fresh Ember goes on the nearest enemy within 8 yd (`spread_yd=8`) |
| 20 | Pyroblast | Quick Pyroblast: 3 s cast | Searing Pyroblast: the burn 50% stronger (an aura row copy) | Ember Blast: the Ember also grows 3 stacks |
| 25 | Talent, `row_key=pyromancer_firestarter` | Hot Streak: a Fireball crit makes the next Pyroblast instant (`proc on=crit;skill=fireball` putting up a `next_cast` aura) | Fire Power: Fireball and Pyroblast deal 10% more (`skill_amount_pct`, two effects) | Ember Ward: a crit taken puts an Ember on the attacker (`proc on=crit_taken;apply_aura_target=aura_searing_ember`) |
| 30 | Kindle | Wildfire: three foes | Far Kindle: 15 yd | Quick Kindle: 6 s cooldown |
| 35 | Counterspell | Long Counter: lockout 8.5 s | Burning Counter: a stopped cast puts an Ember on the target (`periodic_damage if_worn=grow`, as Hush) | Mana Counter: 20% of max mana |
| 40 | Flamestrike | Wide Flamestrike: 12 yd | Quick Flamestrike: 1 s cast | Burning Flamestrike: 50% more damage |
| 45 | Blast Wave | Big Wave: 12 yd | Dazing Wave: 70% slow | Igniting Wave: each enemy hit also takes an Ember |
| 50 | Talent, `row_key=pyromancer_inferno` | Pyromaniac: Fireball and Pyroblast crit 5% more often | Searing Mastery: Embers tick 20% harder (`skill_amount_pct skill=searing_ember;applies_to=tick`) | Fuel: killing a burning foe gives 5% of base mana (`proc on=kill;requires_target_aura=aura_searing_ember;energize_pct_of_base=5`) |

## The trainer

Magus Ilyra Vance keeps a fire in the courtyard of Stormwind's Mage Quarter that has burned since
the Second War. Find: easy.

```
guid = 9300114, entry = 190114, name = "Magus Ilyra Vance", subname = "Pyromancer", level = 50,
display_id = 3293 (Elsharin's look, a placeholder), faction_template = 35, role = "giver",
npc_flags = 0x3, map = 0, position = [-8972.0, 862.0, <the street floor's Z>], orientation = 3.14,
hidden = true, text_id = 64184,
greeting = "Warm your hands, $C. Mind the sleeves. It's been burning longer than you've been alive."
```

The Mage Quarter's streets are a WMO: run `python3.15 tools/twilights-eve/wmo_floor.py -8972 862`
and write the outdoor street floor's Z (the stock Squirrel at -8976, 866 stands on it at about
106), then let `ground_check.py` check it. Pick another open courtyard spot within 15 yd if that
one is inside a building.

## The Advancement Quest, 64403

`kind = "advancement"`, giver 190114, level 30, `zone_or_sort = -161`,
`advancement = { from = "initiate", to = "pyromancer", trial = 64434 }`,
`calling_choice = true`, money 0. Title "The Pyromancer's Flame".

- Objectives: "Find the Pyromancer in Stormwind's Mage Quarter, and pass her trial."
- Details: "You burn well, $N, but you burn one thing at a time. A Pyromancer lights one fire and lets it find the rest.$B$BIn Stormwind's Mage Quarter, among the towers, a magus keeps a fire that has never gone out. Elsa Brightwood knows her. Ask Elsa."
- Progress: "You found the fire. Have you fed it?"
- Completion: "Then it's yours now. Let it spread, and don't stand in it. You start over, Pyromancer. The fire doesn't."

Hint (`hint` row on 64403):

| NPC | Option | Reply |
|---|---|---|
| Elsa Brightwood (190052) | "Who keeps the oldest fire in Stormwind?" | "Magus Ilyra Vance, in the Mage Quarter's courtyard. She lit it the day the Second War ended and hasn't let it go out since. You'll smell it before you see it." |

## The trial

All four: giver and ender 190114, level 30, `min_tier = 1`, `zone_or_sort = 44`. Hard: the
search was easy. The Redridge Canyons are banded 36 to 39 (the arc's ticket 11).

- **64431 Kindling.** `after_taken = 64403`. Kill 8 Redridge Brutes (426) and 6 Redridge Mystics
  (430) in the Redridge Canyons. Money 2500. Objectives: "Kill 8 Redridge Brutes and 6 Redridge
  Mystics in the Redridge Canyons." Details: "Gnolls burn badly and run worse. Go to the canyons
  north of Lakeshire and set the pack alight. Fourteen, and let it spread." Progress: "Smell of
  singed fur?" Completion: "Strong. Good."
- **64432 Gnoll Tallow.** `after = 64431`. Collect 8 Gnoll Tallow (980851) from Redridge Brutes
  (426) and Redridge Mystics (430) at 50%. Money 2500. Objectives: "Bring 8 Gnoll Tallow from the
  gnolls of the Redridge Canyons." Details: "The fire wants feeding. Gnoll fat burns slow and
  hot. Eight lumps." Progress: "The tallow?" Completion: "That will keep it a month. Ugh."
- **64433 The Highway.** `after = 64432`. Kill 10 Blackrock Grunts (440) on Lakeridge Highway.
  Money 2500. Objectives: "Kill 10 Blackrock Grunts on Lakeridge Highway." Details: "Orcs don't
  run from fire. They charge through it. Ten grunts, and learn to step back while they burn."
  Progress: "Still unsinged?" Completion: "Mostly. Good."
- **64434 The Alphas.** `after = 64433`. Kill 6 Redridge Alphas (445) in the Redridge Canyons.
  Money 5000. Objectives: "Kill 6 Redridge Alphas in the Redridge Canyons." Details: "Last. The
  alphas lead the packs and take the fire for them. Six, and come back with your eyebrows."
  Progress: "Eyebrows?" Completion: "Both. Come and sit by the fire, Pyromancer."

Quest item: `{ entry = 980851, name = "Gnoll Tallow", quality = 1, item_level = 30, class = 12,
display_id = <a lump display with a free icon>, stackable = 20 }`; the collect row on 64432 is
`{ item = 980851, count = 8, chance = 50, from = [426, 430] }`.

## Failing tests first

New file `crates/twilight-realm/tests/tier2_pyromancer.rs` (setup as `tests/calling_choice.rs`;
Embers as `tests/initiate_spells.rs` sets them up):

- A level 30 Initiate gets the Calling Choice listing 64403.
- With 64403 taken, Elsa Brightwood shows the hint line; Magus Vance makes no map mark until met.
- The trial chain runs in order and gates 64403's hand-in; after it, the character is a level 1
  Pyromancer who knows Searing Ember, Fireball, Catalyst, Pyroblast and Counterspell, and not
  Kindle until level 10.
- Fireball grows the Ember on its target; Kindle at level 10 copies a 3-stack Ember to two mobs
  within 10 yd.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `build_spells.py --check` is clean; `ground_check.py` flags nothing; `icon_check.py` reports no
  new clash.

Assumptions allowed: every number, `visual` and icon; the Calling icon path; the trainer's exact
spot in the courtyard.
