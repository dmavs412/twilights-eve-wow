Title: Redridge and Duskwood 30-50 arc (spec)
Labels:

Grilled with Davis on 2026-11-07 from `docs/specs/prep/redridge-duskwood-quests.md`. The grill's
16 answers are below and override the prep draft wherever they differ. The tickets carry the
quest text: spine quests as drafts for Davis to review before filing, local quests and bounties as
beats with writing rules. The bible (`docs/lore-bible.md`) wins over any line here.

## The story

Redridge is the Anchor and Duskwood is the warning (bible, "Redridge and Duskwood"). A Tier 2
character goes back and forth between them through Stormwind, and the bands overlap.

- **30-35.** The Tier 2 trainer in Stormwind and a poster beside the Writ Board send the
  character east. In Redridge, Magistrate Solomon's "worse problems" are real: Blackrock
  outrunners carry the same strange coin as Elwynn's Defias, crates with the cult's mark sink in
  Lake Everstill on their way east, and the stone for the Everstill Bridge came from the city
  under a noble house's seal nobody asks about. Marshal Marris sends the character to look at the
  tower on the eastern hill, and the ground hums. In Duskwood, Lord Ebonlocke has written to the
  city for a year without an answer. The Night Watch is three men short, the dead in Tranquil
  Gardens got up the night the dark came, and the candles in the house on the hill have burned
  for twelve years without going out. Madame Eva opens Mistmantle Manor. General Marcus Jonathan
  turns down Solomon's request for soldiers: the court says the Blackrock are a Redridge matter.
- **35-40.** Toren turns up at Lakeshire's inn. He knows the tower and won't go near it. The
  Watch clears the hill and the character kills Morganth in Ilgalar (the dungeon is the dungeons
  spec's; the boss quest is this arc's). The tower keeps humming. The character carries the
  failure notes and a bundle of letters under a crowned seal to Toren, who reads the cipher a page
  a night. Page one names "the Duskwood test". In Duskwood, men in grey robes with a crowned mark
  dig at Raven Hill Cemetery at night, ogres haul crates into the cemetery instead of out, and
  Sven Yorgen wants Morbent Fel, the Grey Robes' overseer, dead.
- **40-45.** Sirra Von'Indi's records and Toren's page agree: Duskwood's night is a machine that
  went wrong. Lord Ebonlocke won't write it to the city. The Grey Robes are after the seam under
  the cemetery; their deacon in Raven Hill Deeps carries page two and orders to find "the
  apprentice who reads M.'s hand". Toren stays at the inn; Marris keeps a man on his stair. In
  Redridge, Tharil'zun's orders are written in the cult's hand: the orcs and the cult take orders
  from the same desk. The orders, the bridge seal and the Darkshire relief ledger go to Marshal
  Dughan, who keeps the evidence for the court chain.
