Title: Gear 31-50: four Loot Bands, dungeon epics, bonus chests and looks that follow item level (spec)
Labels:

Grilled with Davis on 2026-11-08 from `docs/specs/prep/gear-31-60.md`. The grill's answers
override the prep draft wherever they differ. The draft covered 31 to 60; the grill cut it to
31 to 50, so the spec's name is "gear 31-50". The Redridge and Duskwood arc
(`docs/specs/redridge-duskwood/`, #595) and the 30-50 dungeons spec
(`docs/specs/dungeons-30-50/`, #637) already own part of the gear for these levels. This spec
fills the rest and does not write their rows again.

## What it builds

- **Four Loot Bands past 30**: 31-35, 36-40, 41-45 and 46-50, each with 20 World greens on the
  existing `world-*` Lines and six Elite blues on the `elite-*` Lines. No whites past 30. A mob
  above 50 rolls the 46-50 band until the Burning Steppes content spec adds 51-60.
- **Elite rolls in Redridge and Duskwood**: the named elites and rares roll their band's Elite
  pool and greens, as Elwynn's do, from a new `rolls` table by entry in `zones.toml`.
- **Dungeon epics**: three Flavoured epics on each final boss of the four 30-50 dungeons, rolled
  at 5% on top of the set roll. 12 epics, 12 new stock icons.
- **Bonus chests**: one per dungeon, earned by an optional challenge, holding one blue of the
  dungeon's set and coin. A small realm key in the Phased Dungeon file.
- **Looks that follow item level**: every new Chest gets its own Wardrobe, and every new weapon,
  shield, held item and bow its own model, taken from vanilla items near its item level. A
  looks ticket applies the same rule to the arc's Quest Lines and the four dungeon sets.
- **A tool**: `tools/twilights-eve/gear_sheet.py`, which reads, sets, scales and restyles
  `gear.csv` rows by entry. `sheet.py` works on `master.csv` by `calling/key` only, so #579 and
  #592 each wrote a one-off script.
- **A fight-model check** of decision 3 at item levels 33, 38, 43 and 48.

## Decisions (grill of 2026-11-08)

Davis answered each; those marked Claude were Claude's readings that Davis took.

1. **Quest greens** (Davis): Quest Lines that share one icon per slot. The arc's gear tickets
   (#600, #605, #610, #615) create them, with the blue `quest-elite-*` Lines. Not this spec.
2. **Band width** (Davis): five levels.
3. **Tier Step** (Davis): gear stays on the Power Curve and does not take the Tier Step. The
   Tier 2 Tier Bonus closes the gap. A fight-model check measures the gap (ticket 11).
4. **Bands 51-60** (Davis): not now. They come with the Burning Steppes content spec. Mobs above
   50 roll the 46-50 band until then.
5. **Epics** (Davis): three per final boss at 5%, in Mistmantle Manor, the Tower of Ilgalar,
   Raven Hill Deeps and Stonewatch Keep. The Stockade and the Deeprun Tram gain none.
6. **Dungeon-ending quests** (Davis): pay the set's four Chests, the Stockade rule. The arc and
   the dungeons spec build them.
7. **Bonus chests** (Davis): one per dungeon for an optional challenge, holding one set blue and
   coin. Davis left each dungeon's challenge to this spec's reading (Claude, below).
8. **Looks** (Davis): new looks everywhere, one per armour type per band and per dungeon set.
   Looks progress with item level as in WoW: a higher-level item wears a higher-level vanilla
   look. This replaces the "looks come from low-level families" rule for new rows.
9. **Whites** (Davis): no whites past 30. Greens are the floor.
10. **Elite rolls** (Claude): a `rolls` table by entry in `zones.toml`, a small realm change.
11. **Vanilla rows at item level 31 to 42** (Claude): the 26 vanilla rows below entry 900000
    stay. Nothing drops them.
12. **Set bonuses** (Davis): none.

### Readings taken while writing the tickets (Claude)

Each is stated in its ticket; Davis can change any before filing.

- **Band levels.** Set item levels 33, 38, 43 and 48, the arc's Quest Line levels. Required
  level is the band's first level (31, 36, 41, 46), as Elwynn's bands do. The Elite blues sit at
  the same levels.
- **Entries.** World greens `902100 + 20 x band index + Line index` (band index 0 for 31-35 to 3
  for 46-50), 902100 to 902179. Elite blues `904024 + 6 x band index + i`, 904024 to 904047.
  Epics 905909 to 905920. All three ranges continue the sheet's own formulas and are clear of the
  arc's 903200-903318 and the dungeons' 905140-905219.
- **A band with no whites** is `white = []` in `loot_bands.toml`. `Loot::band` already drops
  nothing on an empty list, so a 31+ kill drops a green 4% of the time and nothing else. No
  code change.
- **Epic shapes**, from the draft: Mistmantle three amulets (36), Ilgalar three main hands (38),
  Raven Hill Deeps three off-hand slots (shield, bow, held item; 44), Stonewatch three main hands
  (48). Might, Swift and Mind one each, in the shape of Elwynn's nine epics (905900-905908).
- **Epics roll on the boss's template**, as `extra_rolls = [{ chance = 5, one_of = [...] }]`.
  A creature row's `rolls` stops at the first success, and the set roll is at 100%, so a second
  `rolls` entry would never fire. `extra_rolls` rolls on its own (`world.rs` line 1865) and
  already works on a Phased Dungeon creature, so this needs no code.
- **Stonewatch's epics go on Gath'Ilzogg only.** The dungeons spec gives both roof bosses the
  100% set roll. One 5% roll on the warlord keeps the epic rate the same as the other three
  dungeons.
- **Bonus chests need realm code.** The draft said the Deeprun gold chest makes them free, but
  the gold chest is tram code (`tram.rs`, `Line::take_chests`). Ticket 03 adds a `[bonus_chest]`
  key any Phased Dungeon file can carry, with three challenge kinds: the final boss dies within N
  seconds of the Run's start, the boss dies with listed levers unpulled, or listed creatures all
  die within N seconds of each other. The tram's gold chest stays as it is.
- **The four challenges.** Mistmantle: Stalvan dies within 12 minutes of the Run's start (its
  fights are plain, so speed is the challenge). Ilgalar: Morganth dies with all three feed valves
  still open. Raven Hill Deeps: the Grey Deacon dies with the shoring lever never pulled, so the
  dig face's ghouls kept coming. Stonewatch: Gath'Ilzogg and Singe die within 10 seconds of each
  other, so neither enrages for long.
- **Bonus chest coin**: 100 x the set's item level in copper (3600, 3800, 4400, 4800), a
  placeholder.
- **Which elites roll**: the named elites and rares that still stand in Redridge and Duskwood
  (16), and the arc's two new named elites (the Grey Captain and the Twilight Courier). Each
  rolls the Elite pool and greens of the band holding its lowest spawn level, once the arc's band
  tickets have set the levels. Blackrock Scouts, Sentries, Hunters and Shadowcasters are elite
  packs, not named, and roll the Loot Band only, as Elwynn's packs do. Stormwind's Sewer Beasts
  stay out: Stormwind has no band.
- **Plate looks.** Vanilla has no plate chest below item level 41. The 13 plate Chests of these
  levels (this spec's four, the arc's five, the dungeons' four) take the 13 free plate looks from
  41 to 50 in item level order, assigned in one place (ticket 10). This spec's four band plate
  rows wear the 27-30 row's look until then.
- **Icons on restyled rows.** A Line shows one icon (ADR 0026, ADR 0030). A row that takes a new
  model writes its Line's icon name in its `icon` cell, so the new model does not change the icon.

## Scope

- `tools/twilights-eve/gear_sheet.py` and its tests.
- The `rolls` table in `zones.toml` (code and data).
- The `[bonus_chest]` Phased Dungeon key (code, a Ward Test row, the four dungeons' rows).
- 80 World greens, 24 Elite blues, 12 epics: 116 new `gear.csv` rows.
- Four bands in `loot_bands.toml`.
- New Wardrobes and models for this spec's rows, the arc's Quest Line rows and the four dungeon
  sets' rows.
- A report-only fight-model run at 33, 38, 43 and 48.

## Out of scope

- **Levels 51 to 60**: Loot Bands, Burning Steppes gear and the 50s dungeons (the Altar of
  Storms, Blackrock Stronghold, Dreadmaul Rock) wait for the Steppes content spec. The Steppes
  opening's break quest pays no gear and its camp bounties pay money
  (`docs/specs/steppes-opening/`), so it needs nothing from this spec.
- **Set bonuses** (decision 12) and **trinkets** (their own spec, #458's form).
- **Quest reward rows**: the arc's tickets 05, 10, 15 and 20.
- **The four dungeon sets** (905140-905219) and their boss rolls: the dungeons spec's tickets
  05, 07, 09 and 11. This spec only restyles their looks (ticket 10) and adds epics and chests.
- **The Stockade and Deeprun Tram sets** keep the Hogger's camp looks they copied. Decision 8
  could cover them; this spec does not touch them.
- **Setting the Tier Bonus**: the Tier 2 Callings spec fits it. Ticket 11 only reports the gap.
- **How players learn a bonus chest's challenge**: no hint text here.
- **Retiring the 26 vanilla 31-42 rows** (decision 11).

## Free ids this spec uses

| What | Ids |
|---|---|
| World greens (`gear.csv`) | 902100-902179 |
| Elite blues | 904024-904047 |
| Dungeon epics | 905909-905920 |
| Bonus chest GameObject entries (run-time, no template) | 980720 (the Ward Test's), 980721-980724 (Mistmantle, Ilgalar, Raven Hill Deeps, Stonewatch) |

## Icons

12 new stock icons, all for the epics; every other row extends a Line. Free stock counted
2026-11-07 (`docs/specs/prep/gear-31-60.md`, "Free stock icons"), less the arc's 36 Quest Line
icons:

| Slot | Free after the arc | This spec takes |
|---|---|---|
| Amulet | 22 | 3 (Mistmantle) |
| Sword | 27 | 2 (Ilgalar, Stonewatch) |
| Dagger | 20 | 2 |
| Staff | 9 | 2 |
| Shield | 19 | 1 (Raven Hill Deeps) |
| Held in off hand | 43 | 1 |
| Bow | 1 if the arc's bow Lines took crossbow or gun icons, else 0 | 1, or a crossbow or gun icon (12 free less the arc's) |

No painted icon. `docs/art-assets.md` gains one line, for the bonus chests' placeholder look.

## Child tickets

Build order. "Blocked by" names a file in this folder; `RD:` names a ticket file of the Redridge
and Duskwood arc, `DUNGEONS:` one of the 30-50 dungeons spec. Two builders: tickets 04, 05, 07
and 10 edit `gear.csv` and `gear.toml` and run one after another; the rest edit other files.

- 01 gear_sheet.py: read, set, scale and restyle gear.csv rows by entry
- 02 A rolls table by entry in zones.toml (RD:02-level-bands-code)
- 03 A bonus chest for a Phased Dungeon challenge (DUNGEONS:03-partner-death-link)
- 04 Loot Band gear 31-40: forty World greens and twelve Elite blues (01, RD:20-gear-45-50,
  DUNGEONS:11-stonewatch-loot)
- 05 Loot Band gear 41-50: forty World greens and twelve Elite blues (04)
- 06 Loot Bands 31-35 to 46-50 in loot_bands.toml, with no whites (05)
- 07 Dungeon epics: twelve epics on the four final bosses (05, DUNGEONS:05-mistmantle-loot,
  DUNGEONS:07-ilgalar-loot, DUNGEONS:09-raven-hill-loot, DUNGEONS:11-stonewatch-loot)
- 08 Elite rolls in Redridge and Duskwood (02, 05, RD:21-bands-45-50)
- 09 The four bonus chests (03, 07, DUNGEONS:05, 07, 09 and 11)
- 10 Looks that follow item level for the Quest Lines, the dungeon sets and the plate Chests (07,
  RD:20-gear-45-50, DUNGEONS:11-stonewatch-loot)
- 11 Fight-model check of gear on the Power Curve at 33, 38, 43 and 48 (07)
