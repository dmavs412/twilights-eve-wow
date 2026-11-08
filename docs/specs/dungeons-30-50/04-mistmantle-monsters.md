Title: Mistmantle Manor: the yard, the steward, the lady and Stalvan (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 1, 6, 10, 11 and 12. Blocked by 01-dungeon-templates (templates in the
dungeon file, the Mistmantle spell section, the lore-bible test) and RD:11-bands-35-40 (it edits
Duskwood's rows in `content/zones.toml`, as this ticket does).

Fill `crates/twilight-realm/content/mistmantle_manor.toml` (the shell of cd940cee); its Phase
Tester goes. A run of about 15 minutes, three encounters, plain fights: no meter, no sleepers.
The household went still on the night the dark came, and something under the house still hums.
Nobody in this dungeon says "Anchor", names the cult or shows a Scourge sign (decision 1).

## The footprint

Today the footprint is the house's one indoor group, 14 by 16 yd, in the building's frame. It
grows to an open-air box over the farm, Forest's Edge's form (`forest_edge.toml`): rewrite the
file in the world frame (`origin = [0.0, 0.0, 0.0]`, `yaw = 0.0`) with one box, which holds the
house's old box too.

```toml
[[footprint]]
x = [-10392.0, -10298.0]
y = [-1307.0, -1183.0]
z = [25.0, 70.0]

[wall]
gap_yd = 8.0
```

- The farm has no outbuildings in the game data (decision 10). The box holds the house
  (`duskwoodabandoned_human_farm.wmo`, door at (-10362.6, -1257.0)), the fenced yard north of it
  (fences along x -10313, y -1191 to -1220; a gate torch at (-10315, -1199); a cart and barrels by
  the door) and the walled garden east of it (fences x -10379 to -10337, y -1299 to -1271).
- The Portal moves from the front door to the box's edge, where the track from Darkshire (south
  west of the farm) meets it. First cut: (-10345.0, -1184.0) on the west edge. Move it along the
  edge, up to 20 yd, to the nearest flat spot clear of fences; the `[wall]`'s gap is round the
  Portal. Write its Z as 0.0 for `ground_check.py --fix`. Record the spot.
- The arc's quest 98607 (RD:08-duskwood-30-35) explores the old Portal point at the front door,
  which is now inside the walled box. Say the new Portal position in the closing comment, so the
  filer can point RD:08's explore at it.

## Removed open-world creatures

In `content/zones.toml`, Duskwood's `removed_guids` (line 22, under the Mistmantle comment, beside
Stalvan's 5010), add the ten Fetid Corpses (1270) that stand inside the new box: 4316, 4317, 4829,
4906, 5935, 5936, 5939, 6077, 6080 and 6091. Change the comment to say the list now covers the
yard. The Unseen (3094) inside it go by entry already. The `lore_bible_dungeons` test
`no_open_world_creature_or_hub_npc_stands_inside_them` lists anything missed.

## Templates

In `mistmantle_manor.toml` itself (ticket 01), entries 97225 to 97229, each in `mobs.toml`'s form
(Captain Rook's row, `mobs.toml` line 1899, is the model). Displays and factions are vanilla's
(`creature_template` in the vmangos dump,
`C:\Users\davis\AppData\Local\Temp\vmangos\sqlite-dump\mangos.sqlite`). Multipliers 1.0 unless
said. No template has an aggro line except Stalvan's: the house is silent.

| Entry | Name | Type, rank | Class | Levels | Display, faction | Spells |
|---|---|---|---|---|---|---|
| 97225 | Risen Farmhand | 6, normal | warrior | 32-33 | 559 (Fetid Corpse, 1270), 21 | melee |
| 97226 | Silent Servant | 6, normal | warrior | 33-34 | 3229 (Haunted Servitor, 3875), 21 | melee |
| 97227 | Steward Aldric | 6, elite | warrior | 34 | a stock human ghost display from the dump, not 1562 or 3229, 21 | `hush_steward` |
| 97228 | Lady Corrine Mistmantle | 6, elite | mage | 35 | a stock female human ghost display from the dump, 21 | `terrify_creature`, `shadow_word_pain_creature` |
| 97229 | Stalvan Mistmantle | 6, elite, `boss = true` | warrior | 36 | 1562 (Stalvan, 315), 14 | `candles_flare_stalvan`, `terrify_creature`, `grief_stalvan` |

Stalvan's line: `say = { aggro = ["Hush. You'll wake the house."], aggro_chance = 100 }`. The
names other than Stalvan's are placeholders.

## Spells

In `creature_spells.toml`, under the Mistmantle section's first row (`candles_flare_stalvan`,
ticket 01) and above the Ilgalar section's header. Numbers are placeholders in the Stockade's
style, scaled to the file's levels.

- 1900061 `hush_steward`, "Hush": `silence { duration_ms = 4000 }`, `target = "random"`, 1 s
  cast, every 15 s, range 30, the look of Silence (visual 15487, as `take_down_quill`).
- 1900062 `grief_stalvan`, "Grief": `nova { damage = 150, radius = 10.0 }`, shadow, 2 s cast,
  every 15 s, `below_health_pct = 30`, Shadow Nova's look (visual 686), `warning = "Stalvan's
  grief fills the room!"`.
- Reused as they are: `terrify_creature` (Fear, random, every 30 s) and
  `shadow_word_pain_creature`.

Regenerate the patch (`python3.15 tools/twilights-eve/build_spells.py --data "$WOW_DATA"`) and run
the pipeline unittests.

## Creatures

Guids 974001 up, unique across every content file (grep). 14 in all. Outdoor rows write Z as 0.0
for `ground_check.py --fix`; the house's rows stand on its floor at z 35.91 (the Phase Tester's),
`grounded = false`, checked with `python3.15 tools/twilights-eve/wmo_floor.py <x> <y>`.