- **45-50.** The Lakeshire Watch takes Stonewatch Falls back and faces the keep. Stonewatch Keep
  (the dungeons spec's) holds page three in Gath'Ilzogg's war chest, with the Dark Iron forge
  stamp. A cult courier in Galardell Valley carries page four and Morganth's unsent letter back
  toward the tower. In Duskwood, Sareth's people (never named) camp in Twilight Grove, and
  Mor'Ladim's story stays local. With four pages and the letter, Toren can say how an Anchor
  breaks. The arc stops there and hands off to the Steppes opening spec, where breaking the
  Anchor starts a server-wide progress bar and the Steppes open only when it fills.

## Hubs

- **Lakeshire** (Redridge, 30-45): Magistrate Solomon (344), Marshal Marris (382), Foreman
  Oslow (341), Dockmaster Baren (381), Guard Howe (903), Deputy Feldon (1070) and the inn. Toren
  lives in the inn from 36 to 50. Three Corners (Guard Parker, 464) is the 30-32 post on the
  Elwynn road.
- **Stonewatch Falls camp** (Redridge, 45-50): a new Lakeshire Watch camp of Hub NPCs at about
  (-9515, -3270), on the spot of a vanilla Blackrock outpost that the 45-50 creatures ticket
  removes, facing Stonewatch Keep. Keeshan joins it after his rescue.
- **Darkshire** (Duskwood, 30-50): Lord Ello Ebonlocke (263), Commander Althea Ebonlocke (264),
  Clerk Daltry (267), Sirra Von'Indi (268), Madame Eva (265), Calor (663), the Night Watch.
- **Raven Hill** (Duskwood, 36-46): Jitters (288) in the hamlet; Sven Yorgen (311) and Lars (893)
  at their camp on the Hushed Bank.
- **Stormwind** (hub): Commander Aldous Wray (190071), a poster beside the Writ Board, General
  Marcus Jonathan (466) at the city gate. Marshal Dughan (240) stays in Goldshire with the
  evidence.

## Toren's thread

| Level | Where | Beat | Quest |
|---|---|---|---|
| 36 | Lakeshire inn | Toren, freed at Ridgepoint, turns up. He knows the tower and won't go near it. | 98201 |
| 38 | Ilgalar | Morganth dies. The character keeps the failure notes (page one) and finds letters under a crowned seal. The Anchor keeps running. | 98501 |
| 39 | Lakeshire | The cipher. Toren reads a page a night; Marris puts a man on his stair. | 98502 |
| 40 | Lakeshire, then Darkshire | Page one: "the Duskwood test". Toren sends the character to Sirra. | 98503 |
| 44 | Raven Hill Deeps | Page two, with orders to find "the apprentice who reads M.'s hand". How an Anchor holds its stretch, and why the dead freeze. Toren stays put. | 99011, 99012 |
| 48 | Stonewatch Keep | Page three, in Gath'Ilzogg's war chest. The Dark Iron forge stamp, and where a casing gives. | 98511, 98512 |
| 48 | Galardell Valley | Page four and the unsent letter, in the cult courier's satchel. | 98404 |
| 49 | Lakeshire inn | The method is whole, and the letter reaches Toren. Hand-off line to the break. | 98521 |
| 50 | Ilgalar | The break, in `docs/specs/prep/steppes-opening.md`. Breaking the Anchor does not open the land: it starts a server-wide progress bar, filled by new Writs and bounties at a camp at Blackrock Pass, and the Steppes open when it fills. Not this arc. | |

Sareth's agents take Toren after the break at 50 (Davis). That beat belongs to the Steppes or 60
work, not here.

## Decisions (grill of 2026-11-07)

Davis answered each; the two marked "Claude, a fix" were Claude's corrections that Davis took.

1. **Mob levels** (Davis): (a) sub-zone Level Bands per zone, like Elwynn, with a small code
   ticket first. The linear Level Stretch stays as the fallback until the last band ticket
   covers each zone.
2. **Court chain** (Davis): (a) its own spec with the Cathedral; this arc ships only the
   hand-offs (97405 to 97411), which end at General Jonathan or Marshal Dughan.
3. **Way in at 30** (Davis, against the draft's Dughan letters): no Dughan letters. Both the
   Tier 2 trainer in Stormwind, right after Advancement, and a Writ Board poster send the
   character to Redridge and Duskwood.
4. **Toren** (Davis, against the draft's move at 44): Lakeshire's inn the whole time, 36 to 50.
   No ambush and no move at 44. Sareth's agents take him after the break at 50.
5. **Pages** (Davis): (a) Ilgalar, Raven Hill Deeps, Stonewatch Keep, and a cult courier in
   Galardell Valley.
6. **Page form** (Davis): (a) a plain quest item from a fixed boss or chest, handed to Toren.
7. **Order at 48-50** (Davis, against the draft's Cathedral first): either order. The Cathedral
   (court spec) and the break do not require each other, so 98521 has no court prerequisite.
8. **Black Dragon Whelps** (Davis): (a) removed from Lake Everstill's shore; the model goes to
   Stonewatch (the dungeons spec).
9. **Dungeon levels** (Claude, a fix): the bible wins. Raven Hill Deeps 40-44, Stonewatch Keep
   45-48 (ticket 04).
10. **Vanilla reuse** (Davis): (a) vanilla givers where they stand, new text; Westfall, Ironforge
    and Horde steps dropped.
11. **Keeshan** (Davis, against the draft's rewrite): build escorts now. Escort quests are a new
    realm system with their own short grill and tickets (01-escort-spec); Keeshan's quest
    (ticket 25) is blocked by it.
12. **Density** (Davis): (a) Elwynn's density. The tickets hold 106 quests.
13. **Gear rewards** (Davis, amended the same day): every story quest offers a choice of greens
    from its band's **Quest Lines**, ADR 0026 Lines that share one icon per slot, like the world
    drops. Named-elite item quests offer blues from blue Quest Lines instead. Bounties pay money.
    Dungeon-ending boss quests pay their dungeon set's four Chests (the Stockade rule) and wait
    on the dungeons spec's loot tickets. About 40 new icons, none painted: the tickets use 36.
    This arc goes first on `gear.csv`; each band's gear ticket comes before its quest tickets,
    and the gear 31-60 spec is blocked by this arc's last gear ticket (20).
14. **Diggers** (Davis): (a) "Grey Robes" with a crowned mark; nobody names Sareth.
15. **Ids** (Claude, a fix): Redridge 98101-98599, Duskwood 98601-99099, Stormwind hand-offs
    97401-97499, in new files `quests_redridge.toml` and `quests_duskwood.toml`.
16. **East camp** (Davis): a new Lakeshire Watch camp at Stonewatch Falls facing the Keep;
    Keeshan joins after his rescue.

### Readings taken while writing the tickets (Claude)

These fill gaps the answers left. Each is stated in its ticket; Davis can change any before filing.

- **Id blocks by band.** Redridge 981xx (30-35), 982xx (35-40), 983xx (40-45), 984xx (45-50),
  985xx (dungeon boss quests and Toren's last step). Duskwood 986xx, 987xx, 988xx, 989xx, and
  990xx for its two dungeons. Stormwind 97401-97411.
- **Six new content files**, so two builders never append to one file at once:
  `quests_redridge.toml`, `quests_duskwood.toml`, `quests_redridge_dungeons.toml`,
  `quests_duskwood_dungeons.toml`, `quest_items_redridge.toml`, `quest_items_duskwood.toml`
  (ticket 03). The dungeon boss quests go in the `_dungeons` files because they wait on the
  dungeons spec and would otherwise hold up every later zone ticket.
- **The spine does not wait on the dungeons.** A zone quest never names a dungeon boss quest in
  `after`. Sirra's chain, the Deeps lead-in and the Stonewatch lead-in stand on their own. Only
  the boss quests, Toren's page translations and 98521 wait on the dungeons spec.
- **Quest Lines** (decision 13). 20 green Lines in the World shape (`quest-chest-cloth` to
  `quest-ring-mind-b`: four Chests, seven weapons, three amulets, six rings) with one row per
  band, 80 rows at 903200 to 903279, entry `903200 + 20 x (band - 1) + Line index`. 16 blue
  Lines (`quest-elite-*`) with 19 rows at 903300 to 903318, only where an elite quest uses
  them. 36 icons in all, taken from the free stock (rings 9 of 16, Chests 8 of 39, staves 2 of
  11, the two bow Lines on crossbow or gun icons). Ticket 05 makes the Lines.
- **Band levels for gear.** Item level 33, 38, 43, 48 (the gear 31-60 draft's Loot Band set
  levels); required level the band's lowest quest level, 30, 36, 40, 45.
- **Which slot.** #448 decision 9's cycle in quest-id order across each band (main hand, ring,
  Chest, amulet, off hand, ring, main hand, ring); each quest ticket's "Gear choices" table
  names every quest's `choices`. The six elite quests' blues: the Thresher main hand, Morbent
  Fel rings, Tharil'zun Chests, the Grey Captain amulets, the courier off hand, Mor'Ladim rings.
- **Money stays** as placeholders beside the gear: 35 x level for a story quest with an
  objective, 45 x level for an item quest, 0 for a delivery or talk, 30 x level for a bounty.
- **Toren in Lakeshire is a new Hub NPC** (entry 190083), the Azora/Ridgepoint pattern of a
  second Toren with his own quest list. The Ridgepoint Toren stays where he is.
- **The way in.** Wray gives two deliveries (to Solomon and to Lord Ebonlocke) at Tier 2. The
  poster beside the Writ Board gives two (to Guard Parker and to Althea Ebonlocke). The zones'
  first quests need none of them, so a character who skipped Stormwind still gets the arc.
- **Ilgalar's approach** is its own 35-38 band inside Galardell Valley (45-48), so the dungeon's
  door is reachable at its level (dungeons prep S2).
- **The letters at Ilgalar** are "Letters Under a Crowned Seal", signed with the seal and the
  initial "S.", never the name (decision 14 read strictly).
- **Writs** keep their kill lists. If a band moves a Writ's named creature far from the Writ's
  level, that is a later bug.

## Reveal check

- 30-35: a noble house pays for a bridge nobody asked about; the cult's mark on crates heading
  east; Twilight coin in orc pockets; the house where the candles never go out. No "Anchor" in
  Duskwood yet (dungeons prep S1).
- 35-40: rung 4 lands at Ilgalar (the tower is a working machine and still runs). Grey Robes with
  a crowned mark dig at Raven Hill; someone else is building something.
- 40-45: rung 5 (Anchors can go wrong; the dead freeze, not silenced). The orcs and the cult take
  orders from the same desk. Evidence goes to Dughan; rung 6 is the court spec's.
- 45-50: a winged shape over Stonewatch and the dragonkin inside are rung 8, which Stonewatch
  owns. The Dark Iron forge stamp. Nobody names Prestor, Cedric, Sareth, Deathwing or the Lich
  King.

## Out of scope

- **The court chain** (rung 6 and the Cathedral, rung 7): its own spec. Its first quest names
  97409, 97410 or 97411 in `after`.
- **The four dungeon interiors**: `docs/specs/prep/dungeons-30-50.md`. That spec builds the
  bosses, their drops of gear sets, and each dungeon's repeatable. This arc owns the lead-in
  quests, the boss item quests' text and rewards, and the page drops on the bosses. The dungeons
  spec can drop its X3 ticket (ticket 04 here sets the two levels) and should put its Raven Hill
  and Stonewatch tickets after 04.
- **The Steppes opening**: `docs/specs/prep/steppes-opening.md`. Its break quest names 98521
  "The Unsent Letter" in `after`, and its Toren is the Lakeshire one (190083). Breaking the Anchor does not open the land: it starts a server-wide progress bar, filled by new Writs and bounties at a camp at Blackrock Pass, and the Steppes open when it fills;
  that is all the Steppes spec's.
- **The escort system**: 01-escort-spec, a grill first.
- **Gear past the quest rewards**: Loot Bands 31-60, dungeon sets and epics are the gear 31-60
  and dungeons specs'. This arc goes first on `gear.csv`: their tickets that edit it are blocked
  by 20-gear-45-50, this arc's last gear ticket. The dungeon sets' Chests, which the four
  dungeon-ending quests offer, come from the dungeons spec's loot tickets.
- **Elwynn 26-30** and the Hogger's camp exits (#227 closed without tickets).
- **Azora's shadow stop text** that names Ilgalar's foot (dungeons prep, X3).

## Free ids this arc uses

| What | Range |
|---|---|
| Quests | 97401-97411, 98101-98521, 98601-99012 |
| Quest items | Stormwind 980151-980159, Redridge 980301-980399, Duskwood 980401-980499 |
| Quest GameObject entries | Stormwind 980202, Redridge 980501-980504, Duskwood 980552-980553 |
| Quest GameObject guids | Stormwind 791002, Redridge 792001-792099, Duskwood 793001-793099 |
| Creature templates (`mobs.toml`) | 96048-96055 |
| Placed creature guids (`zones.toml`) | Redridge 961001-961499, Duskwood 961501-961999 |
| Hub NPCs (`hub.toml`) | guids 9300083-9300088, entries 190083-190087 |
| Gear (`gear.csv`) | green Quest Lines 903200-903279, blue Quest Lines 903300-903318 |

## Child specs and tickets

Build order. "Blocked by" names a file in this folder; `DUNGEONS:` names a ticket of the 30-50
dungeons spec, not filed yet.

- 01 Escort quests (to grill), a stub spec
- 02 Level Bands for Redridge and Duskwood: a band file per zone
- 03 Six quest and quest-item files for Redridge and Duskwood (blocked by 02)
- 04 Dungeon levels from the bible, and the whelps leave Lake Everstill (blocked by 02)
- Band 30-35: 05 Quest Lines and gear (no blocker), 06 bands (02), 07 Redridge quests (03, 05, 06), 08 Duskwood
  quests (03, 05, 06), 09 Stormwind way in (07, #593)
- Band 35-40: 10 gear (05), 11 bands and Grey Robes (04, 06), 12 Redridge quests and Toren
  (07, 10, 11, #593), 13 Duskwood quests (08, 10, 11), 14 Stormwind (08, 09, 10)
- Band 40-45: 15 gear (10), 16 bands (11), 17 Redridge quests (12, 15, 16), 18 Duskwood quests
  (13, 15, 16), 19 Stormwind evidence (14, 17)
- Band 45-50: 20 gear (15), 21 bands, the camp ground and new creatures (16), 22 Redridge quests
  and the camp (17, 20, 21), 23 Duskwood quests (18, 20, 21), 24 Stormwind (19, 20)
- 25 Keeshan (01, 22)
- 26 Mistmantle Manor's quests (03, 05, 08, DUNGEONS)
- 27 Tower of Ilgalar's quests (10, 12, DUNGEONS)
- 28 Raven Hill Deeps' quests (12, 15, 18, 26, DUNGEONS)
- 29 Stonewatch Keep's quests (20, 22, 27, DUNGEONS)
- 30 The Unsent Letter (22, 28, 29)

Art: the placeholders are listed once in `docs/art-assets.md` by ticket 03.
