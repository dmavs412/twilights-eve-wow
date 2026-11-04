# The Stockade: the riot and the quiet wing (spec #575)

Status: redesigned 2026-11-04 as two dungeons, tickets ready-for-agent. Unplayed.

Grilled 2026-11-03 from "let's build out the Stockade" and redesigned 2026-11-04 from "split the
Stockade into two dungeons". ADR 0032 lets Phased Dungeons live on an instance map. This note
holds the design the tickets build and every reading taken where Davis left a choice open.
Lore: `docs/lore-bible.md`, "Stormwind City (hub): the court". The first design (one dungeon,
four bosses in a line, a turned warder named Pell) is in this file's history before 2026-11-04.

## Premise

Vanilla's jail (map 34) holds two dungeons with the City Watch between them.

The east wing is **the riot**, levels 30 to 34. Captain Ansel Rook of the Watch, the Warden's
second, sold the quiet wing's list of names to a stranger on the canal. Bertram Ash, the night
turnkey, saw him do it. Rook unbarred the east block and paid Varl Cutter, the Defias
ringleader, a purse of Twilight coin to make sure Ash does not live to say so. The Watch held
the crossing. Rook has "gone down after Ash with two men".

The west wing is **the quiet wing**, levels 36 to 40. The Keep sends people down to it by writ
for talking about the grey, and the Watch is told not to ask what for. Rook held its key for the
Keep. Behind the gate: four prisoners who each saw the grey from a different side, Sister Quill,
a Keep scribe who writes down what each one knows before the Hush takes their voice, and the
Hush himself, a Twilight Silencer who came in by the drain with their names on a writ.

The middle is **the Watch's**: the gate hall, the corridor and its cells. Warden Thelwater runs
the jail from the crossing, a quartermaster repairs, and a sergeant stands across the west
stairs with the Keep's order in his mouth.

## Decisions

1. **Two dungeons, one map, the Watch between** (Davis, 2026-11-04). The corridor is open
   world on map 34: Hub NPCs stand there and characters meet there. Each wing is a Phased
   Dungeon with its own Run, Portal and footprint (ADR 0032, amended).
2. **Brackets: the riot 30 to 34, the quiet wing 36 to 40** (Davis, 2026-11-04). The quiet
   wing sits between Ilgalar (34 to 38) and the Deeprun Tram (38 to 42), and the Writ of Names
   starts the 40s court chain at 40.
