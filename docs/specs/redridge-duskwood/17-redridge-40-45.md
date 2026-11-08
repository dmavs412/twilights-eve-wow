Title: Redridge 40-45: Blackrock bounties, Render's Rock, Kazon and Tharil'zun's orders (#PARENT)
Labels: ready-for-agent

Part of #PARENT. Blocked by 12-redridge-35-40 (same files), 15-gear-40-45 (the band's Quest Line rows) and
16-bands-40-45 (the levels and Tharil'zun's override).

Nine quests, 98301 to 98314 (five story quests and four bounties), in
`content/quests_redridge.toml` under `# ── Redridge 40-45 (#PARENT) ──`, and three quest items
in `content/quest_items_redridge.toml`. Keeshan's escort (98391, 98392) is ticket 25, which waits
on the escort spec; nothing here names it.

## How to write the rows

As ticket 07-redridge-30-35 says (`zone_or_sort = 44`, the money rule, quoted text is the
contract, no em dashes, the never-name list).

## Story quests

**98301 Blackrock Bounty.** Level 41. Giver and ender Guard Howe (903). No prerequisite. Collect 8
Blackrock Champion's Badges (980321) from Blackrock Champions (435) at Render's Camp at 50%.
Money 1435.
Beat: the Watch pays for proof of every Blackrock champion killed; Howe keeps the badges in a
jar behind the gate. Local; no thread.

**98302 Render's Rock.** Level 42. Giver and ender Marshal Marris (382), `after = 98202`. Explore
`{ place = "Render's Rock", position = [-8667.0, -2283.0], radius = 20.0, map = 0 }` and kill 8
Blackrock Trackers (615). Money 1470.
Beat (details): Corporal Keeshan, Marris's best scout, sent his last report from Render's Rock a
week ago and hasn't come back. Completion: there's no body, and the trackers' trail runs into
Render's Camp; Marris hopes he's hiding, not caught. (Ticket 25's "Missing in Action" follows
this quest.)

**98303 Kazon's Camp.** Level 43. Giver and ender Marris, `after = 98302`. Kill Kazon (584) and 8
Blackrock Summoners (4463) at Render's Camp. Money 1505.
Beat (details): Kazon holds Render's Camp and his summoners cast with the same ringing stones the
canyon gnolls carry. Completion: Kazon took his orders from somewhere; Marris names Tharil'zun.

**98304 Tharil'zun.** Level 44, an item quest. Giver and ender Marris, `after = 98303`. Kill
Tharil'zun (486, level 44 by ticket 16's override, in Stonewatch's yard) and collect Tharil'zun's
Orders (980322) from him at 100%. `choices = [903306, 903307, 903308, 903309]` (blue Quest Line rows, ticket 15). Money
1980.
Objectives: "Kill Tharil'zun in the yard of Stonewatch Keep and bring his orders to Marshal Marris in Lakeshire."
Details: "Kazon took his orders from somewhere, and it wasn't his own head. The orc who gives the Blackrock in Redridge their orders is Tharil'zun. He stands in Stonewatch's yard with an honour guard, under the keep's walls. He'll have orders of his own on him. Kill him and bring them to me. I want to see who writes to an orc."
Progress: "Tharil'zun?"
Completion: "Look at the hand. Not orcish. Small and neat, like a clerk's, and that mark at the bottom: the hammer over a black sun, the same as on Baren's crates. The Blackrock and the cult take their orders from the same desk. The magistrate will want to see this, and then someone in Stormwind who can do something about it."
Ticket 19's 97408 carries the orders to the city.

**98305 Render's Valley.** Level 41. Giver and ender Clyde Ranthal (2697, at the Ranthal farm in
Render's Valley). No prerequisite. Kill Boulderheart (14273) and 8 Blackrock Renegades (437).
Money 1435.
Beat: the Ranthals hold their farm with a pitchfork and a dog, and the orcs and the great rock
beast at the valley's foot want it gone. Local; no thread.

## Bounties (`kind = "bounty"`)

| Id | Giver | Level | Objective | Money |
|---|---|---|---|---|
| 98311 | Guard Howe (903) | 42 | Kill 12 Blackrock Champions (435) | 1260 |
| 98312 | Deputy Feldon (1070) | 42 | Kill 10 Blackrock Summoners (4463) | 1260 |
| 98313 | Clyde Ranthal (2697) | 41 | Collect 8 Ranthal Seed Sacks (980323) from Blackrock Outrunners (485) and Blackrock Grunts (440) at 50% | 1230 |
| 98314 | Wilma Ranthal (6295) | 41 | Kill 10 Murloc Tidecallers (545) | 1230 |

## Quest items (`quest_items_redridge.toml`)

| Entry | Name | Form | Description or pages |
|---|---|---|---|
| 980321 | Blackrock Champion's Badge | stack of 20 | "A black iron badge, still warm from the forge it was struck in." |
| 980322 | Tharil'zun's Orders | single, readable, one page; collected by 98304 and given again by 97408 (ticket 19) | page: "To the warlord in Redridge. Hold the keep. Keep the road east open for the carts and closed to the Watch. Let nothing near the tower that the tower has not sent for. Below it, in the same small hand, a hammer over a black sun." |
| 980323 | Ranthal Seed Sack | stack of 20 | "Next year's barley, if there is a next year." |

## Gear choices (decision 13 as amended)

Every story quest offers a choice of greens from its band's Quest Lines (ticket 05 made the
Lines; the band's gear ticket wrote these rows). Add the `choices` line the table gives to each
quest's row. The slots follow #448 decision 9's cycle in quest-id order across the whole band, all
tickets together (main hand, ring, Chest, amulet, off hand, ring, main hand, ring, and again).
Bounties pay money and offer no choice. 98304, a named-elite item quest, offers blue rows instead (above).

| Quest | Slot | `choices` | Rows from |
|---|---|---|---|
| 98301 | amulet | `[903251, 903252, 903253]` | 15-gear-40-45 |
| 98302 | off hand: shield, bow, held item | `[903245, 903247, 903250]` | 15-gear-40-45 |
| 98303 | ring, the b Lines | `[903255, 903257, 903259]` | 15-gear-40-45 |
| 98305 | main hand: sword, dagger, mace | `[903244, 903246, 903249]` | 15-gear-40-45 |

## Failing tests first

`crates/twilight-realm/tests/redridge_30_50.rs` gains:

- All nine quests load with their givers and `zone_or_sort = 44`.
- The chains hold: 98302 after 98202; 98303 after 98302; 98304 after 98303.
- Reaching Render's Rock and killing 8 Trackers meets 98302.
- Tharil'zun's corpse drops his Orders while 98304 wants them; 98304 offers exactly 903306 to
  903309; every story quest offers the choices the gear table names.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `icon_check.py` reports no new clash;
  `ground_check.py` flags nothing.

Assumptions allowed: item display ids and icons, and wording inside a beat.
