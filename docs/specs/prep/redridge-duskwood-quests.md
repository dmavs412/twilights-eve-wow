# Grill prep: the Redridge and Duskwood quest arc, 30 to 50

Draft for a grill, written 2026-11-07 in a read-only pass. Inputs: `docs/lore-bible.md` (rev 53),
`docs/specs/elwynn-10-30/`, `docs/specs/story-rewrite/`, `docs/specs/stockade/README.md`,
`docs/specs/deeprun-tram/README.md`, the content files named below and the vmangos world DB.
The Steppes side of the Anchor break is in `docs/specs/prep/steppes-opening.md`; this draft only
feeds it. Every guess is marked "Assumed:".

## 1. What exists today

- **Spawns, no quests.** `crates/twilight-realm/content/redridge.toml` and `duskwood.toml` are
  generated from vmangos and never edited. `content/zones.toml` lays a linear Level Stretch on
  them: Redridge 14-27 and Duskwood 18-35 onto 30-50 (spec #466). It also removes the creatures
  inside the four dungeon footprints. Friendly vanilla NPCs keep their levels, and any friendly
  spawn is already a Field Giver a quest can name (`quests/content.rs`, `friendly_spawn`).
- **What the stretch does to the dungeon approaches.** Redridge runs west to east: Three Corners
  30-33, Lakeshire and the lake 33-38, the Canyons 35-42, Render's Valley 38-45, Render's Rock
  42-47, the Shadowhide gnolls around Ilgalar 41-48. Duskwood: the north woods 30-35, Tranquil
  Gardens 34-36, Raven Hill Cemetery 37-50 (Mor'Ladim 50), Vul'Gol ogres 38-45, the Nightbane
  worgen 39-45, Roland's Doom 43-45, Fetid Corpses beside Manor Mistmantle 43-44.
- **Four dungeon shells**, one friendly Phase Tester each: `content/ilgalar.toml` (id 8, 34-38),
  `content/mistmantle_manor.toml` (id 10, 32-36), `content/raven_hill_deeps.toml` (id 11, says
  44-48; the bible says 40-44), `content/stonewatch_keep.toml` (id 9, says 44-48; the bible says
  45-48).
- **Stormwind.** `content/quests_stormwind.toml` holds the Stockade's twelve quests, 97301-97312
  (30-40). 97301 is Dughan's hook at 30, and 97311 hands the Writ of Names to Dughan at 40, which
  the bible makes the start of the 40s court chain. The Deeprun Tram (38-42, #586) is built up to
  #592; #593 (Brannoc's "Down the Line") is open. The Writ Board (`content/writs.toml`) holds
  three Redridge Writs (33, 40, 47) and three Duskwood Writs (33, 40, 47) at 95001-95007.
  Commander Aldous Wray (Tier 2 trainer) stands in the barracks (`content/hub.toml`). Spec #556
  (Tier 2 Calling Choice in Stormwind) is open and ungrilled.
- **Where Elwynn's story stops.** `content/quests_elwynn.toml` ends at 94804 "Burn the Boats"
  (26). The Elwynn plan's Hogger's Camp quests (95001-95006) and the exits to Lakeshire
  (95101-95103) were never built: spec 5 (#227) closed without tickets. Their ids are now taken
  by the Writs, and 94901-94906 by Rumours. Nothing sends a character to Redridge or Duskwood.
- **Toren.** Two Hub NPCs named Toren: one at the Tower of Azora, one in Ridgepoint's cellar
  (94704 frees him). Nobody has placed him past 23.
- **Quest systems built** (the `quests.toml` header): kill and family rows, collect with drop
  chances, talk, use a GameObject, explore, `fail_if_seen`, several prerequisites, Quest Starters,
  wave defense (94502), item choices, Rumours, Writs, bounties. Not built: escort and subdue
  (spec 5's). Loot Bands stop at 27-30. The loader reads four quest files by name; a new file is
  one line in `shipped()`.

### Vanilla quests worth reusing

Redridge (zone 44, 36 quests, levels 14-27) all hang off Lakeshire:

- Magistrate Solomon's "Messenger to" quests (Stormwind, Darkshire) are the vanilla model for
  this arc's cross-zone hand-offs. The Westfall one goes.
- Marshal Marris (Blackrock Menace, Shadow Magic, Tharil'zun), Keeshan's Missing in Action
  (escort), Guard Howe's Blackrock Bounty and the Wanted posters (Gath'Ilzogg, Fangore) make the
  Blackrock line into Stonewatch. Theocritus's "Morganth" is the bible's Ilgalar beat.
- Errands to recast as bounties: Foreman Oslow (tools, the Everstill Bridge), Verner Osgood,
  Dockmaster Baren (murloc poachers), Chef Breanna, Deputy Feldon and Guard Parker.

Duskwood (zone 10, 79 quests, levels 20-45), mostly from Darkshire:

- Althea Ebonlocke's Night Watch chain; Mor'Ladim and The Daughter Who Lived (Raven Hill
  Cemetery, stretched to 50); Sven Yorgen against Morbent Fel; Calor's worgen; Abercrombie.
- The Legend of Stalvan (Daltry, Madame Eva, Smitts): eleven steps that end at Manor
  Mistmantle, the site of Mistmantle Manor. The best local story for that dungeon.
- To drop: the Ironforge Lightforge chain, the Nethergarde supplies, the Forsaken quests at
  Forlorn Rowe, and every step that runs to Westfall or Ironforge.

Subzone names that fit the bible: The Hushed Bank and Twilight Grove (Duskwood), Stonewatch Falls
and Galardell Valley (Redridge). Vanilla 1.12 has no camp east of Lakeshire.

## 2. The proposed arc

### Shape

Redridge carries rung 4 (the tower is a working Anchor) and the Blackrock front. Duskwood carries
rung 5 (an untuned Anchor made the night; the dead freeze). Stormwind is where they meet: the 40s
court chain (rung 6) and, from 44, Toren. The bands overlap, so a player goes Redridge, Duskwood,
Redridge, Duskwood, through the city each time. Assumed: Elwynn's three kinds (story, about four
bounties per hub per band, one repeatable per dungeon), with item quests on named elites.

### Hubs

- **Lakeshire** (Redridge, 30-45). Solomon, Marris, Oslow, Baren, Osgood and the inn. Toren
  arrives at the inn at 36. Three Corners (Guard Parker, Deputy Feldon) is the 30-33 bounty post
  on the Elwynn road.
- **Stonewatch Falls camp** (Redridge, 44-50). Assumed: a new Watch camp of Hub NPCs facing
  Stonewatch Keep and Galardell Valley, held by Guard Howe and, once found, Corporal Keeshan.
- **Darkshire** (Duskwood, 30-50). The Ebonlockes, the Night Watch, Madame Eva, Clerk Daltry,
  Sirra Von'Indi the historian, Calor, Smitts.
- **Raven Hill** (Duskwood, 36-46). Jitters, Lars and Sven Yorgen's camp, by the cemetery and the
  Raven Hill Deeps entrance.
- **Stormwind** (hub). General Marcus Jonathan (vanilla), whose requests for troops die in the
  court; Thelwater, Bishop Farthing, Pike, Brannoc, Wray. Dughan stays in Goldshire with the
  evidence.

### Toren's thread

| Level | Where | Beat |
|---|---|---|
| 36 | Lakeshire inn | Toren, freed at Ridgepoint, turns up. He knows the tower and won't go near it. |
| 38 | Ilgalar | Morganth dies. The notebooks are torn three ways; the player keeps the failure notes, and finds Sareth's letters. The Anchor keeps running. |
| 39 | Lakeshire | The cipher. Toren reads one page in a night and asks for time. |
| 40 | Lakeshire, then Darkshire | Page one: "the Duskwood test", a core set without tuning that made night. Sends the player to Sirra's records of the night Manor Mistmantle went quiet. |
| 44 | Raven Hill Deeps | Page two, taken off one of Sareth's diggers, with orders to find "the apprentice who reads M.'s hand". How an Anchor holds its stretch, and why the dead freeze. |
| 44 | Lakeshire, then Stormwind | Agents come for Toren at the inn. The player moves him to the Stockade's crossing, where the Watch, not the Keep, holds the keys. |
| 47 | Stonewatch Keep | Page three, in a Blackrock war chest. The Dark Iron forge stamp and where a casing gives. |
| 48 | Galardell Valley | Page four, from the cult courier carrying it back to the tower. |
| 49 | Stormwind | The method is whole. The last pages hold the unsent letter to Toren, which the player carries to him. |
| 50 | Ilgalar | The break (`steppes-opening.md`, decisions 7 and 11). Assumed: Sareth's agents take Toren after it, which opens the 60 arc. |

### Band 30-35: the bridge and the silent house (about 30 quests)

Redridge, Lakeshire and the lake:

1. A Letter for Lakeshire (30). Dughan sends his report by hand to Solomon. Solomon reads it and
   says Redridge has worse problems: the Blackrock.
2. Gnolls at Three Corners (31). Solomon, via Deputy Feldon. Poachers and mongrels on the road.
3. The Lost Tools, then The Everstill Bridge (32-33). Oslow's crew rebuild the bridge with stone
   paid for by a noble's seal from the city. Nobody asks whose. (Seeds the court chain; no name.)
4. What's on the Lakebed (33). Baren's murloc poachers have been diving crates off barges heading
   east across Everstill. The crates carry the mark.
5. Blackrock Menace (34). Marris. Outrunners scouting Lakeshire carry the same strange coin as
   Elwynn's Defias.
6. Theocritus's Grudge (34). Theocritus leaves his challenge on Ilgalar's door. Shadowhide gnolls
   watch and let the player go. The ground hums.

Duskwood, Darkshire, the north woods and Tranquil Gardens:

1. A Letter for Darkshire (30). Dughan writes to Lord Ello Ebonlocke, who has had no answer from
   the city in a year.
2. The Night Watch (31-32). Althea. Wolves and spiders on the north road; the Watch is three men
   short.
3. The Legend of Stalvan, cut to three or four steps (32-34). Daltry's records: the manor's
   candles have burned without going out for twelve years, since the night the dark came.
4. The Gardens Rose (34). Tranquil Gardens' dead got up the same night.
5. The Silent House (35). Madame Eva opens Mistmantle Manor.

Mistmantle Manor (32-36): three story quests and a repeatable. The household that went quiet the
night the test core was switched on, and Stalvan, their tutor, bound in the cellar. Assumed: the
boss is Stalvan. No one says "Anchor" here yet.

Stormwind: General Marcus Jonathan turns down Solomon's request for soldiers (35): "the court
says the Blackrock are a Redridge matter".

Count: Redridge 7 story and 4 bounties, Duskwood 7 story and 4 bounties, Mistmantle 3 and 1,
Stormwind 2 (one entry, one hand-off). About 28-30.

### Band 35-40: the tower and the west (about 30 quests)

Redridge, the Canyons, Alther's Mill and Ilgalar:

1. Shadow Magic (36). Marris. Blackrock Summoners and Redridge Mystics cast with ringing stones.
2. Toren at the Inn (36). A delivery: Toren, freed at Ridgepoint, asks the player not to tell him
   what is in the tower.
3. The Rethban Diggers (37). Redridge gnolls in the caverns dig ore for the tower's gnolls.
4. Into Ilgalar (36), Morganth (38): the dungeon's chain, Theocritus's vanilla quest recast.
   Morganth dies. The player keeps the failure notes and finds Sareth's letters (Sareth is a name
   on a letter, nothing more, until 60).
5. The Notebooks (38) and The Cipher (39): carry them to Toren. The tower is a working Anchor, and
   it still runs (rung 4 lands).

Duskwood west, Raven Hill, Sven's camp, Addle's Stead and Vul'Gol:

1. Raven Hill's Watch (36). Jitters and Lars: the hamlet hears digging in the cemetery at night.
2. The Grave Robbers (37). Vanilla Grave Robbers, now men in grey robes with a crowned mark, not
   the black sun.
3. Night Runners (38). Addle's Stead Defias, paid in strange coin, keep the west road empty.
4. Ogres the Wrong Way (39). Splinter Fist ogres haul crates into the cemetery, not out of it.
   Someone else is building something. (One thread; it doesn't say who.)
5. Sven's Revenge (40). Morbent Fel, recast as the robed diggers' overseer.

Stormwind and Elwynn: the Stockade's quiet wing (36-40) and the tram (38-42) already sit here.
Page one (40) is the hand-off: Toren in Lakeshire sends the player to Sirra in Darkshire through
the city.

Count: Redridge 6 story and 4 bounties, Ilgalar 3 and 1, Duskwood 6 story and 4 bounties,
hand-offs 2. About 26-30.

### Band 40-45: the warning and the court (about 32 quests)

Duskwood, Raven Hill Cemetery, Brightwood Grove and Roland's Doom:

1. The Duskwood Test (40-41). Sirra's records and Toren's page agree: the night is a machine that
   went wrong. Lord Ebonlocke won't write it to the city ("they'll call it a curse").
2. What the Diggers Want (42). Sareth's diggers are after the seam under the cemetery.
3. Into Raven Hill Deeps (42), The Frozen Dead (43), the boss (44): a ghoul stopped mid-step in a
   jar of grey. The dead aren't silenced; they freeze (rung 5). Page two, and the orders naming
   Toren.
4. Worgen in the Woods (41-43). Calor's vanilla chain, local, no plot thread.
5. Warn Toren (44). Cross-zone: Darkshire to Lakeshire, an ambush at the inn, then Toren to the
   Stockade.

Redridge, Render's Valley and Render's Rock:

1. Blackrock Bounty (41). Guard Howe.
2. Missing in Action (42). Keeshan, scouting Render's Rock, missing a week. Assumed: rewritten
   as find him and kill his captors, since escort isn't built.
3. Tharil'zun (44). His orders are in the cult's hand, with the mark. The orcs and the cult take
   orders from the same desk.
4. What Keeshan Saw (45). Something winged over Stonewatch at night. Opens the Stonewatch Falls
   camp.

Stormwind court chain, start (rung 6). Assumed: this belongs in its own spec with the Cathedral;
this arc only supplies the hand-offs.

1. The Office That Writes Writs (42). Dughan, with the Writ of Names, Rook's letters and the dead
   courier's dispatch, sends the player to General Jonathan.
2. Delayed (44). The falsified dispatch traced to Lady Prestor's office. Her name, not what she
   is.

Count: Duskwood 7 story and 4 bounties, Raven Hill Deeps 3 and 1, Redridge 5 story and 4
bounties, Stormwind 3 to 4. About 30-32.

### Band 45-50: the dragons' shadow and the break (about 28 quests)

Redridge, Stonewatch Keep, Stonewatch Falls and Galardell Valley:

1. The Keep (45), the dungeon's chain to the boss (48). Black dragonkin with the Blackrock (rung
   8). Gath'Ilzogg's Wanted poster leads in. Page three.
2. Hold the Falls (46-47). Bounties from the camp.
3. The Last Courier (48). Galardell: the cult courier carrying page four back to Ilgalar.

Duskwood, Roland's Doom and Raven Hill:

1. Mor'Ladim and The Daughter Who Lived (48). Vanilla's best local story, left local.
2. The Agents' Camp (46-47). Sareth's agents in Twilight Grove, with Toren's stolen drafts.
3. Night Watch bounties (46-49).

Stormwind:

1. Follow the Money (46-47). The bridge seal from Lakeshire and the relief wagons into Darkshire
   lead to one house: Cedric Valemont's (the court chain names Cedric here, per the bible's
   "before the Stormwind chain in the 40s" rule).
2. Sanctuary (48-49): into the Cathedral of Light spec. Cedric confesses; the Anchors close the
   pocket (rung 7).
3. The Unsent Letter (49). Toren's method is whole.
4. The break (50) at Ilgalar, per `steppes-opening.md`.

Count: Redridge 5 story and 3 bounties, Stonewatch 3 and 1, Duskwood 4 story and 3 bounties,
Stormwind 4. About 26-28.

Totals: about 115 quests (about 55 story, 31 bounties, 12 dungeon story, 4 repeatables, 12
Stormwind hand-offs). Item quests: Stalvan, Morganth, Tharil'zun, the Raven Hill Deeps boss,
Stonewatch's boss, the Last Courier, the Agents' Camp leader, Mor'Ladim. Assumed: Elwynn's
density is right; decision 12 asks.

## 3. Open decisions for the grill

1. **Do Redridge and Duskwood get sub-zone Level Bands in place of the linear stretch?**
   - (a) Yes, Elwynn's `bands.rs` grows to read a band file per zone.
   - (b) Keep the stretch and move the dungeons to fit it (Ilgalar would be 44-48).
   - (c) Keep the stretch and add hand-placed camps only on the dungeon approaches.
   - Recommended: (a). Today Ilgalar (34-38) sits among 41-48 gnolls, Mistmantle (32-36) among
     39-45 worgen and Raven Hill Cemetery runs 37-50; only bands fix all three.

2. **Is the 40s court chain part of this arc or its own spec with the Cathedral?**
   - (a) Its own spec; this arc ships only the hand-offs.
   - (b) Part of this arc, band by band.
   - Recommended: (a). It is Stormwind's story, ends in a dungeon of its own, and keeps these
     specs to two zones.

3. **Who sends a character into Redridge and Duskwood at 30?**
   - (a) Dughan in Goldshire: two letters, to Solomon and to Ebonlocke.
   - (b) The Tier 2 trainer in Stormwind, right after Advancement.
   - (c) A poster at the Writ Board.
   - Recommended: (a). Dughan already sends the Stockade hook at 30 (97301), and Elwynn's exits
     were planned as his letters.

4. **Where does Toren live from 36 to 50, and when does Sareth take him?**
   - (a) Lakeshire's inn throughout; taken after the break at 50.
   - (b) Lakeshire until the ambush at 44, then the Stockade crossing in Stormwind; taken at 50.
   - (c) Darkshire under the Night Watch; taken at 50.
   - Recommended: (b). The move shows the hunt, and his translations then pull players through
     the city between the zones.

5. **Where are the four failure-note pages found?**
   - (a) Ilgalar, Raven Hill Deeps, Stonewatch Keep, and a courier in Galardell (as drafted).
   - (b) All four as open-world drops in both zones, in any order.
   - (c) Ilgalar holds all of them; Toren needs only time.
   - Recommended: (a). Each 40s dungeon earns a page, and it keeps both zones on the spine.

6. **How is a page carried: a fixed Story Quest item, a Journal collectible, or a Rumour chest?**
   - (a) A fixed quest item from a fixed creature or chest.
   - (b) A Journal collectible set, like Fish Pages, with Toren translating each.
   - (c) A Rumour's chest.
   - Recommended: (a). The main line must always work; a collectible set of the cult's and
     Sareth's unreadable pages can come later.

7. **Which comes first at 48-50, the Cathedral or the break?**
   - (a) The Cathedral, then the break.
   - (b) The break, then the Cathedral.
   - (c) Either order.
   - Recommended: (a). Cedric's "the Anchors close the pocket" is the reason to break one.

8. **Do the Black Dragon Whelps on Lake Everstill's south shore stay?**
   - (a) Remove them from the open world and use the model in Stonewatch.
   - (b) Keep them with no quest text about them.
   - (c) Swap them for another beast.
   - Recommended: (a). 35 whelps at 35-36 tell a player "dragons" ten levels before rung 8.

9. **Do the dungeon files take the bible's levels?**
   - (a) Yes: Raven Hill Deeps 40-44 and Stonewatch Keep 45-48.
   - (b) Keep both at 44-48 and amend the bible.
   - Recommended: (a). The bible wins, and 44-48 twice leaves 40-44 with only the tram.

10. **How much vanilla Redridge and Duskwood do we reuse?**
    - (a) Vanilla givers where they stand, new text, Westfall, Ironforge and Horde steps dropped.
    - (b) All new NPCs.
    - (c) Vanilla quests nearly as written.
    - Recommended: (a). Friendly spawns are already Field Givers, and the vanilla givers fit the
      bible's local problems.

11. **What happens to Keeshan's escort?**
    - (a) Rewrite it as find him and kill his captors.
    - (b) Build an escort objective now.
    - (c) Drop Keeshan.
    - Recommended: (a). Escort was spec 5's and never built; one quest doesn't justify it.

12. **How many quests: Elwynn's density or leaner?**
    - (a) About 115, Elwynn's density.
    - (b) About 70: fewer bounties, leaning on Writs and dungeon repeatables.
    - (c) About 50: story only.
    - Recommended: (a). Assumed: 20 content levels at Tier 2 need about what Elwynn's 20 took; a
      sim from `docs/balance` can check it before the tickets.

13. **Do item quests ship with gear choices?**
    - (a) Silver only until a 30-50 loot spec, as Elwynn's decision 9.
    - (b) New sets per dungeon now, as the Stockade and the tram did.
    - Recommended: (b) for the four dungeon bosses, (a) for open-world elites. The dungeon sets
      follow a pattern the Build lane has done twice; open-world rows wait for 30-50 Loot Bands.

14. **What are the robed diggers called, and how does a player tell them from the cult?**
    - (a) "Grey Robes" with a crowned mark; nobody names Sareth.
    - (b) Cultists of the Damned, as vanilla Scourge.
    - (c) Indistinguishable from the Twilight's Hammer until 60.
    - Recommended: (a). The bible lets players glimpse Sareth's people early; a different mark
      keeps his goal hidden.

15. **Where do the new quest ids go?**
    - (a) Redridge 98101-98599, Duskwood 98601-99099, Stormwind hand-offs 97401-97499, in new
      files `quests_redridge.toml` and `quests_duskwood.toml`.
    - (b) All in `quests_stormwind.toml` and one new file.
    - Recommended: (a). One block per sub-zone as Elwynn did, clear of the Writs (95001-95007),
      the Rumours (94901-94906) and the Stockade (97301-97312). Assumed: #593's quest takes an
      id in 973xx.

## 4. Ticket sketch

Split as Elwynn did: one spec per band (30-35, 35-40, 40-45, 45-50), in play order, each holding
both zones and its Stormwind hand-offs. Each dungeon interior gets its own grill at the start of
its band, as Jasperlode and Azora did. Text follows the story rewrite's rule: Claude drafts,
Davis reviews, tickets paste reviewed text.

Code tickets (Build lane), before the first band:

1. Level Bands for every zone (if decision 1 is (a)). `bands.rs` reads a band file per zone;
   Redridge's and Duskwood's linear stretch in `zones.toml` gives way to bands. Reuses
   `elwynn_bands.toml`'s format and tests. Grill check: a schema change in a content file, so
   one short grill.
2. Two quest files. `quests_redridge.toml` and `quests_duskwood.toml` added to `shipped()` and
   the content tests. A few lines.
3. Only if decision 11 is (b): an escort objective.

Data tickets per band (each one alone, `ready-for-agent`):

1. Bands and creatures. The band's rows in the zone band file; `[[zone.added]]` camps for the
   Twilight cultists, the Grey Robes and the named elites (templates in `mobs.toml`); removals
   (the whelps). `ground_check.py` gives every z.
2. Redridge quests and quest items for the band, in `quests_redridge.toml` and
   `quest_items.toml`. Reuses kill, collect, talk, use, explore, wave defense and Quest Starters.
3. Duskwood quests and quest items for the band, the same way.
4. Stormwind hand-offs and NPC moves: Toren's Hub NPC rows (Lakeshire inn, then the Stockade
   crossing), General Marcus Jonathan's quests, the Stonewatch Falls camp's Hub NPCs.
5. The band's dungeon: creatures, boss, Creature Spells, its story quests, its repeatable, and its
   gear set in `gear.csv` (`icon_check.py` gate). Blocked by the dungeon's interior grill.
6. Writs: re-point the six Redridge and Duskwood Writs' kill lists if bands change which
   creatures stand where.

The last band feeds `steppes-opening.md`'s ticket 4: its stand-in prerequisite becomes "The
Unsent Letter". Reused, none rebuilt: Field Givers, Hub NPCs, the quest objective kinds, wave
defense, Quest Starters, Rumours, Writs, Phased Dungeons with Head Count, the living map's marks.

## Grill answers (Davis, 2026-11-07)

1. Mob levels: (a) sub-zone Level Bands per zone, like Elwynn; a small code ticket first.
2. Court chain: (a) its own spec with the Cathedral; this arc ships only the hand-offs.
3. Way in at 30: NOT Dughan. Both the Tier 2 trainer in Stormwind (right after Advancement) and a Writ Board poster send the character to Redridge and Duskwood.
4. Toren: Lakeshire's inn the whole time, 36 to 50; no ambush or move at 44. Taken by Sareth's agents after the break at 50.
5. Pages: (a) Ilgalar, Raven Hill Deeps, Stonewatch Keep, and a cult courier in Galardell.
6. Page form: (a) a plain quest item from a fixed boss or chest, handed to Toren.
7. Order at 48-50: either order. The Cathedral (court spec) and the break do not require each other.
8. Black Dragon Whelps: (a) removed from Lake Everstill's shore; the model is used in Stonewatch.
9. Dungeon levels (Claude, a fix): the bible wins. Raven Hill Deeps 40-44, Stonewatch Keep 45-48.
10. Vanilla reuse: (a) vanilla givers where they stand, new text, Westfall/Ironforge/Horde steps dropped.
11. Keeshan: BUILD ESCORTS NOW. Escort quests are a new realm system: its own short grill, then its own ticket(s); Keeshan's quest is blocked by it.
12. Density: (a) about 115 quests, Elwynn's density.
13. Gear rewards: ALL named-elite and dungeon-boss item quests ship gear choices in this spec (not only the dungeons). Rows follow ADR 0026 budgets; they must not race the gear 31-60 spec on gear.csv (Blocked by between them).
14. Diggers: (a) "Grey Robes" with a crowned mark; nobody names Sareth.
15. Ids (Claude, a fix): (a) Redridge 98101-98599, Duskwood 98601-99099, Stormwind hand-offs 97401-97499, files quests_redridge.toml and quests_duskwood.toml.
16. East camp: a new Lakeshire Watch camp at Stonewatch Falls facing the Keep; Keeshan joins after his rescue.
