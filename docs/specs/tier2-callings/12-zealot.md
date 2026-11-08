Title: Zealot rebuilt as a holy tank: the kit into the sheet, ten Talent Rows, Brother-Knight Aldren Vale at Tranquil Gardens, and his trial (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 4, 5, 6, 12 and 14. Blocked by 03-holy-tank-kinds and 11-ironclad
(the Calling tickets run one after another: they share `master.csv`, both generated JSON files,
`callings.csv`, `quests.toml`, `quest_items.toml` and `hub.toml`; 11 also gives Commander Wray
the gossip this ticket's hint needs).

The Zealot stands in its own Consecration, judges its Seal of Iron to mend itself, and is the
dead's worst enemy. It does not lean on blocks. Same role as the Vanguard, tank. The built
two-handed Zealot (Seal of Command, Seal of the Crusader, Mortal Strike, Charge, in
`content/callings.toml`) goes; only Judgement and Hammer of Justice stay, rewritten.

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

## Moving Zealot out of the TOML

- `crates/twilight-realm/content/callings.toml`: delete the `zealot` `[[calling]]` table and its
  six skills (lines 32 to 135), and fix the header comment (lines 28 to 30) so it says no Calling
  is left in the TOML except test Callings, if that is now true.
- `master.csv`: the `zealot` sheet already has a `calling` row with no `tier` (line 4) and six
  skill rows (`seal_of_command`, `seal_of_the_crusader`, `judgement`, `hammer_of_justice`,
  `mortal_strike`, `charge`, spell ids 1000065 to 1000070). Fill the `calling` row as below.
  Rewrite `judgement` and `hammer_of_justice` in place with `sheet.py set` (keep their spell ids,
  so a save's bar keeps them). Remove the other four rows with a small Python script written
  with the Write tool under `target/agent/` (the `csv` module, `newline=''`, written back with
  `write_bytes` and LF), since `sheet.py` has no delete; remove nothing else.
- Tests that lean on the old kit: `tests/zealot_skills.rs`, `tests/rules_seals.rs`,
  `tests/advancement.rs`, `tests/content_callings.rs`, `tests/tier1_skills.rs`,
  `tests/rules_power.rs`, `tests/tier_xp_rate.rs` and `tests/common/mod.rs`'s `tier_0_zealot`
  (line 15). Keep every behaviour still shipped tested. Seal of Command and Seal of the Crusader
  (`Effect::Seal`) are no longer on any shipped Calling: move their tests to a test Calling in
  inline TOML, in the form of `tests/ground_area.rs`, rather than deleting the seal code. Tests
  that used Zealot only as "some Tier 2 Calling" keep using `zealot`.

## The Calling

- `data/twilights-eve/callings.csv`, the `zealot` row: icon `Interface\Icons\Spell_Holy_RighteousFury`;
  armor and blurb as ticket 01 set them.
- `calling` row: `tier=2`, `tier_cap=50`, the row levels, `shapeshift_form` as the Vanguard's,
  `tier_bonus=strength=24;agility=12;stamina=24;intellect=0;spirit=6`.

## The kit

| Key | Name | Learn | Row | What it does | Kinds | Visual |
|---|---|---|---|---|---|---|
| `bulwark_charge` | Bulwark Charge | 1 | 5 | carried: the Vanguard's, starts stronger (17 rage; Consecration 24 + 0.6L holy a second), copying its `consecration` patch row and `aura_consecration_ward` | charge, energize, ground_effect | as the Vanguard's |
| `judgement` | Judgement | 1 | 10 | instant, 10 yd, 10 rage, 8 s cooldown: judges your Seal of Iron, taking it off, for 40 + 3L holy damage, and heals you 10% of max health; refused with no Seal up | judgement (`seal=aura_seal_of_iron`, `heal_pct=10`, ticket 03) | 20271 |
| `seal_of_iron` | Seal of Iron | 1 | 15 | carried: the Vanguard's; its armour spike 22% instead of 20% | grant_aura | as the Vanguard's |
| `hallowed_ground` | Hallowed Ground | 1 | 20 | instant, 15 rage, 20 s cooldown: Consecration at your feet for 10 s, 8 yd, 6 + 0.8L holy a second to enemies inside; while you stand in it you heal 8 + 1L a second. It replaces your Bulwark Charge's Consecration (one patch a caster) | ground_effect (`caster_aura` with `periodic_heal`) | 26573 |
| `righteous_fury` | Righteous Fury | 10 | 30 | instant, no cost: 5 min of 50% more threat from holy damage | grant_aura (aura: `mod_threat_pct pct=50;school=holy`, ticket 03) | 25780 |
| `rebuke` | Rebuke | 1 | 35 | carried Kick: the Vanguard's, lockout 4.5 s; a stopped cast gives 10 rage instead of Conviction (the Zealot has none) | interrupt, energize | as the Vanguard's |
| `holy_wrath` | Holy Wrath | 20 | 40 | instant, 20 rage, 15 s cooldown: 30 + 2.5L holy damage to every enemy within 8 yd, double to undead, and undead are stunned 2 s | school_damage (`vs_type=undead;vs_type_pct=100`), stun (`only_type=undead`) | 2812 |
| `hammer_of_justice` | Hammer of Justice | 30 | 45 | utility. The existing row, rewritten: 10 yd, 15 rage, 60 s cooldown, a 6 s stun through an aura row | stun | 853 |

Aura rows: copies of `aura_seal_of_iron` and `aura_seal_of_iron_armor` (armour 22%),
`aura_consecration_ward`; `aura_hallowed` (periodic_heal, worn while standing in Hallowed
Ground); `aura_righteous_fury`; `aura_holy_wrath_stun` (2 s); `aura_hammer_of_justice` (6 s).

## The Talent Rows

| Level | Row | Choice A | Choice B | Choice C |
|---|---|---|---|---|
| 5 | Bulwark Charge | Long Consecration: lasts 12 s | Rallying Charge: 27 rage | Hallowed Charge: Consecration deals 50% more |
| 10 | Judgement | Judgement of Mending: heals 15% | Harsh Judgement: 50% more holy damage | Swift Judgement: 5 s cooldown |
| 15 | Seal of Iron | Seal of Mending: its heal procs twice as often (as the Vanguard's `aura_seal_of_mending`) | Seal of Fervor: each proc also gives 5 rage | Wrathful Iron: armour spike 40% |
| 20 | Hallowed Ground | Deep Ground: heals 50% more | Wide Ground: 12 yd | Burning Ground: 50% more holy damage |
| 25 | Talent, `row_key=zealot_crusader` | Light's Grace: healing on you 10% stronger (`healing_taken_pct`) | Plated Faith: 10% more armour from items | Fervent: 10 more max rage (`max_power amount=100`, rage in tenths if the build stores it so) |
| 30 | Righteous Fury | Blazing Fury: 80% more threat | Fury's Ward: also 5% less damage taken while up | Fury's Edge: also 10% more holy damage while up (`mod_spell_damage_pct`) |
| 35 | Rebuke | Long Rebuke: lockout 6.5 s | Rebuke of Iron: a stopped cast also puts up Seal of Iron | Judging Rebuke: a stopped cast resets Judgement (`reset_cooldowns`) |
| 40 | Holy Wrath | Wrath of Ages: undead stunned 4 s | Searing Wrath: 50% more damage | Quick Wrath: 10 s cooldown |
| 45 | Hammer of Justice | Long Hammer: 8 s | Thrown Hammer: 20 yd | Quick Hammer: 40 s cooldown |
| 50 | Talent, `row_key=zealot_light_eternal` | Sanctified: Hallowed Ground heals 30% more (`skill_amount_pct`) | Undying Zeal: healing on you 30% stronger under 30% health | Hammer and Anvil: Judgement crits 15% more often |

## The trainer

Brother-Knight Aldren Vale keeps a vigil among the graves of Tranquil Gardens Cemetery in
south-east Duskwood (band 33-36), where the skeletons keep climbing out. Find: medium.

```
guid = 9300111, entry = 190111, name = "Brother-Knight Aldren Vale", subname = "Zealot",
level = 50, display_id = 4885 (Duthorian Rall's look, a placeholder), faction_template = 35,
role = "giver", npc_flags = 0x3, map = 0, position = [-11010.0, -1325.0, 0.0],
orientation = 1.57, hidden = true, text_id = 64181,
greeting = "Kneel if you like, $C. The dead here won't. That's why I stay."
```

## The Advancement Quest, 64003 (rewritten)

Keep the id and `calling_choice = true` (ticket 01). `giver = 190111` (was Wray);
`advancement = { from = "vanguard", to = "zealot", trial = 64418 }`, level 30,
`zone_or_sort = -81`, money 0. Title stays "The Zealot's Vow".

- Objectives: "Find the Zealot who keeps vigil over Duskwood's restless dead, and pass his trial."
- Details: "You fight like one possessed, $N. Plenty of people have stopped believing the Light watches anything at all. The Zealots answer it anyway.$B$BThe last sworn knight of them keeps a vigil where Duskwood laid its dead in quiet gardens, and the dead would not stay quiet. Commander Wray in the Stormwind barracks knows who keeps watch over that town. Ask him who to ask."
- Progress: "You found the gardens. The dead found you first, I expect. Is it done?"
- Completion: "Then swear the vow. You'll stand in the Light when the dead come, and you won't step back. You start over, Zealot. The plate on your back stays."

Hints (`hint` rows on 64003):

| NPC | Option | Reply |
|---|---|---|
| Commander Aldous Wray (190071) | "Where does the last Zealot keep vigil?" | "Duskwood. Commander Althea Ebonlocke runs the Night Watch in Darkshire. She knows every soul that walks her graves at night. Ask her." |
| Commander Althea Ebonlocke (264) | "Who keeps vigil among your dead?" | "A knight in white walks the Tranquil Gardens south and east of town, among the graves the dead keep climbing out of. My watchers leave him be. He prays louder than they scream." |

## The trial

All three: giver and ender 190111, level 30, `min_tier = 1`, `zone_or_sort = 10`. Medium.

- **64416 Hallowed Bones.** `after_taken = 64003`. Collect 8 Hallowed Bones (980821) from
  Skeletal Warriors (48) and Skeletal Mages (203) at 40%. Money 2500. Objectives: "Bring 8
  Hallowed Bones from the skeletons of Tranquil Gardens." Details: "These were Darkshire's
  people once. Break what walks and bring me the bones, and I'll bury them where they'll stay."
  Progress: "The bones?" Completion: "Eight. They'll rest now. The rest won't, yet."
- **64417 The Ghoul in the Crypt.** `after = 64416`. Kill the Insane Ghoul (511). Money 2500.
  Objectives: "Kill the Insane Ghoul that haunts Tranquil Gardens." Details: "Something in the
  crypt eats what I bury. It was a man once. Put it down." Progress: "Is it done?" Completion:
  "Then I can bury the rest without feeding it."
- **64418 Stand in the Light.** `after = 64417`. Kill 12 Skeletal Warriors (48). Money 5000.
  Objectives: "Kill 12 Skeletal Warriors in Tranquil Gardens." Details: "Last. Stand among the
  graves and let them come to you. Twelve, and you don't run." Progress: "Did you run?"
  Completion: "You didn't. Kneel, then, and swear."

Quest item: `{ entry = 980821, name = "Hallowed Bone", quality = 1, item_level = 30, class = 12,
display_id = <a bone display with a free icon>, stackable = 20 }`; the collect row on 64416 is
`{ item = 980821, count = 8, chance = 40, from = [48, 203] }`.

## Failing tests first

New file `crates/twilight-realm/tests/tier2_zealot.rs` (setup as `tests/calling_choice.rs`):

- With 64003 taken, Wray and Althea show their hint lines; `MapMarks` has no mark for the
  Brother-Knight until his gossip is opened.
- The trial chain runs in order and gates 64003's hand-in; after it, the character is a level 1
  Zealot who knows Bulwark Charge, Judgement, Seal of Iron, Hallowed Ground and Rebuke, and not
  Righteous Fury until level 10.
- Judgement with Seal of Iron up takes the Seal off, deals holy damage and heals the Zealot;
  with no Seal it is refused.
- Holy Wrath stuns an undead mob and not a humanoid one (pick shipped templates by
  `creature_type`).
- No shipped Calling is read from `callings.toml` any more, unless test Callings remain there.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `build_spells.py --check` is clean; `ground_check.py` flags nothing; `icon_check.py` reports no
  new clash.

Assumptions allowed: every number, `visual` and icon; how the old Zealot tests are moved; the
trainer's exact spot among the graves (within 20 yd, on open ground).
