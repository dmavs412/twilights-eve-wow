Title: Redridge 45-50: the Lakeshire Watch's camp at Stonewatch Falls, Gath'Ilzogg's poster and the last courier (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 16. Blocked by 17-redridge-40-45 (same files), 20-gear-45-50 (the band's
Quest Line rows) and 21-bands-45-50 (the camp ground and the courier).

The camp's four Hub NPCs and its tents; ten quests, 98401 to 98414 (six story quests and four
bounties), in `content/quests_redridge.toml` under `# ── Redridge 45-50 (#PARENT) ──`; two
quest items, and the three items tickets 29 and 30 use, in `content/quest_items_redridge.toml`.

## How to write the rows

As ticket 07-redridge-30-35 says (`zone_or_sort = 44`, the money rule, quoted text is the
contract, no em dashes, the never-name list). Stonewatch Keep owns rung 8 (the dragons): at the
gate a character may glimpse "big shapes that move like no orc", nothing more.

## The camp (decision 16)

Hub NPCs in `content/hub.toml`, after Toren in Lakeshire (ticket 12), under
`# ── The Lakeshire Watch's camp at Stonewatch Falls (#PARENT) ──`. All `map = 0`,
`faction_template = 35`, position z written as 0.0 for `ground_check.py --fix`, and facing the
keep's gate (`orientation = 1.0`, or `faces = [-9386.3, -3046.8]`).

