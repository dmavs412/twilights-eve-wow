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
