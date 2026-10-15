# The Wailing Caverns: reference data for ticket #16

Offline research for the slice's one Dungeon (spec #1, ticket #16). Everything here is produced
by `tools/twilights-eve/import_wailing_caverns.py` from two sources:

- a vmangos world database (the SQLite `mangos.sqlite` of a vmangos sqlite dump), and
- the 1.12.1 client MPQs (`AreaTrigger.dbc`, `WorldSafeLocs.dbc`, `Map.dbc`,
  `LoadingScreens.dbc`, `World\Maps\WailingCaverns\WailingCaverns.wdt`).

It writes two files into `data/twilights-eve/`:

| file | what |
|---|---|
| `wailing_caverns_spawns_full.json` | all 319 creature spawns on map 43 (patch ≤ 1.12), with template, level, display IDs, faction, flags, rank, per-level health/armour/damage, waypoints, addon and `creature_groups` links |
| `wailing_caverns_run.json` | the slice's run: entrance, exit, death respawn, 10 trash mobs, the boss and the route graph |

Re-run (standard library only; run with `-I`):

```bash
python -I tools/twilights-eve/import_wailing_caverns.py --db <mangos.sqlite> --data <WoW Data dir>
```

## The map

| | |
|---|---|
| map ID | **43**, directory `WailingCaverns`, `InstanceType` 1 (party dungeon) |
| zone (AreaTable) | 718 "Wailing Caverns" |
| loading screen | `Map.dbc` `LoadingScreenID` 143 → `Interface\Glues\LoadingScreens\LoadScreenWailingCaverns.blp` |
| WDT | `MPHD` flag bit 0 set, no ADT tiles: **a WMO-only map**. Its world is one building, `world\wmo\dungeon\kl_wailing\wailingcaverns_instance.wmo`, placed by `MODF` at the origin with no rotation |

## Getting in and out

Both triggers are spheres (`radius != 0`) in `AreaTrigger.dbc`; vmangos' `areatrigger_template`
agrees with the DBC.

| | trigger | where | radius | sends you to |
|---|---|---|---|---|
| entrance | **228** "Wailing Caverns - Entrance" | map 1 (-753.596, -2212.78, 21.54) | 13 yd | map 43 (-158.441, 131.601, -74.255), o 5.847 |
| exit | **226** "Wailing Caverns - Exit" | map 43 (-172.181, 138.98, -66.647) | 12 yd | map 1 (-738.462, -2217.8, 16.919), o 6.021 |

Each destination is authored outside the other side's trigger (arrival is ~17 yd from the exit
trigger's centre, the outside drop ~17 yd from the entrance's), and the client latches a trigger
until you leave it, so a round trip never loops.

vmangos gates the entrance at **level 10** ("You must be at least level 10 to enter."). That matches
the Tier 0 cap; the realm may keep or drop it. If it refuses, send
`SessionEvent::AreaTriggerMessage { text }`.

Map 43 also has trigger **3766** (-108.563, 160.915, -79.791, r 9): the Darkmoon Faire "Fortune
Awaits" chest script. The client will report it as players walk past escort point 3; the realm
should ignore it.

### Death

The spec respawns a player who dies inside at the Dungeon entrance: use the entrance trigger's
destination, map 43 (-158.441, 131.601, -74.255), o 5.847. For reference, vmangos does it the
long way: the ghost goes to the zone's graveyard (`game_graveyard_zone` 718 → `WorldSafeLocs` **10**,
"The Barrens, The Crossroads", map 1 (-592.601, -2523.492, 91.788), facing 1.222, which is also the
Hub) and the corpse run re-enters at `map_template.ghost_entrance` map 1 (-751.131, -2209.24).

## What the realm must send for a map change

How the client handles it (read from `crates/benilla-app/src/area_trigger.rs`, `net/session.rs`,
`player/wire_in.rs`, `player/world_focus.rs`, `loading_screen.rs`):

1. **The client detects the trigger itself.** `AreaTriggerPlugin` tests the player against the
   current map's `AreaTrigger.dbc` rows each frame and sends `ClientCommand::AreaTrigger { trigger_id }`
   once per entry. vmangos re-checks with 5 yd of slop. The spec's `RealmCommand` list has no
   area-trigger command and no worldport ack: the glue needs both (`AreaTrigger(id)`,
   `WorldportAck`).
