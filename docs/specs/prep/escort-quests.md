# Escort quests: grill prep

For the grill on #596 (escort quests), which blocks #620 (Keeshan, "Missing in Action", 98391).
A draft, not a decision. Paths are under `crates/twilight-realm/` unless they start `docs/`.

## 1. What exists to build on

- **No `bevy_ecs` yet.** The realm's `Cargo.toml` has no `bevy_ecs` dependency, and `Mob` is a
  plain struct with optional fields (`src/mobs.rs:1217`: `audience`, `hunts`, `frozen`, `fleeing`).
  ADR 0009 kept the move to `bevy_ecs` as a separate project. An escort follows the same pattern:
  an optional field on `Mob` plus one module, not a component family.
- **Quest objectives.** `Quest` in `src/quests/content.rs:342` has kill, talk, use, collect,
  explore and discover, and `fail_if_seen` (`:412`). The log has four 6-bit counters per quest
  (`Active.kills`, `src/quests/mod.rs:46`), saved as is (`:112`). A talk objective sets its counter
  to 1 (`credit_talk`, `src/quests/mod.rs:340`), so an escort's "arrived" can use a counter too.
- **Quest failure.** `Active.failed` (`src/quests/mod.rs:51`) blocks credit and hand-in until the
  quest is abandoned. `seen()` (`:478`) sends `QuestFailed` and the log's failed state;
  `end_rumour()` (`:451`) removes a quest from the log outright.
- **Talk start.** `talk_options` and `credit_talk` (`src/quests/mod.rs:307`, `:340`) give a quest
  gossip line on a friendly creature. "I'm ready" on Keeshan can be that line.
- **Encounters** (`src/encounters.rs`, `content/encounters.toml`, #238). Waves of creatures at a
  fixed anchor, started on accept-and-arrive or on a use objective (`Start`, `:27`). A run fails
  when no living quest holder is within `leash` of the anchor (`:217`). New creatures turn on the
  nearest quest holder (`:230`). The realm ticks it once for all sessions (`tick_encounters`,
  `src/lib.rs:1662`). Rites at the Stone (94502) is the shipped user. The wave spawn code
  (`:259`-`:287`) is what an ambush needs.
- **Creature walking.** `Mob::walk_path` (`src/mobs.rs:2010`) sends one `MonsterMove` spline through
  a list of points at walk speed; `run_path` (`:1978`) runs it. The tram walks porters point to
  point with `walk_toward` and `there` (`src/tram.rs:1498`, `:1504`, private today). Hub NPCs
  wander with their own `Walk` (`src/hub.rs:366`).
- **Leash and evade.** `rules::leash::LEASH_YD` = 50, measured from `Mob.home`
  (`src/rules/leash.rs:5`). `evade()` (`src/world.rs:2153`) runs a mob back to `home`, and
  `State::Evading` heals it on arrival (`src/world.rs:1326`). An escortee's `home` must move with
  the path, or a lost fight sends Keeshan back to his hideout.
- **Mobs fight players only.** `Mob.target` and the threat table hold player guids. `mob_tick`'s
  `Engaged` arm looks targets up among the session's player and `others` (`src/world.rs:1336`-
  `:1360`). `engage()` (`src/world.rs:2083`) takes a player guid. Friendly creatures are never
  attackable (`Mob::attackable`, `src/mobs.rs:1866`). Ally skills land on players only
  (`src/world/cast.rs:3646`). **Nothing lets a creature hit or be hit by a creature.** This is the
  largest new piece.
- **Mob ticks run per session** (`self.runs(guid)`, `src/world.rs:903`). An escort walking where no
  session runs its mob needs the realm-level tick that Encounters use.
- **Audience** (ADR 0014, `src/audience.rs`, `Shard::spawn_for` `src/shard.rs:118`,
  `despawn_audience` `:141`). It makes a creature seen by a set of characters only. It narrows
  visibility and never hides a shared spawn from someone, so a per-group copy of Keeshan would
  still leave the shared Keeshan visible to that group.
- **Shared credit.** `group.toml` `credit_range_yd = 74.0`; `share_kill` (`src/lib.rs:1242`) credits
  every player on the map within range, alive or dead.
