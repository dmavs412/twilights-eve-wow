# Travel: Open Road and Carts, behind Server Unlocks (spec #560)

Status: tickets #561 to #566 ready-for-agent, written 2026-11-02. Unplayed.

Grilled 2026-11-02 from three of Davis's travel ideas. The purchase model is ADR 0031
(`docs/adr/0031-server-unlocks.md`). This note holds the rules the tickets build and every
reading taken where the grill left a choice open.

## Server Unlocks

- Catalogue `crates/twilight-realm/content/unlocks.toml`: `open_road` and `carts`, both
  `free = true` while the game is tested.
- State file `unlocks.toml` beside the saves, one table per bought unlock with `until` (UTC).
  No `until` is on for good; no file is nothing bought.
- One player buys, the whole server gets it; solo buys the same way. Checked at login and once a
  day on the realm's wall clock. Locked features stay visible and say so.

## Open Road

| Rule | Value |
|---|---|
| Bonus | +30% run speed, `[open_road] speed_pct` in `roads.toml` |
| When | out of combat, on a road or street texture named in `roads.toml` for the map |
| Adds to | the Well Fed speed dish (12 to 30%), summed before the multiplier |
| Grace | 1 s after the last road step, a constant in the realm |
| Shown as | the Open Road buff, spell 1950000 from `content/buffs.toml` |
| Locked | nothing applies and nothing shows |

Readings taken:
- The road test comes from the terrain's texture layers (`Host::ground_texture`), not from
  hand-traced lines, so every road on every map counts at once.
- Town streets count: Stormwind's cobbles and Goldshire's paving go in the list.
- The bonus is out of combat only; mobs keep their own speed and a chase is unchanged.
- The buff gets its own spell row (ADR 0011) in a new `buffs.toml` the pipeline reads the way it
  reads `creature_spells.toml`, ids from 1,950,000.

## Carts

| Rule | Value |
|---|---|
| Routes | Goldshire (north road) to the Stormwind gate; Goldshire (east road) to Eastvale Logging Camp |
| Wagons | one per route, back and forth all day on the wall clock, from the Unix epoch |
| Speed | 24 yards a second |
| Standing | 30 s at each end |
| Seats | 5, free, right-click the standing wagon within 5 yd |
| Leaving | any move key or click, or a second right-click; set down 2 yd to the wagon's right |
| Mobs | riders are off every threat list and unseen, as the invisibility potion; wagon, horse and driver cannot be attacked |
| Look | Gypsy Wagon (display 5392), a draft horse 4.5 yd ahead, a seated driver |
| Path | traced from the road texture by `tools/twilights-eve/road_path.py`, Z from the ground |
| Map | a mark per stop, grey when locked; each wagon drawn where it is |
| Locked | wagon, horse and driver parked at the route's first stop; the driver's line names it |

Readings taken:
- A cart is a timetable you catch, not a flight master who sends you off at once. Davis chose
  the wait and the moving wagon over the instant ride.
- Goldshire has two stops, one per route, a few yards apart on the north and east roads, so the
  two wagons never stand on one spot.
- A rider who stays seated rides through the turnaround and the far stop; nothing unseats them
  but their own act or a logout.
- A wagon's position is a pure function of the wall clock, so it needs no saved state, keeps
  running with nobody logged in (ADR 0009) and every client samples the same function the realm
  does.
- The ride is free; the unlock is the price. A copper fare was offered and declined.
- Stop x,y start from the vanilla spawns and the tracer snaps them to the road within 15 yd.
- No speed potion: cooking's speed dish (Trail Fish Wrap and its tiers) covers idea 5.

## Out of scope

The purchase flow itself; further stops (Northshire, Jerod's Landing, Azora, Lakeshire,
Darkshire), each a stop row and a run of the tracer; the wagon on the minimap.
