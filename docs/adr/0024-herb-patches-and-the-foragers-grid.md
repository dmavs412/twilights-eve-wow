# Herb Patches are each character's own, and the realm runs the Forager's Grid a pull at a time

Herbalism gathers from Herb Patches through a minigame, the Forager's Grid. It is played on a
5 by 5 grid of hidden plants, weeds and thorns, read by Minesweeper numbers. The prototype
Davis approved is the artifact "Forager's Grid" (claude.ai/artifact/MNuLWaZvKds6eMhLZbbyfh). This
ADR settles who knows the grid, where patches stand, and how opening one reaches the realm.
Spec #435.

- **The realm runs the grid, one action at a time.** Fishing seeds the Reeling Bar and lets the
  client run it, with a plausibility check on the result (ADR 0018). The grid is turn-based, so
  the realm keeps every tile. The client sends each pull, tool skill or consumable use. The realm
  answers with the tiles that action revealed, the stake numbers and the pulls left. The client
  never learns a hidden tile, so no check is needed. A round trip per click costs nothing
  in-process, and nothing a player would notice over a socket.
- **A Herb Patch is a run-time GameObject for one character.** Each character has its own
  patches, made with an Audience of that character alone (ADR 0014) through `Realm::spawn_object`.
  Friends never compete for a patch. Patches stand at Herb Spots, the vanilla herb spawn points
  of each zone, imported from vmangos. Each zone keeps a set number live per character. A
  finished patch is removed, and a new one appears after a delay at another free spot in its zone.
  Live patches are not saved: entering the world rolls a fresh set.
- **Every patch looks the same.** It has one stock bush model and no lock. A click is a plain
  `CMSG_GAMEOBJ_USE`, which the realm answers by opening the grid at once, with no cast bar.
  Twilight mode answers no GameObject template query, and a cast aimed at a GameObject never
  reaches the realm, so a locked patch could not be opened.
- **The minimap leaf comes from the realm, not stock tracking.** Stock resource tracking reads
  the template's lock, and a patch has no template and no lock. Learning Herbalism teaches Find
  Herbs. Casting it toggles herb tracking on the character, and the realm then sends the patch
  GameObject entry with the tracking state. The client draws a leaf icon on the minimap for each
  GameObject of that entry while tracking is on.
- **Damage or walking away ends a patch.** A patch is finished when its pulls run out, when the
  player leaves it, when the character takes damage, or when they move out of interaction range.
  Each plant goes into the Herbalism Satchel the moment it is pulled (ADR 0023,
  `journal = "herbalism"`), so finishing early loses nothing already pulled.
- **Herbalist Level is the one level that tunes a roll.** Herbalist Level runs 1 to 30 and is kept
  across Callings, like Angler Level. Davis chose that each level above 1 raises the roll weight
  of uncommon plants by 2% and of rare plants by 3%. Fishing gear never scales with level; this
  is a deliberate exception for herbalism alone. A plant above the character's Herbalist Level
  never grows in their patches.
- **Content is `content/herbs.toml`.** The plants, the zones' pools and their Herb Spots, the
  tools, consumables, perks, XP and grid tuning live in one file loaded by `src/herbs.rs`, as
  fishing's live in `fish.toml`. The Herb Gathering and Find Herbs spell ids live in its
  `[spell]` table, never in a `.rs` file.

## Considered options

- **The client runs the grid, like the Reeling Bar.** Rejected: the client would hold every
  hidden tile, and a turn-based game gains nothing from running locally.
- **Shared patches.** Rejected by Davis: friends would race each other for patches.
- **A patch shows the vanilla herb of its spot, and favours that herb.** Rejected by Davis in
  favour of one generic bush, so the grid alone reveals what a patch holds.
- **A short Herb Gathering cast before the grid opens.** Rejected by Davis: the grid opens on
  the click.
- **A damaged or interrupted patch keeps its state to resume.** Rejected by Davis: damage or
  leaving ends it.
- **Hand-placed patches.** Rejected: hundreds of rows to write, when vmangos already has every
  vanilla herb point.
