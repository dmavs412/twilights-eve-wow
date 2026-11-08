Title: Duskwood 40-45: twelve years ago, a machine that went wrong, what the diggers want and Calor's worgen (#PARENT)
Labels: ready-for-agent

Part of #PARENT. Blocked by 13-duskwood-35-40 (same files), 15-gear-40-45 (the band's Quest Line rows) and 16-bands-40-45 (the Surveyors and
Brightwood's levels).

Ten quests, 98801 to 98814 (six story quests and four bounties), in
`content/quests_duskwood.toml` under `# ── Duskwood 40-45 (#PARENT) ──`; three quest items, and
the two Raven Hill Deeps items ticket 28 uses, in `content/quest_items_duskwood.toml`.

Rung 5 lands here: Duskwood's night was made, and made badly. Sirra's chain stands on its own; it
does not wait for Toren's page one (ticket 27), though the two agree when a player has both.

## How to write the rows

As ticket 08-duskwood-30-35 says (`zone_or_sort = 10`, the money rule, quoted text is the
contract, no em dashes, the never-name list). From level 40 Duskwood may say "machine"; keep
"Anchor" out of Duskwood's mouths anyway, since only Toren and the cult know the word.

## Story quests

**98801 Twelve Years Ago.** Level 40. Giver and ender Sirra Von'Indi (268), `after = 98606`.
`talk` to Watcher Royce (999), Watcher Keller (885) and Watcher Hartin (886), all in Darkshire.
Money 0.
Objectives: "Ask Watchers Royce, Keller and Hartin what they remember of the night the dark came, then return to Sirra Von'Indi."
Details: "Since you found the Mistmantles' house, I've been reading everything we wrote about that night, and it isn't enough. Three of the Watch stood a post that night and still stand one now. Royce, Keller and Hartin. They don't talk about it. They might talk to you. Ask them what they remember, not what they think it means. I'll do the thinking."
Royce, option "What do you remember of the night the dark came?": "The dogs. Every dog in Darkshire went under a bed an hour before. Then the birds stopped. Then it was dark, like a lid coming down, and it never came off."
Keller, option "What do you remember of the night the dark came?": "A sound from the north, under the ground, like a bell somebody struck and never stopped. I felt it in the gate posts. It's still there, if you put your hand on the stones by that house."
Hartin, option "What do you remember of the night the dark came?": "The dead in the gardens sat up. Not all at once. One by one, as if something had woken them by accident and they didn't know what to do about it."
Progress: "Did they talk?"
Completion: "Dogs that hide, a bell under the ground, the dead woken by accident. That isn't a curse. Curses don't ring."

**98802 A Machine That Went Wrong.** Level 41. Giver Sirra, ender Lord Ello Ebonlocke (263),
`after = 98801`. `gives` Sirra's Account (980421); collect it. Money 0.
Objectives: "Carry Sirra Von'Indi's account to Lord Ello Ebonlocke in Darkshire."
Details: "Everything I have says the same. The night over Duskwood was made, and made badly, by someone who set something in that hill and didn't know how to stop it. Not a god, not a curse. Hands. I've written it out plainly. Take it to Lord Ebonlocke. He must send it to the city, and they must listen."
Progress: "From Sirra?"
Completion: "Made by hands. I believe her. I'll not write it to the city. Do you know what they'll call it? A curse. Then they'll call Darkshire cursed, and stop sending even the letters they don't answer. Keep it, Sirra will. One day someone will ask the right question."

**98803 What the Diggers Want.** Level 42. Giver and ender Jitters (288), `after = 98702`. Kill 6
Grey Robe Surveyors (96050) and collect 5 Survey Stakes (980422) from them at 60%. Money 1470.
Beat (details): new Grey Robes have come, not digging but measuring, driving stakes in lines
across the cemetery.
Completion: "Stakes in a line, from the west wall to the crypt door, every one with that crown on it. They're not robbing graves. They're following something under them. A seam, the miners in Elwynn would call it. Whatever's down there, they want all of it."

