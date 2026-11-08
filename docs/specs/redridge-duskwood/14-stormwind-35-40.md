Title: Stormwind 35-40: Darkshire's thirteenth letter and the General's answer (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 2 (court hand-offs only). Blocked by 09-stormwind-30-35 (same files),
08-duskwood-30-35 (97406 follows 98601) and 10-gear-35-40 (the band's Quest Line rows).

Two quests, 97406 and 97407, in `content/quests_stormwind.toml` under the heading ticket 09 made,
and two quest items in `content/quest_items.toml`. The "missing news" motif: the city answers no
letters. Nothing here says who stops them; that is the court spec's.

## How to write the rows

As ticket 07-redridge-30-35 says, with `zone_or_sort = 1519`. The text below is the contract.

**97406 Darkshire's Thirteenth Letter.** Level 37. Giver Lord Ello Ebonlocke (263), ender General
Marcus Jonathan (466), `after = 98601`. `gives` Lord Ebonlocke's Letter (980154); collect it.
Money 0.
Objectives: "Carry Lord Ebonlocke's letter to General Marcus Jonathan at the gate of Stormwind."
Details: "My letters to the Keep go up the hill and come back down to the barracks, I'm told, marked 'for the military'. Very well. Here is one for the military, by hand. Darkshire needs twenty men and a wagon of grain before the frost. Give it to General Jonathan himself. If he says no, I want to hear that he said it."
Progress: "From Darkshire?"
Completion: "Thirteen letters. I've read every one. They reach me in a bundle, opened, a month late, with a note saying the court is aware. I can't send twenty men. I can write back in my own hand under my own seal, and you'll carry it, so it doesn't go up the hill first."

**97407 An Answer for Darkshire.** Level 37. Giver General Jonathan (466), ender Lord Ebonlocke
(263), `after = 97406`. `gives` General Jonathan's Reply (980155); collect it. Money 0.
Objectives: "Carry General Jonathan's reply to Lord Ello Ebonlocke in Darkshire."
Details: "Tell him the truth before he reads it. The army is held at the walls by order, and I've argued the order twice. The letter says the same with more manners."
Progress: "You're back."
Completion: "'I have argued it twice.' A general who argues with the court and loses, and says so to my face. That's an answer, at least. It's the first one I've had in a year."

## Quest items (`quest_items.toml`)

| Entry | Name | Form | Description |
|---|---|---|---|
| 980154 | Lord Ebonlocke's Letter | single, given by 97406 | "Sealed with the Ebonlocke seal. 'For the military. By hand.'" |
| 980155 | General Jonathan's Reply | single, given by 97407 | "Sealed with the General's own seal, not the Keep's." |

Use the next free entries if ticket 09 moved its range, and record it.

## Gear choices (decision 13 as amended)

Every story quest offers a choice of greens from its band's Quest Lines (ticket 05 made the
Lines; the band's gear ticket wrote these rows). Add the `choices` line the table gives to each
quest's row. The slots follow #448 decision 9's cycle in quest-id order across the whole band, all
tickets together (main hand, ring, Chest, amulet, off hand, ring, main hand, ring, and again).
Bounties pay money and offer no choice.

| Quest | Slot | `choices` | Rows from |
|---|---|---|---|
| 97406 | main hand: sword, dagger, staff | `[903224, 903226, 903228]` | 10-gear-35-40 |
| 97407 | ring, the a Lines | `[903234, 903236, 903238]` | 10-gear-35-40 |

## Failing tests first

`crates/twilight-realm/tests/stormwind_30_50.rs` (ticket 09) gains: 97406 is offered only once
98601 is handed in; 97407 only once 97406 is; each accept puts its letter in the bag and each
hand-in takes it.
- Every story quest in the gear table offers exactly the `choices` it names.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `icon_check.py` reports no new clash.