- **Speech.** Creature `say` lines (`src/mobs.rs:175`) go out through `Realm::creature_speech`
  (`src/chat.rs:155`). An escort's lines can use the same path.
- **Level.** Friendly creatures skip the Level Stretch (`src/mobs.rs:739`-`:747`), so Keeshan stays
  at his dump level, 25, unless the escort sets one. Head Count (ADR 0029) only touches Phased
  Dungeons.
- **Keeshan today** is spawn guid 17874 in `content/redridge.toml:666` (generated, do not edit):
  entry 349, level 25, display 1826, at (-8756.77, -2187.12, 139.975).
- **Hub NPCs are never quest-gated.** `hub::in_view` takes only a position (`src/hub.rs:124`,
  `:433`). `shard::in_view` already takes the viewer (`src/shard.rs:518`).
- **Paths.** `tools/twilights-eve/road_path.py` traces a cart road along road textures and writes
  `carts.toml`. It fits a cart, not a canyon walk, but its Z step (`Terrain.ground_z`) helps.

## 2. What vanilla did (quest 219)

- Keeshan himself gave 219 (`creature_questrelation`), and Marris (382) took it in. Level 25.
- On accept (`quest_start_scripts` 219): a scripted map event with a 1200 s (20 min) limit, his
  quest giver flag removed, his faction set to escortee, and his waypoints started.
- `creature_movement_template` 349: 68 points, 1247 yards of walking (532 yards in a straight
  line), about 500 s at walk speed plus 88.5 s of waits. Point 1 waits 8 s and says "my wounds are
  grave". Point 25 sits 45 s ("I need a moment to rest"). Point 26 stands up ("Let's go!"). Point
  43 at (-8914.7, -2579.3) says "So close to blessed Lakeshire" and **switches him to a run**.
  Point 67 reports to Marris, ends the event at +10 s, and despawns him at +30 s.
- **There was no scripted ambush.** The fights were the canyon's own Blackrock spawns, plus a say
  on aggro (`creature_ai_events` 34901). #596 and #620 say point 43 "runs a script (an ambush)".
  It is a say and a run. Our ambush at 43 is new design, which is fine, but nothing copies it.
- The dump's failure conditions (21901, 21902) are missing from `conditions`. Assumed: vanilla
  failed on Keeshan's death or the time limit, and the escort AI failed when no escorting player
  stayed within about 50 yards.

Other vanilla escorts in the 30-50 band (`script_waypoint` or start-waypoint quests): Kinelory
(660, Arathi, 37), Professor Phizzlethorpe (665, Arathi, 40), "Stinky" Ignatz (1222, Dustwallow,
37), Galen Goodward (1393, Swamp of Sorrows, 38), Melizza Brimbuzzle (6132, Desolace, 39), Dalinda
Malem (1440, Desolace, 33), Cork Gizelton's caravan (5821, Desolace, 35, 282 points), Willix (1144,
Razorfen Kraul, 30), Belnistrasz (3525, Razorfen Downs, 37, a wave defence), OOX-22/FE (2767,
Feralas, 45), Rin'ji (2742, Hinterlands, 47), OOX-17/TN (648, Tanaris, 48), OOX-09/HL (836,
Hinterlands, 48), Dorius Stonetender (3367, Searing Gorge, 48), Celebras (7046, Maraudon, 49).
None is in Elwynn, Stormwind, Duskwood or Redridge besides Keeshan. Dorius borders the Steppes
opening. Our own unbuilt one is Elwynn's 94901 (a supply wagon to Westbrook through two gnoll
ambushes, `docs/specs/elwynn-10-30/ELWYNN_10-30_QUEST_DESIGN.md:192`), which may suit a Cart
better than a walker.

## 3. What an escort needs

- **Start.** The holder of the quest speaks to the escortee (a gossip line), or comes within a
  radius of him. He says a line and sets off.
- **Path.** A list of points from his spot to the end, with waits and lines at some points. His
  `home` follows the last point reached, so an evade returns him to the path.
- **Pace.** He walks. He waits when no escorter is near and goes on when one comes back.
- **Ambush.** At point N, a wave of creatures spawns at set spots, once per escort, and attacks.
  The Encounters wave code spawns them; they are cleared on success, failure or reset.