2. **Realm → `SessionEvent::TransferPending { map_id: 43, transport_entry: None }`**
   (`SMSG_TRANSFER_PENDING`). The client raises the loading screen now, with the art resolved
   from `Map.dbc` → `LoadingScreens.dbc` (LoadScreenWailingCaverns.blp), and blocks input.
3. **Realm → `SessionEvent::Worldport { map_id: 43, position, orientation, needs_ack: true }`**
   (`SMSG_NEW_WORLD`). The client despawns **every** tracked object (only a transport whose path
   touches the new map is spared), snaps the player, sets `CurrentMap`, and streams the new map.
   vmangos always pairs it with a `TransferPending` for the same map; a mismatch only warns.
4. **Client → `ClientCommand::WorldportAck`** (`MSG_MOVE_WORLDPORT_ACK`), sent once the player's
   settle hold releases, i.e. once the WMO is loaded and the body has ground. vmangos keeps the
   player out of the world until this ack, with no timeout. The realm should do the same: no
   ticks for that player, no mob aggro, until the ack.
5. **After the ack, the realm re-creates the world**: the player's own object (values update
   with a create block, as vmangos' `SendInitialPacketsAfterAddToMap` does), then every unit of
   the fresh instance copy. Nothing created before the worldport survives on the client.

Leaving is the same sequence with map 1 and the exit destination, followed by re-creating the
Barrens units around the player. The loading cover clears on the client's own readiness poll,
not on a server message.

A death respawn inside is a same-map move: `SessionEvent::Teleport` (`MSG_MOVE_TELEPORT_ACK`)
with the player's guid. A same-map snap over 100 yd also raises the loading cover
(`SNAP_LOAD_MIN_YD`); the walk from Anacondra back to the entrance is about 300 yd but the
straight line is ~175 yd, so expect a brief cover there. Fine either way.

## Spawns

319 spawns on map 43 at patch 1.12, all hostile (faction 270/14/16/7…) except the Disciple of
Naralex and Naralex (35). Levels 17–22, nearly all elite (`rank` 1). Bosses, by guid:

| boss | entry | guid(s) | level | note |
|---|---|---|---|---|
| Lady Anacondra | 3671 | 27366, **300999**, 301000, 301001 | 20 | four authored spawns, the instance script keeps one |
| Lord Cobrahn | 3669 | 27380 | 20 | |
| Lord Pythas | 3670 | 87107 | 21 | |
| Lord Serpentis | 3673 | 38148 | 21 | |
| Kresh | 3653 | 27368 | 20 | patrols in the water (z -106.7), 9 waypoints |
| Skum | 3674 | 87131 | 21 | |
| Verdan the Everliving | 5775 | 33974 | 21 | |
| Deviate Faerie Dragon | 5912 | 517 | 20 | rare elite |
| Mutanus the Devourer | 3654 | — | 22 | no static spawn: summoned by the Naralex event |

Snakes (2914) and Biletoads (3835) are level-1 critters.

## The slice's run

**Boss: Lady Anacondra, guid 300999, at (-53.984, 271.39, -92.844).** Why her:

- She is the first Fanglord on the natural route, and her classic room (guid 300999) lies exactly
  on the Disciple of Naralex escort path (points 14–15), ~300 yd of walking from the arrival
  point. Her other spawns are further along (27366, 301001), or in the side room at (-68, 122)
  (301000) that is only reached through the low hall, past this one.
- She stands still on dry ground. Kresh, the other nearby boss, patrols in deep water (z -106),
  which needs swimming that neither the route graph nor the ground-height host models.
- Level 20 elite, `unit_class` 2 (has mana; her combat spells are in vmangos'
  `creature_spells`, her aggro yell in `creature_ai_events`), ~2165 health at level 20.

**Trash** (all elite, faction 270), in route order:

