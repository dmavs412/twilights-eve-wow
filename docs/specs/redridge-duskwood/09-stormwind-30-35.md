Title: Stormwind 30-35: Wray's two sets of orders, the Writ Board poster and Solomon's request (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 2 and 3. Blocked by 07-redridge-30-35 (97405 follows 98108) and #593
(it edits `quests_stormwind.toml` too).

Five quests, 97401 to 97405, in `crates/twilight-realm/content/quests_stormwind.toml` after the
Deeprun Tram's, under `# ── Redridge and Duskwood: the way in (#PARENT) ──`; one GameObject (the
poster); three quest items in `content/quest_items.toml`.

## How to write the rows

As ticket 07-redridge-30-35 says, with `zone_or_sort = 1519` (Stormwind City) on all five. The
text below is the contract.

## The way in (decision 3)

There are no Dughan letters. Commander Aldous Wray (Hub NPC, entry 190071, the barracks) and a
poster beside the Writ Board each send the character to both zones. All four are level 30, so
the default Tier rule (`quests.toml` header: 30 to 49 is Tier 2) offers them only after
Advancement. None is a prerequisite of anything: each zone's first quests (98101, 98103, 98601,
98602) are offered without them, so a character who skipped the city still gets the arc. A
character can take all four; each goes to a different person.

**97401 Orders for Lakeshire.** Level 30. Giver Wray (190071), ender Magistrate Solomon (344, in
Lakeshire). `gives` Commander Wray's Orders (980151); collect it. Money 0.
Objectives: "Carry Commander Wray's orders to Magistrate Solomon in Lakeshire, in Redridge."
Details: "You've earned your Tier, $C, and the army has nowhere to send you. Not because we don't need you. Because we're not allowed to go. Magistrate Solomon in Lakeshire has asked the barracks for a company three times. The answer comes down from the Keep: no. I can't send soldiers. I can send you, with my seal on it, so he knows somebody here read his letters. East through Elwynn, past Three Corners."
Progress: "From the barracks?"
Completion: "Wray's seal. So someone in that city does read. One fighter instead of a company. I'll take it. Sit down, and I'll tell you what the company would have done."

**97402 Orders for Darkshire.** Level 30. Giver Wray, ender Lord Ello Ebonlocke (263, in
Darkshire). `gives` Commander Wray's Letter (980152); collect it. Money 0.
Objectives: "Carry Commander Wray's letter to Lord Ello Ebonlocke in Darkshire, in Duskwood."
Details: "Lord Ebonlocke in Darkshire writes to the Keep every month. I know, because his letters come back down to the barracks marked 'for the military', and the military isn't allowed to answer them. Take him this. It isn't an answer. It's an apology, and you. South through Elwynn and over the river."
Progress: "From Stormwind? Truly?"
Completion: "An apology from a commander who isn't allowed to help. It's more than the Keep has sent me in a year. Welcome to Darkshire."

**97403 Hands Wanted: The Lakeshire Watch.** Level 30. `giver_object = 980202` (the poster),
ender Guard Parker (464, at Three Corners). No objective. Money 0.
Objectives: "Report to Guard Parker at Three Corners, on the road into Redridge."
Details: "HANDS WANTED. The Lakeshire Watch pays for every able fighter who reports to Guard Parker at Three Corners, on the road east past Elwynn. Orcs, gnolls, murlocs. Bring your own blade. Signed, Marshal Marris."
Progress: "You read the poster?"
Completion: "You read the poster. Marris will be pleased. He paid for the ink himself."

**97404 Hands Wanted: The Night Watch.** Level 30. `giver_object = 980202`, ender Commander
Althea Ebonlocke (264, in Darkshire). No objective. Money 0.
Objectives: "Report to Commander Althea Ebonlocke in Darkshire, in Duskwood."
Details: "HANDS WANTED. The Night Watch of Darkshire takes anyone who can hold a blade after dark. Report to Commander Althea Ebonlocke in Darkshire, south through Elwynn and over the bridge. Pay, board, and no questions about the dark. Signed, A. Ebonlocke."
Progress: "Here about the poster?"
Completion: "Somebody read it. Good. You can start tonight. Every night is tonight, here."

## Solomon's request (decision 2, a court hand-off)

