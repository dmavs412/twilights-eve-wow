# Elwynn Forest spawns (Twilight's Eve, spec #71)

`data/twilights-eve/elwynn_spawns.json` holds every visible creature spawn in Elwynn Forest
(map 0, zone 12) at patch 1.12: hostile mobs, neutral beasts and critters, and friendly NPCs
(guards, vendors, trainers, innkeepers, quest givers). It comes from the same importer and the
same source as The Barrens (`docs/reference/barrens-spawns.md`, which covers the source DB, the
zone-membership rule and the cave/mine WMO lookup).

## Re-running

```bash
python -I tools/twilights-eve/import_barrens_spawns.py \
    --db "$TMP/vmangos/sqlite-dump/mangos.sqlite" --data /path/to/WoW/Data \
    --source-asset db-sqlite-4641790.zip \
    --map 0 --zone 12 --box -12000 -7000 -3500 2500 --all-creatures \
    --out data/twilights-eve/elwynn_spawns.json
```

The importer's defaults are still The Barrens (map 1, zone 17, its box, no `--all-creatures`),
and its Barrens output is byte-identical to before. The run is deterministic and takes about 2 s.
The box only narrows the rows before the exact zone test. Widening it past `-11000 -8000
-2500 1500` adds nothing, so the committed box has margin on every side.

## What `--all-creatures` changes

- No spawn is dropped for npc flags, for being a quest giver or a guard, or for being a critter,
  totem or pet, or because it is friendly to a player faction. Only game-event-only spawns,
  disabled or invisible spawns and unselectable templates are still dropped.
- A spawn friendly to either player faction gets `"reaction": "friendly"`.
- Each spawn gains `npc_flags` and `guard`. Each template gains `npc_flags`.
- Camps are still clustered. The realm does not use them for Elwynn (no rings, no camp regrouping).

## Counts (vmangos 4641790)

1621 spawns of 174 entries: 854 hostile, 571 neutral, 196 friendly. 98 game-event spawns
dropped. Mine interiors come from the WMO rule: Fargodeep 15, Echo Ridge 8, Jasperlode 10.

Spot checks: Marshal McBride (197) in Northshire Valley, Northshire Guards (1642), Marshal Dughan
(240) and Innkeeper Farley (295) in Goldshire, and Hogger (448) at Forest's Edge (two pooled
spawns, one up at a time).

## The realm's copy: `crates/twilight-realm/content/elwynn.toml`

`cargo run -p twilight-realm --example import_elwynn` converts the dump, and
`tests/content_elwynn.rs` fails when the two disagree. Every spawn keeps its vmangos position,
facing and level range and is a camp of its own. Templates carry the vmangos multipliers over the
realm's `mob_class_stats.toml`, as Barrens templates do. Spawns stream in around the player on
map 0 the way Kalimdor's do. Dead mobs come back on the realm's respawn rule
(`rules::respawn`), not the vmangos timer, which the spec's "realm rules" choice covers.

## Friendly NPCs

A template whose spawns are friendly to either player faction gets `friendly = true` and faction
template 35 (friendly to every player race) in place of its own, so guards and vendors show green
to every character (spec #71). The realm never lets an attack, a spell or an area or cleave hit
pick one (`Mob::attackable`). No npc flags are sent, so they offer no menus.
