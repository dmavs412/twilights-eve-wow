Title: Redridge 35-40: Toren at Lakeshire's inn, into Ilgalar, ringing stones and the Rethban diggers (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 4. Blocked by 07-redridge-30-35 (same files), 10-gear-35-40 (the band's Quest Line rows), 11-bands-35-40 and #593
(both edit `content/hub.toml`).

Ten quests, 98201 to 98214 (six story quests and four bounties), in
`content/quests_redridge.toml` under `# ── Redridge 35-40 (#PARENT) ──`; Toren as a new Hub NPC;
four quest items, and the three Ilgalar items ticket 27 uses, in `content/quest_items_redridge.toml`.

## How to write the rows

As ticket 07-redridge-30-35 says (`zone_or_sort = 44`, the money rule, quoted text is the
contract, beats in the Stockade's voice, no em dashes, the never-name list).

## Toren, a Hub NPC (decision 4)

Toren lives in Lakeshire's inn from 36 to 50, with no ambush and no move. He is a third Toren
row, in the form of Ridgepoint's (`hub.toml` guid 9300022, line 1200), so his quest list stays his
own. The Azora and Ridgepoint Torens stay where they are. Add after Station Master Brannoc's row,
under `# ── Toren in Lakeshire (#PARENT) ──` with a comment naming the inn and the floor:

- `guid = 9300083`, `entry = 190083`, `name = "Toren"`, `subname = "Lakeshire Inn"`,
  `level = 20`, `display_id = 3346`, `faction_template = 35`, `role = "giver"`,
  `npc_flags = 0x2`, `map = 0`.
- `position = [-9228.0, -2162.0, 63.73]`, on the inn's ground floor: `wmo_floor.py -9228 -2162`
  lists group 10's floor at z 63.73, a few yards from Innkeeper Brianna (6727, at -9224.0,
  -2157.1). Write that Z and let `ground_check.py` check the row. `orientation = 0.0`.
- `greeting = "Don't say the word. The tower. I hear it in my sleep as it is."`

## Story quests

**98201 Toren at the Inn.** Level 36. Giver Innkeeper Brianna (6727), ender Toren (190083). No
prerequisite, so a character who never freed him at Ridgepoint still meets him. No objective.
Money 0.
Objectives: "Speak with the young man in the corner of Lakeshire's inn."
Details: "There's a lad in the corner who came in off the Elwynn road a fortnight back with nothing but a coat and a pack. He pays in good coin, drinks nothing, and asks every traveller what they've seen in the east. When someone says the word 'tower', he goes white. You look like you've seen the east. Go and talk to him before he wears a hole in my floor."
Progress: "You've come from the east?"
Completion: "My name is Toren. I was a student once, in the tower in Elwynn, and then for a while a prisoner in a cellar at Ridgepoint. The people who kept me fed me and never laid a hand on me, because someone told them not to. I know whose tower stands on that hill. I won't go near it. Please don't tell me what's inside. Not yet."

**98202 Into Ilgalar.** Level 36. Giver and ender Marshal Marris (382), `after = 98109`. Kill 12
gnolls on the tower's hill (family "Gnolls on the tower's hill slain", entries 433, 434, 429,
431), the Ilgalar's Approach band of ticket 11. Money 1260.
Beat (details): the Blackrock keep carrying crates up the hill; Marris wants the slope clear so
the Watch can at least watch the door.
Completion: "Louder, you say. Like the whole hill is a bell somebody keeps striking. I've eleven men and not one of them is going through that door. If you go in, I'll want to know who's ringing it."
Ticket 27's boss quest, 98501 "Morganth", follows this one.

**98203 Shadow Magic.** Level 36. Giver and ender Marris, `after = 98108`. Kill 8 Redridge
Mystics (430) and collect 6 Ringing Stones (980311) from them at 50%. Money 1260.
Beat (details): the canyon gnolls' mystics cast with stones that ring when struck, and they
trade for them with someone in the east.
Completion: "Strike one and it rings. Hold it to your ear and it sounds wrong, like a note held too long. Gnolls don't make these. Somebody east of here gives them out."

**98204 The Rethban Diggers.** Level 38. Giver and ender Foreman Oslow (341), `after = 98105`.
Kill 10 Redridge Drudgers (580) in the Rethban Caverns and collect 6 Humming Ore (980312) from
them at 50%. Money 1330.
Beat (details): the gnolls in the caverns dig day and night, and carts of their ore go east
along the north road.
Completion: "That's not iron. It hums. My men won't keep it in the shed overnight, and I don't blame them. Every cart of this goes the same way, east, toward that hill."

