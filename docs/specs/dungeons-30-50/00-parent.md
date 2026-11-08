Title: The 30-50 dungeons: Mistmantle Manor, the Tower of Ilgalar, Raven Hill Deeps and Stonewatch Keep (spec)
Labels:

Grilled with Davis on 2026-11-08 from `docs/specs/prep/dungeons-30-50.md`. The grill's answers
are below and override the prep draft wherever they differ. The lore bible (`docs/lore-bible.md`)
wins over any line here. The four dungeons are shells today (cd940cee): a footprint, a Portal and
one friendly Phase Tester each. This spec fills them.

## The shape

Each dungeon is a run of about 15 minutes with 3 encounters and 14 to 19 creatures, built from
the parts the realm already has (camps, timed and health calls, levers, `[[cover]]` against a
gaze, `[wall]`, the Boss Bar). Two pieces of realm code are new: the tram's frozen passengers
become a key any dungeon creature can carry, and a partner-death link lets one creature enrage
when another dies. A plumbing ticket first lets each dungeon file carry its own creature
templates, so two builders can fill two dungeons at once.

- **Mistmantle Manor (32-36, Duskwood).** Plain fights. The footprint grows from the one-room
  house to an open-air box over the yard and the walled garden, with a `[wall]`, and the house is
  the last room. The steward in the yard, the lady of the house in the garden, then Stalvan
  Mistmantle in the house, standing over cracked humming ore where the floorboards are torn up.
  Nobody says "Anchor".
- **The Tower of Ilgalar (34-38, Redridge).** Three feed valves on the ramp, each a lever that
  stops one of Morganth's timed calls. A Blackrock overseer on the middle storey. Morganth on the
  top floor beside the Anchor, whose Tuning Pulse only the Anchor's pylons stop. The notebooks
  were split before the players came: Morganth carries only the failure notes, and the tower
  shows the theft (an emptied strongbox and what he and the overseer say). The Anchor stays
  standing, and the top floor keeps room for the Steppes spec's break Encounter.
- **Raven Hill Deeps (40-44, Duskwood).** Commander Felstrom at the foot of the stairs. The dig
  face, where the dig overseer's ghouls come out of a side tunnel until a shoring lever drops it.
  The grey pocket in the seam vault: a placed mist wall, frozen dead that step out at 75, 50 and
  25% of the Grey Deacon's health, and a gaze from a shadow overhead that only rubble heaps stop.
- **Stonewatch Keep (45-48, Redridge).** The quartermaster and the Twilight paymaster among the
  Tribute crates, a dragonspawn envoy in the war room with a brood cage lever, and on the top
  storey Gath'Ilzogg and Singe together: when one dies, the other enrages. Singe drops a scale,
  the only proof the player carries out.
- **Mourncrown over Twilight Grove.** The Necropolis model hangs over Twilight Grove, which loses
  the World Tree and moonwell of the Emerald Dream portal area. It ships only if a capture shows
  it looks right.

## Decisions (grill of 2026-11-08)

Davis answered each; those marked Claude were Claude's readings that Davis took.

1. **Reveal order** (Davis, S1 a): Mistmantle shows a buried humming stone. Nobody there says
   "Anchor"; Ilgalar teaches the word.
