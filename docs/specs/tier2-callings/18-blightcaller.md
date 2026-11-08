Title: Blightcaller, a main healer: the kit, ten Talent Rows, Mother Ysolde Grey at Beggar's Haunt, and her trial (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 4, 5, 10, 12 and 14. Blocked by 05-aura-copies,
08-resurrect-and-blood-heals and 17-trapper (the Calling tickets run one after another: they
share `master.csv`, both generated JSON files, `callings.csv`, `quests.toml`, `quest_items.toml`
and `hub.toml`).

The Blightcaller is one of the game's two main healers. It heals through the Blight it spreads:
the rot grows on the pack, jumps from foe to foe, and every tick, channel and harvest of it mends
the group. Same Line as the Acolyte, cloth, on mana and health.

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

- `data/twilights-eve/callings.csv`: `blightcaller,Blightcaller,Interface\Icons\Spell_Shadow_AbominationExplosion,mana,priest,warrior,100,100,,,,cloth,<the acolyte row's weapons>,Blightcaller,<blurb>`.
  Copy the Acolyte row's `health_pct` and `mana_pct` if they differ from 100.
  Blurb: "A healer who works through rot. The Blight grows on the pack, jumps from foe to foe,
  and every tick of it mends your friends."
- `calling` row: `tier_bonus=strength=0;agility=0;stamina=24;intellect=16;spirit=30`.

## The kit

Blight below is this Calling's `aura_creeping_blight`, a copy of the Acolyte's (growing shadow
damage over time that heals the caster, `leech_pct=50`, `stacks_per_tick=1`; tick 20 + 0.4L by
the amount rule). Plague Tithe, Harvest, Spread Blight, Wasting Communion and Hush all name that
one row, so a Skill Choice that changes the Blight adds params on the skill's own effect (the
skill's params win over the aura row's, `docs/spell_pipeline.md` line 26) instead of naming
another aura row.