**98804 Into the Deeps.** Level 42. Giver and ender Jitters, `after = 98803`. Explore
`{ place = "The crypt beneath Raven Hill", position = [-10418.6, 410.8], radius = 10.0, map = 0 }`
(Raven Hill Deeps' portal on the mausoleum floor, `raven_hill_deeps.toml`). Money 1470.
Beat: the stakes end at the crypt under the mausoleum, and the Grey Robes go down every night
and fewer come up. Completion: "They've broken into the old crypt. If you go down, take friends."
Ticket 28's 99011 "The Deacon" follows this quest.

**98805 Worgen in the Woods.** Level 41. Giver and ender Calor (663, in Darkshire). No
prerequisite. Kill 10 Nightbane Worgen (898) and 10 Nightbane Shadow Weavers (533) in Brightwood
Grove. Money 1435.
Beat: Calor's worgen. The packs hunt the grove's roads and Calor has buried too many friends.
Local; no thread.

**98806 Fenros.** Level 43. Giver and ender Calor, `after = 98805`. Kill Fenros (507) and 8
Nightbane Dark Runners (205). Money 1505.
Beat: Fenros leads the packs. Local; no thread.

## Bounties (`kind = "bounty"`)

| Id | Giver | Level | Objective | Money |
|---|---|---|---|---|
| 98811 | Watcher Royce (999) | 41 | Kill 10 ravagers (family "Black Ravagers slain", 628 and 1258) | 1230 |
| 98812 | Tavernkeep Smitts (273) | 42 | Collect 8 Nightbane Pelts (980423) from Nightbane Worgen (898), Shadow Weavers (533) and Dark Runners (205) at 50% | 1260 |
| 98813 | Jitters (288) | 42 | Kill 10 of the cemetery's ghouls (family "Ghouls slain", 604 and 570) | 1260 |
| 98814 | Watcher Keller (885) | 43 | Kill 10 of the cemetery's skeletons (family "Skeletons slain", 785 and 1110) | 1290 |

## Quest items (`quest_items_duskwood.toml`)

| Entry | Name | Form | Description or pages |
|---|---|---|---|
| 980421 | Sirra's Account | single, given by 98802 | "Six pages in Sirra's hand. The last line: 'Made by hands.'" |
| 980422 | Survey Stake | stack of 20 | "A grey ash stake, a crown burned into its head." |
| 980423 | Nightbane Pelt | stack of 20 | "Coarse and black, and it smells of wet dog and worse." |
| 980486 | Crowned Orders | single, readable, two pages; collected by ticket 28's 99011 | page one: "Dig to the grey and no further. What stands in it belongs to the Master. Touch nothing that stands still." page two: "Find the apprentice who reads M.'s hand. Bring him whole. Below it, a crown." |
| 980487 | Morganth's Notes, Page Two | single; collected by 99011, given again by 99012 | "One page of cipher, torn from a bound book. Grey dust in the creases." |

980486 and 980487 are defined here so ticket 28 edits no item file.

## Gear choices (decision 13 as amended)

Every story quest offers a choice of greens from its band's Quest Lines (ticket 05 made the
Lines; the band's gear ticket wrote these rows). Add the `choices` line the table gives to each
quest's row. The slots follow #448 decision 9's cycle in quest-id order across the whole band, all
tickets together (main hand, ring, Chest, amulet, off hand, ring, main hand, ring, and again).
Bounties pay money and offer no choice.

| Quest | Slot | `choices` | Rows from |
|---|---|---|---|
| 98801 | main hand: sword, dagger, staff | `[903244, 903246, 903248]` | 15-gear-40-45 |
| 98802 | ring, the b Lines | `[903255, 903257, 903259]` | 15-gear-40-45 |
| 98803 | Chest, one per armour type | `[903240, 903241, 903242, 903243]` | 15-gear-40-45 |
| 98804 | amulet | `[903251, 903252, 903253]` | 15-gear-40-45 |
| 98805 | off hand: shield, bow, held item | `[903245, 903247, 903250]` | 15-gear-40-45 |
| 98806 | ring, the a Lines | `[903254, 903256, 903258]` | 15-gear-40-45 |

## Failing tests first

`crates/twilight-realm/tests/duskwood_30_50.rs` gains:

- All ten quests load with their givers and `zone_or_sort = 10`.
- The chains hold: 98801 after 98606; 98802 after 98801; 98803 after 98702; 98804 after 98803;
  98806 after 98805.
- Speaking with the three watchers credits 98801; 98802's accept puts Sirra's Account in the bag
  and Ebonlocke's hand-in takes it.
- Standing at the crypt's portal credits 98804.
- Items 980486 (two pages) and 980487 exist.
- Every story quest in the gear table offers exactly the `choices` it names.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `icon_check.py` reports no new clash;
  `ground_check.py` flags nothing.

Assumptions allowed: item display ids and icons, and wording inside a beat.
