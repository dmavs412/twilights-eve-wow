# The Barrens spawns (Twilight's Eve, ticket #7)

Two data files feed the Local Realm's Hunting Ground in The Barrens, and the realm reads its own
copy of them:

| file | what | made by |
|---|---|---|
| `data/twilights-eve/barrens_spawns.json` | every hostile and neutral creature spawn in The Barrens (map 1, zone 17), with camps | `tools/twilights-eve/import_barrens_spawns.py` |
| `data/twilights-eve/crossroads_low_camps.json` | six hand-placed level 1–6 camps near the Crossroads | `tools/twilights-eve/make_crossroads_low_camps.py` |
| `crates/twilight-realm/content/barrens.toml` | the two above as the realm reads them: templates, spawn rows, camps, suppressed spawns | `cargo run -p twilight-realm --example import_barrens` |

All three are committed. Building the game never needs the database or the scripts. The Python
scripts use only the standard library, plus `tools/twilights-eve/wowmpq.py`, a small read-only
MPQ/ADT/DBC reader written for them. The realm cannot read JSON (the shipped app keeps
`serde_json` out), so `examples/import_barrens.rs` converts the dumps to TOML;
`tests/content_barrens.rs` fails when the committed `barrens.toml` no longer matches them.

## Source

- **Database:** vmangos/core, release `db_latest` ("Development Database Snapshot (2026-09-28)"),
  asset `db-sqlite-4641790.zip` (vmangos commit `4641790`), file `sqlite-dump/mangos.sqlite`.
  Its latest world migration is `20260913120518`. vmangos ships the world DB as a SQLite file
  in this release, so no MySQL is needed.
- **Patch:** 1.12 (vmangos patch index 10). Spawns must have `patch_min <= 10 <= patch_max`, and
  each `creature_template` entry takes its newest row with `patch <= 10`.
- **Game data:** the WoW 1.12.1 MPQs (`terrain.MPQ`, `dbc.MPQ`, `wmo.MPQ`, `patch*.MPQ`) for ADT
  area IDs and heights, `AreaTable.dbc` and `WMOAreaTable.dbc`.

## Re-running

```bash
# 1. Fetch the DB snapshot (about 42 MB zipped, 146 MB unzipped). Check the asset name first:
gh release view db_latest -R vmangos/core --json assets --jq '.assets[].name'
mkdir -p "$TMP/vmangos" && cd "$TMP/vmangos"
curl -LO https://github.com/vmangos/core/releases/download/db_latest/db-sqlite-4641790.zip
unzip db-sqlite-4641790.zip sqlite-dump/mangos.sqlite

# 2. From the repo root. -I keeps the downloaded directory out of Python's import path.
python -I tools/twilights-eve/import_barrens_spawns.py \
    --db "$TMP/vmangos/sqlite-dump/mangos.sqlite" --data /path/to/WoW/Data \
    --source-asset db-sqlite-4641790.zip
python -I tools/twilights-eve/make_crossroads_low_camps.py \
    --db "$TMP/vmangos/sqlite-dump/mangos.sqlite" --data /path/to/WoW/Data

# 3. The realm's copy (no Python, no DB: it reads the two JSON files above).
cargo run -p twilight-realm --example import_barrens
cargo test -p twilight-realm
```

Step 3 alone is enough after editing either JSON file by hand. A creature added to the low
camps needs its template written in `crates/twilight-realm/content/mobs.toml` (the dump has no
stats for Mulgore's and Durotar's creatures); `tests/content_barrens.rs` names any spawned entry
without one.

`--data` defaults to `$WOW_DATA`. Run the importer first: the low-camp script reads
`barrens_spawns.json` to list the imported spawns it overlaps. Both scripts are deterministic;
the same DB and MPQs give byte-identical output. The importer takes about 4 s.

On this machine Python is not installed. The embeddable CPython 3.12 zip from python.org,
unpacked to a temp folder, runs both scripts (`python.exe -I ...`); it includes `sqlite3`.

Importer options: `--camp-link` (default 25 yd), `--camp-radius` (default 30 yd),
`--include-faction-aligned` (see Filters), `--out`, `--source-tag`, `--source-asset`.

## Which spawns are in The Barrens

