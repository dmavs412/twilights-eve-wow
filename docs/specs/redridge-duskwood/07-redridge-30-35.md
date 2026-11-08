Title: Redridge 30-35: Three Corners, Lakeshire's worse problems, the bridge, the lakebed and the tower on the hill (#PARENT)
Labels: ready-for-agent

Part of #PARENT. Blocked by 03-quest-files, 05-gear-30-35 and 06-bands-30-35.

Thirteen quests, 98101 to 98114 (nine story quests and four bounties), in
`crates/twilight-realm/content/quests_redridge.toml` under `# ── Redridge 30-35 (#PARENT) ──`,
and their eight quest items in `content/quest_items_redridge.toml`.

## How to write the rows (the same in every quest ticket of this arc)

- Rows follow the schema in `content/quests.toml`'s header (lines 25 to 60). Copy the forms:
  talk rows as `quests_elwynn.toml` line 202, collect rows as line 113, a list of explore spots
  as line 1170, a bounty as 94105 (line 237), an item handed over at accept (`gives`) and taken
  at hand-in as 97301 in `quests_stormwind.toml`. An "X or Y" kill is a family row:
  `{ family = "...", entries = [...], count = N }`.
- Every giver and ender here is a friendly vanilla spawn, a Field Giver, unless the ticket names a
  Hub NPC. `zone_or_sort = 44` (Redridge).
- **Quoted text is the contract**: paste it as written. Where the ticket gives a beat instead,
  write `objectives`, `details` (60 to 110 words), `progress` (one short line) and `completion`
  (25 to 60 words) in the voice of the Stockade's 97301 to 97312: plain words, short sentences, in
  character, the local problem first, and at most the one thread of the bigger story the beat
  names. Objectives read "Kill 10 X and 6 Y on the Lakeridge Highway, then return to Marshal
  Marris in Lakeshire." No em dashes, straight quotes. `$C` and `$N` work.
- **Never** name Sareth, Cedric, Prestor, Deathwing or the Lich King, and never say "Anchor" in
  Duskwood before level 40. The cult's mark is "the hammer over a black sun"; the Grey Robes'
  mark is "a crown".
- **Gear.** Every story quest offers the greens its ticket's "Gear choices" table names, from
  the band's green Quest Lines. A named-elite item quest offers blue Quest Line rows instead. A
  bounty offers nothing. A dungeon-ending boss quest offers its dungeon set's four Chests.
- `money` is copper, a placeholder: 35 x level for a story quest with an objective, 45 x level
  for an item quest, 0 for a delivery or a talk, 30 x level for a bounty. The table gives it.
- Quest items go in the quest file's item file with `class = 12`, `quality = 1`, the quest's
  level as `item_level`, `max_count = 1` for a single item and `stackable = 20` for a counted
  drop, and a `description` line in the quest's voice. `display_id` from the vanilla item of the
  same or nearest name in the vmangos `item_template`. Each takes an icon `icon_check.py`
  accepts; if none fits, take any free stock icon and say so in the closing comment.

## Story quests

**98101 Gnolls at Three Corners.** Level 30. Giver and ender Guard Parker (464). Kill 10 Redridge
Poachers (424) and 10 Redridge Mongrels (423). Money 1050.
Beat: the Lakeshire Watch's post at Three Corners is Parker, a bell and a hut. Poachers and
mongrels come down off the hills onto the Elwynn road, where families walk east with everything
they own. No thread: a local job.

**98102 Report to Lakeshire.** Level 31. Giver Parker, ender Deputy Feldon (1070), `after = 98101`.
`gives` Parker's Report (980301); collect it. Money 0.
Beat: Parker's weekly report goes by hand because the last runner never arrived. Feldon reads it
and says the magistrate wants to meet anyone who can fight.