2. **Out-of-band approaches** (Claude, S2, following the quest arc's decision 1): sub-zone Level
   Bands over each approach, which the arc's band tickets build (its 06 has Manor Mistmantle at
   32-35 and its 11 Ilgalar's Approach at 35-38). Mistmantle's yard joins the footprint as
   dungeon creatures (decision 9).
3. **Loot past 30** (Claude, S3): each dungeon has a set of 20 blues, the Stockade form. Each set
   has four Chest rows, one per armour type, because the arc's dungeon-ending quests pay them.
   Trash rolls the 31+ Loot Bands once the gear 31-60 spec lands them; until then it rolls the
   27-30 band as the Stockade's does. The gear 31-60 grill may amend this.
4. **Frozen dead** (Claude, S4 a): the tram's frozen passengers become a dungeon creature key,
   and the tram moves onto it. ADR 0033 named the second user as the time.
5. **Two builders** (Claude, S5 b): a plumbing ticket first, so each dungeon file carries its own
   `[[template]]` rows.
6. **Run length** (Davis, S6): about 15 minutes, 3 encounters. Every layout is trimmed to that.
7. **Ilgalar's return at 48-50** (Davis, I1): the Steppes opening spec owns it. The break is a
   quest-started Encounter inside the Ilgalar Run. This spec leaves the Anchor standing and the
   top floor's wave points clear.
8. **The notebooks** (Davis, I2): already split before the fight. Morganth carries only the
   failure notes. A clue in the tower shows the theft. No thieves appear, so no `for_s` key.
9. **Morganth** (Davis, I3 c): feed valves before the boss, then the gaze and the Anchor's pylons
   in the fight.
10. **Mistmantle's layout** (Davis, M1 a): an open-air box over the yard and outbuildings with a
    `[wall]`, the house last. The game data has no outbuildings at the farm: a fenced yard north
    of the house and a walled garden east of it, so the box covers those.
11. **Mistmantle's fights** (Davis, M2 b): plain fights. No meter, no sleepers.
12. **Mistmantle's master** (Davis, M3 a): Stalvan Mistmantle at vanilla's spot, cracked humming
    ore under the house, never named.
13. **Mourncrown** (Davis, R1): it hangs over Twilight Grove and replaces the whole Emerald Dream
    portal area, not the cemetery. Raven Hill's last room has the gaze from its shadow. One ticket
    places the Necropolis model, gated on a `WOW_CAPTURE` screenshot that shows it looks right;
    if it looks wrong, it does not ship.
14. **Sareth** (Davis, R2 a): a crowned seal and "the Master", never his name.
15. **The roof** (Davis, St1 a): Gath'Ilzogg and Singe fight at once, and the survivor enrages.
    This is a new partner-death link.
16. **The dragon proof** (Davis, St2 b): a scale from Singe, nothing written.

### Readings taken while writing the tickets (Claude)

Each is stated in its ticket; Davis can change any before filing.

- **Templates in the dungeon file, spells in sections.** A dungeon's templates go in its own
  file (decision 5). Its creature spells stay in `creature_spells.toml`, in a section per dungeon
  with its own id block, each section started by one row that ticket 01 writes. `build_spells.py`
  writes the client rows in file order, so those four rows keep each dungeon's rows apart in the
  generated `client_dbc_patch.json`, and two builders' edits to either file merge without a clash.
  No pipeline code changes.
- **Rolls.** Each stop boss rolls one blue of its dungeon's set at 50%, each final boss at 100%.
  Both of Stonewatch's roof bosses roll at 100%. Set item levels and required levels follow the
  gear 31-60 draft: 36/33, 38/35, 44/41 and 48/45.
- **The arc owns the page drops.** The arc's tickets 26 to 29 add their quest items (the site
  record, the failure notes and letters, page two and the crowned orders, page three) to the boss
  rows' `drop` lists, and define the items. These tickets build the bosses with stable guids and
  leave `drop` to the arc, so no two tickets add the same drop.
- **Trimmed parts.** Ilgalar's Tuner and Servant damage pairing is dropped (it needs code). Raven
  Hill's seam-vault constructs are dropped, and the grey pocket moves into the seam vault, with
  the rooms beyond it behind the mist wall. Stonewatch's dragonspawn envoy stays, carrying
  nothing. The paymaster heals himself, because a creature `heal` heals only its caster.
- **No "Mourncrown" in player text.** The arc's rule (its ticket 28) is that nobody names Sareth
  or Mourncrown. The gaze is called "Crowned Shadow".
- **Placeholder names.** Steward Aldric, Lady Corrine Mistmantle, the Grey Deacon, the Blackrock
  Quartermaster, the Twilight Paymaster and the Dragonspawn Envoy. Davis renames any.
- **Singe's Scale** is item 980701 in `quest_items_redridge.toml`, always on Singe's corpse. No
  quest collects it yet.
- **The Necropolis** stands at scale 1.0, the size of vanilla's invasion necropolises (about 40
  by 43 yards, 73 tall), so it needs no realm code: a quest-file `[[gameobject]]` row in the form
  of Keane's tents. The client drops the two ADT models of the portal area by a short list.
- **No repeatables.** The grill did not decide a repeatable per dungeon, so none is built here.

## Reveal check

- Mistmantle: the house went still the night the dark came, and something hums under it. No
  "Anchor", no cult name, no Scourge.
- Ilgalar: rung 4. The tower is a working machine, and it keeps running after Morganth dies. Dark
  Iron crates under the cult's mark are the forge clue for the Steppes. The Blackrock overseer
  never says who sent him.
- Raven Hill Deeps: rung 5's second half. The dead in the grey stop, they are not gone. A crowned
  seal and "the Master". Nobody names Sareth or Mourncrown or says the Lich King is frozen.
- Stonewatch: rung 8. Black dragonkin fight beside the Blackrock, and a scale is all the player
  carries out. Nobody names Nefarian, Onyxia, Deathwing or Prestor, and rung 7 stays the
  Cathedral's.