| Guid | Entry | Name | Subname | Level | Display | Role, flags | Position (x, y) | Greeting |
|---|---|---|---|---|---|---|---|---|
| 9300084 | 190084 | Sergeant Willa Crane | Lakeshire Watch | 45 | 3446 | giver, 0x2 | -9512, -3262 | "We took the falls from the Blackrock and we're keeping them. Every blade helps." |
| 9300085 | 190085 | Quartermaster Bram Tuller | Repairs | 40 | 3446 | repairer, 0x4004 (Mae Tollin's form, hub.toml line 2002) | -9520, -3276 | none |
| 9300086 | 190086 | Lakeshire Watchman | Lakeshire Watch | 40 | 2150 | ambient, 0x1, Watchman form (line 2040) | -9500, -3255 | "Carts go into that keep at night. Nothing comes out but orcs." |
| 9300087 | 190086 | Lakeshire Watchman | Lakeshire Watch | 40 | 2150 | ambient, 0x1 | -9528, -3258 | "Eight of us and a sergeant. Marris calls it a garrison." |

Display 3446 is Guard Howe's, 2150 Guard Parker's. Each ambient row takes the next free `text_id`
above the highest in `hub.toml`.

GameObjects in `quests_redridge.toml`, each a `[[gameobject]]` row in the form of Keane's outpost
(`quests_elwynn.toml` line 1594), z 0.0 for `--fix`:

| Guid | Entry | What | Display | Position (x, y) |
|---|---|---|---|---|
| 792011 | 980502 | "Wanted: Gath'Ilzogg", the poster that gives 98403 | 2 (vanilla's Gath'Ilzogg poster) | -9508, -3266 |
| 792012 | 980503 | tent | 6738 | -9525, -3285 |
| 792013 | 980503 | tent | 6738 | -9505, -3285 |
| 792014 | 980504 | campfire | 192 | -9515, -3272 |

Keeshan joins this camp after his rescue; that is ticket 25's, not this one.

## Story quests

**98401 Stonewatch Falls.** Level 45. Giver Marshal Marris (382), ender Sergeant Crane (190084),
`after = 98304`. No objective. Money 0.
Objectives: "Report to Sergeant Willa Crane at the Lakeshire Watch's camp at Stonewatch Falls."
Details: "Solomon found me eight more men. Eight. I've sent them east with Sergeant Crane to take the falls below Stonewatch back from the Blackrock and hold them, so we can see what goes in and out of that keep. Crane will want every blade she can get. So do I, but she's closer to the trouble."
Progress: "You're from Lakeshire?"
Completion: "Marris sent you. Good. We took the falls two days ago and we've held them since, which is more than anyone expected. From here you can see the keep's gate. Carts go in at night. Nothing comes out but orcs."

**98402 Hold the Falls.** Level 46. Giver and ender Crane, `after = 98401`. Kill 12 Galardell
gnolls (family "Shadowhide gnolls slain", entries 432, 568). Money 1610.
Beat: the gnolls of Galardell Valley come at the camp from the north-east every night. Local.

**98403 Wanted: Gath'Ilzogg.** Level 46. `giver_object = 980502` (the camp's poster), ender
Crane, `after = 98401`. Explore
`{ place = "The gate of Stonewatch Keep", position = [-9386.3, -3046.8], radius = 15.0, map = 0 }`
(the dungeon's portal, `stonewatch_keep.toml`). Money 1610.
Objectives: "Look through the gate of Stonewatch Keep, then report to Sergeant Crane."
Details: "WANTED: GATH'ILZOGG, warlord of the Blackrock in Redridge, for raids on Lakeshire and the murder of the Watch. Last seen at Stonewatch Keep. The Lakeshire Watch pays on proof. Below, in another hand: Look before you go in. Sgt. W. Crane."
Progress: "Did you see the gate?"
Completion: "You saw it too, then. Big shapes in the yard that move like no orc I know. I've written it to Marris and nobody else, because nobody in Stormwind would believe it. Gath'Ilzogg is in there with them. If you go in, bring back proof, and anything he keeps in that war chest of his."
Ticket 29's 98511 "Gath'Ilzogg" follows this quest.

**98404 The Last Courier.** Level 48, an item quest. Giver and ender Crane, `after = 98402`. Kill
the Twilight Courier (96054, ticket 21, in Galardell Valley) and collect the Cult Courier's
Satchel (980331) from him at 100%. `choices = [903313, 903314, 903315]` (blue Quest Line rows, ticket 20). Money 2160.
Objectives: "Kill the Twilight Courier in Galardell Valley and bring his satchel to Sergeant Crane."
Details: "My scouts have watched a rider in a hooded cloak come up out of the south three times this week, always to the same place in Galardell, always with two outriders. He carries one satchel and guards it like his life. He isn't Blackrock. He wears the hammer and the black sun on his cloak. Take that satchel off him before he reaches the tower."
Progress: "The courier?"
Completion: "Papers. Cipher, like the scribble Marris says the lad at the inn can read. And a folded letter that isn't cipher at all. I'll keep the satchel locked in my chest. When the lad's ready for it, it's his."
Ticket 30's 98521 "The Unsent Letter" follows this quest.

**98405 Wanted: Lieutenant Fangore.** Level 46. Giver and ender Crane, `after = 98401`. Kill
Lieutenant Fangore (703) and 8 Shadowhide Darkweavers (429) in Galardell Valley. Money 1610.
Beat: Fangore leads the Galardell gnolls against the camp. Local.

**98406 Rohh the Silent.** Level 47. Giver and ender Crane, `after = 98405`. Kill Rohh the Silent
(947) and 8 Shadowhide Slayers (431). Money 1645.
Beat: a gnoll that kills pickets without a sound. Local.

## Bounties (`kind = "bounty"`)

| Id | Giver | Level | Objective | Money |
|---|---|---|---|---|
| 98411 | Sergeant Crane (190084) | 46 | Kill 10 Shadowhide Brutes (432) | 1380 |
| 98412 | Clyde Ranthal (2697) | 46 | Collect 8 Shadowhide Pelts (980332) from Shadowhide Warriors (568) and Brutes (432) at 50% | 1380 |
| 98413 | Sergeant Crane | 47 | Kill 10 gnoll casters and killers (family "Shadowhide slain", 431 and 429) | 1410 |
| 98414 | Sergeant Crane | 47 | Kill 10 Shadowhide Warriors (568) | 1410 |

## Quest items (`quest_items_redridge.toml`)

| Entry | Name | Form | Description or pages |
|---|---|---|---|
| 980331 | Cult Courier's Satchel | single; collected by 98404 | "Oiled leather, the hammer over a black sun tooled into the flap. Papers inside." |
| 980332 | Shadowhide Pelt | stack of 20 | "Ranthal pays for these. His wife makes them into rugs." |
| 980386 | Morganth's Notes, Page Three | single; collected by ticket 29's 98511, given again by 98512 | "One page of cipher. In the corner, pressed into the paper, a smith's stamp: an anvil inside a ring of fire." |
| 980391 | Morganth's Notes, Page Four | single; given and collected by ticket 30's 98521 | "The last page of the failure notes, in cipher. The back is covered in plain writing." |
| 980392 | Morganth's Unsent Letter | single, readable, three pages; given and collected by 98521 | page one: "T. If you are reading this, I was wrong, and you were right to leave. They will tell you I built this for them. I built it to be finished. Nobody ever let me finish anything." page two: "Theocritus took the credit for everything we made at Azora. You never minded. I minded enough for both of us. That is the whole of it, and it was never worth what it bought." page three: "The notes are yours. They show where every one of my stones can break. I wrote them down so that someone could. I am glad it is you. Don't let them find you. M." |

980386, 980391 and 980392 are defined here so tickets 29 and 30 edit no item file. The letter's
text is a draft for Davis's review; it never names Sareth.

## Gear choices (decision 13 as amended)

Every story quest offers a choice of greens from its band's Quest Lines (ticket 05 made the
Lines; the band's gear ticket wrote these rows). Add the `choices` line the table gives to each
quest's row. The slots follow #448 decision 9's cycle in quest-id order across the whole band, all
tickets together (main hand, ring, Chest, amulet, off hand, ring, main hand, ring, and again).
Bounties pay money and offer no choice. 98404, a named-elite item quest, offers blue rows instead (above).

| Quest | Slot | `choices` | Rows from |
|---|---|---|---|
| 98401 | Chest, one per armour type | `[903260, 903261, 903262, 903263]` | 20-gear-45-50 |
| 98402 | amulet | `[903271, 903272, 903273]` | 20-gear-45-50 |
| 98403 | off hand: shield, bow, held item | `[903265, 903267, 903270]` | 20-gear-45-50 |
| 98405 | ring, the b Lines | `[903275, 903277, 903279]` | 20-gear-45-50 |
| 98406 | main hand: sword, dagger, mace | `[903264, 903266, 903269]` | 20-gear-45-50 |

## Failing tests first

`crates/twilight-realm/tests/redridge_30_50.rs` gains:

- The four camp NPCs stand on map 0 near (-9515, -3270); Crane is a quest giver, Tuller repairs.
- The poster (980502) offers 98403 once 98401 is handed in; standing at the keep's gate credits
  it.
- The chains hold: 98401 after 98304; 98402 and 98405 after 98401; 98404 after 98402; 98406 after
  98405.
- The Courier's corpse gives the Satchel while 98404 wants it; 98404 offers exactly 903313 to
  903315; every story quest offers the choices the gear table names.
- Items 980386, 980391 and 980392 exist, 980392 with three pages.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `icon_check.py` reports no new clash;
  `ground_check.py` flags nothing after `--fix`.

Assumptions allowed: NPC names (placeholders), item display ids and icons, the exact camp spots
within 10 yd, and wording inside a beat.
