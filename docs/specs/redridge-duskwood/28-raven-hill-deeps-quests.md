Title: Raven Hill Deeps' quests: the deacon, the crowned orders and page two (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 4, 5 and 14. Blocked by 26-mistmantle-quests (same file),
15-gear-40-45 (the Quest Line rows for its other quests), 18-duskwood-40-45 (98804 and items 980486 and 980487),
12-redridge-35-40 (Toren), and the 30-50 dungeons spec's Raven Hill Deeps tickets that build the
dungeon's creatures and loot (they edit `raven_hill_deeps.toml`; the filer names them here as
DUNGEONS:raven-hill-monsters and DUNGEONS:raven-hill-loot).

Two quests, 99011 and 99012, in `content/quests_duskwood_dungeons.toml` under
`# ── Raven Hill Deeps (#PARENT) ──`, and the deacon's drops. Rung 5's second half lands here:
the dead in the grey are not silenced; they stop. Toren stays at the inn (decision 4): the orders
naming him change nothing but his resolve.

## How to write the rows

As ticket 08-duskwood-30-35 says (`zone_or_sort = 10`, quoted text is the contract, no em dashes,
the never-name list). Nobody names Sareth or Mourncrown, and nobody says the Lich King is frozen.
A collect row for a `drop` item names no `from`.

## The drops

In `content/raven_hill_deeps.toml`, the deacon is the dungeons spec's final boss on the Boss Bar
(a placeholder name in its draft). Add the Crowned Orders (980486) and Morganth's Notes, Page Two
(980487), both defined by ticket 18, to his creature row's `drop` list.

## The quests

**99011 The Deacon.** Level 44, an item quest. Giver and ender Jitters (288), `after = 98804`. Kill
the deacon (his entry from `raven_hill_deeps.toml`) and collect the Crowned Orders and Page Two.
`choices` = the Raven Hill Deeps set's four Chests, the Stockade rule (97303 names the Riot set's 905060 to 905063): the set's cloth, leather, mail and plate rows as the dungeons spec's loot ticket (DUNGEONS:raven-hill-loot) wrote them into `gear.csv`. The gear 31-60 draft reserves 905180 to 905199 for that set; take the entries from `gear.toml`, not from this line. Money 1980.
Objectives: "Go down into the crypt beneath Raven Hill, kill the one who leads the Grey Robes there, and bring what he carries to Jitters."
Details: "The Grey Robes come up out of that crypt fewer every night, and the ones who come up don't talk. Lars went to the top of the stair and came back white. He says there's a man down there in a deacon's robe giving orders, and past him the air goes grey and nothing moves. Nothing. Kill him. Bring me what he carries. I want to know what they found down there."
Progress: "You went down?"
Completion: "Orders, under that crown. 'Dig to the grey and no further. What stands in it belongs to the Master.' What stands in it. You saw them, didn't you? The dead, standing in the grey, stopped mid-step. Not gone. Just stopped. And this page, all cipher, torn out of a book. And the second order: 'Find the apprentice who reads M.'s hand.' That's somebody's death warrant. Whose?"
If the dungeons spec gave the boss a name, keep the text and say so in the closing comment.

**99012 Page Two.** Level 44. Giver Jitters, ender Toren (190083, Lakeshire inn),
`after = [99011, 98201]` (the character has met Toren). `gives` Page Two (980487) and the Crowned
Orders (980486); collect both. Money 0.
Objectives: "Take the torn page and the crowned orders to Toren at the inn in Lakeshire."
Details: "There's a lad at the Lakeshire inn the Watch says reads ciphers. I'd guess he's the apprentice in those orders. Take him both. He'd better see what they say about him before they find him."
Progress: "Another page?"
Completion: "His hand. A page of the failures, torn out by someone who couldn't read it and knew it mattered. It says how one of his stones holds its stretch of grey, and why anything dead that walks into it doesn't fall. It stops. They don't die in there. They wait. And these orders want me whole. Let them come. Marris has a man on my stair, and I've run from a cellar once already. I'm not running again."
The text works whether or not the character has brought Toren page one (98502) yet.

## Gear choices (decision 13 as amended)

Every story quest offers a choice of greens from its band's Quest Lines (ticket 05 made the
Lines; the band's gear ticket wrote these rows). Add the `choices` line the table gives to each
quest's row. The slots follow #448 decision 9's cycle in quest-id order across the whole band, all
tickets together (main hand, ring, Chest, amulet, off hand, ring, main hand, ring, and again).
Bounties pay money and offer no choice. 99011 ends the dungeon and offers its set's Chests instead (above).

| Quest | Slot | `choices` | Rows from |
|---|---|---|---|
| 99012 | main hand: sword, dagger, mace | `[903244, 903246, 903249]` | 15-gear-40-45 |

## Failing tests first

`crates/twilight-realm/tests/duskwood_dungeon_quests.rs` (ticket 26) gains:

- 99011 is offered by Jitters only once 98804 is handed in; the deacon's corpse holds both items
  while 99011 wants them; 99011 offers exactly the Raven Hill Deeps set's four Chests.
- 99012 is offered only once 99011 and 98201 are handed in; its accept puts both items back in the
  bag and Toren's hand-in takes them.
- Every story quest in the gear table offers exactly the `choices` it names.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `icon_check.py` is unchanged;
  `ground_check.py` flags nothing.