Encounter 1, the yard and the steward:
- Three camps of three Risen Farmhands (level 32, 32, 33), camps 1 to 3, guids 974001 to 974009,
  about (-10335, -1214), (-10322, -1248) and (-10346, -1232), each spread 3 to 5 yd.
- Steward Aldric, guid 974010, level 34, at the front door, about (-10357, -1257), facing north
  (orientation 0.0). `on_health = [{ pct = 50, entry = 97226, level = 33, count = 2, position =
  [-10362.6, -1257.0, 35.91] }]`: two servants come out of the door.

Encounter 2, the walled garden:
- Lady Corrine Mistmantle, guid 974013, level 35, about (-10360, -1288), with two Silent Servants
  (levels 33 and 34), guids 974011 and 974012, about (-10350, -1281) and (-10368, -1281), all
  camp 4.

Encounter 3, the house:
- Stalvan Mistmantle, guid 974014, level 36, at the room's south end, about (-10374.5, -1255.5,
  35.91), facing the door (orientation 0.0), there from the Run's start. Leave his `drop` empty:
  the arc's ticket 26 adds the Surveyor's Site Record (980481) to it.

## Cover and GameObjects

Guids 983031 up. A `[[cover]]` and a `[[gameobject]]` share each point, as Jasperlode's pillars do
(`jasperlode.toml` line 362).

| Guid | What | Display | Where | Notes |
|---|---|---|---|---|
| 983031 | The humming ore, where the floorboards are torn up | 2370 (`CaveMineCrystalFormation04`) | at Stalvan's feet, about (-10376.0, -1255.5, 35.91) | scenery; never named in any text |
| 983032 | A stack of crates | 18 (`WestFallCrate`) | about (-10369.0, -1251.5, 35.91) | `[[cover]]` radius 1.2, height 4.0 |
| 983033 | A stack of crates | 18 | about (-10369.0, -1259.5, 35.91) | `[[cover]]` radius 1.2, height 4.0 |
| 983034 | Candles | 100 (`Candle01`) | about (-10364.0, -1250.0, 35.91) | scenery |
| 983035 | Candles | 100 | about (-10364.0, -1261.0, 35.91) | scenery |

Each house point must stand on the room's floor (`wmo_floor.py`); move it up to 2 yd if not, and
record it. The two cover points stand between Stalvan and the door, so a character near the door
can put one between them and him.

## Loot

Steward Aldric and Lady Corrine `rolls = [{ chance = 50, one_of = [...] }]`, Stalvan `chance =
100`. Until ticket 05 lands, the list is the Stockade quiet wing's twenty (905080 to 905099) as a
stand-in, as #578 used the Abbot's.

## Failing tests first

New `crates/twilight-realm/tests/mistmantle_manor.rs`, with the layout checks
`tests/stockade.rs` and `tests/forest_edge_layout.rs` make:

- every creature and GameObject lies inside the footprint; the Portal stands within 5 yd of the
  footprint's edge; the file has a `[wall]`;
- no creature has entry 97100; exactly one creature's template has `boss = true`, Stalvan's;
- the steward's call brings two 97226 at the front door;
- each of the two cover pillars stands within 8 yd of Stalvan, between him and the door;
- only Stalvan's template has an aggro line;
- the camp numbers: three camps of three farmhands, and the garden's three share one camp.

The `lore_bible_dungeons` tests (ticket 01) pass with the new footprint.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `python3.15 -I tools/twilights-eve/ground_check.py` flags nothing; `icon_check.py` is unchanged.

## Assumptions allowed

- Any position moved up to 5 yd (20 for the Portal) to stand on walkable ground clear of fences
  and the house's walls, each recorded.
- The two ghost displays, picked from the dump and named in the commit.
