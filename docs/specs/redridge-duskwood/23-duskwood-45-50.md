Title: Duskwood 45-50: the agents' camp, the Grey Captain, Mor'Ladim and the daughter who lived (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 14. Blocked by 18-duskwood-40-45 (same files), 20-gear-45-50 (the band's
Quest Line rows) and 21-bands-45-50 (the agents' camp, the Rotting Orchard's levels).

Ten quests, 98901 to 98914 (six story quests and four bounties), in
`content/quests_duskwood.toml` under `# ── Duskwood 45-50 (#PARENT) ──`; the agents' camp's tents
and fire; four quest items in `content/quest_items_duskwood.toml`.

## How to write the rows

As ticket 08-duskwood-30-35 says (`zone_or_sort = 10`, the money rule, quoted text is the
contract, no em dashes, the never-name list). Sareth's people are "the Grey Robes" and their
master "the Master"; neither Sareth nor Mourncrown is named, and nothing says what the Master
wants. Mor'Ladim's story is local and carries no thread.

## The agents' camp

GameObjects in `quests_duskwood.toml`, `[[gameobject]]` rows in the form of Keane's outpost
(`quests_elwynn.toml` line 1594), z 0.0 for `ground_check.py --fix`, round ticket 21's camp
centre (-10460, -470):

| Guid | Entry | What | Display | Position (x, y) |
|---|---|---|---|---|
| 793011 | 980552 | tent | 6738 | -10448, -458 |
| 793012 | 980552 | tent | 6738 | -10474, -484 |
| 793013 | 980553 | campfire | 192 | -10455, -477 |

## Story quests

**98901 The Agents' Camp.** Level 46. Giver and ender Commander Althea Ebonlocke (264),
`after = 98602`. Kill 8 Grey Robe Agents (96051) and 4 Grey Robe Wardens (96052) at the camp in
Twilight Grove, and collect 3 Stolen Records (980431) from either at 40%. Money 1610.
Objectives: "Kill 8 Grey Robe Agents and 4 Grey Robe Wardens at the camp in Twilight Grove, and recover 3 Stolen Records for Commander Althea Ebonlocke."
Details: "Someone broke into the hall of records a week ago and took nothing but paper. Sirra's papers, about the night the dark came. My watchers followed the tracks to a camp in Twilight Grove: tents, a fire, men in grey with a crown on their breasts, and not one of them asleep. They're not digging. They're reading. Get Sirra's papers back."
Progress: "Did you find the camp?"
Completion: "Sirra's hand, every page, with notes in the margins in someone else's. They underlined every line about the house on the hill. Where it was, how it went wrong, what it left behind. Whoever they serve is studying our night like a lesson."

**98902 The Grey Captain.** Level 47, an item quest. Giver and ender Althea, `after = 98901`. Kill
the Grey Captain (96053) and collect the Crowned Commission (980432) from him at 100%.
`choices = [903310, 903311, 903312]` (blue Quest Line rows, ticket 20). Money 2115.
Objectives: "Kill the Grey Captain at the camp in Twilight Grove and bring his commission to Commander Althea Ebonlocke."
Details: "One of them gives the orders in that camp. My watchers call him the Grey Captain, for want of a name. He carries a sword and doesn't read. Kill him, and bring me whatever gives him the right to command."
Progress: "Is he dead?"
Completion: "A commission. 'The bearer commands in the Master's name in the dark wood, and answers to the Master alone.' Under it, the crown, and no name. They aren't from Duskwood. Where they are from, I don't know, and I don't like not knowing."

**98903 Mor'Ladim.** Level 48, an item quest. Giver and ender Sirra Von'Indi (268), `after = 98801`.
Kill Mor'Ladim (522, level 48 by ticket 11's override, in Raven Hill Cemetery) and collect
Mor'Ladim's Skull (980433) from him at 100%. `choices = [903316, 903317, 903318]` (blue Quest Line rows, ticket 20).
Money 2160.
Beat (details): Mor'Ladim was Morgan Ladimore, a hero of the Watch, who killed his family in a
madness twenty years ago and was hanged. The night the dark came, he got up with the rest. Sirra
wants him laid down for good. Completion: Sirra says one of his children lived, and still stands
a post in Darkshire. Local; no thread.

