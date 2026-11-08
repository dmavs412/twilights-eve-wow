Title: Stormwind 40-45: the same desk, evidence for Goldshire and the bridge seal (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 2 (court hand-offs only). Blocked by 14-stormwind-35-40 (same files)
and 17-redridge-40-45 (97408 follows 98304; 97410 follows 98105, which ticket 07 landed).

Three quests, 97408 to 97410, in `content/quests_stormwind.toml` under the heading ticket 09
made, and one quest item in `content/quest_items.toml`. Each ends at Marshal Dughan, who keeps the
evidence; the court chain spec's first quest names 97409 or 97410 in `after`. Nothing here names
an office, a house or a person at court.

## How to write the rows

As ticket 07-redridge-30-35 says, with `zone_or_sort = 1519`. The text below is the contract.

**97408 The Same Desk.** Level 44. Giver Marshal Marris (382), ender General Marcus Jonathan
(466), `after = 98304`. `gives` Tharil'zun's Orders (980322, ticket 17); collect it. Money 0.
Objectives: "Take Tharil'zun's orders to General Marcus Jonathan at the gate of Stormwind."
Details: "These orders go to someone who can raise an army. That isn't the magistrate, and it isn't me. General Jonathan, at Stormwind's gate. He turned Solomon down once. Let's see him turn this down."
Progress: "From Lakeshire again?"
Completion: "The cult's mark on a warlord's orders. I believe it. I'd have believed it a year ago. If I take this up the hill it comes back down unread, or not at all. There's a marshal in Goldshire who keeps papers like this where they don't go missing. Take it to him. Not up the hill."

**97409 Evidence for Goldshire.** Level 44. Giver General Jonathan (466), ender Marshal Dughan
(240, in Goldshire), `after = 97408`. `gives` Tharil'zun's Orders (980322); collect it. Money 0.
Objectives: "Carry Tharil'zun's orders to Marshal Dughan in Goldshire."
Details: "Dughan keeps what the Watch finds and the Keep doesn't want to hear about. He's been doing it for a year. Add this to his pile, and tell him the General sent it."
Progress: "More papers?"
Completion: "Orcs, now. The same mark as on the writ from the Stockade. I'll keep it with the others. That pile is getting too heavy for one marshal and one strongbox. Someone in that city is going to have to read the whole of it at once."

**97410 The Bridge Seal.** Level 42. Giver Foreman Oslow (341), ender Marshal Dughan (240),
`after = 98105`. `gives` the Wax Seal Impression (980156); collect it. Money 0.
Objectives: "Carry the wax seal from the bridge stone's note to Marshal Dughan in Goldshire."
Details: "That wax off the note that came with the bridge stone. I kept it, like I said. The magistrate tells me there's a marshal in Goldshire who collects seals that turn up where they shouldn't. Take it to him. If a friend in the city paid for my bridge, I'd like to thank him by name."
Progress: "A seal?"
Completion: "A house's seal, not an office's. A noble house, an old one, with money enough to buy a bridge in Redridge and not sign its name to it. I know most of the seals in Stormwind. Not this one. I'll find it."

## Quest item (`quest_items.toml`)

| Entry | Name | Form | Description |
|---|---|---|---|
| 980156 | Wax Seal Impression | single, given by 97410 | "Grey-green wax with a house's crest pressed into it. Not the crown's." |

## Gear choices (decision 13 as amended)

Every story quest offers a choice of greens from its band's Quest Lines (ticket 05 made the
Lines; the band's gear ticket wrote these rows). Add the `choices` line the table gives to each
quest's row. The slots follow #448 decision 9's cycle in quest-id order across the whole band, all
tickets together (main hand, ring, Chest, amulet, off hand, ring, main hand, ring, and again).
Bounties pay money and offer no choice.

| Quest | Slot | `choices` | Rows from |
|---|---|---|---|
| 97408 | main hand: sword, dagger, staff | `[903244, 903246, 903248]` | 15-gear-40-45 |
| 97409 | ring, the a Lines | `[903254, 903256, 903258]` | 15-gear-40-45 |
| 97410 | Chest, one per armour type | `[903240, 903241, 903242, 903243]` | 15-gear-40-45 |

## Failing tests first

`crates/twilight-realm/tests/stormwind_30_50.rs` gains: 97408 is offered only once 98304 is handed
in, 97409 only once 97408 is, 97410 only once 98105 is; each accept puts its item in the bag and
each hand-in takes it.
- Every story quest in the gear table offers exactly the `choices` it names.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `icon_check.py` reports no new clash.
