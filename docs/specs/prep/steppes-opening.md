# Grill prep: the Burning Steppes open, server-wide

Draft for a grill on 2026-11-07. Not a spec yet. The lore bible says the Steppes stay under mist
until players break the Tower of Ilgalar Anchor around level 48-50, using Toren's translation of
Morganth's failure notes; then "the mist rolls off the pass and the Burning Steppes open"
(`docs/lore-bible.md`, "The mist and the Anchors", "The road to the Steppes", "Burning Steppes
and Blackrock"). This note asks how that works when several players share a server, and on a
solo save, and what the zone needs before anyone can play in it.

## 1. What exists today

- The Steppes are open and empty right now. `data/twilights-eve/closed_zones.toml` closes only
  Westfall (`zones = [40]`). Nothing stops a character walking from Redridge into the Steppes,
  or on into Searing Gorge, the Badlands, the Swamp of Sorrows or the Wetlands. None of those
  zones has spawns.
- Closed Zones (ADR 0015). `tools/twilights-eve/closed_zones.py` writes
  `data/twilights-eve/closed_chunks.json`, which both sides `include_str!` and hold in a
  `OnceLock`. Realm reader: `crates/twilight-realm/src/world/closed.rs` (`closed_at`). Client
  readers: `crates/benilla-formats/src/closed_zones.rs`, used by the ADT cut in
  `crates/benilla-assets/src/adt.rs`, the mist curtains in `crates/benilla-world/src/mist.rs`,
  the horizon in `crates/benilla-world/src/wdl.rs`, the minimap and `ui_world_map.rs`. The list
  is fixed at compile time. Nothing can open a chunk while the game runs.
- Placed mist walls (ADR 0033). `data/twilights-eve/mist_walls.json` holds vertical walls on any
  map. It has one today, the Deeprun tunnel. The realm refuses a step across one; the client
  draws it with the Closed Zones' shader. Also fixed at compile time.
- Server Unlocks (ADR 0031). `crates/twilight-realm/content/unlocks.toml` is the catalogue,
  `crates/twilight-realm/src/unlocks.rs` the state. The state file is `unlocks.toml` beside the
  saves: `<data-dir>/unlocks.toml` on the server (`crates/twilight-server/src/lib.rs`),
  `benilla-config/twilight/unlocks.toml` in solo (`crates/benilla-app/src/twilight/mod.rs`). The
  hosts read it into `Realm::set_unlocks`. The realm never writes it: a purchase does.
- Saves. The realm hands hosts `Storage::Write { guid, blob }` and `Storage::Remove { guid }`
  (`crates/twilight-realm/src/save.rs`); the server writes `characters/<guid>.toml`. There is
  no server-wide file the realm writes.
- The friends server (ADR 0009). Up to five players, always one group, kill and quest credit to
  everyone within 74 yards, the world ticks with nobody logged in, one shared Run per dungeon.
  A server-wide chat channel `/1` exists (`crates/twilight-realm/src/chat.rs`).
- Phased Dungeons (ADR 0006, 0029, 0032). The Tower of Ilgalar is dungeon 8, levels 34-38, a
  shell with one Phase Tester (`crates/twilight-realm/content/ilgalar.toml`). Blackrock
  Stronghold (14, levels 52-55) and Dreadmaul Rock (15, levels 55-58) are shells in zone 46
  already (`blackrock_stronghold.toml`, `dreadmaul_rock.toml`). The Altar of Storms (50-52) has
  no shell. Dungeon creatures and objects already appear and vanish on conditions (`Appears` in
  `crates/twilight-realm/src/phased.rs`: after a stop, a death, a lever).
- Encounters (#238, `crates/twilight-realm/src/encounters.rs`): waves at a point, started by a
  quest step.
- Audience (ADR 0014): a spawned thing only some characters see.
- Toren stands at Ridgepoint as a Hub NPC (`crates/twilight-realm/content/hub.toml`). There is no
  Redridge or Duskwood quest yet, so the failure-notes chain through the 40s is unbuilt.
- Zone imports. Redridge, Duskwood and Stormwind came in through
  `tools/twilights-eve/import_barrens_spawns.py --all-creatures` into
  `data/twilights-eve/*_spawns.json`, then `crates/twilight-realm/examples/import_zone.rs` into
  generated `content/<zone>.toml`, with hand changes and a Level Stretch in
  `crates/twilight-realm/content/zones.toml`. `docs/reference/zone-spawns.md` has the chain.
- Writs (`docs/specs/writs/README.md`) reserve three Burning Steppes Writs, "written by the spec
  that brings in the zone's mobs".
- The painted world map (#421) covers Elwynn, Stormwind, Duskwood and Redridge only.

### The border and the pass

A scan of the Azeroth ADTs (zone 46 rolled up through AreaTable) gives 4,661 Steppes chunks. The
zone touches Elwynn on 108 chunk edges, Redridge on 91, Searing Gorge on 89, the Badlands on 77,
the Wetlands on 21, the Swamp of Sorrows on 17 and Westfall on 8.

The Redridge border runs east to west along x = -8667, north of Render's Camp and Render's
Valley. Blackrock Pass (area 2417) is where the road crosses it, between y = -2483 and y = -2650.
The road climbs north-east through the pass (Flamekin at x -8500, y -2550, z 131) to Morgan's
Vigil at about (-8370, -2750, 187). Assumed: the rest of the Redridge and Elwynn borders are
mountains a character cannot climb, but the realm trusts reported positions (ADR 0009), so only
the realm's wall truly stops a walk.

### What the Steppes hold in vanilla

A dry run of the importer for zone 46 (vmangos 4641790, same flags as Redridge, written to the
scratchpad, not committed): 500 spawns, 67 entries, 418 hostile, 64 neutral, 18 friendly, 181
Scourge Invasion spawns dropped as game-event only. Hostile levels are 50 to 57 for almost all,
with a few at 59-60 (Volchan, Hematos, Gruklash). 58 spawns are level-1 critters.

| Area | Spawns | Who |
|---|---|---|
| Open Burning Steppes | 152 | Flamekin, Flamescale Dragonspawn, scorpids, worgs, Blackrock Soldiers, Black Wyrmkin |
| Dreadmaul Rock | 94 | Firegut Ogres and Ogre Mages (66), Krom'Grul, Gorgon'och |
| Terror Wing Path | 64 | Black Dragonspawn, Black Broodlings, Black Wyrmkin, Ember Worgs |
| Ruins of Thaurissan | 52 | War Reavers, Thaurissan Spies (Dark Iron) |
| Blackrock Stronghold | 33 | Blackrock Warlocks, Slayers, Battlemasters, Gruklash |
| The Pillar of Ash | 24 | Blackrock Slayers and Sorcerers |
| Draco'dar | 19 | Giant Ember Worgs, Flamescale Broodlings |
| Flame Crest | 18 | Horde outpost: Vahgruk, Gruna (hostile to Alliance), goblins, Spirit Healer |
| Blackrock Mountain | 14 | Flamescale Dragonspawn, Hematos |
| Morgan's Vigil | 10 | Alliance outpost: Marshal Maxwell, Borgus Stoutarm, Felder Stover and five more |
| Altar of Storms | 10 | five Blackrock Warlocks, two warlock-quest NPCs |
| Blackrock Pass | 8 | Flamekin, an Obsidian Elemental |

Some of the Blackrock Stronghold and Dreadmaul Rock spawns stand inside those dungeons'
footprints and come out, as Redridge's did for Ilgalar and Stonewatch. Assumed: a few dozen.

## 2. The design space for "server-wide"

Every option below needs the same base work, so its cost is listed once here:

- Hold the Steppes shut before the break. Zone 46 becomes a Closed Zone that a world fact can
  open, so the client draws no Steppes land and the realm refuses the step. The neighbours that
  the Steppes would expose (Searing Gorge 51, the Badlands 3, the Swamp of Sorrows 8, the
  Wetlands 11) close for good in the same edit.
- Make the closed list change at run time on both sides. The realm's `closed_at` and the client's
  five readers consult "closed in the file, and not opened yet". The client reloads the loaded
  tiles of an opened zone so terrain, doodads and water appear. This is the biggest single cost.
  Assumed: medium, because the client already loads tiles by distance and the cut happens at
  parse time.
- Today's content test ("our content stays on open land", ADR 0015) must allow content on land
  that is shut until a fact opens it, or the two dungeon shells and the import fail it.
- Import the Steppes spawns (section 4, ticket 6).

### Option A. The first break opens it for everyone, forever

The first group to break the Anchor sets a server-wide fact. Every client opens the Steppes at
once, and every later login finds them open.

- Cost: one new realm-written state file beside the saves and a `Storage` variant for it, read by
  both hosts like `unlocks.toml`. One session event carrying the opened facts at login and on a
  change. Small on top of the base.
- Behind players lose the gate but keep the zone. A level-35 friend can walk into level-50
  Flamekin, as in vanilla.
- Solo: a one-player server. The solo roster shares the file, so an alt finds the Steppes open.
- A new character on an opened server sees open land and a broken Anchor at Ilgalar.

### Option B. Opened for the server, but each character still needs the quest to cross

The first break opens the land (drawn for everyone), but a character who has not finished the
breaking quest, or a shorter follow-up, still meets the wall.

- Cost: option A, plus a per-character check in the realm's wall and in that client's cut. Each
  client draws only its own character, so the client side is cheap. The realm side is not: it has
  to hide creatures and other players on land closed for the viewer, or a gated player sees a
  friend float over nothing.
- Splits a group at the pass until the behind player catches up.

### Option C. Per-character phasing, no server state

Each character breaks its own Anchor. The Steppes open per character, like Wrath's phasing.

- Cost: no world file, the flag lives in the save. But the realm must hide entities on land closed
  for the viewer (as in B), and the Ilgalar Run is shared (ADR 0006), so the Anchor has to be
  whole for one player and broken for another in the same Run. That needs an Audience-style rule
  keyed on quest state. Medium to large.
- Matches solo story beats best. Weakest "we did this together" moment on a server.

### Option D. A server event with a shared progress bar

Players feed a server-wide counter (turned-in failure-note pages, cultists killed at Ilgalar,
Tribute crates burned). At the target the break becomes a scheduled event at the tower, called on
`/1`, and the land opens for good.

- Cost: option A, plus the counter, the contribution rules, a progress frame (the Boss Bar and
  Noise Bar frames are the model, ADR 0013) and a scheduled Encounter. The target has to scale
  with how many play, or a quiet server stalls. Large.
- Solo: the bar fills from one player's work and reads as a grind.

### Option E. Option A with an aftermath quest for latecomers

As A, plus a short non-gating quest from Toren for any character who did not take part: walk to
the broken Anchor, read the Dark Iron forge stamp, hear what was lost. It gives the story and the
XP without a wall.

- Cost: option A plus one quest row and one GameObject state. Small.

### Can it close again?

- Never in play. A host deletes the fact from the file to reset a test server. Cheapest.
- A recurring re-sealing (the cult retunes the Anchor every few weeks, players break it again).
  Needs the mist to roll back in over players standing in the Steppes, which the wall cannot do
  safely today (ADR 0015 refuses moves onto closed land, so a character inside would be stuck).
  Large.

## 3. Open decisions, most important first

1. Who does the break open the Steppes for?
   - (a) Everyone on the server, at the first break (A or E).
   - (b) The server, but each character still needs a quest to cross (B).
   - (c) Each character only (C).
   - (d) Everyone, after a shared progress bar fills (D).
   - Recommended: (a). Five friends make it a shared moment, and it costs least on top of the base.

2. What does a character who missed the break get?
   - (a) Nothing; the land is just open.
   - (b) A short non-gating aftermath quest from Toren (E).
   - (c) Toren's full chain as an attunement before crossing (B).
   - Recommended: (b). It keeps the story for latecomers and alts without splitting a group.

3. Can the Steppes ever close again?
   - (a) Never in play; a host or test resets by editing the file.
   - (b) A recurring re-sealing event.
   - Recommended: (a). Re-sealing traps characters inside, and nothing in the lore asks for it.

4. Where does the opened fact live?
   - (a) A new realm-written file beside the saves (working name `milestones.toml`, with a
     catalogue in `content/milestones.toml`), written through a new `Storage` variant.
   - (b) A realm-written entry in `unlocks.toml`.
   - (c) In each character's save.
   - Recommended: (a). ADR 0031 says a purchase writes `unlocks.toml`; mixing in realm writes
     breaks that, and (c) is option C.

5. How is the Steppes held shut before the break?
   - (a) Zone 46 is a Closed Zone a fact can open: no land drawn, mist curtain at the border.
   - (b) The land stays drawn, with one placed mist wall across Blackrock Pass.
   - (c) Both: closed land, plus a wall at the pass that fades on the break.
   - Recommended: (a). The pass is 6 of the zone's 411 land border edges, Blackrock Mountain links
     on to Searing Gorge, and a curtain that rolls back is what the bible describes.

6. Do Searing Gorge, the Badlands, the Swamp of Sorrows and the Wetlands close for good now?
   - (a) Yes, with Westfall in `closed_zones.toml`.
   - (b) Only Searing Gorge, the one with a road out of the Steppes.
   - (c) Leave them open and empty.
   - Recommended: (a). The bible puts all of them behind mist, and today anyone can walk there.

7. Where is the Anchor broken?
   - (a) Inside the Ilgalar Run: a quest from Toren starts an Encounter at the Anchor, with its own
     48-50 waves, and the last wave's end sets the fact.
   - (b) An open-world Anchor at the tower's foot.
   - (c) A new Phased Dungeon at 48-50 on the tower top.
   - Recommended: (a). Encounters and quest-started waves exist, the Run is shared, and the Head
     Count scales the waves. Assumed: the Anchor stands in a footprint box Ilgalar already has.

8. Who gets credit for the break?
   - (a) Everyone within 74 yards when it breaks (ADR 0009's rule).
   - (b) Everyone in Ilgalar's Phase.
   - (c) Everyone online.
   - Recommended: (a). It is the rule for every kill and quest today; the opened land covers the
     rest.

9. What do players online see at the moment of the break?
   - (a) The land appears at once, with a line on `/1`.
   - (b) The border mist fades over some seconds and the land appears behind it, with a line on
     `/1` and a yell from Toren.
   - (c) A countdown event that players gather for.
   - Recommended: (b). The mist shader exists, and a fade is small next to the tile reload.

10. On solo, does one character's break open the Steppes for the other characters in the roster?
    - (a) Yes, the solo roster is one server (ADR 0031's rule).
    - (b) No, solo keeps it per character.
    - Recommended: (a). One rule, one file, and the aftermath quest gives alts the story.

11. Can the break ship before Toren's failure-notes chain through the 40s exists?
    - (a) Yes: Toren gives a stand-in "translation complete" quest at 48, and the chain slots in
      before it later.
    - (b) No: build the Redridge and Duskwood chain first.
    - Recommended: (a). The opening mechanism is independent, and the chain is a whole arc.

12. What level range do the Steppes run at?
    - (a) Vanilla levels as they are (50-57, rares to 60).
    - (b) A Level Stretch from 50-57 onto 50-60.
    - (c) A Level Stretch from 50-58 onto 50-60.
    - Recommended: (b). The bible's band is 50-60, and Mourncrown at 60 needs something near 60.

13. What happens to the outposts?
    - (a) Flame Crest's Horde NPCs go; Morgan's Vigil's stock NPCs become Hub NPCs (the Goldshire
      pattern); the goblins and the Spirit Healer stay.
    - (b) Flame Crest's orcs turn hostile, as Blackrock.
    - (c) Import both as they are.
    - Recommended: (a). The game is Alliance-side, and Gruna and Vahgruk are hostile to Alliance.

14. Is there any level gate at the pass after it opens?
    - (a) None.
    - (b) A scout at the pass who warns a low character.
    - (c) A hard level gate.
    - Recommended: (b). Vanilla had none, and a warning costs one Hub NPC line.

## 4. Ticket sketch

Assumed: recommendations 1(a), 2(b), 3(a), 4(a), 5(a), 6(a), 7(a). Items 4 and 5 change a
generated file's schema and cross the client/realm seam, so they need the grill before tickets.

1. Realm and tools: shut-until zones. `closed_zones.toml` gains zones that a named fact opens
   (zone 46 opened by `ilgalar_broken`), and the closed-for-good list grows by 51, 3, 8 and 11.
   `closed_zones.py` writes each chunk's gate key into `closed_chunks.json`. The realm's
   `closed_at` asks the opened facts. The content test lets content stand on shut-until land.
   Tests: a point at Morgan's Vigil is refused before the fact and allowed after.
2. Realm: Milestones. A catalogue in `content/milestones.toml` (key, name, the `/1` line), a
   realm-written state file `milestones.toml` beside the saves through a new
   `Storage::Milestones { blob }`, `Realm::set_milestones(text)` read by both hosts at start,
   and a session event that names the opened facts at login and on a change. A dev command or
   probe sets one. Tests on the realm clock and a host round trip.
3. Client: run-time closed land. `benilla-formats`' closed reader takes the opened facts from the
   session event. The ADT cut, mist curtains, horizon, minimap and world map read it. An opened
   zone's loaded tiles reload, and the border curtain fades. Background test build.
4. Realm and data: the break. An Anchor GameObject in `ilgalar.toml`, a quest from Toren
   (Ridgepoint) at 48 with a stand-in prerequisite, an Encounter of 48-50 waves at the Anchor,
   and a quest step that sets `ilgalar_broken`. `Appears` gains `after_milestone` and
   `until_milestone`, so the whole Anchor gives way to a broken one on every host.
5. Data: the aftermath quest from Toren for any character who did not get the break credit, and
   the pass scout's warning line.
6. Data and import: Burning Steppes spawns at 50-60. Run `import_barrens_spawns.py --map 0
   --zone 46 --box -9000 -6800 -3600 200 --all-creatures` into
   `data/twilights-eve/burning_steppes_spawns.json`; add the zone to `import_zone.rs`'s `ZONES`;
   add a `[[zone]]` to `zones.toml` with the Level Stretch, `removed_guids` for the Blackrock
   Stronghold and Dreadmaul Rock footprints and Flame Crest's Horde NPCs; move Morgan's Vigil's
   NPCs to `hub.toml`; update the counts in `docs/reference/zone-spawns.md`. Gates:
   `ground_check.py` and `icon_check.py`. Assumed: the importer needs no change, since the dry
   run worked with Redridge's flags.
7. Data: three Burning Steppes Writs and the Steppes' map marks.
8. Davis (ready-for-human): paint the Steppes onto the world map canvas, and list the art in
   `docs/art-assets.md`.

Later, not in this spec: the Altar of Storms shell (50-52), Toren's failure-notes chain through
the 40s, Blackrock Mountain's interior and its instance doors.

## Grill answers (Davis, 2026-11-07/08)

1. Who opens: a SHARED PROGRESS BAR (option D), shaped as follows.
   - The bar starts when the first group breaks the Ilgalar Anchor. The break does not open the land by itself.
   - What fills it: NEW Writs and bounties at a camp at Blackrock Pass that appears after the break (holding back what comes out of the thinning mist). Nothing else counts.
   - Size: fixed and big, the same goal on every server, a server-wide push even if solo takes long. The target is a content tunable. Assumed: a number a solo player fills in about 8 to 10 hours of pass bounties.
   - Shown: no frame on screen. Players ask an NPC near the Writ Board in Stormwind and one in Redridge, who say how close it is.
   - When it fills, the Steppes open for everyone on the server.
2. Latecomers: NOTHING. No aftermath quest; the land is just open.
3. Re-close: never. Only a host or a test resets it by editing the file.
4. Storage (Claude, a fix): (a) a realm-written file beside the saves (working name milestones.toml, catalogue in content/milestones.toml), apart from unlocks.toml, which only purchases write (ADR 0031). It holds the break, the bar's count and the opened fact.
5. Held shut: (a) zone 46 is a Closed Zone a fact can open: no land drawn, mist curtain along the whole border; it rolls back when the bar fills.
6. Neighbours: (a) Searing Gorge, the Badlands, the Swamp of Sorrows and the Wetlands join Westfall in closed_zones.toml for good.
7. The break: (a) inside the Ilgalar Run: a quest from Toren starts an Encounter at the Anchor, 48-50 waves scaled by the Head Count; the last wave's end records the break and starts the bar (it does not open the land).
8. Credit (Claude, existing rule): everyone within 74 yards when the Anchor breaks (ADR 0009). Server-wide, the bar is what matters.
9. The opening (when the bar fills): (b) the border mist fades over some seconds, the land appears behind it, a line on /1 and a yell from Toren.
10. Solo: (a) the solo roster is one server; one character's work counts for the alts.
11. Stand-in (Claude): yes. The break quest names 98521 "The Unsent Letter" (Redridge/Duskwood arc ticket 30) in `after`, but may ship first with a stand-in "translation complete" quest from Toren at 48 that the arc replaces.
12. Levels: sub-zone Level Bands spanning 50-60, the same mechanism as Redridge and Duskwood (that arc's Level Bands code ticket).
13. Outposts: (b) Flame Crest's orcs turn hostile, as Blackrock. Morgan's Vigil's stock NPCs become Hub NPCs (Goldshire pattern); goblins and the Spirit Healer stay.
14. Level gate: (a) none.
15. Opening voice (Davis, after filing): the pass camp's captain yells at the opening; Toren is already taken (comment on #630).
16. Altar of Storms (Davis, after filing): becomes a later dungeon, about 56-58, keeping vanilla levels around it; the lore bible changes (comment on #634).