3. **The story is "the turnkey"** (Davis's pick of five, 2026-11-04): the riot is cover for
   killing a guard who saw a Watch captain sell the quiet wing's names.
4. **The lock is Rook's key.** Thelwater takes it off Rook's body as evidence and will not turn
   it until Dughan has read Rook's letters. At 36 he hands it to the player: "I said I wouldn't
   turn it. I'm not. You are." The west stairs carry a sealed door in the Keep gate's form
   (`stormwind_keep.toml`, `[portal.door]`), opened once by a character carrying the key.
5. **The Hush is the finale**, as decided 2026-11-03. Hogger stays kill only. Map 34 behind a
   loading screen. Lockdown levers in the riot, no meter, no fail state.
6. **The two wings play differently.** The riot is loud: packs, a ringleader who calls more
   rioters out of every unlocked row, fire bottles, levers, a captain behind a shield wall, and
   every creature shouts an aggro line. The quiet wing is quiet: elite warders in pairs, casters,
   prisoners whose voices are gone and who make no sound, a scribe who silences and roots, and
   the Hush. No creature in the quiet wing has an aggro line.

## The building

Vanilla's jail, drawn in `map.html` (open it in a browser). The loading screen lands at
(49.0, 0.5, -16.4) and the corridor runs north along x; west is +y. The jail is a cross: one
long hall of cells with a round room at each end, crossed near its middle by the corridor from
the door. The hall runs from the south-east round room (Bazil Thredd's old spot) to the
north-west one (Dextren Ward's), about 15 degrees off the y axis, a level below the corridor; a
stair on each side of the crossing goes down into it.

- **The Watch's ground**, z about -25: the gate hall, the corridor from the door north to
  Targorr's old spot at its end, and the cells on either side of its first stretch, which hold
  convicts behind bars. Open world. Thelwater at the crossing, Quartermaster Mae Tollin in the
  gate hall, Sergeant Hale at the head of the west stairs, Watchmen at the east stairs, the door
  and the corridor's end. Each dungeon's Portal stands at the head of its stairs.
- **The riot**, the hall's east wing (y below 0, z about -35), from the east stairs down. Row A
  is the near half of its cells and row B the far half, each with a lever and a portcullis where
  the row begins. Varl Cutter holds the hall where the rows meet. Captain Rook holds the round
  room at the wing's end, in front of the cell where Ash has barred himself in.
- **The quiet wing**, the hall's west wing (y above 0, z about -35), from the west stairs down.
  The sealed door is at the stair head, on the Watch's side. The four writ cells are the first
  cells past the foot of the stairs. The scribe's room is a cell halfway along. The lower gallery
  is the wing's far half, where the Hushed walk. The drain is the round room at the wing's end,
  Dextren Ward's old room, with the grate in its floor.

## Bosses

The riot:

- **Varl Cutter, 32, elite.** The Defias ringleader who took Rook's purse, where row A meets
  row B, with two Ringleaders. Every 20 s he calls two Rioters out of each row whose bars are
  still up; lock both rows and he fights with his Ringleaders alone. Shiv: a bleed on his target
  every 12 s. Fire Bottle: every 20 s he throws a bottle at a random character, a fire patch of
  4 yd for 10 s where it lands, with a warning. He drops the purse of Twilight coin.
- **Captain Ansel Rook, 34, elite, the Boss Bar.** The Watch's captain, in the round room at the
  wing's end with two Bought Watchmen, between the player and Ash's cell. He draws on sight:
  nobody was meant to get this far. Shield Wall every 30 s: 60% less damage taken and 30% less
  dealt for 8 s, so the burn is between walls. Captain's Cut: a blow at 200% every 10 s.
  Hamstring: a 50% snare for 6 s every 15 s. He drops Rook's Keyring and Rook's Letters.

The quiet wing:

- **Sister Quill, 38, elite.** The Keep's scribe, in the scribe's room with two Twilight
  Listeners. Take Down: a 4 s silence on a random character every 15 s; she writes your words
  down and you lose them. Ink: a 2 s cast every 25 s with a warning, then every character within
  8 yd of her cannot move for 3 s. Shadow Bolt. She drops the Book of Takings.
- **The Hush, 40, elite, the Boss Bar.** A Twilight Silencer who came up the drain, in the round
  room at the wing's end. Hush: a 5 s silence on a random character every 18 s. Fear every 30 s.
  Shadow Bolt. Stillness: every 40 s a shield that soaks damage for 10 s. He carries the Writ of
  Names, every prisoner in the quiet wing in a clerk's hand with a hammer over a black sun below
  them. The Stockade Sucker meal halves his silence (`cc_half`).

## The monsters

The riot, about 28 creatures:

- **Defias Rioter**, 30 to 31, normal. The trash. Each has an aggro line.
- **Defias Ringleader**, 31 to 32, elite. Pack leaders and the lever guards.
- **Bought Watchman**, 33, elite. Two, with Rook. Watch tabards; they turned with him.
- Packs: two Rioters and a Ringleader at the foot of the stairs; three cells of two Rioters in
  each row, a Ringleader and a Rioter at each lever post; Cutter with two Ringleaders where the
  rows meet; Rook with two Bought Watchmen in the round room. Bertram Ash, friendly, in the last
  cell before the round room.

The quiet wing, about 24 creatures:

- **Writ Warder**, 36 to 37, elite. The Keep's own warders, who turned with the coin. Two at
  the foot of the stairs, and one in each patrol pair.
- **Twilight Listener**, 37 to 38, normal. Cultists who came up the drain. Casters: Shadow Bolt
  and a short snare. One in each patrol pair, two with Quill.
- **The Hushed**, 38 to 39, normal. Prisoners whose voices the Hush took. Slow, hit hard, make
  no sound. Three packs of three in the lower gallery.
- The four witnesses, friendly, in the first four cells. Nothing in the wing has an aggro line.

## The witnesses

Four cells, four lines, one thread each, in keeping with the bible's rumour rule.

- **Wender Cogg**, a gnome tram engineer. Stopped the tram because the rails run into grey.
  Charged with sabotage. He plants the Deeprun Tram at 38 to 42.
- **Hesper Lowe**, a Westfall farmwife, the last one out of Moonbrook. The hooded men packed
  their crates and left the day before the grey came. They knew.
- **Dorrin Vask**, a harbour pilot. A mile out the sea goes grey and the water under it does not
  move, and the Keep's own cutter was out there, watching it, before he was.
- **Tobin Marsh**, a Keep page. He copied an order closing the Westfall road, dated three days
  before the mist came, and asked who signed it. He gives the quest for Quill.

## Levers and the lock

- A lever is a dungeon GameObject with `use = "lever"` (#577). Pulling it is Run state: a
  portcullis with `after_used` drops and a timed call with `until_used` stops. Levers reset with
  the Run.
- The riot has one lever per cell row. Each drops that row's portcullis and stops Cutter's calls
  from it. A row is a stretch of cells on a through hall, so a dropped portcullis seals nothing
  the player needs. Cutter's men unbarred the rows; dropping them is Thelwater's "get my cells
  back", and it is optional.
- The quiet wing has no lever. Its door at the west stair head is sealed, in the Keep gate's
  form, and opens once for a character carrying Rook's Key, which quest 97308 gives. Sergeant
  Hale speaks the refusal. Head Count and the Run start past the door.

## Quests

Twelve quests, 97301 to 97312. The full text is in #580 and is the contract.

| # | Quest | Lvl | Giver | Task |
|---|---|---|---|---|
| 97301 | A Report for the Watch | 30 | Dughan | Carry his report to Thelwater at the crossing inside the Stockade |
| 97302 | Quell the Riot | 30 | Thelwater | Kill 12 Defias Rioters |
| 97303 | Varl Cutter | 32 | Thelwater | Kill Cutter, bring the purse. Item quest |
| 97304 | The Turnkey | 33 | Thelwater | Find Bertram Ash in the east wing (Rook is in the way) |
| 97305 | The Captain's Keys | 34 | Ash | Bring Rook's Keyring and Rook's Letters to Thelwater. Item quest |
| 97306 | Rook's Letters | 34 | Thelwater | Carry the Letters to Dughan |
| 97307 | The Seal | 36 | Dughan | Carry Dughan's answer back to Thelwater |
| 97308 | The Quiet Wing | 37 | Thelwater | Take Rook's Key, open the west door, speak with the four |
| 97309 | The Scribe | 38 | Marsh | Kill Sister Quill, bring the Book of Takings. Item quest |
| 97310 | The Drain | 40 | the drain grate | Kill the Hush, bring the Writ of Names. Item quest |
| 97311 | Not Up the Hill | 40 | Thelwater | Carry the Book and the Writ to Dughan |
| 97312 | Twilight Coin | 32, repeatable | Thelwater | Bring 10 Twilight Coins |

## Loot

- Two sets of twenty blues, one per line of the Hogger's camp set, as the dungeons before them.
  The Riot set, entries 905060 to 905079, item level 34, required level 31, stems Convict's,
  Watchman's and Rioter's. The Quiet Wing set, entries 905080 to 905099, item level 40, required
  level 37, stems Hushed, Scribe's and Warder's. No epics.
- Cutter and Quill roll one of their set at 50%; Rook and the Hush at 100%.
- The item quests choose among their set's four chests.
- Trash rolls the last Loot Band (27 to 30) until a later pass extends the bands past 30.

## Hooks forward

- The Writ of Names, the Book of Takings and the seal on Rook's letters are the 40s court
  chain's starting point. The Stockade never says which office.
- Rook's Letters go to Dughan, not up the hill. Dughan matches the seal to the one on his own
  returned reports.
- Cogg's stopped tram is the Deeprun Tram's door. Lowe's day-before is the cult's foreknowledge
  of the mist. Marsh's order is the Keep's. Vask's cutter is the harbour's.
- Spec #558 says Stockade inmates recognise a notorious player; the corridor's cells are where.
- The four witnesses stay in their cells. Thelwater cannot open a writ cell; only the office that
  wrote the writ can. Their release is the court chain's payoff. He says so at the end of The
  Drain.

## Whose side

The player helps the Watch, never the Keep. Dughan sends them to Thelwater because he is
"Watch, not court". The riot is the Watch's own house: its captain sold names and bought a
riot to bury the witness, and the Watch cleans that up itself. The quiet wing is the Keep's.
Opening it is the Watch breaking the Keep's rule, with the player as its hand, and Thelwater
sends what comes out of it to Dughan instead of up the hill. The city splits into the Watch,
honest and in the dark, and the Keep, which wrote the writs.

## Reveal check

At 30 to 34 the player learns that a Watch captain sold a list of names, that the cult pays in
its own coin, and that the Keep keeps a wing nobody asks about. At 36 to 40 they learn that the
city locks up people who talk about the grey, that the cult knew the mist was coming, and that
the Keep closed the Westfall road before it did. Nobody names Prestor, Cedric, the Anchors or
the dragons.

## Readings

- Dungeon ids 16 (the riot) and 17 (the quiet wing); id 3 is the test grounds. Map 34, zone 717
  (The Stockade). Creature guids 973001 up, GameObjects 983001 up, templates 97200 to 97219,
  quests 97301 to 97312, gear 905060 to 905099, Hub NPC guids 9300072 up.
- An instance map is content of its own (`instance_maps.toml`): its door triggers, its zone and
  its map mark. A Phased Dungeon on it is a footprint like any other; the map outside every
  footprint is open world. ADR 0032, amended 2026-11-04.
- North is +x and west is +y, so the east wing is y below 0 and the west wing y above 0.
- Thelwater's stock spawn at the city door (`stormwind.toml` guid 89325) is deleted and he
  stands inside as a Hub NPC on map 34. The Dungeon mark and the Portal of the whole jail stay at
  the city door.
- The map's walls are a schematic of the jail's shape, taken from a client map of the Stockade;
  the spawn points on it are measured, the walls are not. The Build lane fits every box, spawn
  and object to the 93 stock spawns.
- Every Z on map 34 is written from the vanilla dump with `grounded = false`; Hub NPCs there the
  same.
- Entrance and exit are vanilla's triggers 101 and 503, on the instance map.
- The Dughan hook has no prerequisite beyond level 30. Witness talks are quest dialogue, not
  rumours.
- Rook is hostile from the Run's start. He never appears in the hub; Thelwater says where he
  went.
- The Bought Watchmen use the stock Stockade Guard display; Rook the Stormwind guard captain's.
  Quill a stock robed woman's. No new models.
- Silence counts as crowd control for `cc_half` and not for `fear_half`.
- Chases stay straight lines; a route graph like the Caverns' waits for play.
- Art: no new models. One icon row (the Twilight Coin) in `docs/art-assets.md`.

## Tickets

- #576 Phased Dungeons on an instance map: the Stockade shell on map 34 (the first design's
  shell, one dungeon over the whole jail; it was mid-build when the split was decided and lands
  as written)
- #577 Lockdown levers and the silence effect
- #581 Instance maps and the Watch's corridor: split the Stockade shell into two dungeons
  (ADR 0032 amended; blocked by #576)
- #578 The riot: Cutter, Rook, the rows and the levers (blocked by #576, #577 and #581)
- #582 The quiet wing: the sealed door, the witnesses, Quill and the Hush (blocked by #576,
  #577 and #581)
- #579 Two Stockade sets: forty blues and the bosses' rolls (blocked by #578 and #582)
- #580 The Stockade's twelve quests, Dughan's hook, Thelwater and the quest items (blocked by
  #578, #582 and #579)
