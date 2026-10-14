# Twilight's Eve runs against an in-process local realm

Twilight's Eve has no server today, but one may be added later. So its game simulation (characters, quests, mobs, loot, talents, dungeons) runs as a local realm inside the client process, behind the same `ClientCommand` / `SessionEvent` seam a real server feeds through `crates/benilla-app/src/net.rs`. The client runs in its normal-WoW mode on top: player controller, FrameXML quest log, spellbook, trainer and talent UI.

## Considered Options

- **Survivors-style local ECS with a custom HUD.** Rejected. It means rebuilding the quest, spellbook, talent, trainer, bag and loot UI by hand, and adding a server later would mean rewriting the game.
- **A rules core free of the engine, with a custom UI now and a protocol port later.** Rejected. It pays both costs: the custom UI now and the port later.

## Consequences

- The local realm has to speak the client's protocol faithfully: object update fields, GUIDs, spell start and go, and aura slots.
- Twilight's Eve's custom rules (changing class, granting every non-talent skill) live in the local realm. A future real server has to reproduce them, and a stock 1.12 server cannot.
- The Local Realm is a crate free of Bevy, `crates/twilight-realm`. Commands go in, events come out, and it is ticked by its host. Ground height and line of sight come through a trait its host implements. `crates/benilla-app/src/twilight/` is only glue: the boot flag, moving commands and events to and from `net.rs`'s channels, the per-frame tick, and the terrain queries. A future server is this crate behind a socket, so the custom rules move over unchanged.
