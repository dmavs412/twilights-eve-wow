Title: The Burning Steppes open, server-wide (spec)
Labels:

Grilled with Davis on 2026-11-07 and 2026-11-08 from `docs/specs/prep/steppes-opening.md`. The
grill's 14 answers are below and override the prep draft wherever they differ. ADR: draft in
`adr-draft.md` here, numbered 0034; ticket 01 commits it to `docs/adr/`. The lore bible
(`docs/lore-bible.md`) wins over any line here.

## The shape

1. A character at 48 to 50 takes Toren's quest in Lakeshire and climbs the Tower of Ilgalar. At
   the Anchor on the top floor, four waves of the cult come to save it, scaled by the Head Count.
   When the last wave dies, the Anchor breaks for the whole server. The land does not open yet.
2. The break starts a server-wide Tally. A Blackrock Pass Watch camp appears on the Redridge side
   of the pass, with creatures that walk out of the thinning mist. The camp's bounties and Writs
   are the only things that fill the Tally.
3. Nothing on screen shows the Tally. A herald beside the Writ Board in Stormwind and a Watchman
   in Lakeshire say how far it has come.
4. When the Tally fills, the mist along the whole border of the Burning Steppes fades over a few
   seconds and the land appears behind it. Every character in the world sees a line on `/1` and
   hears Toren's yell. The Steppes stay open for good.

## Decisions (grill of 2026-11-07/08)

Davis answered each; those marked Claude were Claude's fixes or readings of an existing rule that
Davis took.