vmangos' `creature` table has no zone column, so membership is decided from the game data,
the way the client and server decide it:

1. A coarse box on map 1 (x −5200..2200, y −5200..400) narrows the rows; it is larger than
   The Barrens on every side.
2. The spawn's area is the area ID of the ADT terrain chunk (MCNK, 33⅓ yd square) under it.
3. If the spawn is more than 3 yd below the terrain surface, it is inside a cave or mine, where
   the game takes the area from the WMO, not the terrain above. The importer then finds the
   smallest WMO placement (MODF) in that ADT whose bounding box holds the spawn and takes the
   area `WMOAreaTable.dbc` gives that WMO's root. A WMO area on another map is ignored: the
   Wailing Caverns cave is shared with the instance (map 43), so its outer cave stays Barrens.
4. The spawn is kept when that area, or its `AreaTable.dbc` parent, is 17 (The Barrens).

Step 3 matters. Without it, Mulgore's Venture Co. Mine (70 spawns), Ashenvale's Demon Fall
Ridge (Diathorus, 20), Dustwallow's Darkmist Cavern (14) and Thousand Needles' Splithoof Hold
(14) all lie under Barrens terrain and were counted as Barrens. `counts.wmo_area_lookups` in
the output lists every WMO decision that moved a spawn into or out of the zone. 19 kept spawns
took their area from a WMO (Boulder Lode Mine, Bael'dun Keep, Dreadmist Den).

Limits: the WMO test uses the placement's axis-aligned bounds, not group geometry, so a spawn
under the terrain inside two overlapping WMOs takes the smaller one. Spawns above ground inside
buildings keep the terrain area. Neither case changes the zone in The Barrens today.

## Filters

A spawn is dropped when any of these hold (counts from the current run):

| reason | dropped | examples |
|---|---|---|
| critter, totem or non-combat pet (`type` 8, 11, 12) | 243 | Adder, Gazelle, Prairie Dog, Swine |
| friendly to Horde or Alliance players | 239 | Theramore Marine, Bael'dun Soldier, Barrens Guard, Peon |
| any `npc_flags` (vendor, trainer, quest giver, flight master, innkeeper, gossip…) | 151 | |
| guard (`flags_extra` 0x400) | 55 | |
| game-event-only spawn (`game_event_creature`) | 46 | holiday spawns |
| invisible or unselectable (`flags_extra` 0x80, `static_flags1` 0x200) | 5 | triggers |
| quest starter or ender (`creature_questrelation` / `involvedrelation`) | 0 | (all already have npc flags) |
| spawn disabled or not visible (`spawn_flags` 0x02, 0x40) | 0 | |

`dropped_by_name` in the output lists every dropped name per reason.

**Reaction** follows vmangos' `FactionTemplateEntry::IsHostileTo` / `IsFriendlyTo`, evaluated
against the faction templates of all eight player races (orc 2, undead 5, tauren 6, troll 116;
human 1, dwarf 3, night elf 4, gnome 115), plus the faction's base reputation for that race
(≤ −3000 hostile, ≥ 3000 friendly). Each spawn records `reaction_horde` and `reaction_alliance`;
`reaction` is `hostile` if either side is hostile, else `neutral`.

The Alliance mobs of The Barrens (Northwatch Hold's Theramore Marines, Bael'dun's dwarves) are
friendly to Alliance players, so they are dropped: Twilight's Eve lets every race play from a
Horde Hub. `--include-faction-aligned` keeps the ones that are hostile to one side and friendly
to the other; their `reaction_horde` / `reaction_alliance` then say which side they serve.

## Output: `barrens_spawns.json`

Top-level keys: `source`, `method` (the rules above, as data), `counts`, `dropped_by_name`,
`templates` (per creature entry: subname, rank, type, displays with scale and probability,
walk/run speed, detection, call-for-help and leash ranges, health/damage/armour multipliers,
attack time, loot id, gold), `camps`, and `spawns`, one record per line.

Each spawn has: `guid`, `entry`, `alt_entries` (vmangos `id2..id5` random alternates; empty in
The Barrens today), `name`, `x`/`y`/`z`/`orientation`, `display_id` (the template's most likely
display; `templates[entry].displays` has all of them), `level_min`/`level_max`,
`faction_template`, `reaction`, `reaction_horde`, `reaction_alliance`, `wander_radius`,
`movement_type` (0 idle, 1 random wander, 2 waypoints), `has_waypoints` (waypoints are in
vmangos' `creature_movement`, not exported), `respawn_secs_min`/`max` (vanilla's; the spec
overrides it with about 60 s), `area_id`, `area_name`, `area_source` (`terrain` or `wmo`),
`pool` (null throughout The Barrens) and `camp_id`.

Coordinates are vmangos server coordinates on map 1: +x north, +y west, z up, orientation in
radians from +x toward +y. vmangos spawn Z sits on the ground: the median of spawn Z minus ADT
terrain height is 0.08 yd; the spawns far below the terrain are in caves.

### Camps

Social aggro pulls a camp, so every spawn has a `camp_id`. A camp is built in two steps:

1. Single-linkage clustering of spawns that share a **faction template and creature type**,
   linking spawns within 25 yd in the plane and 15 yd in height. vmangos' call for help only
   reaches creatures with the same faction template (`Creature::CanBeTargetedByCallForHelp`);
   the type keeps, for example, a hyena and a plainstrider of one neutral template apart.
2. Single linkage chains whole mines and forts into one 30+ mob camp, so any cluster with a
   radius above 30 yd is split by recursive 2-means until each part is within 30 yd.

Each camp records `faction_template`, `creature_type`, `assists` (the template's
RESPOND_TO_CALL_FOR_HELP flag, which vmangos requires for a creature to answer a call for
help), `names`, `size`, levels, `center`, `radius`, `area_name` and member `guids`. Camp ids are
numbered by their lowest member guid, so they are stable across runs of the same DB.

Result: 2146 camps, 323 of them with more than one member (sizes up to 12). Most plains beasts
are single-spawn camps, as they are in the game. The realm decides whether a non-assisting camp
(`assists: false`) still pulls together; the ticket says pulling one mob pulls its camp.

## Summary

2819 spawns of 141 creature entries: 2184 hostile, 635 neutral.

| level band | spawns |
|---|---|
| 1–9 | 33 |
| 10–14 | 1228 |
| 15–19 | 1069 |
| 20–24 | 370 |
| 25–29 | 15 |
| 30–39 | 104 (the Razorfen Downs approach elites, and Jorb) |

Movement: 2319 wander randomly, 386 stand idle, 114 walk waypoints. Vanilla respawn times are
mostly 413 s (1778 spawns) and 300 s (784).

By subzone (median spawn position, vmangos coordinates; the Crossroads is at −441, −2596):

| subzone | spawns | median x, y | levels | who |
|---|---|---|---|---|
| The Barrens (open plains) | 1231 | −520, −2642 | 4–25 | plainstriders, zhevras, lions, hyenas, raptors, giraffes, kodos, roaming Kolkar |
| Southern Barrens | 241 | −2621, −2236 | 16–26 | thunderheads, stormhides, hecklefang stalkers, zhevra coursers |
| Agama'gor | 163 | −1855, −1849 | 16–22 | Bristleback quilboar, stormsnouts |
| The Dry Hills | 130 | 550, −1394 | 13–20 | Witchwing harpies |
| Thorn Hill | 123 | −127, −3177 | 10–18 | Razormane quilboar |
| Razorfen Downs (approach) | 103 | −4554, −2237 | 33–36 | Razorfen elites, Death's Head |
| Field of Giants | 96 | −3174, −1943 | 18–27 | silithid |
| Blackthorn Ridge | 94 | −3864, −1727 | 20–26 | Razormane pathfinders, seers, warfrenzies |
| Bramblescar | 91 | −2071, −2612 | 16–22 | Bristleback quilboar |
| Lushwater Oasis | 80 | −1059, −2114 | 12–20 | Kolkar, oasis snapjaws |
| The Merchant Coast | 70 | −1511, −3854 | 12–19 | Southsea pirates, makrura |
| The Stagnant Oasis | 68 | −1257, −2960 | 13–17 | Kolkar, snapjaws, hyenas |
| The Sludge Fen | 67 | 1050, −3082 | 13–19 | Venture Co. |
| Boulder Lode Mine | 52 | 1355, −3513 | 16–19 | Venture Co. |
| Dreadmist Peak | 46 | 331, −2211 | 10–13 | Burning Blade |
| The Forgotten Pools | 35 | 89, −1912 | 12–16 | Kolkar |
| Southfury River | 31 | 214, −3782 | 9–16 | Dreadmaw crocolisks |
| others (Honor's Stand, Grol'dom Farm, Raptor Grounds, Northwatch Hold, Gold Road, Fray Island, Razorfen Kraul, Dreadmist Den, Ratchet, Shrine of the Fallen Warrior, Camp Taurajo, Mor'shan Rampart) | 98 | | | |

The Wailing Caverns' outer cave (Deviate elites, Devouring Ectoplasm, Mad Magglish, Trigore,
Boahn) is in "The Barrens" area, since its WMO area is on the instance map.

Every creature in the file, by level:

| Creature | Entry | Level | Reaction | Spawns | Main areas |
|---|---|---|---|---|---|
| Savannah Cub | 5766 | 4-5 | hostile | 9 | The Barrens |
| Dreadmaw Crocolisk | 3110 | 9-11 | hostile | 24 | Southfury River |
| Burning Blade Bruiser | 3379 | 10-11 | hostile | 26 | Dreadmist Peak, The Barrens |
| Razormane Thornweaver | 3268 | 10-11 | hostile | 8 | Thorn Hill |
| Razormane Water Seeker | 3267 | 10-11 | hostile | 23 | Thorn Hill |
| Burning Blade Acolyte | 3380 | 11-12 | hostile | 19 | Dreadmist Peak, The Barrens |
| Corrupted Dreadmaw Crocolisk | 3231 | 11-12 | hostile | 1 | Southfury River |
| Greater Plainstrider | 3244 | 11-12 | neutral | 93 | The Barrens, Thorn Hill |
| Razormane Hunter | 3265 | 11-12 | hostile | 9 | Thorn Hill |
| Savannah Huntress | 3415 | 11-12 | hostile | 47 | The Barrens, Thorn Hill |
| Sunscale Lashtail | 3254 | 11-13 | hostile | 9 | The Barrens, Thorn Hill |
| Fleeting Plainstrider | 3246 | 12-13 | neutral | 145 | The Barrens, Dreadmist Peak |
| Kolkar Wrangler | 3272 | 12-13 | hostile | 70 | The Barrens, Lushwater Oasis |
| Razormane Defender | 3266 | 12-13 | hostile | 23 | Thorn Hill |
| Razormane Geomancer | 3269 | 12-13 | hostile | 16 | Thorn Hill |
| Sand Shark (elite) | 5435 | 12-13 | hostile | 1 | The Merchant Coast |
| Savannah Highmane | 3243 | 12-13 | hostile | 6 | The Barrens |
| Southsea Brigand | 3381 | 12-13 | hostile | 24 | The Merchant Coast |
| Dishu (rare) | 5865 | 13 | hostile | 1 | The Barrens |
| Kolkar Packhound | 4316 | 13 | hostile | 13 | The Barrens, The Stagnant Oasis |
| Kolkar Stormer | 3273 | 13-14 | hostile | 54 | The Barrens, Lushwater Oasis |
| Razormane Mystic | 3271 | 13-14 | hostile | 6 | Thorn Hill |
| Southsea Cannoneer | 3382 | 13-14 | hostile | 19 | The Merchant Coast |
| Sunscale Screecher | 3255 | 13-15 | hostile | 169 | The Barrens, The Stagnant Oasis |
| Tazan | 6494 | 13 | hostile | 1 | The Merchant Coast |
| Venture Co. Peon | 3285 | 13-14 | hostile | 15 | The Barrens |
| Zhevra Runner | 3242 | 13-14 | neutral | 148 | The Barrens, Grol'dom Farm |
| Kolkar Bloodcharger | 3397 | 14-15 | hostile | 18 | The Barrens, The Stagnant Oasis |
| Kolkar Pack Runner | 3274 | 14-15 | hostile | 12 | The Barrens, The Stagnant Oasis |
| Lost Barrens Kodo | 3234 | 14-15 | neutral | 27 | The Barrens, The Stagnant Oasis |
| Muck Frenzy | 8236 | 14-15 | hostile | 5 | Fray Island |
| Savannah Prowler | 3425 | 14-15 | hostile | 103 | The Barrens, The Dry Hills |
| Southsea Cutthroat | 3383 | 14-15 | hostile | 22 | The Barrens |
| Southsea Privateer | 3384 | 14-15 | hostile | 17 | The Barrens |
| Venture Co. Drudger | 3284 | 14-15 | hostile | 51 | The Sludge Fen, The Barrens |
| Witchwing Harpy | 3276 | 14-15 | hostile | 27 | The Dry Hills |
| Barrens Giraffe | 3248 | 15-16 | neutral | 75 | The Barrens, The Stagnant Oasis |
| Deviate Coiler (elite) | 3630 | 15-16 | hostile | 4 | The Barrens |
| Deviate Creeper (elite) | 3632 | 15-16 | hostile | 12 | The Barrens |
| Deviate Stalker (elite) | 3634 | 15-17 | hostile | 4 | The Barrens |
| Elder Mystic Razorsnout (rare elite) | 3270 | 15 | hostile | 1 | Thorn Hill |
| Hecklefang Hyena | 4127 | 15-16 | hostile | 108 | The Barrens, The Stagnant Oasis |
| Kolkar Marauder | 3275 | 15-16 | hostile | 14 | The Barrens, The Stagnant Oasis |
| Kreenig Snarlsnout | 3438 | 15 | hostile | 1 | Thorn Hill |
| Oasis Snapjaw | 3461 | 15-16 | hostile | 37 | Lushwater Oasis, The Stagnant Oasis |
| Rathorian (rare) | 3470 | 15 | hostile | 1 | Dreadmist Den |
| Savannah Patriarch | 3241 | 15-16 | hostile | 7 | The Barrens |
| Stonearm (rare) | 5837 | 15 | hostile | 1 | The Barrens |
| Venture Co. Mercenary | 3282 | 15-16 | hostile | 15 | The Sludge Fen |
| Witchwing Roguefeather | 3277 | 15-16 | hostile | 18 | The Dry Hills |
| Barak Kodobane | 3394 | 16 | hostile | 1 | The Barrens |
| Baron Longshore | 3467 | 16 | hostile | 3 | The Merchant Coast |
| Bristleback Water Seeker | 3260 | 16-17 | hostile | 32 | Agama'gor, Bramblescar |
| Deviate Lurker (elite) | 3641 | 16-17 | hostile | 8 | The Barrens |
| Deviate Slayer (elite) | 3633 | 16-17 | hostile | 14 | The Barrens |
| Deviate Stinglash (elite) | 3631 | 16-17 | hostile | 7 | The Barrens |
| Devouring Ectoplasm (elite) | 3638 | 16-17 | hostile | 10 | The Barrens |
| Ornery Plainstrider | 3245 | 16-17 | hostile | 89 | The Barrens, Southern Barrens |
| Overseer Glibby | 6606 | 16 | hostile | 1 | The Sludge Fen |
| Sunscale Scytheclaw | 3256 | 16-18 | hostile | 89 | The Barrens, Raptor Grounds |
| Tinkerer Sniggles | 3471 | 16 | hostile | 1 | The Barrens |
| Venture Co. Enforcer | 3283 | 16-17 | hostile | 29 | Boulder Lode Mine |
| Witchwing Slayer | 3278 | 16-17 | hostile | 48 | The Dry Hills |
| Bristleback Hunter | 3258 | 17-18 | hostile | 60 | Agama'gor, Bramblescar |
| Brokespear (rare) | 5838 | 17 | hostile | 1 | The Stagnant Oasis |
| Rocklance (rare elite) | 5841 | 17 | hostile | 1 | The Stagnant Oasis |
| Savannah Matriarch | 3416 | 17-18 | hostile | 10 | The Barrens |
| Snort the Heckler (rare) | 5829 | 17 | hostile | 1 | The Barrens |
| Venture Co. Overseer | 3286 | 17-18 | hostile | 22 | Boulder Lode Mine |
| Witchwing Ambusher | 3279 | 17-18 | hostile | 2 | The Dry Hills |
| Witchwing Windcaller | 3280 | 17-18 | hostile | 23 | The Dry Hills |
| Zhevra Charger | 3426 | 17-18 | neutral | 58 | The Barrens, Southern Barrens |
| Bristleback Thornweaver | 3261 | 18-19 | hostile | 29 | Bramblescar, Agama'gor |
| Hecklefang Snarler | 4129 | 18-19 | hostile | 28 | Agama'gor, The Barrens |
| Mad Magglish (elite) | 3655 | 18 | hostile | 1 | The Barrens |
| Silithid Protector | 3503 | 18-19 | hostile | 1 | Field of Giants |
| Slimeshell Makrura | 6020 | 18-19 | hostile | 26 | The Merchant Coast, Northwatch Hold |
| Stormsnout | 3240 | 18-19 | hostile | 70 | Agama'gor, Southern Barrens |
| Supervisor Lugwizzle | 3445 | 18 | hostile | 1 | The Sludge Fen |
| Thunderhawk Hatchling | 3247 | 18-20 | hostile | 14 | Agama'gor, Southern Barrens |
| Wandering Barrens Giraffe | 3463 | 18-19 | neutral | 41 | Agama'gor, Bramblescar |
| Barrens Kodo | 3236 | 19-20 | neutral | 6 | Southern Barrens |
| Boss Copperplug | 9336 | 19 | hostile | 1 | Boulder Lode Mine |
| Bristleback Geomancer | 3263 | 19-20 | hostile | 34 | Bramblescar, Agama'gor |
| Engineer Whirleygig (rare) | 5836 | 19 | hostile | 1 | The Sludge Fen |
| Foreman Grills (rare) | 5835 | 19 | hostile | 1 | The Sludge Fen |
| Geopriest Gukk'rok (rare) | 5863 | 19 | hostile | 2 | Agama'gor |
| Hezrul Bloodmark | 3396 | 19 | hostile | 1 | Lushwater Oasis |
| Sister Rathtalon (rare elite) | 5830 | 19 | hostile | 1 | The Dry Hills |
| Sludge Beast (rare) | 3295 | 19 | hostile | 1 | The Sludge Fen |
| Takk the Leaper (rare elite) | 5842 | 19 | hostile | 1 | The Barrens |
| Trigore the Lasher (rare elite) | 3652 | 19 | hostile | 1 | The Barrens |
| Boahn (rare elite) | 3672 | 20 | hostile | 1 | The Barrens |
| Gesharahan (rare elite) | 3398 | 20 | hostile | 1 | Lushwater Oasis |
| Razormane Pathfinder | 3456 | 20-21 | hostile | 13 | Blackthorn Ridge |
| Serena Bloodfeather | 3452 | 20 | hostile | 1 | The Dry Hills |
| Silithid Creeper | 3250 | 20-21 | hostile | 25 | Field of Giants |
| Silithid Grub | 3251 | 20 | hostile | 24 | Field of Giants |
| Thunderhawk Cloudscraper | 3424 | 20-22 | hostile | 6 | Southern Barrens |
| Thunderhead | 3239 | 20-21 | hostile | 51 | Southern Barrens, Field of Giants |
| Zhevra Courser | 3466 | 20-21 | neutral | 33 | Southern Barrens, Field of Giants |
| Ashenvale Bear | 3809 | 21-22 | hostile | 1 | The Mor'shan Rampart |
| Kuz | 3436 | 21 | hostile | 1 | Blackthorn Ridge |
| Silithid Swarmer | 3252 | 21-22 | hostile | 28 | Field of Giants |
| Swiftmane (rare elite) | 5831 | 21 | neutral | 1 | The Barrens |
| Foreman Silixiz | 7287 | 22 | hostile | 1 | The Barrens |
| Hecklefang Stalker | 4128 | 22-23 | hostile | 38 | Southern Barrens, Field of Giants |
| Lakota'mani | 3474 | 22 | neutral | 4 | Agama'gor, Bramblescar |
| Razormane Stalker | 3457 | 22-23 | hostile | 23 | Blackthorn Ridge |
| Stormhide | 3238 | 22-23 | hostile | 41 | Southern Barrens, Field of Giants |
| Swinegart Spearhide (rare elite) | 5864 | 22 | hostile | 1 | Agama'gor |
| Venture Co. Drone | 7067 | 22 | hostile | 2 | The Barrens |
| Venture Co. Patroller | 7308 | 22 | hostile | 2 | The Barrens |
| Grand Foreman Puzik Gallywix (elite) | 7288 | 23 | hostile | 1 | The Barrens |
| Greater Thunderhawk | 3249 | 23-24 | hostile | 16 | Southern Barrens, Blackthorn Ridge |
| Humar the Pridelord (rare elite) | 5828 | 23 | hostile | 1 | The Barrens |
| Nak | 3434 | 23 | hostile | 1 | Blackthorn Ridge |
| Razorfen Servitor (elite) | 6132 | 23-24 | hostile | 4 | Razorfen Kraul |
| Razormane Seer | 3458 | 23-24 | hostile | 22 | Blackthorn Ridge |
| Venture Co. Lookout | 7307 | 23 | hostile | 3 | The Barrens |
| Greater Barrens Kodo | 3235 | 24-25 | neutral | 1 | Southern Barrens |
| Owatanka | 3473 | 24 | hostile | 4 | Southern Barrens |
| Razormane Warfrenzy | 3459 | 24-25 | hostile | 17 | Blackthorn Ridge |
| Silithid Harvester (rare) | 3253 | 24 | hostile | 1 | Field of Giants |
| Thunderstomp (rare) | 5832 | 24 | hostile | 1 | Southern Barrens |
| Azzere the Skyblade (rare) | 5834 | 25 | hostile | 1 | Southern Barrens |
| Lok Orcbane | 3435 | 25 | hostile | 1 | Blackthorn Ridge |
| Mutated Venture Co. Drone | 7310 | 25 | hostile | 2 | The Barrens |
| Washte Pawne | 3472 | 25 | hostile | 4 | Blackthorn Ridge, Southern Barrens |
| Wooly Kodo | 3237 | 25-26 | neutral | 1 | Southern Barrens |
| Hagg Taurenbane (rare elite) | 5859 | 26 | hostile | 4 | Blackthorn Ridge |
| Brontus (rare elite) | 5827 | 27 | neutral | 1 | Field of Giants |
| Isha Awak | 3476 | 27 | hostile | 1 | Northwatch Hold |
| Death's Head Cultist (elite) | 7872 | 33-34 | hostile | 14 | Razorfen Downs |
| Razorfen Battleguard (elite) | 7873 | 33-34 | hostile | 49 | Razorfen Downs |
| Razorfen Thornweaver (elite) | 7874 | 33-34 | hostile | 24 | Razorfen Downs |
| Withered Warrior (elite) | 7327 | 34-35 | hostile | 6 | Razorfen Downs |
| Death's Head Geomancer (elite) | 7335 | 35-36 | hostile | 7 | Razorfen Downs |
| Ambassador Bloodrage (rare elite) | 7895 | 36 | hostile | 2 | Razorfen Downs |
| Ambassador Malcin (elite) | 12865 | 36 | hostile | 1 | Razorfen Downs |
| Jorb | 3659 | 37 | neutral | 1 | Ratchet |

## Proposed level 1–6 camps near the Crossroads

The Barrens starts at level 10 (nine Savannah Cubs at 4–5 and the Southfury crocolisks at 9
are the only exceptions), so a Basic Warrior at level 1 has nothing to fight there.
`crossroads_low_camps.json` proposes six camps of real Mulgore and Durotar creatures, placed on
open Barrens ground 200–350 yd from the Crossroads and further out as the levels rise.

| id | camp | creatures (entry, display) | level | reaction | members | centre x, y | yd from town | direction |
|---|---|---|---|---|---|---|---|---|
| low-1 | plainstriders | Plainstrider (2955, 1219) | 1–2 | neutral | 5 | −614, −2496 | 200 | south-west |
| low-2 | mottled_boars | Mottled Boar (3098, 503) | 1–2 | neutral | 5 | −506, −2837 | 250 | south-east |
| low-3 | scorpid_workers | Scorpid Worker (3124, 2485) | 3 | neutral | 4 | −376, −2837 | 250 | east |
| low-4 | vile_familiars | Vile Familiar (3101, 1185) | 3–4 | hostile | 5 | −731, −2518 | 300 | south |
| low-5 | bristleback_camp | Bristleback Quilboar (2952, 1967) ×3, Bristleback Shaman (2953, 6090) ×2, Bristleback Battleboar (2954, 6807) | 3–5 | hostile | 6 | −316, −2379 | 250 | north-west |
| low-6 | prairie_wolves | Prairie Wolf (2958, 1100) | 5–6 | hostile | 4 | −744, −2771 | 350 | south-east |

How the spots were chosen: a survey of rings 150–500 yd around the Crossroads, keeping points
whose terrain varies less than 6 yd across a 48 yd square, whose area is The Barrens (not
Crossroads 380, the Gold Road 458 or Thorn Hill 1699) and with no WMO overhead, then picking
spots off the roads and at least ~120 yd apart. Members sit on fixed offsets 8–15 yd from the
centre, facing it.

Each member has a placeholder `guid` from 9000001 up (outside vmangos' range; the realm may
renumber), the entry, name, levels, display ID and faction template from the vmangos DB,
`reaction`, `wander_radius`, `respawn_secs: 60` (the spec's timer), and `z_hint`. **z_hint is
only the ADT terrain height at that x/y. The host must resolve ground Z at runtime** through the
Local Realm's terrain trait (ADR 0001); the hint is for tooling and sanity checks.

These camps sit among The Barrens' own level 11–13 wanderers (Greater Plainstriders, Savannah
Huntresses, Fleeting Plainstriders). Each camp lists `suppress_guids`: the imported spawns whose
home is within 70 yd of its centre (2–6 per camp, 23 in all). The realm should not spawn those,
so a level-1 player does not meet a level-11 lion at the boars. Wanderers from further out can
still walk in.

`assists` is false for the plainstriders, boars and scorpids (neutral, and their faction
template has no call-for-help flag) and for the Vile Familiars (faction template 14 has none
either); the quilboars and wolves have it. Whether a camp pulls together is the realm's camp rule.

## How the realm plays them

- **Streaming.** Only spawns whose home is within 100 yd of the player are in the world
  (`src/population.rs`, a 64 yd grid over the homes, looked over every 8 yd moved or every
  second). Idle and dead mobs leave past 110 yd; a mob in a fight or running home stays.
- **Level and model.** Each spawn comes in at a level in its range, from a hash of its guid
  rather than the realm's dice. Its model is one of the template's displays, weighted by their
  probabilities and picked by guid at import time.
- **Aggro.** Templates are `hostile` when the dump's reaction is hostile and `no_aggro` is unset;
  only hostile mobs aggro on sight, by `rules::aggro::radius`. Neutral ones fight when attacked.
- **Camps.** Pulling a mob, by aggro or by attacking it, pulls every idle member of its camp,
  whether or not the camp's faction has the call-for-help flag (`assists`).
- **Leash.** A mob in a fight more than 50 yd from home (`rules::leash`) evades: it runs home,
  heals to full and resets.
- **Respawn.** A killed mob comes back 60 s after the player has gone more than 60 yd from its
  home (`rules::respawn`); the clock starts over whenever the player comes back. Vanilla's
  `respawn_secs` are ignored, rares included.
- **Low camps.** Their members are spawned; the imported spawns in `suppress_guids` are not.
- **Not used yet:** `wander_radius` (mobs stand at their spawn point), waypoint paths, and the
  `alt_entries`.

## Caveats

- vmangos is a reconstruction. Spawn positions and counts follow its current snapshot, not
  necessarily retail 1.12.
- `display_id` is the template's most likely display. Creatures with several models (Kolkar,
  Razormane, Venture Co.) list them all in `templates[entry].displays`; pick by probability.
- Waypoint paths (114 spawns with `movement_type` 2) are not exported, only flagged.
- The reaction model uses faction templates and base reputation only, not player reputation
  changes over time.
- Rares appear as ordinary spawns with vanilla respawn times (often 1–3 days); none are in
  spawn pools in this DB. The realm decides how often they appear.
- Some creatures in the file sit at the zone's edges because the terrain says they are in The
  Barrens: the Razorfen Downs approach (33–36 elites) and Jorb in Ratchet (a neutral level-37 bodyguard; the realm may
  want to drop him).
