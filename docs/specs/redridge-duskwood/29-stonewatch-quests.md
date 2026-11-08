Title: Stonewatch Keep's quests: Gath'Ilzogg's war chest and page three (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 5 and 6. Blocked by 27-ilgalar-quests (same file), 20-gear-45-50 (the
Quest Line rows for its other quests), 22-redridge-45-50 (Sergeant Crane, 98403 and item 980386), and the 30-50
dungeons spec's Stonewatch Keep tickets that build the dungeon's creatures and loot (they edit
`stonewatch_keep.toml`; the filer names them here as DUNGEONS:stonewatch-monsters and
DUNGEONS:stonewatch-loot).

Two quests, 98511 and 98512, in `content/quests_redridge_dungeons.toml` under
`# ── Stonewatch Keep (#PARENT) ──`, and Gath'Ilzogg's drop. Stonewatch owns rung 8 (the dragons
are in it); the dungeons spec shows it. These quests may say what came down on the roof was no
orc, and no more.

## How to write the rows

As ticket 07-redridge-30-35 says (`zone_or_sort = 44`, quoted text is the contract, no em dashes,
the never-name list: no Nefarian, no Onyxia, no Prestor). A collect row for a `drop` item names no
`from`.

## The drop

In `content/stonewatch_keep.toml`, Gath'Ilzogg is a Boss Bar creature on the keep's roof (the
dungeons spec's draft has him first and Singe after, St1). Add Morganth's Notes, Page Three
(980386, defined by ticket 22), to Gath'Ilzogg's creature row's `drop` list: the page is in his
war chest. If the dungeons spec built a war-chest object instead, put the page in it and record
it.

## The quests

**98511 Gath'Ilzogg.** Level 48, an item quest. Giver and ender Sergeant Crane (190084),
`after = 98403`. One kill row for each Boss Bar creature on the keep's roof as the dungeons spec
built them (Gath'Ilzogg, and Singe if he fights there), and collect Page Three.
`choices` = the Stonewatch Keep set's four Chests, the Stockade rule (97303 names the Riot set's 905060 to 905063): the set's cloth, leather, mail and plate rows as the dungeons spec's loot ticket (DUNGEONS:stonewatch-loot) wrote them into `gear.csv`. The gear 31-60 draft reserves 905200 to 905219 for that set; take the entries from `gear.toml`, not from this line. Money 2160.
Objectives: "Go into Stonewatch Keep, kill Gath'Ilzogg on its roof, and bring Sergeant Crane what he keeps in his war chest."
Details: "Gath'Ilzogg keeps a war chest on the keep's roof, my scouts say, and sleeps beside it. Whatever is in it matters to him more than gold. Go in through the gate, up through the halls, all the way to the roof. Kill him. Bring me what's in that chest. And if the shapes in the yard are what I think they are, don't stop to argue with them."
Progress: "The roof?"
Completion: "Dead. And whatever came down on that roof after him wasn't an orc, was it. I can see it on you. This was in the chest: one page of cipher, wrapped in oilcloth like treasure, with a smith's stamp pressed into the corner. The lad at the inn will want it."
If the roof holds Singe too, the objectives line names both; keep the rest.

**98512 Page Three.** Level 48. Giver Crane, ender Toren (190083, Lakeshire inn),
`after = [98511, 98201]`. `gives` Page Three (980386); collect it. Money 0.
Objectives: "Take the page from Gath'Ilzogg's war chest to Toren at the inn in Lakeshire."
Details: "Take it to Lakeshire. I'd rather it was on the lad's table than in my chest, with the keep's survivors looking for it."
Progress: "Another one?"
Completion: "The stamp. An anvil in a ring of fire. That's Dark Iron work, from the forges under the mountain past the pass. He never told me he bought his casings from them. And this page says where a casing gives, if you know where to strike. Three pages. There's one more. I can feel the shape of it, the way you know a sentence isn't finished."

## Gear choices (decision 13 as amended)

Every story quest offers a choice of greens from its band's Quest Lines (ticket 05 made the
Lines; the band's gear ticket wrote these rows). Add the `choices` line the table gives to each
quest's row. The slots follow #448 decision 9's cycle in quest-id order across the whole band, all
tickets together (main hand, ring, Chest, amulet, off hand, ring, main hand, ring, and again).
Bounties pay money and offer no choice. 98511 ends the dungeon and offers its set's Chests instead (above).

| Quest | Slot | `choices` | Rows from |
|---|---|---|---|
| 98512 | ring, the a Lines | `[903274, 903276, 903278]` | 20-gear-45-50 |

## Failing tests first

`crates/twilight-realm/tests/redridge_dungeon_quests.rs` (ticket 27) gains:

- 98511 is offered by Crane only once 98403 is handed in; Gath'Ilzogg's corpse (or the chest)
  holds Page Three while 98511 wants it; 98511 offers exactly the Stonewatch Keep set's four Chests.
- 98512 is offered only once 98511 and 98201 are handed in; its accept puts the page back in the
  bag and Toren's hand-in takes it.
- Every story quest in the gear table offers exactly the `choices` it names.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `icon_check.py` is unchanged;
  `ground_check.py` flags nothing.
