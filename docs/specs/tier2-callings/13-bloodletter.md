Title: Bloodletter: the kit, ten Talent Rows, Sabine Redmark at the poachers' fire, and her trial (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 4, 5, 7, 12 and 14. Blocked by 04-blade-kinds and 12-zealot (the
Calling tickets run one after another: they share `master.csv`, both generated JSON files,
`callings.csv`, `quests.toml`, `quest_items.toml` and `hub.toml`).

The Bloodletter keeps every wound open and grows wilder with every drop it walks through. Its
bleeds spill more droplets and Bloodlust stacks higher, so a long fight keeps getting faster.
Same role as the Cutthroat, melee damage.

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

- `data/twilights-eve/callings.csv`: `bloodletter,Bloodletter,Interface\Icons\Ability_Rogue_Rupture,energy,rogue,warrior,100,100,25,aura_blood_droplet,50,cloth|leather,<the cutthroat row's weapons>,Bloodletter,<blurb>`.
  Blurb: "Keeps every wound open and grows wilder with every drop of blood it walks through."
- `calling` row: `tier_bonus=strength=12;agility=24;stamina=24;intellect=0;spirit=6`.

## The kit

| Key | Name | Learn | Row | What it does | Kinds | Visual |
|---|---|---|---|---|---|---|
| `rupture` | Rupture | 1 | 5 | carried: the Cutthroat's finisher, starts stronger (tick 26 + 0.6L, per point 2 + 0.05L) | periodic_damage | as the Cutthroat's |
| `hemorrhage` | Hemorrhage | 1 | 10 | builder. Melee, 35 energy: 110% weapon damage + 28 + 2L, 1 combo point, and for 15 s the target takes 20% more from your bleeds | weapon_strike, add_combo_points, mod_periodic_taken_pct (ticket 04) | 16511 |
| `garrote` | Garrote | 1 | 15 | carried: the Cutthroat's bleed and combo point, starts stronger (tick 35 + 0.8L). It needs no Stealth here (the Bloodletter has none): drop `stealth=yes` from `requires` and the stance from `client_stances` | periodic_damage, add_combo_points | as the Cutthroat's |
| `open_veins` | Open Veins | 1 | 20 | finisher. Melee, 30 energy: each of your bleeds on the target deals one tick at once per combo point | tick_now (`ticks_per_combo=1`, ticket 04) | 1943 |
| `blood_fan` | Blood Fan | 10 | 30 | 40 energy, 10 s cooldown: up to 5 enemies in an 8 yd cone in front bleed for 10 + 1L every 2 s for 8 s, each tick spilling a droplet at 25% | periodic_damage (`drop_chance=25`; `area_shape=cone`, `area_center=caster`, `max_targets=5`, `realm_radius_yd=8`) | 13877 |
| `kick` | Kick | 1 | 35 | carried Kick: the Cutthroat's, lockout 5.5 s, a stopped cast gives a combo point | interrupt, add_combo_points | as the Cutthroat's |
| `red_frenzy` | Red Frenzy | 20 | 40 | no cost, 30 s cooldown: spends every Bloodlust stack for 10 s of Red Frenzy, 6% faster swings a stack | consume_self_stacks (`aura=aura_bloodlust;grant=aura_red_frenzy`, ticket 04) | 13750 |
| `blood_pool` | Blood Pool | 30 | 45 | utility. 25 energy, 30 s cooldown: spills three blood droplets around you | spill_droplets (`count=3`, ticket 04) | 1943 |

Aura rows: copies of the Cutthroat's `aura_blood_droplet` (its `grants` names this Calling's
`aura_bloodlust`), `aura_bloodlust` and `aura_garrote` (tick by the amount rule);
`aura_hemorrhage` (`mod_periodic_taken_pct pct=20;school=physical`, 15 s, `target=enemy`);
`aura_blood_fan` (physical, `mechanic=bleed`); `aura_red_frenzy` (`mod_melee_haste_pct` 6,
`max_stacks=5`, `stack_rule=stack`, 10 s).

## The Talent Rows

| Level | Row | Choice A | Choice B | Choice C |
|---|---|---|---|---|
| 5 | Rupture | Deep Rupture: ticks 30% harder | Spilling Rupture: droplets at 80% (`drop_chance=80`) | Long Rupture: 3 s per combo point |
| 10 | Hemorrhage | Open Wound: 30% more from your bleeds | Twin Cut: 1 more combo point on a bleeding target (`bleeding_bonus=1`) | Gushing Strike: 30% more on a bleeding target (`bleeding_pct=30`) |
| 15 | Garrote | Deep Garrote: ticks 30% harder | Bleeding Garrote: droplets at 80% | Long Garrote: lasts 6 s longer (an aura row copy) |
| 20 | Open Veins | Torrent: two ticks per combo point | Spilling Veins: each forced tick spills a droplet at 50% (`drop_chance=50`) | Feeding Veins: heals you 30% of what it dealt (`heal_pct=30`) |
| 25 | Talent, `row_key=bloodletter_blood_sense` | Thirst: a droplet picked up heals 2% of max health (`pickup_heal_pct`) | Quickening: a droplet picked up gives 10 energy (`pickup_energize`) | Deep Wounds: melee crits leave a bleed of 30% of the crit (`crit_bleed`, `of=crit_damage`) |
| 30 | Blood Fan | Wide Fan: up to 8 enemies | Long Fan: 12 s | Spilling Fan: droplets at 50% |
| 35 | Kick | Long Kick: lockout 7.5 s | Bleeding Kick: a stopped cast also makes the target bleed 6 s | Double Kick: 2 combo points |
| 40 | Red Frenzy | Sustained Frenzy: 15 s | Frenzied Blood: also 3% more damage a stack (`mod_damage_done_pct`) | Quick Frenzy: 20 s cooldown |
| 45 | Blood Pool | Deep Pool: five droplets | Quick Pool: 10 s cooldown | Lasting Pool: the droplets last 16 s (`duration_ms=16000`) |
| 50 | Talent, `row_key=bloodletter_exsanguinate` | Arterial: Rupture, Garrote and Blood Fan tick 15% harder (`skill_amount_pct applies_to=tick`, three effects) | Butcher: Hemorrhage crits 10% more often | Last Drop: a killing blow heals 5% of max health and gives 20 energy (`proc on=kill`) |

