# What another client sees

How the realm decides which objects and events reach each player's client, in
`crates/twilight-realm/src/lib.rs` and `shard.rs`. Read this before adding anything other
players should see: a new object kind, a new event about a unit, a new field on a player.

## Sessions and shards

Each logged-in character is a session. One session is served at a time (`Realm::serve`). It owns
its `World`, and the others wait in `parked`. A shard is one `Place`: a map plus a Phase (the
open world, or one Phased Dungeon). It owns the creatures there and its Hub NPCs.

- **Lending.** `lend` moves the shard's mobs into the served session's `World` (`w.mobs`), so
  that session's combat code runs on them. `give_back` returns them and remembers the events the
  session made from `lent_mark` on.
- **`seen`.** Each `World` keeps `seen`, the guids its client has been sent a create for. A guid
  enters `seen` when its create is sent and leaves when the client is told to remove it.

## The view, every tick (`tick_shards`)

For each shard, each player on it gets an `OnShard` row for the tick: position, health, their
create as others see it (`remote_create`, public fields only), whether they are hidden
(stealth), and their patches on the ground. Then, for every session on the shard:

1. **Streamed events.** Whatever the shard made this tick (`Shard::update`: spawns, despawns,
   wanderers' walks) goes to the client only for a unit in its `seen`.
2. **The view.** The view is the set `(guid, create)`. It holds the mobs and Hub NPCs from
   `Shard::in_view`, filtered by distance (`population::VIEW_YD`) and Audience (`Mob::admits`,
   ADR 0014). It adds the other players within `VIEW_YD` who are not hidden, and the other
   players' ground patches whose centre is within `VIEW_YD` (ADR 0017).
3. **Diff against `seen`.** A guid in `seen` but not in the view, unless `Shard::keeps` it (a
   mob still fighting), goes out in one `ObjectsRemoved`. A guid in the view but not in `seen`
   gets its create, and a player also gets a `Callings` line first.

## Events the served session makes (`give_back`, `show_others`)

The served session's own events go to its own client directly. `give_back` then does two
things with them:

- **Its own creates enter its `seen`,** so the view does not create them a second time. A kind
  the session streams itself is left out: a GameObject (its portals and doors) and a DynamicObject
  (its own ground patches). Those would otherwise leave `seen` on the next view diff, and the
  client would be told to remove them.
- **`show_others` relays the events to every parked session on the same shard,** when
  `shard::shown_units` names a unit that session has in `seen`. `as_seen_by_others` filters the
  relayed copy: a player's values keep only the fields `public_player_field` allows (the object
  and unit block below `UNIT_FIELD_STAT0`, flags, looks, visible gear), and a corpse is lootable
  only by its killer. An event `shown_units` does not list is never relayed: creates, destroys
  and the session's own UI events.

## Adding something other players see

- **A unit field** (a channel, a stand state): send `values(guid, ...)` from the session. If the
  field index is below `UNIT_FIELD_STAT0`, `show_others` relays it. Above that, add it to
  `public_player_field`.
- **An event about a unit:** add its variant to `shard::shown_units`, naming the units it shows.
- **An object that lives with a player** (a ground patch): give `OnShard` a field holding its
  guid, position and create, and add it to the view in `tick_shards`. The session sends its own
  create and destroy directly, and `give_back` keeps that kind out of `seen`.
- **An object that lives with the shard** (a creature): put it on the shard. `in_view` streams
  it, and `admits` limits it to an Audience.

Tests that run two sessions use `tests/common/party.rs` (`Party`). `players_see_each_other.rs`
shows a player coming into view and leaving it.
