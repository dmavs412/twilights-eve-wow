Title: Nightblade: the kit, ten Talent Rows, Silas Dunmere in the Riot's last room, and his trial (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 4, 5, 7, 12 and 14. Blocked by 04-blade-kinds and 13-bloodletter (the
Calling tickets run one after another: they share `master.csv`, both generated JSON files,
`callings.csv`, `quests.toml`, `quest_items.toml` and the content files).

The Nightblade strikes from the dark, slips back into it in the middle of a fight, and strikes
again with a crit, while the Bloodletter never stops. Same role as the Cutthroat, melee damage.

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
- Content: quests in `content/quests.toml` after 64013 under
  `# ── Tier 2 Advancement Quests and trials (#PARENT) ──`, in the form of the rows above them.
  This Calling's trainer is not a Hub NPC (below). Pick quest item displays whose icon no other
  item uses; `icon_check.py` must report no new clash.
- `docs/art-assets.md`: list the trainer's borrowed look and every placeholder icon, under a
  "Tier 2 Callings" heading (make it if missing).

## The Calling

- `data/twilights-eve/callings.csv`: `nightblade,Nightblade,Interface\Icons\Ability_Stealth,energy,rogue,warrior,100,100,25,,,cloth|leather,<the cutthroat row's weapons>,Nightblade,<blurb>`.
  Blurb: "Strikes from the dark, slips back into it, and strikes again before the body falls."
- `calling` row: `tier_bonus=strength=12;agility=24;stamina=24;intellect=0;spirit=6`.

## The kit

| Key | Name | Learn | Row | What it does | Kinds | Visual |
|---|---|---|---|---|---|---|
| `eviscerate` | Eviscerate | 1 | 5 | carried: the Cutthroat's finisher, starts stronger (49 + 1.3L, per point 50 + 1.3L) | school_damage | as the Cutthroat's |
| `backstab` | Backstab | 1 | 10 | builder. Melee, 40 energy, from behind (`requires: position=behind`): 150% weapon damage + 20 + 2L, 1 combo point | weapon_strike, add_combo_points | 53 |
| `stealth` | Stealth | 1 | 15 | carried: the Cutthroat's, its aura copied (`opener_crit=yes`); cooldown 9 s instead of 10 | stealth | as the Cutthroat's |
| `ambush` | Ambush | 1 | 20 | melee, 60 energy, from Stealth and from behind (`requires: stealth=yes;position=behind`, `client_stances` the stealth form): 250% weapon damage + 40 + 3L, 2 combo points | weapon_strike, add_combo_points | 8676 |
| `vanish` | Vanish | 10 | 30 | no cost, 2 min cooldown: every foe forgets you, you leave combat and go into Stealth | drop_threat (ticket 04), grant_aura (`aura_stealth`) | 1856 |
| `kick` | Kick | 1 | 35 | carried Kick: the Cutthroat's, lockout 5.5 s, a stopped cast gives a combo point | interrupt, add_combo_points | as the Cutthroat's |
| `cold_blood` | Cold Blood | 20 | 40 | no cost, 1 min cooldown: your next skill hit within 20 s is a sure crit | grant_aura (aura: `skill_crit_pct skill=any;pct=100`, one charge, ticket 04) | 14177 |
| `kidney_shot` | Kidney Shot | 30 | 45 | utility. Finisher, melee, 25 energy, 20 s cooldown: stuns 1 s per combo point | stun (`per_combo` duration, an aura row) | 408 |

Aura rows: a copy of `aura_stealth`; `aura_cold_blood`; `aura_kidney_shot`.

## The Talent Rows

