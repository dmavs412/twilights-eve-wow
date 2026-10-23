# Fishing: the realm seeds the catch, the client runs the Reeling Bar

Fishing (spec #323) adds a Stardew-style minigame to vanilla's cast-and-wait. The bar takes
input every frame: the player holds the mouse button to lift a catch zone while the fish moves.
The realm will sit behind a socket for friends (ADR 0009), so a bar the realm ticks would show
every millisecond of latency. This ADR settles who decides what, where the water comes from, and
how Fishing lives outside the Callings' master CSV.

- **The realm decides the catch before the bar opens.** When a fish bites, the realm has already
  picked the fish, its level, its movement style and difficulty, whether a treasure chest appears
  and what is in it, and a random seed. It sends all of that in one Twilight-only session event
  (`FishingBite`, the ADR 0013 path). Nothing the client does can change which fish or which
  treasure it gets.
- **The client runs the bar and reports the outcome.** The client simulates the bar from the
  seed and the event's numbers, and sends one `RealmCommand::FishingResult` with whether it won,
  whether the catch was perfect, whether the chest filled, and how many seconds the bar ran.
- **The realm checks plausibility, not a replay.** It refuses a win shorter than the fastest
  possible fill (from the start fill to full at the event's fill rate), a result with no bar
  open, and a bar older than its timeout. It does not replay the input. A friend who edits their
  client can land every fish they hook, which this game accepts; they still cannot choose the fish.
- **The realm ends a bar on its own.** Moving, taking damage, casting or logging out loses the
  fish. The realm sends `FishingEnd` and stops listening for a result.
- **Water comes from the Host.** `Host::liquid_at(map, x, y)` returns the water surface under a
  point from the terrain the host already streams: ADT `MCLQ` and WMO `MLIQ`, the way `area_at`
  reads the `MCNK` area id. The in-process client and the Friends Server each answer it. There is
  no offline extraction and no generated water file. Content can mark a Fishing Water by
  AreaTable id or by a circle; it changes which pool bites, never whether there is water.
- **Fishing is not a Calling skill.** Its spell is vanilla's stock Fishing row, which the client
  already has in `Spell.dbc`. Its id is a row in `content/fish.toml`, not in `master.csv`, and
  nothing about it passes through the spell pipeline. A character learns it from a trainer and
  keeps it across Callings. The realm handles a cast of that id before the Calling skill lookup.

## Considered options

- **The realm simulates the bar from streamed button state.** Rejected: it is authoritative, but
  over a socket the bar would lag behind the mouse.
- **The client runs the bar and the realm replays its input log.** Rejected for now: it needs the
  simulation in a crate both sides reach, a deterministic float path on both, and a log on the
  wire, all to stop cheating in a game played among friends.
- **An offline tool extracts fishable water into a generated file.** Rejected: both hosts
  already load the terrain that holds the liquid, and a generated file would need regenerating
  for every new zone.
- **Fishing as a `master.csv` skill row.** Rejected: the sheet's rows belong to a Calling, and
  Fishing would need a Calling-less row kind and a new effect kind in the pipeline for one spell
  whose client row is already stock.
