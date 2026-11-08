Title: Raven Hill Deeps: Felstrom, the dig face and the Grey Deacon (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 4, 6, 13 and 14. Blocked by 02-frozen-until (the `frozen_until` key;
it lands after 01, so the templates in the dungeon file and the Raven Hill spell section are
there too) and RD:04-dungeon-levels-whelps (it sets this file's `levels` to 40-44 and edits its
Phase Tester row).

Fill `crates/twilight-realm/content/raven_hill_deeps.toml` (the shell of cd940cee); its Phase
Tester goes. A run of about 15 minutes, three encounters. Rung 5's second half lands here: the
Grey Robes dig the seam under the cemetery, break into an old pocket of grey, and the dead in it
are not gone, they stand still. The dead here speak (aggro lines), unlike Mistmantle. The Grey
Robes' master shows only as a crowned seal and "the Master" (decision 14). No text names Sareth
or Mourncrown, or says the Lich King is frozen.

## The building and the route

The file keeps its building frame and footprint boxes. Positions are world coordinates; every
row is interior, `grounded = false`, with the floor's Z from
`python3.15 tools/twilights-eve/wmo_floor.py <x> <y>`. The crypt runs about 260 yd under the
cemetery; this run uses its first half, at about z 25, and the mist wall closes the rest.

1. The Portal on the mausoleum floor, (-10418.6, 410.8), and the stairs down (group 3).
2. The upper vaults (group 12): floors z 29.08 at (-10380, 400) and z 24.39 at (-10399, 347),
   vanilla Felstrom's spot.
3. The hall (group 1), upper floor z 25.43 at (-10324, 448).
4. Vault01 (group 2), upper floor z 25.25 at (-10265, 385).
5. The seam vault, vault06 (group 4), floor z 20.87, about x -10230 to -10190, y 305 to 340.
   Its floor ends near y 302, where the way down to XBox (group 9, z 17.23 at (-10218, 295))
   begins.

The WMO's portals were not read. If `wmo_floor.py` or the client's collision shows that vault01
does not lead to vault06 at these floors, put encounter 3 in the room the route reaches next,
move the mist wall to that room's far side, and record it.

## Templates

In `raven_hill_deeps.toml` itself (ticket 01), entries 97260 to 97266, in `mobs.toml`'s form.
Displays and factions from the vmangos dump. Multipliers 1.0 unless said.

| Entry | Name | Type, rank | Class | Levels | Display, faction | Spells | Aggro line |
|---|---|---|---|---|---|---|---|
| 97260 | Skeletal Warder | 6, normal | warrior | 40-41 | 612 (Skeletal Warder, 785), 21 | melee | "You walk where we were buried." (30%) |
| 97261 | Grey Robe Gravecaller | 7, normal | mage | 41 | 2492 (the arc's Grey Robes), 14 | `shadow_bolt_creature` | "This ground is the Master's." (50%) |
| 97262 | Commander Felstrom | 6, elite | warrior | 42 | 7848 (Commander Felstrom, 771), 21 | `terrify_creature`, `last_stand_felstrom` | "I held this stair for the king. I hold it still." (100%) |
| 97263 | Ghoul Digger | 6, normal | warrior | 41-42 | 646 (Ghoul, 846), 21 | melee | "Dig... deeper..." (30%) |
| 97264 | The Dig Overseer | 7, elite | mage | 43 | a stock robed necromancer display from the dump, not 4272 (Morbent Fel's) or 2492, 14 | `feast_overseer`, `blight_overseer`, `shadow_bolt_creature` | "Back to the face! The seam won't open itself." (100%) |
| 97265 | The Grey Deacon | 7, elite, `boss = true` | mage | 44 | a stock robed priest display from the dump (a Scarlet or Cathedral priest's), 14 | `crowned_shadow_deacon`, `bone_armor_deacon`, `shadow_bolt_creature` | "What stands in the grey belongs to the Master." (100%) |
| 97266 | Stilled Dead | 6, normal | warrior | 43 | 7550 (Skeletal Fiend, 531), 21 | melee | none |

"The Dig Overseer" and "The Grey Deacon" are placeholder names. The arc's quest text calls the
boss "a man in a deacon's robe"; any name fits it.

## Spells

In `creature_spells.toml`, under the Raven Hill section's first row (`crowned_shadow_deacon`,
ticket 01) and above the Stonewatch section's header. Placeholders.

- 1900081 `bone_armor_deacon`, "Bone Armor": `absorb { amount = 130, duration_ms = 10000 }`, 1 s
  cast, every 40 s, range 0, visual 17.
- 1900082 `last_stand_felstrom`, "Last Stand": `stance { damage_dealt_pct = 30,
  damage_taken_pct = -30, duration_ms = 0 }`, no cast, cooldown 60000, range 0,
  `below_health_pct = 50`, Shield Wall's look (visual 871).
- 1900083 `feast_overseer`, "Feast": `heal { pct_max_health = 15 }`, 2 s cast, every 15 s,
  `corpse_within = 10.0` (the form of `creature_spells.toml` line 227), the Abbot's heal look
  (visual 2050).
- 1900084 `blight_overseer`, "Blight": `ground_patch { radius = 5.0, duration_ms = 10000,
  tick_ms = 1000, tick = { base = 11 } }`, shadow, `target = "random"`, 1.5 s cast, every 18 s,
  range 30, Flame Patch's look (visual 2120).
- Reused: `terrify_creature` and `shadow_bolt_creature`.

Regenerate the patch (`build_spells.py --data "$WOW_DATA"`) and run the pipeline unittests.

## Creatures

Guids 974201 up, unique across every content file. 19 in all, 6 of them frozen until the last
fight.

Encounter 1, the upper vaults:
- Two Skeletal Warders and a Grey Robe Gravecaller, camp 1, about (-10380, 400, 29.08), guids
  974201 to 974203.
- Commander Felstrom, guid 974204, at vanilla's spot (-10399, 347, 24.39), with two Skeletal
  Warders (level 41), guids 974205 and 974206, camp 2.

Encounter 2, the dig face:
- Three Ghoul Diggers, camp 3, in the hall about (-10335, 440, 25.43), guids 974207 to 974209.
- The Dig Overseer, guid 974210, in vault01 about (-10265, 385, 25.25), with two Ghoul Diggers,
  guids 974211 and 974212, camp 4. His timed call: `every = [{ every_s = 20, entry = 97263,
  level = 42, count = 2, position = [-10255.0, 372.0, 25.25], until_used = 983051 }]`, ghouls out
  of the side tunnel until the shoring lever drops it.

Encounter 3, the grey pocket:
- The Grey Deacon, guid 974213, about (-10207, 325, 20.87), facing the wall (orientation about
  4.7), there from the Run's start. Leave his `drop` empty: the arc's ticket 28 adds the Crowned
  Orders (980486) and page two (980487) to it.
- Six Stilled Dead along the wall, half in the mist, guids 974214 to 974219, about (-10222, 309),
  (-10214, 307), (-10206, 306), (-10198, 307), (-10192, 310) and (-10228, 312), all z 20.87, each
  with `frozen_until = { guid = 974213, pct = <mark> }`: the first two at 75, the next two at 50,
  the last two at 25 (ticket 02; the marker is Frozen in the Mist).

## GameObjects, cover and the mist wall

Guids 983051 up. Displays are stock.

| Guid | What | Display | Where | Rules |
|---|---|---|---|---|
| 983051 | The shoring lever | 295 (`G_LeverMetal`) | in the hall, about (-10322, 446, 25.43) | `use = "lever"`, `used_text = "The props give way. Rock fills the side tunnel."` |
| 983052 | The fallen side tunnel | 2230 (`G_CaveIn`) | across the side tunnel's mouth in vault01, about (-10255, 372, 25.25) | `after_used = 983051` |
| 983053 to 983055 | Rubble heaps from the dig | 2230 | vault06, about (-10215, 335), (-10198, 330) and (-10226, 325), z 20.87 | each with a `[[cover]]` of radius 2.5, height 6.0 |

The mist wall, a new row in `data/twilights-eve/mist_walls.json` (ADR 0033, `deeprun_tunnel`'s
form):

```json
{"key": "raven_hill_grey", "map": 0, "from": [-10228.0, 302.0], "to": [-10192.0, 302.0], "bottom": 15.0, "top": 35.0, "floor": 20.87, "open_side": [-10207.0, 325.0]}
```

The wall spans vault06's far end where its floor stops; move its ends so it meets the room's
walls (check points along y 300 to 306 with `wmo_floor.py`), and record it. The realm refuses a
step across it (`src/world/closed.rs`) and the client draws it (`benilla-formats`'s
`mist_walls.rs`), with no code change.

## Loot

Commander Felstrom and the Dig Overseer `rolls = [{ chance = 50, one_of = [...] }]`, the Grey
Deacon `chance = 100`. Until ticket 09 lands, the list is the Stockade quiet wing's twenty
(905080 to 905099) as a stand-in.

## Failing tests first

New `crates/twilight-realm/tests/raven_hill_deeps.rs`, with the layout checks
`tests/stockade.rs` makes and the harness of `tests/phased_wards.rs`:

- every creature and GameObject lies inside the footprint; no creature has entry 97100;
- exactly one Boss Bar creature, the Grey Deacon; the overseer's call names lever 983051, and
  983052 waits on it;
- the six Stilled Dead name the Deacon in `frozen_until`, two each at 75, 50 and 25;
- the three cover pillars stand within 25 yd of the Deacon, on his side of the mist wall;
- the mist wall `raven_hill_grey` loads, its open side lies inside the footprint, and a step
  from the open side across it is refused (the check `tests/deeprun_tram.rs` makes of
  `deeprun_tunnel`);
- in a Run, bringing the Deacon to 70% wakes exactly two Stilled Dead.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `python3.15 -I tools/twilights-eve/ground_check.py` flags nothing; `icon_check.py` is unchanged.

## Assumptions allowed

- Any position moved up to 3 yd to stand on the floor named, each recorded; a floor level
  swapped for the other one `wmo_floor.py` lists at that point where the route clearly uses it.
- The two stock displays, picked from the dump and named in the commit.