**98205 Brutes in the Canyons.** Level 37. Giver and ender Bailiff Conacher (900). No
prerequisite. Kill 12 Redridge Brutes (426). Money 1295.
Beat: the canyon gnolls raid the north farms for grain. Local; no thread.

**98206 Yowler.** Level 38. Giver and ender Conacher, `after = 98205`. Kill Yowler (518) and 8
Redridge Alphas (445). Money 1330.
Beat: Yowler leads the raids and the farmers want proof he's gone. Local; no thread.

## Bounties (`kind = "bounty"`)

| Id | Giver | Level | Objective | Money |
|---|---|---|---|---|
| 98211 | Guard Howe (903) | 37 | Kill 10 Redridge Bashers (446) in the canyons | 1110 |
| 98212 | Verner Osgood (415) | 36 | Collect 8 Greater Tarantula Fangs (980313) from Greater Tarantulas (505) at Alther's Mill at 50% | 1080 |
| 98213 | Martie Jainrose (342) | 37 | Collect 8 Stolen Grain Sacks (980314) from Redridge Brutes (426) and Redridge Mystics (430) at 50% | 1110 |
| 98214 | Deputy Feldon (1070) | 36 | Kill 10 Blackrock Grunts (440) on the Lakeridge Highway | 1080 |

## Quest items (`quest_items_redridge.toml`)

| Entry | Name | Form | Description or pages |
|---|---|---|---|
| 980311 | Ringing Stone | stack of 20 | "A grey stone that rings when you knock it. The note doesn't stop when it should." |
| 980312 | Humming Ore | stack of 20 | "Dull ore with a hum you feel in your fingers." |
| 980313 | Greater Tarantula Fang | stack of 20 | "Osgood pays by the fang. Don't ask what for." |
| 980314 | Stolen Grain Sack | stack of 20 | "Martie's mark, stitched on the corner." |
| 980381 | Morganth's Failure Notes | single; ticket 27's 98501 collects it and 98502 gives it back | "Cipher, page after page, in a small, proud hand. The cover says only: 'What went wrong.'" |
| 980382 | Letters Under a Crowned Seal | single, readable, two pages; collected by 98501 | page one: "Engineer. They give you a tower and keep your name off every stone you set. I would give you the credit you are owed, and a name that outlasts every one of theirs. Consider it. You know how to reach my people." page two: "Below it, a crown pressed into grey wax, and one letter: S." |
| 980383 | Toren's Translation | single, given by ticket 27's 98503 | "Three pages in Toren's careful hand. The first line reads: 'The Duskwood test.'" |

980381 to 980383 are defined here so ticket 27 edits no item file; no quest uses them until it
lands. The letters never name their writer (decision 14).

## Gear choices (decision 13 as amended)

Every story quest offers a choice of greens from its band's Quest Lines (ticket 05 made the
Lines; the band's gear ticket wrote these rows). Add the `choices` line the table gives to each
quest's row. The slots follow #448 decision 9's cycle in quest-id order across the whole band, all
tickets together (main hand, ring, Chest, amulet, off hand, ring, main hand, ring, and again).
Bounties pay money and offer no choice.

| Quest | Slot | `choices` | Rows from |
|---|---|---|---|
| 98201 | Chest, one per armour type | `[903220, 903221, 903222, 903223]` | 10-gear-35-40 |
| 98202 | amulet | `[903231, 903232, 903233]` | 10-gear-35-40 |
| 98203 | off hand: shield, bow, held item | `[903225, 903227, 903230]` | 10-gear-35-40 |
| 98204 | ring, the b Lines | `[903235, 903237, 903239]` | 10-gear-35-40 |
| 98205 | main hand: sword, dagger, mace | `[903224, 903226, 903229]` | 10-gear-35-40 |
| 98206 | ring, the a Lines | `[903234, 903236, 903238]` | 10-gear-35-40 |

## Failing tests first

`crates/twilight-realm/tests/redridge_30_50.rs` (ticket 07) gains:

- Toren (190083) stands on map 0 at (-9228.0, -2162.0, 63.73) and is a quest giver.
- 98201 is offered by Brianna with no prerequisite and completes at Toren with nothing to
  collect.
- The chains hold: 98202 after 98109; 98203 after 98108; 98204 after 98105; 98206 after 98205.
- All ten quests load with their givers and `zone_or_sort = 44`; items 980381 to 980383 exist,
  980382 with two pages.
- Every story quest in the gear table offers exactly the `choices` it names.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `icon_check.py` reports no new clash;
  `ground_check.py` passes with Toren's row.

Assumptions allowed: item display ids and icons, and wording inside a beat.
