Title: The Tower of Ilgalar: the valves, the overseer and Morganth (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 6, 7, 8 and 9. Blocked by 01-dungeon-templates (templates in the
dungeon file, the Ilgalar spell section, the lore-bible test).

Fill `crates/twilight-realm/content/ilgalar.toml` (the shell of cd940cee); its Phase Tester goes.
A run of about 15 minutes, three encounters. Rung 4 lands here: the tower is a working machine
and it keeps running after Morganth dies. Morganth's notebooks were split before the players
came (decision 8): he carries only the failure notes, and the tower shows the theft. The Anchor
stays standing, and the top floor keeps room for the Steppes spec's break Encounter
(SO:05-anchor-break). Nobody here says "Anchor" (the arc's Toren is the first, in 98502), names
the cult's master or says who sent the Blackrock.

## The building

The file keeps its building frame (`origin`, `yaw`) and its four footprint boxes. Creature and
GameObject positions are world coordinates, as today's Phase Tester's. The floors, from
`python3.15 tools/twilights-eve/wmo_floor.py <x> <y>`:

- the ground floor, z 115.67 (group 4; group 1 at the door);
- the ramp, group 4, rising through z 120 to 140 round the tower's inside;
- the middle storey, z about 141.3 (group 3);
- the top floor, z 150.09 (group 2), where vanilla Morganth stood.

Interior rows are `grounded = false` with the floor's Z, checked with `wmo_floor.py`.

## Templates

In `ilgalar.toml` itself (ticket 01), entries 97240 to 97244, in `mobs.toml`'s form (Captain
Rook's row, `mobs.toml` line 1899). Displays are vanilla's (`creature_template` in the vmangos
dump). Multipliers 1.0 unless said.

| Entry | Name | Type, rank | Class | Levels | Display, faction | Spells | Aggro line |
|---|---|---|---|---|---|---|---|
| 97240 | Twilight Tuner | 7, normal | mage | 35-36 | 2882 (Twilight Acolyte), 14 | `shadow_bolt_creature` | "The hum must not stop!" (30%) |
| 97241 | Servant of Ilgalar | 3, normal | warrior | 35-37 | 3916 (Servant of Ilgalar, 819), 14 | melee | none |
| 97242 | Blackrock Porter | 7, normal | warrior | 36 | 6033 (Blackrock Grunt, 440), 40 | melee | "Crates go up. You go down." (30%) |
| 97243 | Blackrock Overseer | 7, elite | warrior | 37 | 6044 (Blackrock Champion, 435), 40 | `shield_wall_rook`, `mortal_blow_overseer`, `sunder_kobold` | "Two lots of buyers came for those books. You're the third, and I'm not selling." (100%) |
| 97244 | Morganth | 7, elite, `boss = true` | mage | 38 | 3383 (Morganth, 397), 14 | `tuning_pulse_morganth`, `shadow_word_pain_creature`, `void_barrier_morganth` | "Too late. They took the pages that worked. All you'll find is what failed." (100%) |

## Spells

In `creature_spells.toml`, under the Ilgalar section's first row (`tuning_pulse_morganth`, ticket
01) and above the Raven Hill section's header. Placeholders in the Stockade's style.

- 1900071 `void_barrier_morganth`, "Void Barrier": `absorb { amount = 110, duration_ms = 10000 }`,
  1 s cast, every 40 s, range 0, Power Word: Shield's look (visual 17), as `stillness_the_hush`.
- 1900072 `mortal_blow_overseer`, "Mortal Blow": `hit { pct_of_swing = 200 }`, no cast, every
  10 s, range 5, visual 78, as `captains_cut_rook`.
- Reused as they are: `shadow_bolt_creature`, `shadow_word_pain_creature`, `shield_wall_rook`
  (Shield Wall every 30 s) and `sunder_kobold` (Sunder Armor).

Regenerate the patch (`build_spells.py --data "$WOW_DATA"`) and run the pipeline unittests.

## Creatures

Guids 974101 up, unique across every content file. 14 in all.

Encounter 1, the ground floor and the feed valves:
- Two pairs on the ground floor, a Twilight Tuner and a Servant of Ilgalar each: camp 1 about
  (-9275, -3300) and (-9277, -3302); camp 2 about (-9270, -3290) and (-9272, -3292). Guids 974101
  to 974104.
- Three valve guards, a Twilight Tuner and a Blackrock Porter each, 2 to 3 yd from their valve
  (below): camp 3 at valve A, camp 4 at valve B, camp 5 at valve C. Guids 974105 to 974110.

Encounter 2, the middle storey:
- The Blackrock Overseer, guid 974111, about (-9288, -3308, 141.31), with two Blackrock Porters,
  guids 974112 and 974113, about (-9285, -3305) and (-9291, -3305), camp 6.

Encounter 3, the top floor:
- Morganth, guid 974114, about (-9272, -3310, 150.04), facing the Anchor (orientation about
  3.6), there from the Run's start. Three timed calls, one per valve:
  `every = [{ every_s = 20, entry = 97241, level = 37, count = 1, position = [-9285.0, -3285.0,
  150.13], until_used = 983041 }, ...983042, ...983043]`. Leave his `drop` empty: the arc's
  ticket 27 adds the failure notes (980381) and its letters (980382) to it.

Keep every creature and GameObject at least 3 yd from the Steppes spec's wave points on the top
floor, (-9300, -3300) and (-9296, -3318), and none within 4 yd of the Anchor.

## GameObjects and cover

Guids 983041 to 983050, plus the Anchor at the Steppes spec's guid. Displays are stock.

| Guid | What | Display | Where | Rules |
|---|---|---|---|---|
| 983041 | Valve A | 755 (`DeadMineValve`) | the ramp's foot, about (-9278, -3310, 115.67) | `use = "lever"`, `used_text = "The valve groans shut. Something above stops answering."` |
| 983042 | Valve B | 755 | on the ramp, about (-9281, -3302), the ramp floor near z 125.4 | as valve A |
| 983043 | Valve C | 755 | the ramp's head, about (-9272, -3310, 140.34) | as valve A |
| 983044 | The emptied strongbox, its lock cut | 259 (`TreasureChest01`) | the middle storey, about (-9290, -3295, 141.26) | scenery, no `use` |
| 983045 | A Dark Iron crate under the cult's mark | 1868 (`DarkIronCrate01`) | ground floor, about (-9281, -3305, 115.67) | scenery |
| 983046 | A Dark Iron crate | 1869 (`DarkIronCrate02`) | ground floor, about (-9284, -3308, 115.67) | scenery |
| 983047 | A Dark Iron crate | 1868 | the middle storey, about (-9283, -3316, 141.36) | scenery |
| 983048 to 983050 | Pylons (see below) | 3080 (`UngoroCrystalPylon01`) | top floor | with `[[cover]]` |
| 794101 | The Anchor | 1667 (`FloatingPurpleCrystal01`) | `position = [-9288.0, -3308.0, 150.09]`, `orientation = 1.47`, `sides = "inside"` | the Steppes spec's row exactly, without `until_milestone` |

The pylons: three `[[cover]]` (radius 1.5, height 6.0) with a `[[gameobject]]` of display 3080 at
each, about (-9281, -3302, 151.30), (-9295, -3303, 150.13) and (-9283, -3316, 150.14), all on the
top floor about 7 yd round the Anchor. A comment above the Anchor row: "The
Anchor (#PARENT). SO:05-anchor-break gives this row `until_milestone =
"ilgalar_anchor_broken"` and adds the broken Anchor, 794102, beside it."

## Loot

The Blackrock Overseer `rolls = [{ chance = 50, one_of = [...] }]`, Morganth `chance = 100`. Until
ticket 07 lands, the list is the Stockade quiet wing's twenty (905080 to 905099) as a stand-in.

## Azora's wrong "foot of the tower"

The tower's door is at (-9282, -3328). Azora's shadow stop is at about (-9235, -2700), 630 yd
north of it, and two lines call that "the foot of the Tower of Ilgalar":

- `content/azora.toml` line 24 (a comment): "the ground in the hills east of Lakeshire, about
  (-9235, -2700), on the road toward the Tower of Ilgalar (its door is at (-9282, -3328))."
- `content/quests_elwynn.toml` line 981 (player text): "and one at the foot of the Ilgalar tower"
  becomes "and one in the hills on the road to the Ilgalar tower".

## Failing tests first

New `crates/twilight-realm/tests/ilgalar.rs`, with the layout checks `tests/stockade.rs` makes,
and the harness of `tests/phased_wards.rs` line 319:

- every creature and GameObject lies inside the footprint; no creature has entry 97100;
- exactly one Boss Bar creature, Morganth; each of valves A, B and C is named by exactly one of
  his timed calls;
- the three cover pillars stand within 10 yd of the Anchor (794101, display 1667);
- nothing stands within 3 yd of (-9300, -3300) or (-9296, -3318), nor within 4 yd of the Anchor;
- a Run with valve A pulled brings Morganth's Servants only for valves B and C;
- `quests_elwynn.toml` no longer says "foot of the Ilgalar tower".

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `python3.15 -I tools/twilights-eve/ground_check.py` flags nothing; `icon_check.py` is unchanged.

## Assumptions allowed

- Any interior position moved up to 3 yd to stand on the floor `wmo_floor.py` lists for its
  storey, each recorded. Valve B on whichever ramp floor between z 120 and 135 stands under that
  point.
- The aggro lines' wording, if a line breaks the never-name list; record it.
