# Milestones: server-wide facts the realm writes, and land a Milestone opens

The lore bible keeps the Burning Steppes under mist until players break the Anchor in the Tower
of Ilgalar. On the friends server (ADR 0009) up to five players share one world, and solo play is
a one-player server (ADR 0031). Davis wants the break to start a push the whole server makes
together, and the Steppes to open for everyone when that push is done. Today the realm remembers
nothing about the whole server, and the Closed Zones (ADR 0015) are fixed when the game is built.
Grilled 2026-11-07 and 2026-11-08 (`docs/specs/steppes-opening/`).

## Decision

- **A Milestone is a fact about the whole server.** Once reached, it stays reached in play, and
  nothing un-reaches it. A host or a test resets one by editing the state file while the realm is
  stopped. Solo play is a one-player server, so every character in the roster shares one file.
- **A catalogue in content.** `crates/twilight-realm/content/milestones.toml` lists each
  Milestone: `key`, `name`, and optionally `after` (a Milestone that must be reached first),
  `goal` and `feeds` (below), `announce` (a line on `/1`) and `yell` (a Hub NPC's yell that every
  character in the world hears).
- **The realm writes its own file.** `milestones.toml` in the realm's data folder (the folder
  that holds the saves: `<data-dir>/milestones.toml` on the server,
  `benilla-config/twilight/milestones.toml` in solo) has one table per Milestone that has a count
  or has been reached, with `count` and `reached`, a UTC date-time. The realm hands its host the
  whole file as `Storage::Milestones { blob }`, at most once a tick, and the host writes it
  atomically. The host reads it at start and passes it to `Realm::set_milestones`. A missing file
  means nothing is reached. An unknown key or a bad date is a logged warning. The file is apart
  from `unlocks.toml`, which only a purchase writes (ADR 0031).
- **A Tally fills toward a goal.** A Milestone with a `goal` is reached when its count reaches the
  goal. Its `feeds` name quests and points. Each hand-in of a fed quest, by any character, adds
  that quest's points while the `after` Milestone is reached and this one is not. The goal is
  fixed and the same on every server, whatever its size (Davis). Nothing decays.
- **Clients hear the reached set.** `SessionEvent::Milestones { reached }` carries every reached
  key. The realm sends it once at login, before the character's world, and to every session in
  the world each time a Milestone is reached. Nothing on screen shows a Tally (Davis). NPCs say in
  words how far it has come.
- **A Closed Zone can wait for a Milestone.** `data/twilights-eve/closed_zones.toml` lists zones
  closed for good and zones that a Milestone opens. `closed_zones.py` writes each gated chunk with
  its Milestone key into `closed_chunks.json`. Both readers (ADR 0015) call a gated chunk closed
  until its key is in the reached set. The realm's wall and the client's drawing both follow the
  set, so a zone opens with no new build, and it never closes again in play.
- **Content may stand on gated land.** The content test fails only on land closed for good. The
  realm sends no creature, Hub NPC, GameObject or map mark that stands on closed land, gated or
  not, so a player at the border never sees creatures over the cut. A character who enters the
  world on closed land (a test server that was reset) goes to their bind point.
- **Content can wait for a Milestone.** Hub NPCs, quest GameObjects, hand-placed zone creatures,
  quest offers, and Phased Dungeon creatures and GameObjects (`Appears`) take `after_milestone`
  and `until_milestone`. They come or go the moment the Milestone is reached, in a Run that is
  under way too.
- **The client reloads land that opens.** On the event, the client adds the newly opened keys to
  the set that its ADT loader, horizon, minimap and world map read. It reloads every loaded tile
  that holds a newly opened chunk and fades that border's mist curtain over a few seconds while
  the land loads behind it. At login the set arrives before the tiles load, and nothing fades.

## Consequences

- Every later "the server did this" fact uses the same parts. The next Anchor that breaks is a
  catalogue row, an `[[opens]]` row in `closed_zones.toml` and a rerun of the tool.
- The realm now writes a file that is not a character's save. Two realms run from one data folder
  would overwrite each other's Milestones, as they would each other's saves.
- An edit to `milestones.toml` while a server runs is lost at the realm's next write. Hosts edit it
  while stopped.
- A reached Milestone cannot be taken back in play, so a gated zone has no way to close again.
  Mist that rolls back over characters standing in it needs a decision of its own.
- A solo player fills a Tally alone, at the same goal. Davis accepted that as a long push.
- `SessionEvent` and `Storage` each gain a variant, so the client, the server and the realm change
  together.
