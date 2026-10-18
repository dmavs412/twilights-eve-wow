# A friends server: the realm behind a socket, for up to five players

Twilight's Eve gets a multiplayer mode for Davis and up to four friends, hosted on his PC or a
cheap VPS. A new headless binary, `twilight-server`, owns one `Realm` and its world; clients
connect to it over TCP. Solo play stays as it is: the realm in-process in the client, now as the
one-session case of the same multi-session realm. Multiplayer is a launch option naming a server
address.

The seam itself travels: `RealmCommand` in, `SessionEvent` out, serialized with serde and bincode,
length-prefixed on a TCP stream. The client's 1.12.1 packet stack is not used between client and
server. Only this client can play the custom rules anyway, so real packets would buy nothing and
cost an encoder per message type forever.

The realm becomes multi-session before any networking exists: `InWorld` splits into per-player
state, keyed by a session, and shared per-map state (mobs, population, runs). That is a split of
the existing structs, not the move to `bevy_ecs` that `docs/bevy_guidelines.md` asks for; that
move stays a separate project.

## Rules settled with Davis (2026-10-18)

- Everyone is in one group by default; there is no party invite.
- At a mob's death, every player within 74 yards (a content value), alive or dead, gets kill and
  quest credit, XP split by vanilla's party formula, and a personal roll of the loot table.
- Mobs use a vanilla threat table: damage 1:1, healing at half split across the healed player's
  attackers, the 110% melee / 130% ranged pull-over.
- Players are always friendly to each other; heals and buffs can target another player. Duels are
  the only PvP.
- The server trusts each client's reported position.
- Logging out takes 20 seconds outside an inn or city; a dropped connection leaves the character in
  the world for 20 seconds, then saves and removes it.
- The world keeps ticking with nobody logged in. ADR 0006's one shared Run per dungeon holds as is.
- Accounts register on first login through SRP, gated by a server-wide join password from the
  server's config. Accounts and characters are plain files: `accounts.toml` and one save blob per
  character, the blob solo already writes. Server and solo rosters are separate; no import.
- A client whose build id (git commit and a hash of the content files) differs from the server's
  is refused at login, with both ids in the message.
- Chat (say, yell, whisper, a server-wide channel), emotes, party frames for the other players,
  and trading are in scope.

## Considered Options

- **Real 1.12.1 packets on the wire.** Rejected: an encoder per event type, paid again by every
  feature, for compatibility with stock clients that cannot play the custom rules.
- **A listen server in Davis's client.** Rejected: the world exists only while he plays, and his
  client streams terrain only around him, so friends elsewhere would have no ground or line of
  sight.
- **Line of sight always true at first.** Rejected by Davis: the server builds real collision
  (terrain, doodads, WMO walls) without the renderer before the first session together.
- **Moving the realm to `bevy_ecs` as the multi-session refactor.** Rejected for now: far bigger,
  and it keeps the Callings tickets paused longer.
- **Contribution-based credit, or XP in full to each.** Rejected: range alone decides who counts,
  and XP divides as in vanilla.

## Consequences

- `twilight-realm` gains serde (and bincode at the server and client hosts).
- Terrain height, area and collision must load from game data without Bevy's app or renderer; the
  height and area math is already in `benilla-formats`, collision is not.
- Every system that means "the player" means "a player"; events carry the session they go to.
- Friends supply their own 1.12.1 game data and a client zip of the server's exact build.
- Order: M1 multi-session realm in-process; M2 server, LOS, accounts, saves, version check, chat,
  emotes, logout rule; M3 threat, 74-yard credit, XP split, personal loot, party frames, friendly
  targeting; M4 trading and duels.
