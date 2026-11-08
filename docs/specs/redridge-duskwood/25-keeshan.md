Title: Keeshan: Missing in Action, his escort out of Render's Camp, and what he saw (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 11 and 16. Blocked by 01-escort-spec (the escort system: this ticket
cannot start until that spec's grill and tickets have landed and the spec is closed) and
22-redridge-45-50 (same files, and the camp Keeshan joins).

Two quests, 98391 and 98392, in `content/quests_redridge.toml` under
`# ── Keeshan (#PARENT) ──`; Keeshan's escort as the escort spec built it; Keeshan at the camp.

## Read first

The escort spec's issue and the tickets it closed with, and the escort objective's schema in
`content/quests.toml`'s header as they left it. Field names below that say "the escort's" mean
whatever that spec named them. Where the escort spec settled a question this ticket also
answers, the escort spec wins and the session records the difference.

## How to write the rows

As ticket 07-redridge-30-35 says (`zone_or_sort = 44`, the money rule, quoted text is the
contract, no em dashes, the never-name list).

## The quests

**98391 Missing in Action.** Level 42. Giver and ender Marshal Marris (382), `after = 98302`. The
escort's objective: Corporal Keeshan (349, the friendly vanilla spawn at -8756.8, -2187.1 in
Render's Camp) walks from his hideout to Marris at (-9277.2, -2296.2). Path: vmangos's
`creature_movement_template` rows for entry 349 (68 points, down the canyon road and over the
bridge), cut down or kept as the escort spec's path format allows. One ambush near point 43
(-8914.7, -2579.3): three Blackrock Champions (435) or Trackers (615) at the band's levels
(41-44), unless the escort spec chose another ambush form. The quest fails if Keeshan dies.
Money 1470.
Objectives: "Find Corporal Keeshan in Render's Camp and see him safely back to Marshal Marris in Lakeshire."
Details: "Keeshan's alive. One of the Ranthal boys saw a man in a Watch tabard waving from the rocks above Render's Camp, two nights running. That's Keeshan. He's hiding up there with a whole Blackrock camp between him and home, and he won't come down without someone to watch his back. Go and get him. Bring him to me on his own two feet."
Progress: "Keeshan?"
Completion: "You walked him out through the lot of them. He's thinner, he smells like a cave, and he won't stop talking about what he saw from those rocks. Let him tell it. Then he's going east, to Crane's camp. He insists."
Keeshan's lines on the walk, if the escort system speaks: at the start, "About time. Stay close, they patrol the ridge."; at the ambush, "Here they come!"; at Marris, "Marshal. I've got a report you won't like."

**98392 What Keeshan Saw.** Level 45. Giver Corporal Keeshan at the camp (below), ender Sergeant
Crane (190084), `after = [98391, 98401]`. Explore
`{ place = "The rocks above Stonewatch", position = [-9250.0, -2960.0], radius = 15.0, map = 0 }`.
Money 1575.
Objectives: "Climb to the rocks above Stonewatch Keep, look at the keep's roof, then report to Sergeant Crane."
Details: "Three nights in those rocks I watched the sky over Stonewatch. On the second night something flew over the keep. Wings like a ship's sail. It came down on the roof and the orcs in the yard knelt. Marris thinks I've been in a cave too long. Go up to the rocks above the keep and look at that roof. Then tell Crane what you saw, not what I said."
Progress: "Well?"
Completion: "Scorched stone all along the roof's edge, and claw marks you could lie down in. Then Keeshan isn't mad. I'm going to need more than eight men."

## Keeshan at the camp

A Hub NPC in `content/hub.toml`, with the camp's rows (ticket 22): `guid = 9300088`,
`entry = 190087`, `name = "Corporal Keeshan"`, `subname = "Lakeshire Watch"`, `level = 42`,
`display_id = 1826` (his vanilla display), `faction_template = 35`, `role = "giver"`,
`npc_flags = 0x2`, `map = 0`, `position = [-9506.0, -3260.0, 0.0]` (z for `--fix`),
`faces = [-9386.3, -3046.8]`, `greeting = "I owe you a drink, and Marris owes me a week's pay."`
Whether everyone sees him at the camp, or only characters who have finished 98391, is the escort
spec's question 11; build what it decided. If it decided nothing, he stands at the camp for
everyone, and record that as an assumption.

## Gear choices (decision 13 as amended)

Every story quest offers a choice of greens from its band's Quest Lines (ticket 05 made the
Lines; the band's gear ticket wrote these rows). Add the `choices` line the table gives to each
quest's row. The slots follow #448 decision 9's cycle in quest-id order across the whole band, all
tickets together (main hand, ring, Chest, amulet, off hand, ring, main hand, ring, and again).
Bounties pay money and offer no choice.

| Quest | Slot | `choices` | Rows from |
|---|---|---|---|
| 98391 | ring, the a Lines | `[903254, 903256, 903258]` | 15-gear-40-45 |
| 98392 | ring, the a Lines | `[903274, 903276, 903278]` | 20-gear-45-50 |

## Failing tests first

`crates/twilight-realm/tests/redridge_30_50.rs` gains, in the escort spec's test form:

- Accepting 98391 from Marris starts Keeshan's walk; Keeshan reaching Marris meets the objective.
- Keeshan dying fails 98391, and he stands at his hideout again afterwards as the escort spec's
  reset rule says.
- The ambush fires once per escort.
- 98392 is offered by the camp's Keeshan only once 98391 and 98401 are handed in; reaching the
  rocks credits it.
- Every story quest in the gear table offers exactly the `choices` it names.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `ground_check.py` flags nothing after
  `--fix`.

Assumptions allowed: the path's points (vanilla's or fewer), Keeshan's lines, and the ambush's
creatures within the band.