## The trainer

Sabine Redmark sits at the poachers' fire on Lakeridge Highway, south-west of Lake Everstill
(band 33-36), among men who think she is one of them. Find: medium.

```
guid = 9300112, entry = 190112, name = "Sabine Redmark", subname = "Bloodletter", level = 50,
display_id = 1297 (Keryn Sylvius's look, a placeholder), faction_template = 35, role = "giver",
npc_flags = 0x3, map = 0, position = [-9785.0, -2300.0, 0.0], orientation = 0.0,
hidden = true, text_id = 64182,
greeting = "Sit, $C. They think I skin rabbits. I let them think it."
```

## The Advancement Quest, 64401

`kind = "advancement"`, giver 190112, level 30, `zone_or_sort = -162`,
`advancement = { from = "cutthroat", to = "bloodletter", trial = 64423 }`,
`calling_choice = true`, money 0. Title "The Bloodletter's Salt".

- Objectives: "Find the Bloodletter who sits at the poachers' fire in Redridge, and pass her trial."
- Details: "You cut well, $N, but you let them stop bleeding. There's a woman who never does.$B$BShe sits at a fire on the highway south of Lake Everstill, among men who think she is one of them. Jorik Vane knows a man in Lakeshire who sells her the salt. Ask Jorik."
- Progress: "You found the fire. Are your hands red yet?"
- Completion: "Good. Now you know what I know: a wound is a door, and you keep it open. You start over, Bloodletter. The blood stays."

Hints (`hint` rows on 64401):

| NPC | Option | Reply |
|---|---|---|
| Jorik Vane (190051) | "Who sells blood for a living?" | "Wiley the Black, in Lakeshire. He sells salt and hooks to a woman who skins what she bleeds. Tell him I sent you. Or don't. He'll guess." |
| Wiley the Black (266) | "Who buys your salt?" | "Sabine. She sits at the poachers' fire on Lakeridge Highway, south and west of the lake. They think she's one of them. They're wrong about a lot of things." |

## The trial

All three: giver and ender 190112, level 30, `min_tier = 1`, `zone_or_sort = 44`. Medium.

- **64421 First Blood.** `after_taken = 64401`. Kill 10 Redridge Poachers (424). Money 2500.
  Objectives: "Kill 10 Redridge Poachers on Lakeridge Highway." Details: "They're my cover, but
  they're getting greedy. Ten of them, and let them bleed out slow so the others hear." Progress:
  "Did they bleed?" Completion: "I heard. So did they."
- **64422 Mongrel Hides.** `after = 64421`. Collect 8 Torn Mongrel Hides (980831) from Redridge
  Mongrels (423) at 50%. Money 2500. Objectives: "Bring 8 Torn Mongrel Hides from the Redridge
  Mongrels." Details: "The gnoll mongrels run with the poachers. Bring me eight hides. Torn is
  fine. Torn is the point." Progress: "Hides?" Completion: "Torn the right way. You learn."
- **64423 Spider Blood.** `after = 64422`. Kill 8 Tarantulas (442) at Three Corners. Money 5000.
  Objectives: "Kill 8 Tarantulas around Three Corners." Details: "Last. Spiders don't bleed like
  men, and that's why I send you. Eight. Find out how they do." Progress: "Well?" Completion:
  "Green and slow. Everything bleeds, $N. Sit down."

Quest item: `{ entry = 980831, name = "Torn Mongrel Hide", quality = 1, item_level = 30, class =
12, display_id = <a hide display with a free icon>, stackable = 20 }`; the collect row on 64422
is `{ item = 980831, count = 8, chance = 50, from = [423] }`.

## Failing tests first

New file `crates/twilight-realm/tests/tier2_bloodletter.rs` (setup as `tests/calling_choice.rs`;
droplets as `tests/cutthroat_blood.rs`):

- A level 30 Cutthroat gets the Calling Choice listing 64401.
- With 64401 taken, Jorik Vane and Wiley the Black show their hint lines; Sabine makes no map
  mark until met.
- The trial chain runs in order and gates 64401's hand-in; after it, the character is a level 1
  Bloodletter who knows Rupture, Hemorrhage, Garrote, Open Veins and Kick, and not Blood Fan
  until level 10.
- The Bloodletter's bleed ticks spill droplets (its `callings.csv` row names
  `aura_blood_droplet`), and its Garrote casts outside Stealth.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `build_spells.py --check` is clean; `ground_check.py` flags nothing; `icon_check.py` reports no
  new clash.

Assumptions allowed: every number, `visual` and icon; the Calling icon path; the trainer's exact
spot at the camp (within 15 yd of the stock poachers' fire, if one stands near).