| Key | Name | Learn | Row | What it does | Kinds | Visual |
|---|---|---|---|---|---|---|
| `creeping_blight` | Creeping Blight | 1 | 5 | carried: the Acolyte's, starts stronger | periodic_damage | as the Acolyte's |
| `plague_tithe` | Plague Tithe | 1 | 10 | filler. Channel 3 s, a tick a second, 30 yd, 6% base mana: 30 + 1.5L shadow a tick, the Blight grows a stack, and the most hurt friend within 30 yd heals all that tick dealt over 4 s | school_damage, grow_aura, periodic_heal (`ally=lowest_health;radius_yd=30;center=caster;pct_of=hit;pct=100`, as Blood Lance's), `cast_kind=channel` | 15407 |
| `miasma` | Miasma | 1 | 15 | carried: the Acolyte's cloud, starts stronger; its ally aura's healing bonus 17% instead of 15% | ground_area | as the Acolyte's |
| `harvest` | Harvest | 1 | 20 | instant, 30 yd, 10% base mana, 10 s cooldown: takes the Blight off the target for 12 + 0.8L shadow a stack, and every friend within 30 yd of it shares a heal of all that dealt | consume_aura (`per_stack=yes;heal_allies_pct=100;radius_yd=30`) | 1120 |
| `spread_blight` | Spread Blight | 10 | 30 | instant, 30 yd, 8% base mana, 12 s cooldown: the Blight on the target, with its stacks and time left, is copied to the three nearest foes within 10 yd | copy_aura (`targets=3;radius_yd=10`, ticket 05) | 172 |
| `hush` | Hush | 1 | 35 | carried Kick: the Acolyte's, lockout 4.5 s, a stopped cast grows the Blight | interrupt, periodic_damage (`if_worn=grow`) | as the Acolyte's |
| `wasting_communion` | Wasting Communion | 20 | 40 | instant at a ground point within 30 yd, 15% base mana, 30 s cooldown: for 10 s an 8 yd area heals friends inside 20 + 2L every 2 s and grows the Blight on every foe inside every 2 s | ground_area (`ally_aura`, `grow_aura=aura_creeping_blight;grow_every_ms=2000`), `target=dest`, `client_radius_index` for 8 yd | 15237 |
| `rise_again` | Rise Again | 30 | 45 | utility. 10 s cast, 30 yd, 20% base mana, out of combat: a dead friend returns at 30% health | resurrect (`health_pct=30`, ticket 08), `target=friend`, `requires=combat=out` | 7328 |

Aura rows: copies of `aura_creeping_blight`, `aura_miasma` and `aura_miasma_ally` (17%);
`aura_communion` (periodic_heal for friends inside).

## The Talent Rows

| Level | Row | Choice A | Choice B | Choice C |
|---|---|---|---|---|
| 5 | Creeping Blight | Virulent Blight: after each tick, 20% chance to spread to the nearest foe within 8 yd (`tick_spread_chance=20;tick_spread_yd=8` on the skill's effect) | Hungry Blight: heals you 75% of its damage (`leech_pct=75`) | Deep Blight: ticks 20% harder |
| 10 | Plague Tithe | Generous Tithe: heals 150% of the damage | Rotting Tithe: the Blight grows 2 stacks a tick | Long Tithe: 4 s, four ticks |
| 15 | Miasma | Healing Miasma: friends inside take 30% more healing (an ally aura copy) | Wide Miasma: 12 yd | Breeding Miasma: foes inside grow the Blight every 2 s (`grow_aura`, `grow_every_ms`) |
| 20 | Harvest | Bountiful Harvest: heals 150% of the damage | Seeded Harvest: a fresh Blight stack goes on the nearest foe within 8 yd (`spread_yd=8`) | Quick Harvest: 6 s cooldown |
| 25 | Talent, `row_key=blightcaller_rot_communion` | Contagion: when a foe wearing your Blight dies, the Blight with all its stacks jumps to the nearest foe within 10 yd (`proc on=wearer_death;aura=aura_creeping_blight;spread_aura=10;spread_stacks=all`, ticket 05) | Shared Rot: each Blight tick heals friends within 30 yd for 20% of its damage, shared (`proc on=periodic_tick;skill=creeping_blight;heal_allies_pct=20;radius_yd=30`) | Withering: your shadow damage 10% higher on foes wearing the Blight (`damage_pct_vs_aura`) |
| 30 | Spread Blight | Wide Spread: five foes | Far Spread: 15 yd | Quick Spread: 8 s cooldown |
| 35 | Hush | Long Hush: lockout 6.5 s | Choking Hush: a stopped cast grows the Blight 3 stacks | Mending Hush: a stopped cast heals the most hurt friend within 30 yd 40 + 4L over 4 s (`periodic_heal ally=lowest_health`) |
| 40 | Wasting Communion | Deep Communion: heals 50% more (an ally aura copy) | Wide Communion: 12 yd | Fast Communion: grows the Blight every second |
| 45 | Rise Again | Strong Return: 50% health | Quick Rites: 5 s cast | Far Rites: 40 yd |
| 50 | Talent, `row_key=blightcaller_plaguelord` | Blight Mastery: Blight ticks 15% harder (`skill_amount_pct skill=creeping_blight;applies_to=tick`) | Sustaining Rot: Plague Tithe deals 20% more, and so heals 20% more (`skill_amount_pct skill=plague_tithe`) | Harvest Moon: Harvest's cooldown 3 s shorter (spellmod `COOLDOWN`) |

## The trainer

Mother Ysolde Grey keeps a garden of rot behind the ruined house at Beggar's Haunt, on the hill
east of Darkshire (band 31 to 33), where two friendly Forsaken apothecaries already work (Apothecary
Faustin, 5414, at -10351.1, -1527.9; Deathstalker Zraedus, 5418; both `faction_template = 35` in
`duskwood.toml`). Find: hard.

```
guid = 9300118, entry = 190118, name = "Mother Ysolde Grey", subname = "Blightcaller", level = 50,
display_id = 4272 (Morbent Fel's look, a placeholder: pick another robed human if it reads as the
boss), faction_template = 35, role = "giver", npc_flags = 0x3, map = 0,
position = [-10372.0, -1552.0, 0.0], orientation = 0.8, hidden = true, text_id = 64188,
greeting = "Mind the beds, $C. Everything here is growing, and most of it is hungry."
```

Keep her 20 yd or more from Faustin and Zraedus, on the hilltop.

## The Advancement Quest, 64407

`kind = "advancement"`, giver 190118, level 30, `zone_or_sort = -262`,
`advancement = { from = "acolyte", to = "blightcaller", trial = 64452 }`,
`calling_choice = true`, money 0. Title "The Garden of Rot".

- Objectives: "Find the Blightcaller who keeps a garden of rot, and pass her trial."
- Details: "You rot them and you drink them, $N. A Blightcaller lets the rot do the mending: it grows on the foe, and the friend beside you heals.$B$BWhere the dead were left to the crows above a town that waits for dawn, the rot itself keeps a garden. Brother Corwin knows a seer in Darkshire who reads such things. Ask him."
- Progress: "You found the garden. Did you plant anything?"
- Completion: "Then it's yours to tend. Let it grow on them, and let it heal us. You start over, Blightcaller. The rot doesn't."

Hints (`hint` rows on 64407):

| NPC | Option | Reply |
|---|---|---|
| Brother Corwin Ashgrave (190054) | "Who reads the rot?" | "Madame Eva, in Darkshire. She reads cards for the Night Watch. Ask her about the garden." |
| Madame Eva (265) | "What do the cards say of the rot?" | "The Tower, reversed. Then the Mother. Blind Mary, in the house on the hill south-east of town, sees what I only read. Go to her." |
| Blind Mary (302) | "Who keeps the rot's garden?" | "Mother Grey. Up at Beggar's Haunt, past the dead men who make medicine. She feeds the beds what the beds want. Don't eat anything she grows." |

## The trial

Both: giver and ender 190118, level 30, `min_tier = 1`, `zone_or_sort = 10`. Easy: the search
was hard. Manor Mistmantle's grounds are banded 32 to 35 (the arc's ticket 06).

- **64451 Compost.** `after_taken = 64407`. Kill 6 Fetid Corpses (1270) on the grounds of Manor
  Mistmantle. Money 2500. Objectives: "Kill 6 Fetid Corpses on the grounds of Manor Mistmantle,
  north of Beggar's Haunt." Details: "The corpses around the manor walk because the rot was left
  untended. Lay six down, and let your Blight do the work while you mend yourself." Progress:
  "Down?" Completion: "Six. The beds will be glad."
- **64452 Rotting Hearts.** `after = 64451`. Collect 5 Rotting Hearts (980891) from Fetid Corpses
  (1270) at 50%. Money 5000. Objectives: "Bring 5 Rotting Hearts from the Fetid Corpses." Details:
  "Last. Bring me five hearts. The rot keeps them beating, if you know how to listen." Progress:
  "The hearts?" Completion: "Still warm. Sit down, Blightcaller."

Quest item: `{ entry = 980891, name = "Rotting Heart", quality = 1, item_level = 30, class = 12,
display_id = <a heart display with a free icon>, stackable = 20 }`; the collect row on 64452 is
`{ item = 980891, count = 5, chance = 50, from = [1270] }`.

## Failing tests first

New file `crates/twilight-realm/tests/tier2_blightcaller.rs` (setup as `tests/calling_choice.rs`;
the Blight as `tests/acolyte_rot.rs`; a second player as `tests/common/party.rs`):

- A level 30 Acolyte gets the Calling Choice listing 64407.
- With 64407 taken, Corwin, Madame Eva and Blind Mary show their hint lines; Mother Grey makes no
  map mark until met.
- The trial chain runs in order and gates 64407's hand-in; after it, the character is a level 1
  Blightcaller who knows Creeping Blight, Plague Tithe, Miasma, Harvest and Hush, and not Spread
  Blight until level 10.
- Healing through the Blight: with a hurt second player beside the Blightcaller, Plague Tithe on
  a mob heals that player, and Harvest on a Blighted mob heals them by a share of its damage.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `build_spells.py --check` is clean; `ground_check.py` flags nothing; `icon_check.py` reports no
  new clash.

Assumptions allowed: every number, `visual` and icon; the Calling icon path; the trainer's look
and exact spot on the hilltop.