**98103 Worse Problems.** Level 31. Giver and ender Magistrate Solomon (344). No prerequisite, so
a character who skipped Stormwind and Three Corners still starts here. `talk` to Marshal Marris
(382), Foreman Oslow (341) and Dockmaster Baren (381). Money 0.
Objectives: "Speak with Marshal Marris, Foreman Oslow and Dockmaster Baren in Lakeshire, then return to Magistrate Solomon."
Details: "Stormwind writes to me about wolves. I have wolves. I also have orcs on the hills, a bridge in the lake and barges that sink in calm water. I've written all of it down three times and sent it west, and the answers say the city has every confidence in me. If you're here to help, hear it from the people doing the work. Marris has the Watch. Oslow has the bridge. Baren has the docks. Then come back and tell me which you'll take first."
Marris, option "What does the Watch need?": "Men. I have eleven. The Blackrock have sent outrunners down the highway south of the lake twice this month, looking at our walls and counting. Orcs don't count unless someone is paying them to."
Oslow, option "What happened to the bridge?": "The old one went in the spring flood, and my crew's tools went with it. Then a wagon of good cut stone turned up from the city, paid for, and nobody here ordered it. I'm not asking. I'm building."
Baren, option "What's wrong with the barges?": "Three barges this season, all heading east across Everstill, all loaded heavy, all gone down in flat water. The murlocs dive them before the crews are dry. I'd like to know what was in those crates. The crews won't say."
Progress: "Well?"
Completion: "Orcs counting, stone nobody ordered and crates nobody names. That's Redridge. Pick one, and I'll find you coin for the rest."

**98104 The Lost Tools.** Level 31. Giver and ender Oslow, `after = 98103`. Collect 6 Mason's
Tools (980302) from Murloc Flesheaters (422) and Murloc Scouts (578) at 40%. Money 1085.
Beat: the flood took the crew's tools into the lake, and the murlocs keep anything that shines.
No thread.

**98105 The Everstill Bridge.** Level 33. Giver and ender Oslow, `after = 98104`. Collect 6 Iron
Pikes (980303) from Blackrock Outrunners (485) at 50% and 6 Iron Rivets (980304) from Blackrock
Grunts (440) at 50%. Money 1155.
Beat (details): the Blackrock strip iron from the old bridge's footings for spearheads; the last
span needs pikes and rivets back.
Completion: "That'll hold the last span. You'll want to know about the stone. It came on a wagon from the city with a note: 'For the Lakeshire bridge, with the compliments of a friend.' A house's seal on the wax. Not the crown's, and not one I know. The magistrate didn't ask, and nor did I. I kept the wax, though."

**98106 What's on the Lakebed.** Level 33. Giver and ender Baren, `after = 98103`. Kill 12 lake
murlocs (family "Lake murlocs slain", entries 1083, 548, 422, 578) and collect 5 Marked Crate
Lids (980305) from the same four at 35%. Money 1155.
Beat (details): the murlocs dive the sunken barges and carry the cargo off; Baren wants to see
what the crews wouldn't name.
Completion: "This is off one of mine. That mark, the hammer over a black sun, burned into every lid. I hear Elwynn has trouble with that mark. So the crates go east across my lake, and the barges carrying them go down. I've stopped believing in calm-water accidents."

**98107 The Thresher.** Level 34, an item quest. Giver and ender Baren, `after = 98106`. Kill the
Lake Thresher (14357, an elite, two spawns on the lake) and collect the Thresher's Jaw (980306)
from it at 100%. `choices = [903300, 903301, 903302]` (blue Quest Line rows, ticket 05). Money 1530.
Beat: something big in the deep water cracks the barges' keels before the murlocs dive them.
The crews swear it follows the boats. Completion: Baren is glad it's dead and a little sorry; it
was older than the town. No thread.

**98108 Blackrock Menace.** Level 33. Giver and ender Marris, `after = 98103`. Kill 10 Blackrock
Outrunners (485) and 6 Blackrock Renegades (437) on the Lakeridge Highway. Money 1155.
Beat (details): the outrunners count Lakeshire's walls from the highway south of the lake;
Marris wants them gone before they finish counting.
Completion: "Every one of them had a purse, and every purse had this. Coin I've never seen, with faces no king ever wore. I'm told the Defias in Elwynn are paid in the same. Orcs don't take wages. Somebody's paying them anyway."

