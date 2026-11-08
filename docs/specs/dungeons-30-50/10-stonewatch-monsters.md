Title: Stonewatch Keep: the quartermaster, the envoy, Gath'Ilzogg and Singe (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 6, 15 and 16. Blocked by 03-partner-death-link (the
`on_partner_death` key; it lands after 01 and 02, so the templates in the dungeon file and the
Stonewatch spell section are there too) and RD:04-dungeon-levels-whelps (it sets this file's
`levels` to 45-48 and edits its Phase Tester row).

Fill `crates/twilight-realm/content/stonewatch_keep.toml` (the shell of cd940cee); its Phase
Tester goes. A run of about 15 minutes, three encounters. Rung 8 lands here: black dragonkin
fight beside the Blackrock, which shows that the orcs and the cult answer to the same master.
Nobody names Nefarian, Onyxia, Deathwing or Prestor, and nothing reveals the Cathedral's rung 7.
The proof the player carries out is Singe's scale (ticket 11), nothing written.

## The building

The file keeps its building frame and footprint boxes. Positions are world coordinates; every
row is interior, `grounded = false`, with the floor's Z from
`python3.15 tools/twilights-eve/wmo_floor.py <x> <y>`:

- the gate hall (group 13, generalarea), z 139.22 at (-9385, -3059), the Phase Tester's spot;
- the right halls, z 140.68: group 7 (entry) at (-9399, -3062), group 10 (righthall09) at
  (-9406, -3072);
- the rear storage (group 8), z 140.68 at (-9408, -3059);
- the staging area (group 15, stagingarea02), z 140.68 at (-9368, -3069);
- the war room (group 2), z 152.55 at (-9366, -3062), and left_upper03 (group 3), z 149.50 at
  (-9368, -3069);
- the top storey (group 0), z 158.07 at (-9383, -3082), vanilla Gath'Ilzogg and Singe's spot,
  and (-9378, -3076).

## Templates

In `stonewatch_keep.toml` itself (ticket 01), entries 97280 to 97288, in `mobs.toml`'s form.
Displays and factions from the vmangos dump. Multipliers 1.0 unless said.

| Entry | Name | Type, rank | Class | Levels | Display, faction | Spells | Aggro line |
|---|---|---|---|---|---|---|---|
| 97280 | Blackrock Sentry | 7, normal | warrior | 45 | 6029 (Blackrock Sentry, 4065), 40 | melee | "Intruders in the keep!" (30%) |
| 97281 | Blackrock Shadowcaster | 7, normal | mage | 45 | 6039 (Blackrock Shadowcaster, 436), 40 | `shadow_bolt_creature` | "The keep is ours now." (30%) |
| 97282 | Blackrock Gladiator | 7, elite | warrior | 46 | 6042 (Blackrock Gladiator, 4464), 40 | melee | "Blood for the warlord!" (30%) |
| 97283 | Blackrock Quartermaster | 7, elite | warrior | 46 | 6034 (Blackrock Outrunner, 485), 40 | `shield_wall_rook` | "Every crate is counted. So are you." (100%) |
| 97284 | Twilight Paymaster | 7, elite | mage | 46 | 2882 (Twilight Acolyte), 14 | `dark_mending_paymaster`, `shadow_bolt_creature` | "The coin is for them, not for you." (100%) |
| 97285 | Dragonspawn Envoy | 2, elite | warrior | 47 | 143 (Black Dragonspawn, 7040), 103 | `flame_breath_dragonkin`, `terrify_creature`, `sunder_kobold` | "Small things. You will burn with the rest." (100%) |
| 97286 | Black Broodling | 2, normal | warrior | 45-46 | 397 (Black Broodling, 7047), 103 | melee | none |
| 97287 | Gath'Ilzogg | 7, elite, `boss = true` | warrior | 48 | 6050 (Gath'Ilzogg, 334), 40 | `shield_wall_rook`, `cleave_gathilzogg`, `rushing_charge` | "Ask my ally what it thinks of you." (100%) |
| 97288 | Singe | 2, elite, `boss = true` | warrior | 48 | 497 (Singe, 335), 103 | `flame_breath_dragonkin`, `wing_buffet_singe`, `terrify_creature` | none |

"Blackrock Quartermaster", "Twilight Paymaster" and "Dragonspawn Envoy" are placeholder names.

## Spells

In `creature_spells.toml`, under the Stonewatch section's first row (`flame_breath_dragonkin`,
ticket 01) and above whatever section follows it (ticket 03 put its test rows there).
Placeholders.

- 1900091 `wing_buffet_singe`, "Wing Buffet": `nova { damage = 180, radius = 10.0 }`, physical,
  1 s cast, every 20 s, Shadow Nova's look (visual 686).
- 1900092 `cleave_gathilzogg`, "Warlord's Cleave": `hit { pct_of_swing = 200 }`, no cast, every
  10 s, range 5, visual 78.