- **He fights.** He swings at whatever attacks him. He needs a level, health and damage that a
  band-42 ambush does not kill in seconds.
- **Failure.** He dies; or no living escorter is within the leash for some seconds (leaving,
  dying, logging out and the 20 s logout rule all count); or a time limit, if there is one.
- **Credit.** Every holder of the quest within range when he reaches the end.
- **Other players on a server.** Whether they see him walk, can help, can start their own.
- **Reset.** After success or failure he vanishes and stands at his spot again after a delay. The
  quest can be retried without a long walk back.
- **Tests.** A realm test walks an escort to the end on the realm clock; one kills him and checks
  the fail; one checks the ambush comes once.

## 4. Open decisions, most important first

1. **Can creatures hurt the escortee, and does he fight back?**
   (a) Two-way: a creature can target him and he melees back, which is new code in threat, chase
   and swing. (b) One way: ambushers hit him, he does not swing. (c) He cannot be hurt; the escort
   fails only on leash or time.
   Recommended (a), melee only: #620's test is "Keeshan dying fails 98391", and the same target
   rule serves later escorts, guards and Belnistrasz-style defences.
2. **Who sees the walking escortee on a server?**
   (a) The one shared spawn walks; everyone sees him; one escort at a time; a later player waits
   for the reset. (b) A copy per starting group, made for an Audience, with the shared spawn
   hidden from that group (a new "hidden from" rule). (c) A copy per character.
   Recommended (a): ADR 0009 puts everyone in one group, and ADR 0014 has no way to hide a shared
   spawn from a few.
