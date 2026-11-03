# Writs: Stormwind's board of huge bounties (spec #557)

Status: grilled 2026-11-02, tickets ready-for-agent. Unplayed.

Grilled 2026-11-02 from #557, one of the four "why go to Stormwind" specs. This note holds the
rules the tickets build and every reading taken where the grill left a choice open.

## The board

- A Writ Board stands in the Trade District. Clicking it offers the Writs showing now.
- One City Watch officer stands beside it and takes every Writ back, whoever issued it.
- Each Writ names its issuer in its text: the City Watch, the House of Nobles or Lady Prestor's
  office. Prestor's Writs send players after orcs and the dead and never near the cult. Nothing
  in the game says so yet; the 40s court chain can point it out later (lore bible, Stormwind).

## Pools and showings

| Zone | Writs | Levels |
|---|---|---|
| Elwynn Forest | 1 | 24 |
| Redridge Mountains | 3 | 33, 40, 47 |
| Duskwood | 3 | 33, 40, 47 |
| Burning Steppes | 3 | written by the spec that brings in the zone's mobs |

- Each zone shows one Writ at a time. The board changes every other day at midnight UTC, all
  zones at once, on the realm's wall clock.
- A zone draws its next Writ at random, never the one it just showed. Duskwood's draw also skips
  the level band Redridge drew, so the two never show the same band together. Elwynn's pool of
  one shows every window.
- Everyone on the realm sees the same board.
- A character can do each showing once. A Writ in the log stays there when the board changes and
  is still handed in to the officer.

## A Writ

- A sweep of about 100 kills plus the zone's own named creatures, about two hours solo.
- A kill line can count a whole family ("Blackrock orcs slain: 0/50"). A line counts at most 63,
  the client's counter, so a sweep is two lines and the named creatures take the other two.
- A named rare becomes a rare elite, and a named normal creature becomes an elite. A zone short
  of named creatures may get new ones.
- XP: each Writ names `levels`, 1 to 2, set by its difficulty. A character at the Writ's level
  gains that many levels from the hand-in. Notoriety's Heat Level adds on top. A character above
  the Writ's level gets the same XP, so less of their own level.
- Money only besides the XP. No items.

## Readings taken

- **Levels after the Tier XP Rate.** "2 levels at its own level" is read as the total a
  character of the Writ's Tier gains. The realm pays `levels x xp_to_next(level)` divided by the
  Tier XP Rate of the Tier the Writ is offered from, and the rate multiplies it back. A Tier 3
  character doing a Tier 2 Writ gains 7/5 of that.
- **Money.** `140 x levels x level` copper. A field bounty pays about 28 copper per level for
  ten kills; a Writ is ten times the work, paid per half level of XP.
- **Difficulty.** Low Writs pay 1 level, mid 1.5, high 2, and Elwynn's 1.5.
- **The draw.** A pure function of the window number, counted from an anchor date in
  `content/writs.toml`, so every host agrees and nothing is saved. A wall clock before the anchor
  is window 0.
- **Once per showing.** The save keeps the window of each Writ's last hand-in. A Writ is not
  offered when that window is the current one.
- **Issuers.** City Watch: Elwynn, Duskwood high. House of Nobles: Redridge low and mid,
  Duskwood low. Prestor's office: Redridge high, Duskwood mid.

## The seven Writs

| Id | Zone | Level | levels | Issuer | Family lines | Named |
|---|---|---|---|---|---|---|
| 95001 | Elwynn | 24 | 1.5 | City Watch | Prowlers and forest bears 50 (118, 822); Defias 50 (116, 474) | Dead-Tooth Jack (6093), Thuros Lightfingers (61) |
| 95002 | Redridge | 33 | 1 | House of Nobles | Redridge gnolls 60 (423, 424, 426, 430); Lakeshire murlocs 40 (1083, 548, 422) | Ribchaser (14271), Squiddic (14270) |
| 95003 | Redridge | 40 | 1.5 | House of Nobles | Blackrock orcs 50 (440, 485, 437, 4064, 4065); condors and tarantulas 50 (428, 505) | Seeker Aqualon (14269), Chatter (616) |
| 95004 | Redridge | 47 | 2 | Prestor's office | Shadowhide gnolls 50 (433, 434, 432, 568, 429, 431); Blackrock veterans 50 (435, 4463, 615, 436, 4462) | Tharil'zun (486), Kazon (584) |
| 95005 | Duskwood | 33 | 1 | House of Nobles | Duskwood spiders 50 (539, 217, 569); dire wolves 50 (213, 565) | Lupos (521) |
| 95006 | Duskwood | 40 | 1.5 | Prestor's office | The restless dead 60 (48, 203, 202, 531, 3, 210, 1110, 604); Black Ravagers 40 (923, 628, 1258) | Naraxis (574) |
| 95007 | Duskwood | 47 | 2 | City Watch | Nightbane worgen 60 (898, 533, 205, 206, 920); Splinter Fist ogres 40 (889, 891, 1251, 212) | Morbent Fel (1200), Mor'Ladim (522) |

The game levels in the family lines come from each zone's Level Stretch (Redridge 14-27 and
Duskwood 18-35 onto 30-50) and Elwynn's Level Bands (Eastvale & Ridgepoint 20-24, Mirror Lake &
Jerod's Landing 23-26).

## Tickets

- #572 Kill lines that count a creature family, and the 63 cap
- #573 The Writ kind, the board's showings and the save
- #574 The Writ Board, its officer and the seven Writs (blocked by #572 and #573)