| guid | mob | level | at | joins route at |
|---|---|---|---|---|
| 26202 | Deviate Guardian | 18–19 | (-100.6, 180.9, -78.4) | escort_4, 9.9 yd |
| 26220 | Deviate Ravager (pack leader) | 18–19 | (-105.5, 228.7, -89.9) | escort_7, 5.4 yd |
| 26224 | Deviate Ravager (with 26220) | 18–19 | (-107.0, 240.2, -92.8) | escort_7, 8.9 yd |
| 26216 | Deviate Ravager | 18–19 | (-67.9, 203.8, -93.6) | escort_10, 4.2 yd |
| 26223 | Deviate Ravager | 18–19 | (-67.4, 214.5, -93.4) | escort_9, 6.0 yd |
| 26217 | Deviate Guardian | 18–19 | (-35.2, 198.1, -96.6) | patrol_26225_8, 2.4 yd |
| 26227 | Deviate Ravager | 18–19 | (-35.0, 209.6, -96.4) | escort_11, 9.4 yd |
| 26230 | Deviate Ravager | 18–19 | (-47.7, 254.8, -92.8) | escort_14, 6.8 yd |
| 26239 | Druid of the Fang (4-point patrol) | 19–20 | (-50.9, 263.8, -92.8) | escort_14, 10.2 yd |
| 26252 | Druid of the Fang | 19–20 | (-55.3, 274.7, -92.8) | escort_15, 2.0 yd |

Each mob in the JSON carries its display IDs, faction, level range, speeds, aggro and
call-for-help ranges, attack time, and per-level health/armour/average melee damage computed the
vmangos way (`creature_classlevelstats` × the template's multipliers).

## The route graph

In `wailing_caverns_run.json` under `route`: 37 nodes, 36 undirected edges, every node with
`x, y, z` on map 43 and a `source`.

- `entrance`: the arrival point.
- `escort_0` … `escort_15`: the Disciple of Naralex escort (`script_waypoint` entry 3678). He
  walks these in a scripted escort through the tunnels, so each consecutive pair is
  known-walkable. Point 0 is his spawn, 24 yd from the arrival point.
- `patrol_26225_*`: a Druid of the Fang patrol (`creature_movement`, guid 26225), a second
  corridor from the low hall (joins `escort_11`) south to the room at (-68, 122).
- `mob_<guid>`: each run mob's spawn, joined to the nearest route node by one straight spur of
  at most 20 yd. These spurs are the only edges no vmangos NPC walks; all are 2–10 yd.

Walking length from `entrance` to `mob_300999`: about 301 yd.

The vmangos dump has more walked paths on map 43 if the graph needs to grow: 380
`creature_movement` points over all patrolling spawns (in the full spawn file, per spawn under
`waypoints`), and the escort continues past point 15 to point 31 at (114.6, 236.9, -96.0).
`creature_movement_template` has nothing for these entries.

## Ground height inside the cave: risks

- **No terrain heightmap.** Map 43 is WMO-only, so `benilla_formats` terrain `height_at` and the
  WDL have nothing; the only ground is the WMO's walk collider (avian trimesh on the `Walk`
  layer, `benilla-world/src/collision.rs`). The terrain streamer registers the map's one WMO
  (`terrain_stream.rs`, "A WMO-only map is one building") and builds its colliders off-thread,
  attached under a 2 ms/frame budget. The loading cover waits for `colliders_pending == 0`, so
  ground exists once the cover clears and the worldport ack is sent.
- **Stacked floors.** The cave has walkways around -78…-96 above water channels whose floor is
  near -106, under a ceiling. A ground query that casts down from high above hits the ceiling or
  the wrong level. The host trait should cast from a little above the query's own z (the route
  nodes' z are vmangos' server values, good seeds) down a few yards, on the walk layer, and keep
  the realm's z on a miss. The client's own creature ground clamp does the same: it probes from
  the seat down at most 4 yd (`GROUND_CLAMP_DOWN`, `net/motion/spline.rs`), so a chase spline's
  z must stay within ~4 yd above the real floor or the unit keeps the server z.
- **Line of sight.** The camera/LOS collider is a separate WMO layer (`Camera`); LOS for the host
  must use it, not the walk layer.
- **Water.** Crocolisks (5053), Kresh and some Ectoplasm sit in the water at z ≈ -106. None is in
  the run; leave them out unless swimming is modelled.
- The route's spur edges are straight lines that were not checked against collision. All are
  under 11 yd in open rooms, but tests with walls should treat the graph as the only walkable
  set.

## Caveats

- The vmangos dump is the sqlite conversion found in the shared cache, not a fresh export from the
  vmangos SQL; patch filtering uses `patch_min ≤ 10 ≤ patch_max` and the latest template patch
  ≤ 10.
- vmangos levels are 17–22 elite. The realm will likely rescale them to the player's level band.
- `static_flags1`/`flags_extra` are vmangos' own flag words, not the client's `UNIT_FIELD_FLAGS`;
  they are carried for reference only.
