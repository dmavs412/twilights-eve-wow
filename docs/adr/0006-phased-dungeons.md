# Phased Dungeons on the open world

Twilight's Eve's custom dungeons (Northshire Abbey first, then three more in Elwynn) live in
buildings, mines and fields on the open world map, not on instance maps. A dungeon is a set of
3D footprint boxes in its content file; a character standing in any box is in that dungeon's
Phase and sees and targets only that Phase's creatures. Every character shares one Run per
dungeon: it starts when the first character enters, keeps a clock for timed events, and resets
the dungeon fully the moment no one is inside (death included: the dungeons are short, so a wipe
is meant to cost the run).

We chose this over vanilla's instance maps because the project cannot author new maps, and the
places we want (the Abbey, Hogger's camp, a mine, a mage tower) already exist on Eastern
Kingdoms. Phase membership comes from where a character stands, as Wrath's area phasing does,
rather than from a doorway trigger, so there is no side-of-the-door state to fall out of sync on
relog or death. One shared Run, not one copy per party, keeps the realm to a single set of
creatures per dungeon.

## Considered Options

- Reuse a vanilla instance map (Scarlet Monastery behind the Abbey door): rejected; the theme
  must fit the place, and loading screens break the open-world feel.
- A doorway trigger that toggles a remembered Phase: rejected for the sync problem above.
- A grace period before reset: rejected; empty means reset.

## Consequences

Later dungeons add data, not code: timed events, game objects (levers, doors), escort routes and
per-Run rolls are generic components driven from each dungeon's content file.

ADR 0029 scales a Run's creatures with the Head Count, the living players inside its Phase.