## Scope

- Templates in Phased Dungeon files; per-dungeon sections of `creature_spells.toml`; the
  `lore_bible_dungeons` test made to accept a filled dungeon.
- The `frozen_until` dungeon creature key, with the tram's passengers moved onto it.
- The `on_partner_death` dungeon creature key.
- The four dungeons' creatures, spells, levers, cover, walls and GameObjects; Mistmantle's new
  footprint; Raven Hill's mist wall; Azora's wrong "foot of the tower" text.
- Four sets of 20 blues and the bosses' rolls; Singe's Scale.
- The Necropolis over Twilight Grove, with the capture that gates it.

## Out of scope

- **The quests** into and out of the four dungeons, and the page items and their drops: the
  Redridge and Duskwood arc (`docs/specs/redridge-duskwood/`, tickets 26 to 30).
- **Raven Hill Deeps' and Stonewatch Keep's levels**: the arc's ticket 04 sets them to 40-44 and
  45-48 (the prep's X3).
- **Level Bands on the approaches**: the arc's band tickets (06, 11, 16, 21).
- **The Anchor's break and its Encounter**: the Steppes opening spec
  (`docs/specs/steppes-opening/`, ticket 05). It adds `until_milestone` to the Anchor this spec
  places and the broken Anchor beside it.
- **Loot Bands past 30, dungeon epics and dungeon chests**: the gear 31-60 spec.
- **A repeatable per dungeon** (the prep's Scourge token idea): not decided.
- **The lore bible**: Davis's live doc should say Mourncrown hangs over Twilight Grove, where the
  Emerald Dream portal area was.

## Free ids this spec uses

| What | Ids |
|---|---|
| Creature templates (in each dungeon file) | Mistmantle 97225-97239, Ilgalar 97240-97259, Raven Hill 97260-97279, Stonewatch 97280-97299 |
| Placed creature guids | Mistmantle 974001-974099, Ilgalar 974101-974199, Raven Hill 974201-974299, Stonewatch 974301-974399 |
| Phased Dungeon GameObject guids | Mistmantle 983031-983039, Ilgalar 983041-983050, Raven Hill 983051-983059, Stonewatch 983061-983069; the Anchor 794101 (the Steppes spec's id, same object) |
| Creature spells | Mistmantle 1900060-1900069, Ilgalar 1900070-1900079, Raven Hill 1900080-1900089, Stonewatch 1900090-1900099 |
| Gear (`gear.csv`) | Mistmantle set 905140-905159, Ilgalar 905160-905179, Raven Hill 905180-905199, Stonewatch 905200-905219 |
| Items | Singe's Scale 980701 |
| Quest-file GameObject | the Necropolis, guid 983071, entry 980711 |

## Child tickets

Build order. "Blocked by" names a file in this folder; `RD:` names a ticket file of the Redridge
and Duskwood arc, `SO:` one of the Steppes opening spec.

- 01 Templates in dungeon files, spell sections per dungeon, and a lore-bible test that accepts a
  filled dungeon (RD:02-level-bands-code, RD:04-dungeon-levels-whelps,
  SO:03-milestone-gated-content)
- 02 Frozen until a boss's health mark: a dungeon creature key, and the tram moved onto it (01)
- 03 A partner-death link: the survivor enrages (02)
- 04 Mistmantle Manor: the yard, the steward, the lady and Stalvan (01, RD:11-bands-35-40)
- 05 Mistmantle Manor's set: twenty blues and the rolls (04, RD:20-gear-45-50)
- 06 The Tower of Ilgalar: the valves, the overseer and Morganth (01)
- 07 The Tower of Ilgalar's set (06, 05, RD:20-gear-45-50)
- 08 Raven Hill Deeps: Felstrom, the dig face and the Grey Deacon (02, RD:04-dungeon-levels-whelps)
- 09 Raven Hill Deeps' set (08, 07, RD:20-gear-45-50)
- 10 Stonewatch Keep: the quartermaster, the envoy, Gath'Ilzogg and Singe (03,
  RD:04-dungeon-levels-whelps)
- 11 Stonewatch Keep's set and Singe's Scale (10, 09, RD:20-gear-45-50, RD:22-redridge-45-50)
- 12 Mourncrown over Twilight Grove, gated on a capture (RD:23-duskwood-45-50,
  SO:07-client-opening)

Art: no new models. The placeholders (Singe's Scale icon, the stand-in props) are listed once in
`docs/art-assets.md` by ticket 01.
