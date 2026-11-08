Title: Trapper: the kit, ten Talent Rows, Hale Brannock at the Eastvale logging camp, and his trial (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 4, 5, 9, 12 and 14. Blocked by 07-trap-springs and 16-leyweaver (the
Calling tickets run one after another: they share `master.csv`, both generated JSON files,
`callings.csv`, `quests.toml`, `quest_items.toml` and `hub.toml`).

The Trapper is the Ranger Line's mobile branch: it lays traps and ground areas on the move, never
stops, and shoots the whole way. Same role as the Ranger, ranged weapon damage. Every trap is
instant, so it is laid while running.

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

- `data/twilights-eve/callings.csv`: `trapper,Trapper,Interface\Icons\Spell_Fire_SelfDestruct,mana,hunter,warrior,100,100,,,,leather,<the ranger row's weapons>,Trapper,<blurb>`.
  Assumed: decision 14's "leather" is `leather` alone, a step lighter than the Ranger's mail.
  Blurb: "Never stops moving. Lays fire, frost and wire behind it, and shoots the whole way."
- `calling` row: `tier_bonus=strength=0;agility=30;stamina=20;intellect=10;spirit=0`.

## The kit

Every trap is a `ground_area` at the caster's feet in the form of the Ranger's Snare Trap
(`target=self`, `arm_ms`, `first_only=yes`, `object_display`), springing on every enemy within
`spring_radius_yd` (ticket 07). Each trap skill keeps one trap at a time, and different traps
stand together (one area per caster and skill).

| Key | Name | Learn | Row | What it does | Kinds | Visual |
|---|---|---|---|---|---|---|
| `auto_shot` | Auto Shot | 1 | none | copy of the Ranger's, unchanged | weapon_strike | as the Ranger's |
| `snapshot` | Snapshot | 1 | 5 | carried: the Ranger's instant shot, starts stronger (bonus 57 + 1.3L) | weapon_strike | as the Ranger's |
| `immolation_trap` | Immolation Trap | 1 | 10 | instant, 6% base mana, 15 s cooldown: a trap at your feet, armed after 1 s, lasting 60 s; the first enemy to step in springs it, and every enemy within 6 yd burns for 15 + 1.2L every 3 s for 12 s | ground_area (`radius_yd=3`, `arm_ms=1000`, `first_only=yes`, `spring_radius_yd=6`) | 13795 |
| `vault` | Vault | 1 | 15 | carried: the Ranger's backward leap; 17 yd instead of 15 | blink | as the Ranger's |
| `frost_trap` | Frost Trap | 1 | 20 | instant, 6% base mana, 20 s cooldown: as Immolation Trap, springing on every enemy within 8 yd: 50% slower and 6 + 0.5L frost every 2 s for 8 s | ground_area (`spring_radius_yd=8`) | 13809 |
| `caltrops` | Caltrops | 10 | 30 | instant, 5% base mana, 20 s cooldown: for 8 s a 4 yd area follows you; enemies inside bleed 8 + 0.8L a second and are 30% slower | ground_area (`follow_caster=yes`, `radius_yd=4`) | 2764 |
| `silencing_shot` | Silencing Shot | 1 | 35 | carried Kick: the Ranger's, lockout 3.5 s, a stopped cast resets Vault | interrupt, reset_cooldowns | as the Ranger's |
| `spring_the_traps` | Spring the Traps | 20 | 40 | instant, at a ground point within 35 yd, 8% base mana, 12 s cooldown: 50 + 2.5L fire to every enemy within 8 yd, and every armed trap of yours within 10 yd of the point springs at once | school_damage, detonate_traps (`radius_yd=10`, ticket 07); `target=dest`, `area_center=ground`, `realm_radius_yd=8`, `client_radius_index` for 8 yd | 13813 |
| `tripwire` | Tripwire | 30 | 45 | utility. Instant, 6% base mana, 30 s cooldown: a trap at your feet springing on every enemy within 6 yd, rooting them 4 s | ground_area (`spring_radius_yd=6`, enemy aura with `root`) | 1499 |

Aura rows: `aura_immolation_trap` (fire periodic_damage, 12 s, `target=enemy`),
`aura_frost_trap` (`mod_run_speed -50` and frost periodic_damage, 8 s, as the Ranger's
`aura_barbed_arrow` holds two effects), `aura_caltrops` (physical periodic_damage and
`mod_run_speed -30`, worn while inside), `aura_tripwire` (`root`, 4 s).

## The Talent Rows

| Level | Row | Choice A | Choice B | Choice C |
|---|---|---|---|---|
| 5 | Snapshot | Hard Snapshot: 25% more bonus | Running Snapshot: a crit takes 2 s off Vault's cooldown (`crit_cooldown_skill=vault;crit_cooldown_ms=2000`) | Cheap Snapshot: 30% less mana |
| 10 | Immolation Trap | Hot Trap: the burn 50% stronger (an aura row copy) | Wide Trap: springs 9 yd | Quick Trap: 10 s cooldown |
| 15 | Vault | Trap Vault: lays an Immolation Trap where you land (`cast_skill=immolation_trap`) | Far Vault: 22 yd | Quick Vault: 10 s cooldown |
| 20 | Frost Trap | Deep Freeze: 70% slower | Wide Frost: springs 11 yd | Biting Frost: frost ticks 50% stronger |
| 25 | Talent, `row_key=trapper_trapcraft` | Trap Mastery: Immolation Trap, Frost Trap and Tripwire cooldowns 20% shorter (spellmod `pct COOLDOWN -20`) | Running Fire: after 1 s of moving, Auto Shot fires on the move for 3 s, renewed while you run (`proc on=moving_for;ms=1000` putting up a `cast_while_moving skill=auto_shot` aura) | Light Feet: after 3 s of moving, 15% faster run for 4 s (`proc on=moving_for;ms=3000`) |
| 30 | Caltrops | Sharp Caltrops: bleed 50% stronger | Sticky Caltrops: 50% slower | Long Caltrops: 12 s |
| 35 | Silencing Shot | Long Silence: lockout 5 s | Quieting Snare: a stopped cast also slows the target 50% for 4 s | Reset Traps: a stopped cast also resets Immolation Trap (`skill=vault|immolation_trap`) |
| 40 | Spring the Traps | Wide Spring: 15 yd | Burning Spring: 50% more damage | Quick Spring: 8 s cooldown |
| 45 | Tripwire | Long Wire: 6 s root | Wide Wire: springs 9 yd | Bleeding Wire: the caught also bleed 8 s (an aura row copy with a second effect) |
| 50 | Talent, `row_key=trapper_huntsman` | Lure: Snapshot crits 10% more often | Momentum: after 5 s of moving, 15% ranged haste for 5 s (`proc on=moving_for;ms=5000`) | Killing Ground: Snapshot deals 20% more within 10 yd (`skill_amount_pct skill=snapshot;max_distance_yd=10`) |

## The trainer

Hale Brannock sets snares along the tree line south of the Eastvale Logging Camp in east Elwynn
(Elwynn's 20 to 24 band). Find: easy. Keep 15 yd or more from the camp's quest givers and Marshal
Haggard (about -9545, -1401).

```
guid = 9300117, entry = 190117, name = "Hale Brannock", subname = "Trapper", level = 50,
display_id = 4327 (Sven Yorgen's look, a placeholder), faction_template = 35, role = "giver",
npc_flags = 0x3, map = 0, position = [-9495.0, -1425.0, 0.0], orientation = 3.14,
hidden = true, text_id = 64187,
greeting = "Watch your feet, $C. Half this tree line is wire, and I've stopped remembering which half."
```

## The Advancement Quest, 64406

`kind = "advancement"`, giver 190117, level 30, `zone_or_sort = -261`,
`advancement = { from = "ranger", to = "trapper", trial = 64449 }`, `calling_choice = true`,
money 0. Title "The Trapper's Line".

- Objectives: "Find the Trapper at the Eastvale logging camp in Elwynn, and pass his trial."
- Details: "You never stand still long, $N, and you don't need to. A Trapper leaves the fight behind him, laid out in fire and wire, and keeps shooting while it springs.$B$BAt the logging camp in Eastvale, in east Elwynn, a trapper sets snares along the tree line. Thalyssa Windbough can tell you more."
- Progress: "Found the wire? Good. Did you find the work?"
- Completion: "Then you're a Trapper, and you'll never stand still again. You start over. The ground behind you doesn't."

Hint (`hint` row on 64406):

| NPC | Option | Reply |
|---|---|---|
| Thalyssa Windbough (190053) | "Where does the Trapper set his snares?" | "Hale Brannock works the tree line south of the Eastvale logging camp. Follow the cut wire, and step where he stepped." |

## The trial

All four: giver and ender 190117, level 30, `min_tier = 1`, `zone_or_sort = 44`. Hard: the
search was easy.

- **64446 Run Them Down.** `after_taken = 64406`. Kill 10 Redridge Mongrels (423) at Three
  Corners. Money 2500. Objectives: "Kill 10 Redridge Mongrels around Three Corners." Details:
  "Go east into Redridge. The mongrels hunt in packs. Lead them over your traps, and don't let
  them catch you standing." Progress: "Moving?" Completion: "Still moving. Good."
- **64447 Spider Silk.** `after = 64446`. Collect 8 Tarantula Silk (980881) from Tarantulas (442)
  at 50%. Money 2500. Objectives: "Bring 8 Tarantula Silk from the Tarantulas around Three
  Corners." Details: "Wire rusts. Silk doesn't. Eight bundles from the tarantulas." Progress:
  "Silk?" Completion: "Strong stuff. That'll hold a boar."
- **64448 Goretusks.** `after = 64447`. Kill 8 Great Goretusks (547) in Redridge. Money 2500.
  Objectives: "Kill 8 Great Goretusks in Redridge." Details: "Goretusks charge in a straight line.
  Put a trap in the line. Eight." Progress: "Straight lines?" Completion: "They never learn. You
  did."
- **64449 Bellygrub.** `after = 64448`. Kill Bellygrub (345), the great boar in the fields west of
  Lakeshire (about -9289, -1919). Money 5000. Objectives: "Kill Bellygrub in the fields west of
  Lakeshire." Details: "Last. Bellygrub is the biggest boar in Redridge and the farmers have
  given up on him. Lay the field with traps and run him through every one." Progress: "Is he
  down?" Completion: "Down, and you never stopped running. Come here."

Quest item: `{ entry = 980881, name = "Tarantula Silk", quality = 1, item_level = 30, class =
12, display_id = <a silk display with a free icon>, stackable = 20 }`; the collect row on 64447
is `{ item = 980881, count = 8, chance = 50, from = [442] }`.

## Failing tests first

New file `crates/twilight-realm/tests/tier2_trapper.rs` (setup as `tests/calling_choice.rs`;
traps as `tests/snare_trap.rs`):

- A level 30 Ranger gets the Calling Choice listing 64405 and 64406.
- With 64406 taken, Thalyssa shows the hint line; Hale Brannock makes no map mark until met.
- The trial chain runs in order and gates 64406's hand-in; after it, the character is a level 1
  Trapper who knows Auto Shot, Snapshot, Immolation Trap, Vault, Frost Trap and Silencing Shot,
  and not Caltrops until level 10.
- Immolation Trap and Frost Trap laid at once both stand; a mob walking into Immolation Trap
  springs it on every mob within 6 yd.
- The Trapper cannot equip mail (its `armor` is `leather`).

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `build_spells.py --check` is clean; `ground_check.py` flags nothing; `icon_check.py` reports no
  new clash.

Assumptions allowed: every number, `visual`, icon and trap `object_display`; the Calling icon
path; the trainer's exact spot on the tree line.