1. **Who opens it** (Davis, against the draft's first-break-opens): a shared Tally. The first
   group to break the Anchor starts it; the break does not open the land. Only new Writs and
   bounties at a camp at Blackrock Pass fill it, and the camp appears after the break, holding back
   what comes out of the thinning mist. The goal is fixed and big, the same on every server, a
   content tunable sized so a solo player fills it in about 8 to 10 hours of pass bounties. No
   frame on screen: players ask an NPC near the Writ Board in Stormwind or one in Redridge. When it
   fills, the Steppes open for everyone on the server.
2. **Latecomers** (Davis, against the draft's aftermath quest): nothing. The land is just open.
3. **Closing again** (Davis): never. A host or a test resets it by editing the file.
4. **Storage** (Claude, a fix): a realm-written file beside the saves, `milestones.toml`, with its
   catalogue in `content/milestones.toml`, apart from `unlocks.toml`, which only purchases write
   (ADR 0031). It holds the break, the Tally's count and the opened fact.
5. **Held shut** (Davis): zone 46 is a Closed Zone that a Milestone opens. No land is drawn, and
   the mist curtain stands along the whole border until the Tally fills.
6. **Neighbours** (Davis): Searing Gorge (51), the Badlands (3), the Swamp of Sorrows (8) and the
   Wetlands (11) join Westfall in `closed_zones.toml` for good.
7. **The break** (Davis): inside the Ilgalar Run. A quest from Toren starts an Encounter at the
   Anchor, with 48-50 waves scaled by the Head Count. The last wave's end records the break and
   starts the Tally. It does not open the land.
8. **Credit** (Claude, the existing rule): everyone within 74 yards when the Anchor breaks
   (ADR 0009). Server-wide, the Tally is what matters.
9. **The opening** (Davis): the border mist fades over some seconds, the land appears behind it,
   a line goes out on `/1` and Toren yells.
10. **Solo** (Davis): the solo roster is one server, so one character's work counts for the alts.
11. **Stand-in** (Claude): the break quest names 98521 "The Unsent Letter" (Redridge and Duskwood
    arc, ticket 30) in `after`, but it may ship first behind a stand-in "translation complete"
    quest from Toren at 48 that the arc replaces.
12. **Levels** (Davis): sub-zone Level Bands spanning 50-60, by the mechanism of the Redridge and
    Duskwood arc's Level Bands code ticket.
13. **Outposts** (Davis, against the draft's removal): Flame Crest's orcs turn hostile, as the
    Blackrock are. Morgan's Vigil's stock NPCs become Hub NPCs (the Goldshire pattern). The goblins
    and the Spirit Healer stay.
14. **Level gate** (Davis): none.

### Readings taken while writing the tickets (Claude)

Each is stated in its ticket. Davis can change any before filing.

- **Names.** A Milestone is a server-wide fact (ADR 0034). The bar is a Tally. The two
  Milestones are `ilgalar_anchor_broken` and `steppes_open`; `steppes_open` is the Tally, with
  `after = "ilgalar_anchor_broken"`.
- **Goal and points** (decision 1's "8 to 10 hours"). Points are roughly solo minutes: a camp
  bounty is worth 15, a camp Writ 120, and the goal is 540, nine hours of bounties alone. All three
  are rows in `content/milestones.toml`.
- **Whose hand-in counts.** Each character's own hand-in adds its points. Five friends handing in
  the same bounty add 75.
- **After the opening** the camp, its creatures, bounties and Writs stay. They feed nothing.
- **The camp** stands on the Redridge side of the pass at (-8735, -2590), on the road below the
  border at x -8667. Its creatures are new templates placed on Redridge land between the camp and
  the mist, with their own levels (48-51). They appear at the break and stay.
- **Camp Writs** are a pool of three on the camp's own board (pool `zone_or_sort = 46`). The three
  Burning Steppes Writs that the Writs spec reserves for the Stormwind board are not these, and
  stay for a later spec.
- **Readouts** speak in words, in seven stages: before the break, under a quarter, a quarter,
  half, three quarters, nine tenths, and open. No exact number. Each stage has its own `npc_text`
  id, because the client caches a greeting's text by id. The Redridge one stands in Lakeshire, not
  at the camp, so he answers before the break too.
- **The `/1` line and Toren's yell** go to every character in the world, wherever they stand.
- **The break quest** is offered only while the Anchor stands. A character still holding it after
  another group's break can run the Encounter and finish it.
- **Encounters** learn to run inside a Phased Dungeon's Run (a `dungeon` key), because today they
  run only in the open world.
- **Waves.** Four waves at levels 48 to 50, the last led by the Anchorwarden, an elite whose kill
  is the quest's objective, so ADR 0009's 74-yard rule gives the credit.
- **Steppes bands** follow vanilla's order of sub-zones: 50-51 at the pass up to 58-60 on
  Blackrock Mountain (ticket 08's table).
- **Flame Crest's orcs** are Gruna (3708) and Vahgruk (13177). They take the Blackrock faction
  (40) and lose their vendor and flight-master flags and subnames. Ragged John, Mathredis Firestar,
  the four goblins and the Spirit Healer keep their vanilla rows.
- **Morgan's Vigil's eight** are ambient Hub NPCs with a greeting each, no stock and no flights.
- **The break quest** pays money and offers no gear choice yet.
- **The Anchor's look** is a placeholder pair: display 1667 (FloatingPurpleCrystal01) whole and
  1687 (FloatingPurpleCrystalBroken02) broken.
- **The world map.** The painted canvas covers four zones and stays as it is; the stock world map
  mists zone 46 until it opens. Painting the Steppes is Davis's (ticket 09).

## Scope

- Milestones: catalogue, state file, `Storage::Milestones`, both hosts, `SessionEvent::Milestones`.
- Closed Zones that a Milestone opens; zone 46 gated; zones 51, 3, 8 and 11 closed for good.
- `after_milestone` and `until_milestone` on Hub NPCs, quest GameObjects, hand-placed zone
  creatures, quest offers and Phased Dungeon rows.
- The Tally, its two readout NPCs, and the opening's `/1` line and yell.
- Encounters in a Phased Dungeon; the Anchor; Toren's stand-in and break quests.
- The Blackrock Pass Watch camp: its captain, board, creatures, three bounties and three Writs.
- The client: run-time opening, tile reload and the curtain fade.
- The Burning Steppes spawn import, its Level Bands, Flame Crest and Morgan's Vigil.
- The swap from the stand-in to 98521 once the arc's ticket 30 lands.

## Out of scope

- **Toren's failure-notes chain** through the 40s: the Redridge and Duskwood arc
  (`docs/specs/redridge-duskwood/`). Its Toren (190083, ticket 12) gives this spec's quests.
- **Ilgalar's interior** (Morganth, the valves, the pylons): `docs/specs/prep/dungeons-30-50.md`.
  This spec adds only the Anchor's two GameObjects and the Encounter to `ilgalar.toml`.
- **Steppes quests, Writs on the Stormwind board, Altar of Storms**, Blackrock Mountain's
  interior and its doors, and the Blackrock Stronghold and Dreadmaul Rock interiors.
- **Sareth's agents taking Toren** after the break (the arc's parent): Steppes or 60 work.
- **Gear** for the break quest and the camp: the gear 31-60 spec.
- **Extending the painted world map's canvas** to the Steppes: a code ticket once Davis decides
  in ticket 09.

## Free ids this spec uses

| What | Ids |
|---|---|
| Quests | 99501-99502 (stand-in, break), 99511-99513 (camp bounties), 99521-99523 (camp Writs) |
| Creature templates (`mobs.toml`) | 97501-97508 (the pass), 97511-97513 (the break's waves) |
| Placed creature guids (`zones.toml`, Redridge `[[zone.added]]`) | 964001-964030 |
| Hub NPCs (`hub.toml`) | guids 9300089-9300099; new entries 190088-190090; Morgan's Vigil keeps its stock entries |
| `npc_text` ids | 64301-64307 herald, 64311-64317 Lakeshire Watchman, 64331-64338 Morgan's Vigil |
| Quest GameObject | guid 794001, entry 980601 (the camp's board) |
| Phased Dungeon GameObjects (`ilgalar.toml`) | 794101 (the Anchor, whole), 794102 (broken) |
| Milestones | `ilgalar_anchor_broken`, `steppes_open` |

## Child tickets

Build order. "Blocked by" names a file in this folder; `RD:` names a ticket file of the Redridge
and Duskwood arc (`docs/specs/redridge-duskwood/`).

- 01 Milestones: the catalogue, the realm-written state file and the session event
- 02 Closed Zones that a Milestone opens, and the four neighbours closed for good (01)
- 03 Content that waits for a Milestone (02, RD:02-level-bands-code, RD:03-quest-files)
- 04 The Tally, the two readout NPCs and the opening's line and yell (03, RD:12-redridge-35-40)
- 05 The break: an Encounter at the Ilgalar Anchor, and Toren's quests (04, RD:12-redridge-35-40,
  RD:03-quest-files)
- 06 The Blackrock Pass Watch camp, its creatures, bounties and Writs (05, RD:21-bands-45-50)
- 07 The client opens a gated zone at run time and fades its mist (02)
- 08 The Burning Steppes spawns, Level Bands 50-60, Flame Crest and Morgan's Vigil (06,
  RD:02-level-bands-code, RD:21-bands-45-50)
- 09 Paint the Burning Steppes onto the world map (ready-for-human, no blocker)
- 10 The break quest follows The Unsent Letter (06, RD:30-toren-unsent-letter)

The index (`99-index.md`) lists each ticket's files and what can run alongside it.