3. **Who gets credit when he arrives?**
   (a) Every holder of the quest within 74 yards of him, alive or dead (ADR 0009's rule). (b) Only
   the character who started him. (c) Every holder who was within range at any point of the walk.
   Recommended (a): it is the rule kills use, and `credit_range_yd` already holds the number.
4. **What happens after a failure?**
   (a) The quest goes to "Failed"; abandon it and take it again from Marris, as `fail_if_seen`
   does. (b) The quest stays in the log, not failed, and a word with Keeshan restarts the walk.
   (c) It fails, and Keeshan offers a restart that clears the fail.
   Recommended (b): Marris stands 532 yards away, and vanilla's retry was at Keeshan, who gave 219.
5. **How does he move when the escorters fall behind or run ahead?**
   (a) Fixed path; he waits when no escorter is within about 20 yards; the escort fails after 10 s
   with none within 60 yards. (b) Fixed path, he never waits. (c) He follows the character.
   Recommended (a): it reads like vanilla, and the Encounters leash rule already works this way.
6. **What starts the walk?**
   (a) A gossip line on Keeshan once 98391 is in the log ("I'm ready. Lead the way."). (b) Coming
   within a radius of him with the quest. (c) Accepting the quest from Marris.
   Recommended (a): Marris is far from Keeshan, and a gossip line lets the player get ready first.
7. **Where do the path, lines and ambush live?**
   (a) A new hand-written `content/escorts.toml`, one `[[escort]]` per quest, beside
   `encounters.toml`. (b) Fields on the quest row. (c) On the creature's template.
   Recommended (a): Encounters set the pattern of a content file keyed by quest, and the generated
   zone files cannot be edited.
8. **Which path for Keeshan?**
   (a) Vanilla's 68 points with the 45 s rest kept. (b) Vanilla's points cut to about 30, rest
   shortened to 10 s. (c) A new path drawn by a tool.
   Recommended (b): the route is proven on this terrain; 9 minutes plus a 45 s sit is long for a
   level-42 quest.
9. **What is the ambush?**
   (a) One wave of 3 Blackrock (435 or 615) at point 43, opening on Keeshan. (b) Two waves (point
   25 and 43). (c) No ambush; the canyon's spawns are enough, as in vanilla.
   Recommended (a): #620 asks for one, and opening on Keeshan makes the player pull threat off him.
10. **How strong is Keeshan while escorted, and can players heal him?**
    (a) Level 42, extra health, no heals from players. (b) Level 42, normal health, players can
    heal and buff him (new ally targeting on creatures). (c) His dump level, 25.
    Recommended (a): ally skills land on players only (`cast.rs:3646`), and heals can come later.
11. **Keeshan at the Stonewatch camp afterwards.**
    (a) Everyone sees the camp Keeshan, and everyone sees the Render's Camp Keeshan. (b) The camp
    Keeshan only for those who handed in 98391, and the Render's Camp one only for those who have
    not (one per-viewer quest-state rule for creatures and Hub NPCs). (c) Only the camp one is
    gated.
    Recommended (b): it is one small viewer filter, and it avoids two Keeshans for one character.
12. **Does the log show anything, and is there a time limit?**
    (a) Vanilla: no counter, no timer; the log turns complete on arrival. (b) A counter line
    "Corporal Keeshan escorted 0/1". (c) (a) plus a 20 min timer in the log's timer field.
    Recommended (a): the leash already ends a stalled escort, and vanilla showed nothing.

Settled by the above, so not asked: saving holds no escort state (a logout counts as leaving,
and the 20 s logout rule applies), and the arrival is one log counter set to 1, saved as `kills`.
Head Count and the Tier Step do not touch him (open world).

## 5. Ticket sketch

Assumed: the recommended answers. Each ticket has tests first in `tests/escorts.rs`, using test
quests through `Realm::with_quests`, as `tests/encounters.rs` does.

1. **Realm: creatures fighting creatures.** A creature can be a threat entry and a target:
   `engage`, the threat table, chase and swing take a unit guid that may be a creature; a friendly
   creature can be attacked by a hostile one; it swings back at its attacker. A creature killed by
   a creature shares credit with players in range as `share_kill` does. Tests: a hostile spawned
   beside a friendly fights it to the death; neither notices a player who does nothing.
2. **Realm: the escort walker.** `src/escorts.rs` and `content/escorts.toml` (`key`, `quest`,
   `spawn` guid, `level`, `health_mul`, `start_option`, `path` of points with optional `wait_s`,
   `say` and `run`, `wait_yd`, `leash_yd`, `leash_s`, `reset_s`, `[[escort.ambush]]` at a point
   with creatures as Encounters write them). A realm-level tick beside `tick_encounters`. The
   start gossip line, the walk with `home` moving along the path, waiting, the ambush (Encounters'
   wave code moved to a shared helper), arrival credit to holders within 74 yards, failure on
   death or leash, the reset. The `quests.toml` header gains the escort's line. Tests: end to end
   on the realm clock; death fails; leash fails; the ambush comes once; a retry after a failure.
3. **Realm: per-viewer quest-state visibility** (only if decision 11 is b). One field on a spawn
   row and a Hub NPC row, read in `shard::in_view` and `hub::in_view` (which gains the viewer).
4. **Data: Keeshan.** This is #620 as written, after tickets 1 to 3. A tool imports and thins the
   vmangos points; `ground_check.py` checks the ambush spots.

Client: none. Movement is `MonsterMove`, lines are creature speech, failure is `QuestFailed` and
the log's failed state, all of which the client already handles. Assumed: the client draws a
friendly creature in combat with a hostile one without changes, since both are ordinary units.

**ADR: yes.** Creatures that fight creatures changes ADR 0009's "mobs fight players" model, and
the shared-escortee choice (decision 2) is a visibility rule next to ADR 0014. Assumed number:
0035, since `docs/specs/steppes-opening/adr-draft.md` claims 0034.

## Grill answers (Davis, 2026-11-08)

1. Server: OWN COPY EACH. Each player (with their group, see 4) gets a private Keeshan. This needs per-viewer visibility in the realm (a creature or Hub NPC seen only by some players), which also serves the camp Keeshan shown after a rescue.
2. He fights: YES, and he is HEALABLE by players.
3. Failure: he dies, or no escorter is near him for a few seconds. He resets at his hideout and a word with him restarts the walk; no trip back to the quest giver. No time limit.
4. Group: the group shares the escort. Group members see that Keeshan and its ambushes, can fight, and every member holding the quest gets credit when he arrives.
5. The walk: SEVERAL scripted ambushes along a fixed path (two or three), plus whatever canyon mobs join in. He waits when escorters fall behind.
Claude's readings: a gossip line starts it; path, lines and ambushes in content/escorts.toml; vanilla's 68 points thinned to about 30; Keeshan level 42 with extra health; no quest log counter; an ADR (number after the Steppes draft's 0034).
