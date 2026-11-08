Title: Duskwood 35-40: Raven Hill's watch, the Grey Robes, ogres the wrong way and Sven's revenge (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 14. Blocked by 08-duskwood-30-35 (same files), 10-gear-35-40 (the band's
Quest Line rows) and 11-bands-35-40 (the Grey Robes and the cemetery's levels).

Ten quests, 98701 to 98714 (six story quests and four bounties), in
`content/quests_duskwood.toml` under `# ── Duskwood 35-40 (#PARENT) ──`, and four quest items in
`content/quest_items_duskwood.toml`.

## How to write the rows

As ticket 08-duskwood-30-35 says (`zone_or_sort = 10`, the money rule, quoted text is the
contract, no em dashes, the never-name list, no "Anchor" in Duskwood yet). The Grey Robes' mark
is a crown; their master is never named. The cult's mark is the hammer over a black sun.

## Story quests

**98701 Raven Hill's Watch.** Level 36. Giver and ender Jitters (288, in Raven Hill). No
prerequisite. Explore a list of two spots, map 0: `{ place = "The open graves by the west wall",
position = [-10470.0, 250.0], radius = 15.0 }` and `{ place = "The fresh earth by the north wall",
position = [-10500.0, 320.0], radius = 15.0 }` (two of ticket 11's Grey Robe camps). Money 1260.
Beat: the hamlet hears digging in the cemetery every night and Jitters, who jumps at his own
shadow, wants someone braver to look. Completion: "Men. Not the dead. Men in grey, with lanterns,
digging." No more than that yet.

**98702 The Grave Robbers.** Level 37. Giver and ender Jitters, `after = 98701`. Kill 8 Grey Robe
Diggers (96048) and 4 Grey Robe Lantern-bearers (96049), and collect 4 Crowned Tokens (980411)
from either at 50%. Money 1295.
Objectives: "Kill 8 Grey Robe Diggers and 4 Grey Robe Lantern-bearers in Raven Hill Cemetery, and bring 4 Crowned Tokens to Jitters."
Details: "Every night, lanterns in the cemetery and the sound of spades. I went close enough to see, once. Men in grey robes, working like a farm crew, and the dead standing all around them and not touching them. Not touching them. Get me something off one of them, something with a mark on it. I want to know who sends men to dig among the dead and lets them come home whole."
Progress: "Did you get close?"
Completion: "That's not the cult's mark. I've seen theirs, a hammer over a black sun, on crates in Darkshire's square. This is a crown. Different men, a different master, digging in our dead. I don't know which is worse."

**98703 Night Runners.** Level 38. Giver and ender Watcher Dodds (888, on the south road). No
prerequisite. Kill 10 Defias Night Runners (215) and 6 Defias Night Blades (909) around Addle's
Stead. Money 1330.
Beat: the Defias keep the west road empty after dark so that nobody sees what moves on it.
Thread: their purses hold the strange coin nobody has seen before.

**98704 Ogres the Wrong Way.** Level 39. Giver and ender Watcher Paige (499, beside Dodds),
`after = 98703`. Collect 6 Crate Seals (980412) from Splinter Fist ogres (889, 212, 891, 892,
1251) at 40%. Money 1365.
Beat (details): the ogres of Vul'Gol haul crates north along the old road at night; Paige wants
to know what and where.
Completion: "They carry the crates into the cemetery. Not out of it. Into it. Somebody is paying ogres to haul timber and iron to men who dig among graves. And this seal on the lids is a crown. Somebody is building something out there."

**98705 Sven's Camp.** Level 37. Giver and ender Sven Yorgen (311, at his camp on the Hushed
Bank). No prerequisite. Kill 8 Skeletal Fiends (531) and 6 Carrion Recluses (949) on the
cemetery's west side. Money 1295.
Beat: Sven lost his farm the night the dark came and lives in a camp by the river with his boy;
the cemetery's dead and spiders come to his fire. Local; it sets up 98706.

**98706 Sven's Revenge.** Level 40, an item quest. Giver and ender Sven, `after = 98705`. Kill
Morbent Fel (1200, level 40 by ticket 11's override, at Forlorn Rowe) and collect Morbent's
Ledger (980413) from him at 100%. `choices = [903303, 903304, 903305]` (blue Quest Line rows, ticket 10). Money 1800.
Objectives: "Kill Morbent Fel at Forlorn Rowe, at the north end of Raven Hill Cemetery, and bring his ledger to Sven Yorgen."
Details: "Twelve years ago the night came down on my farm and I ran with my boy and nothing else. Since then I've watched a man walk that cemetery like he owns it. Morbent Fel, he calls himself. He keeps the grey robes digging, and the dead step aside for him. He lives in the old house at Forlorn Rowe, at the cemetery's north end. Kill him. Bring me whatever he writes in. I want to read what my farm was traded for."
Progress: "Is he dead?"
Completion: "A ledger. Spades, lamps, timber and men, column after column, and at the top of every page that crown. No name. Whoever he answered to doesn't sign. Thank you. My boy and I can sleep tonight, I think. First time in twelve years."

## Bounties (`kind = "bounty"`)

| Id | Giver | Level | Objective | Money |
|---|---|---|---|---|
| 98711 | Jitters (288) | 38 | Kill 10 Flesh Eaters (3) in Raven Hill Cemetery | 1140 |
| 98712 | Watcher Paige (499) | 39 | Kill 10 Splinter Fist Warriors (212) at Vul'Gol | 1170 |
| 98713 | Lars (893) | 36 | Kill 10 Black Widow Hatchlings (930) in Twilight Grove | 1080 |
| 98714 | Tavernkeep Smitts (273) | 37 | Collect 8 Ravager Haunches (980414) from Young Black Ravagers (923) at the Yorgen Farmstead at 50% | 1110 |

## Quest items (`quest_items_duskwood.toml`)

| Entry | Name | Form | Description or pages |
|---|---|---|---|
| 980411 | Crowned Token | stack of 20 | "A grey wooden token, a plain crown cut into one face." |
| 980412 | Crate Seal | stack of 20 | "Grey wax off a crate lid, pressed with a crown." |
| 980413 | Morbent's Ledger | single, readable, one page | page: "Spades, twelve. Lamps, twenty. Timber, three carts, by the ogres. Men, fourteen. All to the hill, as ordered. At the top, a crown." |
| 980414 | Ravager Haunch | stack of 20 | "Smitts says it stews well. Smitts has never been wrong about a stew." |

## Gear choices (decision 13 as amended)

Every story quest offers a choice of greens from its band's Quest Lines (ticket 05 made the
Lines; the band's gear ticket wrote these rows). Add the `choices` line the table gives to each
quest's row. The slots follow #448 decision 9's cycle in quest-id order across the whole band, all
tickets together (main hand, ring, Chest, amulet, off hand, ring, main hand, ring, and again).
Bounties pay money and offer no choice. 98706, a named-elite item quest, offers blue rows instead (above).

| Quest | Slot | `choices` | Rows from |
|---|---|---|---|
| 98701 | Chest, one per armour type | `[903220, 903221, 903222, 903223]` | 10-gear-35-40 |
| 98702 | amulet | `[903231, 903232, 903233]` | 10-gear-35-40 |
| 98703 | off hand: shield, bow, held item | `[903225, 903227, 903230]` | 10-gear-35-40 |
| 98704 | ring, the a Lines | `[903234, 903236, 903238]` | 10-gear-35-40 |
| 98705 | main hand: sword, dagger, mace | `[903224, 903226, 903229]` | 10-gear-35-40 |

## Failing tests first

`crates/twilight-realm/tests/duskwood_30_50.rs` (ticket 08) gains:

- All ten quests load with their givers and `zone_or_sort = 10`.
- The chains hold: 98702 after 98701; 98704 after 98703; 98706 after 98705.
- Reaching both cemetery spots credits 98701, each on its own log row.
- Killing a Grey Robe Digger credits 98702 and can drop a Crowned Token while the quest wants
  more.
- 98706 offers exactly 903303, 903304 and 903305, and every story quest offers the choices the gear table names.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `icon_check.py` reports no new clash;
  `ground_check.py` flags nothing.

Assumptions allowed: item display ids and icons, and wording inside a beat.
