Title: The Unsent Letter: page four and Morganth's letter reach Toren (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 4, 5, 6 and 7. Blocked by 29-stonewatch-quests (same file; 98512),
28-raven-hill-deeps-quests (99012) and 22-redridge-45-50 (98404, Sergeant Crane, items 980391
and 980392). 27-ilgalar-quests (98502) lands before 29.

One quest, 98521, in `content/quests_redridge_dungeons.toml` under
`# ── The Unsent Letter (#PARENT) ──`. It ends the arc. With all four pages, Toren knows how an
Anchor breaks; the break itself is the Steppes opening spec's (`docs/specs/prep/steppes-opening.md`),
whose break quest names 98521 in `after` and is given by this Toren (190083). Breaking the Anchor does not open the land: it starts a server-wide progress bar, filled by new Writs and bounties at a camp at Blackrock Pass, and the Steppes open when it fills.
Nothing in this quest's text promises that the break opens the Steppes. Decision 7: nothing
here asks for the court chain or the Cathedral.

## How to write the row

As ticket 07-redridge-30-35 says (`zone_or_sort = 44`, quoted text is the contract, no em dashes,
the never-name list). The letter's own text is item 980392's pages (ticket 22).

**98521 The Unsent Letter.** Level 49. Giver Sergeant Crane (190084), ender Toren (190083),
`after = [98404, 98502, 99012, 98512]`: the courier's satchel taken, and pages one, two and three
already with Toren. `gives` Morganth's Notes, Page Four (980391) and Morganth's Unsent Letter
(980392); collect both. Money 0.
Objectives: "Take the courier's last page and the letter folded inside it to Toren at the inn in Lakeshire."
Details: "I said the lad could have the satchel when he was ready. Word from Lakeshire is he's had three pages off you already and sits up nights with them. Here: the last page, and the letter that was folded inside it. I didn't read the letter. Much. It isn't mine to read."
Progress: "Is that from the courier?"
Completion: "The last page. That's all of it, then: every way his stones can fail, start to end. I could tell you how to break one now. The one on the hill, even. And this. His own hand, without the cipher. He never once wrote to me plainly. Give me tonight with it. When you're ready to climb that tower again, come back, and I'll tell you how to bring it down."

## Gear choices (decision 13 as amended)

Every story quest offers a choice of greens from its band's Quest Lines (ticket 05 made the
Lines; the band's gear ticket wrote these rows). Add the `choices` line the table gives to each
quest's row. The slots follow #448 decision 9's cycle in quest-id order across the whole band, all
tickets together (main hand, ring, Chest, amulet, off hand, ring, main hand, ring, and again).
Bounties pay money and offer no choice.

| Quest | Slot | `choices` | Rows from |
|---|---|---|---|
| 98521 | main hand: sword, dagger, staff | `[903264, 903266, 903268]` | 20-gear-45-50 |

## Failing tests first

`crates/twilight-realm/tests/redridge_dungeon_quests.rs` gains:

- Crane offers 98521 only once 98404, 98502, 99012 and 98512 are all handed in, and not when any
  one is missing.
- Its accept puts Page Four and the Unsent Letter in the bag; Toren's hand-in takes both.
- The Unsent Letter (980392) reads as three pages.
- Every story quest in the gear table offers exactly the `choices` it names.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `icon_check.py` is unchanged.