**98904 The Daughter Who Lived.** Level 48. Giver Sirra, ender Watcher Ladimore (576, in
Darkshire), `after = 98903`. `gives` Mor'Ladim's Skull (980433); collect it. Money 0.
Beat: Sarah Ladimore was the child who hid. She stands on the Watch now and has walked past the
cemetery every night for twenty years. She takes the skull without a word for a long moment.

**98905 A Daughter's Love.** Level 49. Giver and ender Watcher Ladimore (576), `after = 98904`.
Explore
`{ place = "Morgan Ladimore's grave", position = [-10495.0, 431.0], radius = 6.0, map = 0 }`, a
plot at Raven Hill Cemetery's north edge. Money 1715.
Beat: she can't bring herself to go; she asks the character to lay her father in his own grave,
with the flowers she has kept. Completion: quiet thanks. Local; no thread.

**98906 The Rotting Orchard.** Level 46. Giver and ender Watcher Dodds (888). No prerequisite.
Kill 12 Nightbane Vile Fangs (206) in the Rotting Orchard. Money 1610.
Beat: the worgen of the orchard have started hunting the south road. Local.

## Bounties (`kind = "bounty"`)

| Id | Giver | Level | Objective | Money |
|---|---|---|---|---|
| 98911 | Althea (264) | 47 | Kill 10 Nightbane Tainted Ones (920) in the Rotting Orchard | 1410 |
| 98912 | Watcher Paige (499) | 47 | Kill Nefaru (534) and 6 Nightbane Vile Fangs (206) | 1410 |
| 98913 | Chef Grual (272) | 46 | Collect 8 Worgen Haunches (980434) from Nightbane Vile Fangs (206) and Tainted Ones (920) at 50% | 1380 |
| 98914 | Jitters (288) | 48 | Kill 8 Grey Robe Agents (96051) in Twilight Grove | 1440 |

## Quest items (`quest_items_duskwood.toml`)

| Entry | Name | Form | Description or pages |
|---|---|---|---|
| 980431 | Stolen Records | stack of 20 | "Pages in Sirra's hand, with someone else's notes in the margins." |
| 980432 | Crowned Commission | single, readable, one page | page: "The bearer commands in the Master's name in the dark wood, and answers to the Master alone. Below it, a crown pressed into grey wax." |
| 980433 | Mor'Ladim's Skull | single; collected by 98903, given again by 98904 | "Heavy, and colder than it should be." |
| 980434 | Worgen Haunch | stack of 20 | "Grual will not say what he does with these." |

## Gear choices (decision 13 as amended)

Every story quest offers a choice of greens from its band's Quest Lines (ticket 05 made the
Lines; the band's gear ticket wrote these rows). Add the `choices` line the table gives to each
quest's row. The slots follow #448 decision 9's cycle in quest-id order across the whole band, all
tickets together (main hand, ring, Chest, amulet, off hand, ring, main hand, ring, and again).
Bounties pay money and offer no choice. 98902 and 98903, named-elite item quests, offer blue rows instead (above).

| Quest | Slot | `choices` | Rows from |
|---|---|---|---|
| 98901 | ring, the b Lines | `[903275, 903277, 903279]` | 20-gear-45-50 |
| 98904 | Chest, one per armour type | `[903260, 903261, 903262, 903263]` | 20-gear-45-50 |
| 98905 | amulet | `[903271, 903272, 903273]` | 20-gear-45-50 |
| 98906 | off hand: shield, bow, held item | `[903265, 903267, 903270]` | 20-gear-45-50 |

## Failing tests first

`crates/twilight-realm/tests/duskwood_30_50.rs` gains:

- All ten quests load with their givers and `zone_or_sort = 10`.
- The chains hold: 98901 after 98602; 98902 after 98901; 98903 after 98801; 98904 after 98903;
  98905 after 98904.
- The Grey Captain's corpse gives the Commission while 98902 wants it, and 98902 offers exactly
  903310 to 903312; 98903 offers exactly 903316 to 903318; every story quest offers the choices the gear table names.
- 98904's accept puts the Skull in the bag and Ladimore's hand-in takes it.
- Reaching the grave credits 98905.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `icon_check.py` reports no new clash;
  `ground_check.py` flags nothing after `--fix`.

Assumptions allowed: item display ids and icons, the camp's object spots within 10 yd, and
wording inside a beat.