**97405 A Magistrate's Request.** Level 34. Giver Solomon (344), ender General Marcus Jonathan
(466, the stock spawn at the city gate, -8954, 521), `after = 98108`. `gives` Solomon's Request
(980153); collect it. Money 0.
Objectives: "Take Magistrate Solomon's request to General Marcus Jonathan at the gate of Stormwind."
Details: "Marris says the orcs are being paid. I'll not wait for them to finish counting. This is a request for one company of the army, in my hand, with the Watch's numbers and yours. Take it to General Marcus Jonathan at the city gate. Not to the Keep. My letters to the Keep don't come back."
Progress: "A letter from Lakeshire?"
Completion: "Solomon. A good man, and right about the orcs, I'd wager. I can't send him a company. The court has ruled that the Blackrock are a Redridge matter, and the army stays at the walls until it says otherwise. I've asked. Tell him I asked."

## The poster

A `[[gameobject]]` row in `quests_stormwind.toml`, in the form of the Writ Board's in
`writs.toml` (line 23): `guid = 791002`, `entry = 980202`, `display = 17` (vanilla's Wanted
Poster, a placeholder listed by ticket 03), `map = 0`, `position = [-8823.0, 667.0, 95.43]`,
`orientation = 0.0`. It stands on the Trade District's street floor beside the board:
`wmo_floor.py -8823 667` lists group 277's floor at z 95.43. Let `ground_check.py` check it.

## Quest items (`quest_items.toml`, after 980131)

| Entry | Name | Form | Description |
|---|---|---|---|
| 980151 | Commander Wray's Orders | single, given by 97401 | "Sealed with the barracks' seal. For Magistrate Solomon, Lakeshire." |
| 980152 | Commander Wray's Letter | single, given by 97402 | "Sealed with the barracks' seal. For Lord Ebonlocke, Darkshire." |
| 980153 | Solomon's Request | single, given by 97405 | "A request for one company of soldiers, signed and sealed by the Magistrate of Lakeshire." |

If #593 or another landed ticket has taken any of 980151 to 980159, use the next free entries
above the highest in `quest_items.toml` and record it.

## Gear choices (decision 13 as amended)

Every story quest offers a choice of greens from its band's Quest Lines (ticket 05 made the
Lines; the band's gear ticket wrote these rows). Add the `choices` line the table gives to each
quest's row. The slots follow #448 decision 9's cycle in quest-id order across the whole band, all
tickets together (main hand, ring, Chest, amulet, off hand, ring, main hand, ring, and again).
Bounties pay money and offer no choice.

| Quest | Slot | `choices` | Rows from |
|---|---|---|---|
| 97401 | main hand: sword, dagger, staff | `[903204, 903206, 903208]` | 05-gear-30-35 |
| 97402 | ring, the a Lines | `[903214, 903216, 903218]` | 05-gear-30-35 |
| 97403 | Chest, one per armour type | `[903200, 903201, 903202, 903203]` | 05-gear-30-35 |
| 97404 | amulet | `[903211, 903212, 903213]` | 05-gear-30-35 |
| 97405 | off hand: shield, bow, held item | `[903205, 903207, 903210]` | 05-gear-30-35 |

## Failing tests first

New file `crates/twilight-realm/tests/stormwind_30_50.rs` (tickets 14, 19 and 24 extend it), with
a Tier 2 party as `tests/stockade.rs`'s `questing_party` (line 1103) makes one:

- Wray offers 97401 and 97402 to a level 30 Zealot and not to a level 30 Tier 1 character.
- The poster (980202) offers 97403 and 97404 to a level 30 Zealot; both complete at their enders
  with nothing to collect.
- 97401's, 97402's and 97405's accepts put their item in the bag, and the hand-in takes it.
- 97405 is offered by Solomon only once 98108 is handed in.
- None of 98101, 98103, 98601 or 98602 names a 974xx quest in `after`.
- Every story quest in the gear table offers exactly the `choices` it names.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `icon_check.py` reports no new clash;
  `ground_check.py` flags nothing.

Assumptions allowed: item display ids and icons.

Note for #556 (Tier 2 Calling Choice in Stormwind, not yet grilled): if that spec moves Wray or
replaces him, 97401 and 97402 move with whoever gives Tier 2 Advancement.
