# Closed zones: one chunk list, cut on the client, walled in the realm

The game is set in Elwynn, but the stock world goes on past it. A player who walks west sees
Westfall and can walk into it. A realm wall alone stops the walk but leaves the land in view.
Spec #297 removes a closed zone from what the client draws and from what the realm lets a
character enter. This ADR fixes where the list of closed land lives and how each side reads it,
so a later zone closes or opens by editing one line.

- **The unit is the MCNK chunk.** A closed zone is a set of AreaTable zone ids. A chunk is
  closed when its area id rolls up through AreaTable's parents to a closed zone. A chunk is
  33.3 yards on a side, so the border follows the chunk grid. The Great Sea's parent area is
  Westfall, so closing Westfall also closes the sea off its coast.
- **One generated file, compiled into both sides.** `data/twilights-eve/closed_zones.toml` names
  the closed zone ids. `tools/twilights-eve/closed_zones.py` scans the Azeroth ADTs and writes
  `data/twilights-eve/closed_chunks.json`, committed like the other generated files. The client
  and the realm `include_str!` it, as they do `client_dbc_patch.json`. Realm tests read it with
  no game data.
- **Two readers, one set of test points.** The realm depends only on `benilla-protocol`, and
  that stays true. The realm has its own reader in `world/`, and the client has one in
  `benilla-formats`. Each is a few lines over the same file. A test on each side checks the same
  fixed world points, so the two readers cannot drift apart unnoticed.
- **The client cuts, the realm enforces.** The client draws no terrain, water, clutter,
  placements or horizon cell on a closed chunk, and marks the chunk impassable. A placement
  goes when its centre is on a closed chunk. An animated mist curtain stands on every edge
  between an open chunk and a closed one. The realm refuses any reported move that ends on a
  closed chunk, living or dead and however long the step, and says "You cannot go that way."
- **Our content stays on open land.** A realm test fails when any placed creature,
  GameObject, quest area or dungeon footprint stands on a closed chunk.

## Considered options

- **Scan the ADTs at run time from zone ids.** Rejected: the realm has no game data, so the
  client would have to send it the list across the seam.
- **A file named by an environment variable**, as the prototype did. Rejected: each `play/`
  copy and each realm needs the path set, and tests cannot see it.
- **One shared reader in `benilla-protocol` or a new crate.** Rejected: the protocol crate
  would carry world geometry, and a new crate adds a realm dependency for about 30 lines.
- **Raise the closed land into a ridge, screen it with trees, or guard it.** Rejected in #297:
  placements float or sink, the player sees through a screen, and a fast character outruns
  guards.

## Consequences

- Opening or closing a zone is a one-line edit to `closed_zones.toml` and a rerun of the tool.
- The world map and minimap still show closed zones. That is a separate issue.
- Forest's Edge's wall lets the dead through. The closed-zone wall does not, because a ghost
  would walk into the void.

## Amended 2026-11-08: one row of closed land is drawn inside the mist

Spec #662, the mist look. Davis played the #554 curtain and called it a flat painted wall that
follows the chunk grid in straight steps. Land that stops at a sheet shows exactly where the world
ends; the lore's mist is a grey that the roads run into.

- **The rim row draws its land.** A rim chunk is a closed chunk with an open chunk among its eight
  neighbours. The client draws its terrain and its water, and fogs both toward the colour of the
  mist's foot by their distance past the edge, so the land is pure mist 30 yards in.
- **Everything else on a rim chunk stays cut.** It grows no clutter and keeps no doodad or WMO. It
  stays impassable and keeps its fence. Its horizon cell stays cut.
- **The realm does not change.** It refuses a move onto any closed chunk, rim or not, with "You
  cannot go that way."
- **The mist stands inside the rim.** The curtain is five shells from the edge to the rim's far
  side, 33.3 yards in. The last one is solid, so the empty chunks beyond the rim stay hidden.
- **Chunks past the rim** draw nothing, as before.

### Consequences

- A terrain cell that holds a rim chunk carries a mist-depth vertex attribute, and the terrain
  and liquid shaders each have a rim variant chosen by that attribute.
- A zone that a Milestone opens (ADR 0034) loses its rim when its tiles reload; the fogged rim
  shows clear land from that frame on.
- The minimap and the world map are unchanged.
- Content rules are unchanged: the realm test still fails on anything placed on a closed chunk,
  and a rim chunk is a closed chunk.

### Considered options

- **Two rows** (66.7 yards). Davis chose one. The solid back shell already stands at the end of
  the first row, so a second row would sit behind it unseen.
- **Keep the cut and improve only the curtain.** Rejected: a better sheet is still a sheet with
  the land stopping in front of it.