**98109 The Tower on the Hill.** Level 34. Giver and ender Marris, `after = 98108`. Explore
`{ place = "The door of the Tower of Ilgalar", position = [-9282.2, -3327.6], radius = 15.0, map = 0 }`
(the dungeon's portal, `ilgalar.toml`). Money 1190.
Objectives: "Find the door of the tower on the hill in eastern Redridge, then return to Marshal Marris in Lakeshire."
Details: "There's a tower on a hill past Stonewatch, as far east as Redridge goes. Gnolls hold the slope, and lately the Blackrock carry crates up it. My scouts say the ground up there hums. One of them won't go back. I need someone who can reach the door and tell me what they hear. Don't go in. Not yet."
Progress: "Did you reach it?"
Completion: "Gnolls on every path, crates at the door and a hum in the ground you could feel in your teeth. Whatever's in that tower, they're guarding it. I'll want more than eleven men before I knock."

## Bounties (`kind = "bounty"`)

| Id | Giver | Level | Objective | Money |
|---|---|---|---|---|
| 98111 | Guard Parker (464) | 30 | Kill 10 thrashers and tarantulas (family "Thrashers and tarantulas slain", 712 and 442) near Three Corners | 900 |
| 98112 | Chef Breanna (343) | 31 | Collect 8 Goretusk Flanks (980307) from Great Goretusks (547) at 50% | 930 |
| 98113 | Verner Osgood (415) | 32 | Collect 8 Tarantula Silk (980308) from Tarantulas (442) at 50% | 960 |
| 98114 | Deputy Feldon (1070) | 33 | Kill 10 Dire Condors (428) over Lake Everstill | 990 |

Each bounty's text: a small local need and the giver's voice, no thread.

## Quest items (`quest_items_redridge.toml`)

| Entry | Name | Form | Description |
|---|---|---|---|
| 980301 | Parker's Report | single, given by 98102 | "Three Corners. Two men, one bell, the road open. Send more men." |
| 980302 | Mason's Tools | stack of 20 | "Chisels and a mallet, green with lake weed." |
| 980303 | Iron Pike | stack of 20 | "Bridge iron, hammered into a spearhead." |
| 980304 | Iron Rivet | stack of 20 | "A rivet as long as your hand, pulled from the old bridge." |
| 980305 | Marked Crate Lid | stack of 20 | "Waterlogged pine with a hammer over a black sun burned into it." |
| 980306 | Thresher's Jaw | single | "Rows of teeth, and splinters of keel between them." |
| 980307 | Goretusk Flank | stack of 20 | "Tough, but Breanna can make anything tender." |
| 980308 | Tarantula Silk | stack of 20 | "Thick as twine and very sticky." |

## Gear choices (decision 13 as amended)

Every story quest offers a choice of greens from its band's Quest Lines (ticket 05 made the
Lines; the band's gear ticket wrote these rows). Add the `choices` line the table gives to each
quest's row. The slots follow #448 decision 9's cycle in quest-id order across the whole band, all
tickets together (main hand, ring, Chest, amulet, off hand, ring, main hand, ring, and again).
Bounties pay money and offer no choice. The named-elite item quest, 98107, offers blue Quest Line rows instead (above).

| Quest | Slot | `choices` | Rows from |
|---|---|---|---|
| 98101 | ring, the b Lines | `[903215, 903217, 903219]` | 05-gear-30-35 |
| 98102 | main hand: sword, dagger, mace | `[903204, 903206, 903209]` | 05-gear-30-35 |
| 98103 | ring, the a Lines | `[903214, 903216, 903218]` | 05-gear-30-35 |
| 98104 | main hand: sword, dagger, staff | `[903204, 903206, 903208]` | 05-gear-30-35 |
| 98105 | ring, the b Lines | `[903215, 903217, 903219]` | 05-gear-30-35 |
| 98106 | Chest, one per armour type | `[903200, 903201, 903202, 903203]` | 05-gear-30-35 |
| 98108 | amulet | `[903211, 903212, 903213]` | 05-gear-30-35 |
| 98109 | off hand: shield, bow, held item | `[903205, 903207, 903210]` | 05-gear-30-35 |

## Failing tests first

New file `crates/twilight-realm/tests/redridge_30_50.rs` (the later Redridge tickets extend it),
modelled on `tests/goldshire.rs`:

- All thirteen quests load with their givers, enders and `zone_or_sort = 44`.
- The chains hold: 98102 after 98101; 98104 and 98106 and 98108 after 98103; 98105 after 98104;
  98107 after 98106; 98109 after 98108.
- Speaking with Marris, Oslow and Baren credits 98103, and it is offered with no prerequisite.
- 98102's accept puts Parker's Report in the bag and its hand-in at Feldon takes it.
- Walking to the tower's door credits 98109.
- 98107 offers exactly 903300, 903301 and 903302, and every story quest above offers the choices the gear table names.
- Every story quest in the gear table offers exactly the `choices` it names.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `icon_check.py` reports no new
  clash; `ground_check.py` flags nothing.

Assumptions allowed: item display ids and icons, and wording inside a beat. Record them as
`Assumed:` lines.
