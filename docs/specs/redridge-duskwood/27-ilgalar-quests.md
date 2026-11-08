Title: The Tower of Ilgalar's quests: Morganth, the cipher and page one (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 4, 5 and 6. Blocked by 10-gear-35-40 (the Quest Line rows for its other quests),
12-redridge-35-40 (98202, Toren and items 980381 to 980383), and the 30-50 dungeons spec's Tower
of Ilgalar tickets that build the dungeon's creatures and loot (they edit `ilgalar.toml`; the
filer names them here as DUNGEONS:ilgalar-monsters and DUNGEONS:ilgalar-loot).

Three quests, 98501 to 98503, in `content/quests_redridge_dungeons.toml` under
`# ── The Tower of Ilgalar (#PARENT) ──`, and Morganth's drops. Rung 4 lands here: the tower is a
working Anchor and it keeps running after Morganth dies. Toren is the first to say "Anchor".

## How to write the rows

As ticket 07-redridge-30-35 says (`zone_or_sort = 44`, quoted text is the contract, no em dashes,
the never-name list). The letters' writer is never named (decision 14). A collect row for an item
the boss holds through its `drop` names no `from`, as the Stockade's 97305 does.

## The drops

In `content/ilgalar.toml`, Morganth is the dungeons spec's final boss on the Boss Bar. Add
Morganth's Failure Notes (980381) and Letters Under a Crowned Seal (980382), both defined by
ticket 12, to his creature row's `drop` list, as Captain Rook's row carries his keyring and
letters in `stockade_riot.toml`.

## The quests

**98501 Morganth.** Level 38, an item quest. Giver and ender Marshal Marris (382),
`after = 98202`. Kill Morganth (his entry from `ilgalar.toml`) and collect the Failure Notes and
the Letters. `choices` = the Tower of Ilgalar set's four Chests, the Stockade rule (97303 names the Riot set's 905060 to 905063): the set's cloth, leather, mail and plate rows as the dungeons spec's loot ticket (DUNGEONS:ilgalar-loot) wrote them into `gear.csv`. The gear 31-60 draft reserves 905160 to 905179 for that set; take the entries from `gear.toml`, not from this line. Money 1710.
Objectives: "Kill Morganth at the top of the Tower of Ilgalar and bring Marshal Marris whatever he was writing."
Details: "You found who's ringing the bell. A mage the gnolls call Morganth, who keeps the Blackrock carrying and the tower humming. He's at the top. I can't send the Watch up there, and you know it. Kill him, and bring me his papers. A man who builds something like that writes it down."
Progress: "Is he dead?"
Completion: "Dead, and the hill still hums. Listen to it. Whatever he built doesn't need him to keep running. And this, all of it cipher. A book of it, and letters under a seal I've never seen. There's a lad at the inn who goes white when anyone says 'tower'. I'd bet a month's pay he can read this."

**98502 The Cipher.** Level 39. Giver Marris, ender Toren (190083, Lakeshire inn),
`after = 98501`. `gives` the Failure Notes (980381) and the Letters (980382); collect both.
Money 0.
Objectives: "Take Morganth's notes and letters to Toren at the inn in Lakeshire."
Details: "Howe will be on the inn's stair tonight, and every night after, for as long as the lad is reading. Nobody goes up without my word. Take him these. If he can't read them, nobody can, and we've a tower humming on a hill and no idea why."
Progress: "What have you brought me?"
Completion: "He's dead, then. I knew it when you came in. This is his hand, and our cipher, the one we made up as apprentices so the master couldn't read our notes. I can read a page a night, no faster. The first lines are enough. The tower is a machine. An Anchor, he calls it, and it pulls the grey in. The grey that took Westfall comes from things like this. He built it, and it doesn't stop because he did. And these letters, under the crown. Someone offered him the one thing nobody ever gave him. Credit. Let me read. Come back when you've slept."

**98503 Page One.** Level 40. Giver Toren (190083), ender Sirra Von'Indi (268, Darkshire),
`after = 98502`. `gives` Toren's Translation (980383); collect it. Money 0.
Objectives: "Carry Toren's translation of the first page to Sirra Von'Indi in Darkshire."
Details: "The first page of the failures isn't Redridge. It's Duskwood. 'The Duskwood test', he calls it. A stone set without its tuning, twelve years ago, to see what an untuned one would do. It made night. It woke the dead. He wrote it down as a lesson. There's a historian in Darkshire who'll have records of that night. Take her my translation. She should know what was done to her town, and by whom."
Progress: "From Lakeshire?"
Completion: "'The Duskwood test.' Twelve years of my life, explained on three pages by a boy in Lakeshire and a dead man's notes. It was an experiment, and we were it. Thank him for me. And tell him I'm sorry for whatever it costs him to read his friend's hand."

Sirra's own chain (98801, ticket 18) does not depend on this quest; the two agree.

## Gear choices (decision 13 as amended)

Every story quest offers a choice of greens from its band's Quest Lines (ticket 05 made the
Lines; the band's gear ticket wrote these rows). Add the `choices` line the table gives to each
quest's row. The slots follow #448 decision 9's cycle in quest-id order across the whole band, all
tickets together (main hand, ring, Chest, amulet, off hand, ring, main hand, ring, and again).
Bounties pay money and offer no choice. 98501 ends the dungeon and offers its set's Chests instead (above).

| Quest | Slot | `choices` | Rows from |
|---|---|---|---|
| 98502 | main hand: sword, dagger, staff | `[903224, 903226, 903228]` | 10-gear-35-40 |
| 98503 | ring, the b Lines | `[903235, 903237, 903239]` | 10-gear-35-40 |

## Failing tests first

New file `crates/twilight-realm/tests/redridge_dungeon_quests.rs` (tickets 29 and 30 extend it):

- 98501 is offered by Marris only once 98202 is handed in; Morganth's corpse holds the Notes and
  the Letters while 98501 wants them; 98501 offers exactly the Tower of Ilgalar set's four Chests.
- 98502's accept puts both papers back in the bag and Toren's hand-in takes them.
- 98503 is offered by Toren only once 98502 is handed in, gives the Translation, and Sirra's
  hand-in takes it.
- Every story quest in the gear table offers exactly the `choices` it names.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `icon_check.py` is unchanged;
  `ground_check.py` flags nothing.