| Level | Row | Choice A | Choice B | Choice C |
|---|---|---|---|---|
| 5 | Eviscerate | Gutting: 25% more per combo point | Relentless: a killing blow gives back 20 energy (`kill_energize=20`) | Fatal Edge: 50% more under 35% health |
| 10 | Backstab | Deep Backstab: 180% weapon damage | Cruel Backstab: crits deal 30% more (`crit_damage_pct`) | Swift Backstab: 30 energy |
| 15 | Stealth | Quick Shadows: 4 s cooldown | Shadow Speed: 30% faster run in Stealth (the aura's second effect `mod_run_speed`) | Stalker: your first Backstab out of Stealth costs nothing (aura `skill_cost`, one charge) |
| 20 | Ambush | Brutal Ambush: 300% weapon damage | Ambush and Run: 3 combo points | Lingering Ambush: the target also bleeds 9 s |
| 25 | Talent, `row_key=nightblade_shadowcraft` | Shadow Dance: Vanish's cooldown 40 s shorter (spellmod `COOLDOWN`) | Assassin's Mark: Ambush and Backstab crit 10% more often (`skill_crit_pct`, two effects) | Quick Recovery: each finisher gives back 15 energy (`proc on=finisher;energize=15`) |
| 30 | Vanish | Fleeting Vanish: also 40% faster run for 4 s | Ready Vanish: also resets Kidney Shot and Cold Blood (`reset_cooldowns`) | Shadow Heal: also heals you 60 + 6L (`heal`) |
| 35 | Kick | Long Kick: lockout 7.5 s | Kick from the Dark: usable in Stealth without breaking it (`on_cast: breaks_stealth=no`) | Double Kick: 2 combo points |
| 40 | Cold Blood | Second Wind: also gives 30 energy | Quick Blood: 40 s cooldown | Two Charges: the next two skill hits |
| 45 | Kidney Shot | Long Kidney: 1.5 s per combo point | Bruising Kidney: also 10 + 1L damage per combo point | Quick Kidney: 15 s cooldown |
| 50 | Talent, `row_key=nightblade_deathmark` | Executioner: Eviscerate crits 10% more often | Ruthless: a finisher leaves a combo point 20% of the time (`proc on=finisher;chance=20;add_combo_points=1`) | Cheat Death: healing on you 40% stronger under 30% health |

## The trainer

Silas Dunmere sits in the Riot's last room, the round room at the end of the Stockade's east wing
(map 34, `content/stockade_riot.toml`, levels 30 to 34), in the cell nobody ever locked. He is a
friendly creature of the Riot, a Field Giver in the form of Bertram Ash (template 97205 in
`content/mobs.toml` line 1922, spawn guid 973024 in `stockade_riot.toml` line 330), not a Hub
NPC, so the Phased Dungeon's own rules hide him from the Living Map. Find: hard.

- `content/mobs.toml`, after the Riot's templates: `entry = 97290`, `name = "Silas Dunmere"`,
  `subname = "Nightblade"` if templates take one (else leave it), `creature_type = 7`,
  `display_id = 1507` (Osborne the Night Man's look, a placeholder), `faction_template = 35`,
  `hostile = false`, `friendly = true`, level 50, the rest as Bertram Ash's. 97206 to 97288 are
  taken or claimed by other specs; 97290 is free.
- `content/stockade_riot.toml`, after Captain Rook's room: `guid = 973028`, `entry = 97290`,
  `level = 50`, `position = [84.0, -146.0, -33.9396]`, `grounded = false` (Z: Bazil Thredd 79124,
  as every row of the room), facing the room's mouth. Keep him at least 8 yd from Rook (91.0,
  -140.0) and the Bought Watchmen; move him within the round room if that spot is taken, and
  record it.

## The Advancement Quest, 64402

`kind = "advancement"`, giver 97290, level 30, `zone_or_sort = -162`,
`advancement = { from = "cutthroat", to = "nightblade", trial = 64427 }`,
`calling_choice = true`, money 0. Title "The Unlocked Cell".

- Objectives: "Find the Nightblade in the cell that was never locked, and pass his trial."
- Details: "There's a kind of Cutthroat who doesn't run when the fight starts. He leaves, and then he's back, behind you.$B$BWhere the city keeps the men it forgot, below the street and past the riot, one cell was never locked. Ask the one who taught you to hide. He knows who pours the drinks for the Watch."
- Progress: "You got in. Most don't. Did you get out again?"
- Completion: "Then you know the trick: be gone, then be back. You start over, Nightblade. The dark stays."

Hints (`hint` rows on 64402):

| NPC | Option | Reply |
|---|---|---|
| Jorik Vane (190051) | "Who teaches what comes after the knife?" | "Not me. Ask Farley at the Lion's Pride in Goldshire who pours for the Watch on their nights off." |
| Innkeeper Farley (295) | "Who drinks with the Stockade's Watch?" | "The warden himself, Thelwater, when he can get away. He says one cell in the riot wing never locked. He says it like it's a joke." |
| Warden Thelwater (1719, map 34) | "Which cell never locked?" | "The last one. Past the riot, past Rook, in the round room at the end of the east wing. Whoever sits in it never asked to be let out, and nobody I send in comes back with a description." |

Farley and Thelwater are gossiping Hub NPCs already (`hub.toml`, `text_id` set).

## The trial

Both: giver and ender 97290, level 30, `min_tier = 1`, `zone_or_sort = 717` (the Stockade). Easy:
the search was hard.

- **64426 Out the Way You Came.** `after_taken = 64402`. Kill 6 Defias Rioters (97200). Money
  2500. Objectives: "Kill 6 Defias Rioters in the Riot." Details: "You crept in. Now go back up
  the hall and make six of them never know you were there." Progress: "Six?" Completion: "Six,
  and nobody screamed. Good."
- **64427 The Ringleaders.** `after = 64426`. Kill 3 Defias Ringleaders (97201). Money 5000.
  Objectives: "Kill 3 Defias Ringleaders in the Riot." Details: "Three ringleaders, one at a
  time. Strike, step back into the dark, strike again. If they see you twice, you did it wrong."
  Progress: "Did they see you?" Completion: "Once each. Close enough. Sit."

## Failing tests first

New file `crates/twilight-realm/tests/tier2_nightblade.rs`, with a party in the Riot as
`tests/stockade.rs` makes one (`questing_party`, line 1103) and the capped-character setup of
`tests/calling_choice.rs`:

- A level 30 Cutthroat gets the Calling Choice listing 64401 and 64402.
- With 64402 taken, Jorik Vane, Innkeeper Farley and Warden Thelwater show their hint lines.
- In a Run of the Riot, Silas Dunmere is a friendly creature in the round room who offers 64426
  once 64402 is taken; the chain gates 64402's hand-in; after it, the character is a level 1
  Nightblade who knows Eviscerate, Backstab, Stealth, Ambush and Kick, and not Vanish until 10.
- Ambush is refused out of Stealth and from the front; Vanish in combat leaves the Nightblade in
  Stealth with no mob attacking.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `build_spells.py --check` is clean; `ground_check.py` flags nothing; `icon_check.py` reports no
  new clash.

Assumptions allowed: every number, `visual` and icon; the Calling icon path; the trainer's spot
within the round room.
