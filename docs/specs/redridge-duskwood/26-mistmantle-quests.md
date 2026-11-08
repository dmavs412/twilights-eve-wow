Title: Mistmantle Manor's quests: Stalvan and the surveyor's site record (#PARENT)
Labels: ready-for-agent

Part of #PARENT. Blocked by 03-quest-files (the file), 05-gear-30-35 (the Quest Line rows for its other quests),
08-duskwood-30-35 (98607 and item 980481), and the 30-50 dungeons spec's Mistmantle Manor tickets
that build the dungeon's creatures and loot (they edit `mistmantle_manor.toml`; the filer names
them here as DUNGEONS:mistmantle-monsters and DUNGEONS:mistmantle-loot).

Two quests, 99001 and 99002, in `content/quests_duskwood_dungeons.toml` under
`# ── Mistmantle Manor (#PARENT) ──`, and one `drop` on the manor's master. The dungeon itself is
the dungeons spec's; this ticket owns only the boss quest's text and reward and the drop.

## How to write the rows

As ticket 08-duskwood-30-35 says (`zone_or_sort = 10`, quoted text is the contract, no em
dashes, the never-name list). **No "Anchor"** at Mistmantle (dungeons prep S1): the record says
"the stone". A collect row for an item the boss holds through its `drop` names no `from`, as the
Stockade's 97305 and 97309 do (`quests_stormwind.toml`).

## The drop

In `content/mistmantle_manor.toml`, the manor's master is the creature the dungeons spec put on
the Boss Bar (`boss = true` in its template; Stalvan Mistmantle in that spec's draft). Add the
Surveyor's Site Record (980481, defined by ticket 08) to his creature row's `drop` list, as
Sister Quill's row carries the Book of Takings in `stockade_quiet_wing.toml`. If the dungeons spec
built no single master, the drop goes on its last Boss Bar creature; record it.

## The quests

**99001 Stalvan.** Level 36, an item quest. Giver and ender Madame Eva (265), `after = 98607`.
Kill the master (his entry from `mistmantle_manor.toml`, one kill row) and collect the Surveyor's
Site Record (980481). `choices` = the Mistmantle Manor set's four Chests, the Stockade rule (97303 names the Riot set's 905060 to 905063): the set's cloth, leather, mail and plate rows as the dungeons spec's loot ticket (DUNGEONS:mistmantle-loot) wrote them into `gear.csv`. The gear 31-60 draft reserves 905140 to 905159 for that set; take the entries from `gear.toml`, not from this line. Money 1620.
Objectives: "Go into the house on the hill, put its master to rest, and bring Madame Eva whatever he guards."
Details: "Stalvan taught the Mistmantle girl her letters. He was in the house the night it went quiet, and he's in it still. I see him at the cellar door, every night, standing over something. Whatever stopped that house is under it, and he guards it because nobody ever told him to stop. Go in. Put him to rest. Bring me what he stands over, if it can be carried."
Progress: "Is it over?"
Completion: "Papers, from the cellar. Not his hand. A surveyor's, with the hammer and the black sun at the foot, and 'the stone' written three times as if it were a name. Something was buried under that house on purpose, by people who wrote down what it did and went on to the next place. Take it to Sirra. She reads better than I see."
If the dungeons spec named the master something other than Stalvan, keep the title and the text
and say so in the closing comment, so Davis can rewrite the line.

**99002 The Site Record.** Level 36. Giver Madame Eva, ender Sirra Von'Indi (268),
`after = 99001`. `gives` the Surveyor's Site Record (980481); collect it. Money 0.
Objectives: "Take the surveyor's site record to Sirra Von'Indi in Darkshire."
Details: "Sirra has spent twelve years on that house. She deserves to read this before anyone else does."
Progress: "From the house? Show me."
Completion: "'Untuned, as ordered. Night over the whole wood within the hour.' Twelve years I've called it a curse because I had no other word. It was an order, carried out badly. And there are other sites. The hill of graves, west of here. A tower in Redridge. They didn't stop at us."

## Gear choices (decision 13 as amended)

Every story quest offers a choice of greens from its band's Quest Lines (ticket 05 made the
Lines; the band's gear ticket wrote these rows). Add the `choices` line the table gives to each
quest's row. The slots follow #448 decision 9's cycle in quest-id order across the whole band, all
tickets together (main hand, ring, Chest, amulet, off hand, ring, main hand, ring, and again).
Bounties pay money and offer no choice. 99001 ends the dungeon and offers its set's Chests instead (above).

| Quest | Slot | `choices` | Rows from |
|---|---|---|---|
| 99002 | off hand: shield, bow, held item | `[903205, 903207, 903210]` | 05-gear-30-35 |

## Failing tests first

New file `crates/twilight-realm/tests/duskwood_dungeon_quests.rs` (ticket 28 extends it):

- 99001 is offered by Eva only once 98607 is handed in; the master's corpse holds the Site Record
  while 99001 wants it; 99001 offers exactly the Mistmantle Manor set's four Chests.
- 99002's accept puts the Site Record back in the bag and Sirra's hand-in takes it.
- Both load with `zone_or_sort = 10`.
- Every story quest in the gear table offers exactly the `choices` it names.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `icon_check.py` is unchanged;
  `ground_check.py` flags nothing.
