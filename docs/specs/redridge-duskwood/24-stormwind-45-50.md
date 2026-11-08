Title: Stormwind 45-50: relief that never came (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 2 (court hand-offs only). Blocked by 19-stormwind-40-45 (same files;
97411 follows 97410) and 20-gear-45-50 (the band's Quest Line rows).

One quest, 97411, in `content/quests_stormwind.toml` under the heading ticket 09 made, and one
quest item in `content/quest_items.toml`. It is the last hand-off: Dughan now holds a seal that
paid for Lakeshire's bridge and the same seal on Darkshire's missing charity. The court chain
spec's first quest names 97411 in `after` and names the house. This quest does not.

## How to write the row

As ticket 07-redridge-30-35 says, with `zone_or_sort = 1519`. The text is the contract.

**97411 Relief That Never Came.** Level 46. Giver Lord Ello Ebonlocke (263), ender Marshal Dughan
(240, in Goldshire), `after = [97407, 97410]`. `gives` Darkshire's Relief Letter (980157); collect
it. Money 0.
Objectives: "Carry the letter that promised Darkshire relief to Marshal Dughan in Goldshire."
Details: "Two winters ago a noble house in the city sent Darkshire charity: twenty wagons of grain and blankets, paid for and promised in writing. Not one wagon came. I have the letter that promised them, and the seal on it. General Jonathan tells me there's a marshal in Goldshire who keeps seals that turn up where they shouldn't. Take him this. Charity that never arrives is still somebody's money."
Progress: "From Darkshire, now?"
Completion: "That seal. I've had its twin on my desk a fortnight, off a bridge in Lakeshire. A noble house that pays for bridges it never visits and wagons that never arrive. Somebody's money moves east under that seal, and it isn't buying bread. I know where to start asking now."

## Quest item (`quest_items.toml`)

| Entry | Name | Form | Description |
|---|---|---|---|
| 980157 | Darkshire's Relief Letter | single, given by 97411 | "'Twenty wagons, for the people of Darkshire, with the compliments of a friend.' The same grey-green wax." |

## Gear choices (decision 13 as amended)

Every story quest offers a choice of greens from its band's Quest Lines (ticket 05 made the
Lines; the band's gear ticket wrote these rows). Add the `choices` line the table gives to each
quest's row. The slots follow #448 decision 9's cycle in quest-id order across the whole band, all
tickets together (main hand, ring, Chest, amulet, off hand, ring, main hand, ring, and again).
Bounties pay money and offer no choice.

| Quest | Slot | `choices` | Rows from |
|---|---|---|---|
| 97411 | main hand: sword, dagger, staff | `[903264, 903266, 903268]` | 20-gear-45-50 |

## Failing tests first

`crates/twilight-realm/tests/stormwind_30_50.rs` gains: 97411 is offered only once both 97407 and
97410 are handed in; its accept puts the letter in the bag and Dughan's hand-in takes it.
- Every story quest in the gear table offers exactly the `choices` it names.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `icon_check.py` reports no new clash.
