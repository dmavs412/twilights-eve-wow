# Phased Dungeons may live on an instance map

ADR 0006 put every Phased Dungeon in a building, mine or field on the open world, because the
project cannot author maps and the places it wanted already stood on Eastern Kingdoms. The
Stockade does not: vanilla's jail is its own map (34), one WMO with no terrain, and the city's
jail door is a single small room. Grilled 2026-11-03 (spec #575). Amended 2026-11-04, when
the Stockade split into two dungeons with the Watch holding the corridor between them.

## Decision

- **An instance map is content of its own.** `instance_maps.toml` lists each with its
  `entrance` and `exit` triggers in the Caverns' form, its zone, its name and its levels:
  walking into the entrance worldports the character to the map behind a loading screen, and
  the exit worldports them back to the door outside. A Phased Dungeon on it says `map = 34`
  and nothing about the doors.
- **The map outside every footprint is open world.** A Phased Dungeon on an instance map is one
  or more footprint boxes like any other, with its Portal where its Phase begins, and two
  dungeons may share a map. Between them Hub NPCs stand and characters meet, as in a town.
  Phase and Run rules are unchanged (ADR 0006, ADR 0029): a Run starts with the first character
  in the footprint and resets when the last one leaves it. Dying and releasing, logging out and
  the exit all leave the map, so each ends the character's part in the Run as today.
- **Out is the door.** Release Spirit brings the character out at the map's exit destination,
  and a save made inside logs back in there. No corpse run through a loading screen.
- **The map's Dungeon mark stands at the door on the open world**, where vanilla's instance
  portal stood, named by the map and carrying its levels, so the city map shows one mark for
  the jail. A dungeon on an instance map makes no mark of its own; its Portal stands inside, at
  its own door.
- **Zs are authored.** A map without terrain gets every Z from the vanilla data, with
  `grounded = false`, as the Caverns' rows do; `ground_check.py` leaves such rows alone.
- **Chases stay straight lines**, as in every Phased Dungeon. A route graph like the Caverns'
  is a later decision if play shows creatures walking through walls.

## Considered options

- A footprint in the city WMO's jail room: rejected, the room has no cells.
- Placing `StormwindJail.wmo` on map 0 under the city: rejected, the client and the realm would
  both need a new placement path and collision for it.
- Growing the Caverns' one-dungeon `dungeon::content()` into a list: rejected. The Caverns stay
  the slice's old-style Dungeon, and instance maps are a table of their own.
- One dungeon with the whole map as its footprint, and the triggers on the dungeon (the
  decision of 2026-11-03): set aside the next day, when Davis split the Stockade into two
  dungeons with the Watch's corridor between them.

## Consequences

- The loading screen ADR 0006 avoided comes back for places that exist only as their own map:
  the Stockade now, and the Deeprun Tram's tunnels later if they need it.
- `in_dungeon`, the respawn point, `come_out`, the elite XP kind and the map marks read the
  instance map's triggers instead of the Caverns' constant.
- A map without terrain can hold Hub NPCs: their Zs are written, as the dungeon rows' are.
