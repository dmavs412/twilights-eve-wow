# The Deeprun Tram's trams are Run state, and a mist wall can be placed by hand

The Deeprun Tram (spec in `docs/specs/deeprun-tram/README.md`) is the first dungeon whose
creatures arrive on a schedule instead of standing in place, and the first whose edge is mist
inside a building. Grilled 2026-11-06.

## Decision

- **The realm drives the trams.** Vanilla's subway cars loop on a fixed `TransportAnimation.dbc`
  cycle that nothing can speed up. The tram's cars are type 11 transports on the Carts' road
  drive (spec #560): the realm sends each car a `SessionEvent::CartRoute` before its create, and
  the client samples `twilight_realm::carts::sample` from the create's anchor. A tram car runs
  one out-and-back and is removed; the realm makes a new one for the next tram. A car backs out
  facing the way it came in, which the route event says.
- **The timetable is Run state.** The tram line belongs to the dungeon's Run in its shard. It
  starts when the station bell (a lever, spec #575) is used, and resets with the Run. Its
  creatures are rows in `crate::mobs`' spawn list that the line brings into the shard, as an
  Encounter's waves are (`src/encounters.rs`).
- **Tram code is for the tram only.** Davis chose this over a general wave-dungeon kind. A second
  wave dungeon is the time to generalise it. Its numbers stay in the dungeon's content file.
  Amended (#637): the frozen passengers' thaw is now the generic `frozen_until` dungeon creature
  key; the rest stays tram code.
- **A mist wall can be placed.** `data/twilights-eve/mist_walls.json` lists vertical mist walls
  on any map, each a line from one point to another and a height band. The client draws each one
  with the Closed Zones' shader and makes it solid, and the realm refuses any step that crosses
  one with ADR 0015's "You cannot go that way." The two readers share the file, as the Closed
  Zones' chunk list is shared.

## Considered options

- Vanilla's own subway loop: rejected, Headway changes when trams come.
- A general Wave Dungeon content kind: set aside by Davis for now.
- Thick fog in place of a wall: rejected, the trams would fade in rather than break through.

## Consequences

- `SessionEvent::CartRoute` gains a field, so the client and the realm change together.
- A Run can now hold creatures it was not authored with beyond timed calls: the tram line's.
