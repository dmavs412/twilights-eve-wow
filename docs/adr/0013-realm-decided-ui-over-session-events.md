# Realm-decided UI over Twilight-only session events

The 1.12 protocol carries what vanilla's interface shows. A boss health bar is not in it: the
stock interface has no boss frame, and the wire has no message that says "this fight matters".
The first case is the Boss Bar (spec #280): rows at the top of the screen for the significant
creatures a character can see in a fight. The realm alone knows whether a creature is
significant (a key on its template), whether it is in a fight (its threat table) and who has it
in view (its stream radius and Phase). This ADR settles how such a realm decision reaches the
client's interface, so the next one follows the same path instead of inventing its own.

- **The realm decides, and says so in a Twilight-only session event.** A new variant of
  `SessionEvent` carries the decision as data (here, the ordered list of boss guids in that
  character's view), sent to one character, on every change, and complete each time. The
  client stores the last list it heard and shows it. It infers nothing from combat flags,
  names or ranks, and it never decides on its own that a bar should appear or vanish. The
  Calling-keys event (ADR 0011, #207) already works this way; this ADR makes it the rule.
- **Not a 1.12 opcode.** The event lives on the in-process channel of ADR 0001 and in the
  Friends Server's wire of ADR 0009 as its own message, never as a repurposed vanilla opcode.
  The client under a vanilla server (`WOW_STOCK_UI=1`, or a vmangos realm) never receives it
  and shows nothing new.
- **The interface reads it through unit tokens, not new Lua globals.** The client feeds each
  guid the event names as a base unit token (`boss1` to `boss3`), the way it feeds `target` and
  `targettarget`, so the frame in our UI layer is plain FrameXML over the stock `Unit*`
  getters. A new event adds tokens or data to existing feeds; it does not add a parallel API.
- **The client's per-unit state stays the client's.** Health, name, level and the cast in
  progress come from the object store and the cast record the client already keeps for every
  unit in view. The event names the unit; it does not repeat what the stream already carries.

## Considered options

- **The client infers the bar from a shipped list of boss entries and the unit's combat flag.**
  Rejected: the client cannot tell whom the creature is fighting, so a boss fighting someone
  out of view would raise a bar, and the rule would live in two places.
- **Raise the bar with a raid warning or a system message.** Rejected: chat carries no guid,
  so the bar would have nothing to read health from, and a message is a one-shot while a bar
  is state.
- **Repurpose a vanilla opcode or unit field (a dynamic flag bit).** Rejected: every bit of
  those is read by the stock interface and the reference client's own code paths; a hidden
  meaning there is a bug waiting in every later feature that reads the field honestly.

## Consequences

- `benilla-protocol`'s `SessionEvent` grows a variant per realm-decided UI feature. The Friends
  Server wire (ADR 0009) must carry each one; a variant the wire cannot encode is a bug.
- The realm sends the complete state on every change, so a client that joins a fight late or
  reconnects needs no replay: the next change, or the event sent on first sight, restores it.
- The client's UI layer (`assets/ui/layer.toc`) gains frames the stock interface never had.
  Each is ours, listed in the layer, and off under `WOW_STOCK_UI=1`.
- A realm test asserts the event's contents per character; a client Lua test asserts the frame
  against pushed tokens. Neither needs the other or the game data.
