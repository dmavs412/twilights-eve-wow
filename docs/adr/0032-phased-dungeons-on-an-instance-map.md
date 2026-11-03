# Phased Dungeons may live on an instance map

ADR 0006 put every Phased Dungeon in a building, mine or field on the open world, because the
project cannot author maps and the places it wanted already stood on Eastern Kingdoms. The
Stockade does not: vanilla's jail is its own map (34), one WMO with no terrain, and the city's
jail door is a single small room. Grilled 2026-11-03 (spec #575).

## Decision

- **A Phased Dungeon's `map` may be an instance map.** It then carries an `entrance` and an
  `exit` trigger in the Caverns' form: walking into the entrance worldports the character to the
  map behind a loading screen, and the exit worldports them back to the door outside.
- **The whole map is the footprint.** One box covers the jail, so a character on the map is in
  the Phase. Phase and Run rules are unchanged (ADR 0006, ADR 0029): the Run starts with the
  first character on the map and resets when the last one leaves. Dying and releasing, logging
  out and the exit all leave the map, so each ends the character's part in the Run as today.
- **Out is the door.** Release Spirit brings the character out at the exit's destination, and a
  save made inside logs back in there. No corpse run through a loading screen.
- **The Portal and the map mark stand at the door on the open world**, where vanilla's instance
  portal stood, so the swirl and the Dungeon mark are on the city map, not the jail's.
- **Zs are authored.** A map without terrain gets every Z from the vanilla data, with
  `grounded = false`, as the Caverns' rows do; `ground_check.py` leaves such rows alone.
- **Chases stay straight lines**, as in every Phased Dungeon. A route graph like the Caverns'
  is a later decision if play shows creatures walking through walls.

## Considered options

- A footprint in the city WMO's jail room: rejected, the room has no cells.
- Placing `StormwindJail.wmo` on map 0 under the city: rejected, the client and the realm would
  both need a new placement path and collision for it.
- Growing the Caverns' one-dungeon `dungeon::content()` into a list: rejected. The Caverns stay
  the slice's old-style Dungeon, and Phased Dungeons gain the triggers instead.

## Consequences

- The loading screen ADR 0006 avoided comes back for places that exist only as their own map:
  the Stockade now, and the Deeprun Tram's tunnels later if they need it.
- `in_dungeon`, the respawn point, `come_out`, the elite XP kind and the map marks read a
  dungeon's own map and triggers instead of the Caverns' constant.
