# Audience: creatures and GameObjects that only some characters can see

Everything the realm places is seen by every character who shares its Phase: the open world is
one Phase, each Phased Dungeon another, and a Bounty Hunter hunts one owner but stands in plain
view of everyone. Rumours (spec #284) need something narrower. A Rumour spawns a Field Giver,
named creatures and a chest for the few characters who stood near the kill that dropped it, and
nobody else may see, fight, click or loot them. This ADR adds that narrower rule without a Phase
per character, so the next feature that spawns something for a few people follows the same path.

- **An Audience is a set of character guids on a spawned entity.** A creature or GameObject may
  carry `Audience(Vec<u64>)`, fixed when it is spawned. The view code that already filters by
  Phase and distance also drops an entity whose Audience does not hold the viewer. An entity
  with no Audience is seen as today. An Audience never changes after the spawn: a character
  who was not there is not added later, and one who leaves the group stays in it.
- **Not a Phase.** Phases are places: a character is in exactly one, and everything in it is
  shared. An Audience is a property of the thing, not of the viewer, so a character sees their
  own Rumour's giver and the Abbey's Brother Neals in the same frame. The Phase check still
  runs first; an Audience narrows it and never widens it.
- **Audience entities act only on their Audience.** A creature with an Audience notices,
  threatens and damages only characters in it, and is damaged only by them; a GameObject with
  an Audience answers a click only from one of them. The Bounty Hunter's `hunts` field stays:
  it is a target choice on a creature everyone sees, not a visibility rule.
- **A Rumour's spawns die with the Rumour.** When the thing that spawned them ends (the Rumour
  hands in or times out), every entity carrying that Audience despawns; it is never left for a
  respawn timer. Audience entities are not saved: a quit loses them, and spec #284 says so.
- **GameObjects grow a loot chest.** The realm already spawns portals, doors and use objects
  (`GAMEOBJECT_TYPE_GENERIC`, `DOOR`, and the quest `use_object` rows). It gains
  `GAMEOBJECT_TYPE_CHEST` (3): a click opens the stock loot window on the chest's rolled items,
  and the chest despawns once emptied. A chest is the first GameObject with loot and the first
  spawned at run time outside a content row.

## Considered options

- **A Phase per character or per group.** Rejected: a character inside a Rumour would stop
  seeing the open world's other players and creatures, or the realm would need an entity to be
  in several Phases at once, which is this rule under another name with a heavier Phase type.
- **Spawn for everyone and gate the quest alone.** Rejected: a passer-by could kill the named
  creature or loot the chest, and a Field Giver standing in the open with a `!` that most
  characters cannot take reads as a bug.
- **Reuse `hunts` (the Bounty Hunter owner) as a visibility rule.** Rejected: a Bounty Hunter is
  meant to be seen, and `hunts` is one guid where a Rumour needs a set.

## Consequences

- `shard::in_view`, `hub::in_view` and the per-session GameObject stream take the viewer's
  guid, not only their position.
- Threat, aggro and damage paths check the Audience before the Phase check they already make.
  A test spawns an Audience creature beside two characters and asserts the outsider never
  receives its create, is never attacked and cannot damage it.
- The Friends Server (ADR 0009) carries an Audience as guids; the host's realm owns the spawn
  and its despawn, as it owns Phased Dungeon state.
- The client changes nothing: it shows what it is streamed, and the stock loot window already
  handles a chest.

The view code that applies this rule, and how a new kind of object reaches other clients:
`docs/realm_view.md`.