- 1900093 `enraged_partner`, "Enraged": `stance { damage_dealt_pct = 50, damage_taken_pct = 0,
  duration_ms = 0 }`, chance 0 and cast columns 0 (ticket 03: only `on_partner_death` puts it
  up), `warning = "A roar of rage shakes the roof!"`, Shield Wall's look (visual 871), Enrage's
  icon (`Interface\Icons\Spell_Shadow_UnholyFrenzy` if no better stock icon).
- 1900094 `dark_mending_paymaster`, "Dark Mending": `heal { pct_max_health = 20 }`, 3 s cast (a
  Kick stops it, ADR 0020), every 15 s, `below_health_pct = 70`, the Abbot's heal look (visual
  2050). A creature heal heals only its caster, so the paymaster mends himself.
- Reused: `shadow_bolt_creature`, `shield_wall_rook`, `terrify_creature`, `sunder_kobold`,
  `rushing_charge`.

Regenerate the patch (`build_spells.py --data "$WOW_DATA"`) and run the pipeline unittests.

## Creatures

Guids 974301 up, unique across every content file. 17 in all.

Encounter 1, the gate hall, the right halls and the stores:
- Two Blackrock Sentries and a Shadowcaster, camp 1, in the gate hall about (-9385, -3059,
  139.22), guids 974301 to 974303.
- Two pairs of Blackrock Gladiators: camp 2 about (-9399, -3062, 140.68), guids 974304 and
  974305; camp 3 about (-9406, -3072, 140.68), guids 974306 and 974307.
- The Blackrock Quartermaster, guid 974308, and the Twilight Paymaster, guid 974309, camp 4, in
  the rear storage about (-9408, -3059, 140.68), among the Tribute crates.

Encounter 2, the staging area and the war room:
- A Sentry, a Shadowcaster and a Sentry, camp 5, in the staging area about (-9368, -3069,
  140.68), guids 974310 to 974312.
- The Dragonspawn Envoy, guid 974313, in the war room about (-9366, -3062, 152.55), with two Black
  Broodlings, guids 974314 and 974315, camp 6. His timed call: `every = [{ every_s = 20, entry =
  97286, level = 45, count = 2, position = <the brood cage's point>, until_used = 983061 }]`.

Encounter 3, the top storey:
- Gath'Ilzogg, guid 974316, at (-9383, -3082, 158.07), and Singe, guid 974317, at (-9378, -3076,
  158.07), camp 7, so one pull brings both. Both are on the Boss Bar at once (it holds up to
  three). Gath'Ilzogg carries `on_partner_death = { guid = 974317, spell = "enraged_partner" }`
  and Singe `on_partner_death = { guid = 974316, spell = "enraged_partner" }` (ticket 03): when
  one dies, the other enrages. Leave both `drop` lists empty: the arc's ticket 29 adds page three
  (980386) to Gath'Ilzogg's, and ticket 11 adds Singe's Scale to Singe's.

## GameObjects

Guids 983061 up. Displays are stock.

| Guid | What | Display | Where | Rules |
|---|---|---|---|---|
| 983061 | The brood cage lever | 295 (`G_LeverMetal`) | the war room, about (-9364, -3058, 152.55) | `use = "lever"`, `used_text = "The cage slams shut on the brood."` |
| 983062 | The brood cage | 676 (`G_Cage`) | left_upper03, about (-9368, -3070, 149.50); the Envoy's call comes from here | scenery |
| 983063 | A Tribute crate under the cult's mark | 1868 (`DarkIronCrate01`) | the rear storage, about (-9410, -3056, 140.68) | scenery |
| 983064 | A Tribute crate | 1869 (`DarkIronCrate02`) | about (-9411, -3062, 140.68) | scenery |
| 983065 | A Tribute crate | 1868 | about (-9405, -3055, 140.68) | scenery |

## Loot

The Quartermaster and the Envoy `rolls = [{ chance = 50, one_of = [...] }]`; Gath'Ilzogg and
Singe each `chance = 100`. Until ticket 11 lands, the list is the Stockade quiet wing's twenty
(905080 to 905099) as a stand-in.

## Failing tests first

New `crates/twilight-realm/tests/stonewatch_keep.rs`, with the layout checks `tests/stockade.rs`
makes and the harness of `tests/phased_wards.rs`:

- every creature and GameObject lies inside the footprint; no creature has entry 97100;
- exactly two Boss Bar creatures, Gath'Ilzogg and Singe, in one camp, each naming the other in
  `on_partner_death` with `enraged_partner`;
- the Envoy's call names lever 983061;
- in a Run, killing Singe while Gath'Ilzogg fights puts Gath'Ilzogg in the Enraged stance, and
  the other way round;
- a Run with the lever pulled brings no Broodlings from the Envoy's call.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `python3.15 -I tools/twilights-eve/ground_check.py` flags nothing; `icon_check.py` is unchanged.

## Assumptions allowed

- Any position moved up to 3 yd to stand on the floor named, each recorded; the brood cage and
  lever placed where the war room's floor allows, the cage reachable from the Envoy's room.
- The Quartermaster's display swapped for another stock Blackrock orc if 6034 reads wrong.
