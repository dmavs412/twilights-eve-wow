# The Stockade: the quiet cells (spec #575)

Status: grilled 2026-11-03, tickets ready-for-agent. Unplayed.

Grilled 2026-11-03 from "let's build out the Stockade". ADR 0032 lets a Phased Dungeon live on
an instance map. This note holds the design the tickets build and every reading taken where the
grill left a choice open. Lore: `docs/lore-bible.md`, "Stormwind City (hub): the court".

## Premise

The Stockade is the first Stormwind dungeon, levels 30 to 34, on vanilla's jail map (34). The
east block holds the Defias the Watch has caught since Elwynn. The west block, called the quiet
wing by the guards, holds people sent down by writ "from the Keep" for spreading panic. None of
them is a criminal. Each one saw the grey and said so.

An hour before the player arrives, a guard named Pell took a purse of Twilight coin and left the
lower doors unbarred. The Defias think they are breaking out. The purse was paid to empty the
quiet wing. The Watch's reinforcements are "delayed", and the player is at the door with Dughan's
report, so the Warden hands them the riot.

## Decisions (grill 2026-11-03)

1. **Hogger stays kill only.** The Elwynn 10-30 decision of 2026-10-20 stands. The bible's line
   about resolving his capture here is rewritten (rev 52).
2. **The lore is three ideas in one.** The quiet cells (witnesses the city locked up), the turned
   guard (Pell opened the doors) and the last of Westfall (Mara Fenn saw the cult leave before the
   mist). The Stockade is filled by those who know about the greater world.
3. **The Hush is the finale.** A Twilight Silencer seen in the flesh, the first cultist inside
   the city walls. Named cultists already appear as elites in Elwynn.
4. **Map 34 behind a loading screen.** The real cell blocks, as Wailing Caverns. The realm learns
   to run a Phased Dungeon on another map (ADR 0032).
5. **Lockdown levers, no meter, no fail state.** The cells are the shape; levers lock rows and
   open the quiet wing.

## The building

Vanilla's jail, drawn in `map.html` (open it in a browser). The loading screen lands at
(49.0, 0.5, -16.4) and the corridor runs north along x; west is +y. The jail is a cross: one
long hall of cells with a round room at each end, crossed near its middle by the corridor from
the door. The hall runs from the south-east round room (Bazil Thredd's old spot) to the
north-west one (Dextren Ward's), about 15 degrees off the y axis, a level below the corridor;
a stair on each side of the crossing goes down into it.

- The gate hall and the corridor, z about -25, from the door north to Targorr's old spot at the
  corridor's end, with cells on either side of its first stretch.
- The Defias block is the hall's east wing (y below 0, z about -35), down the east stairs.
  Row A is the near half of its cells and row B the far half, each with a lever and a
  portcullis where the row begins. Red Dace holds the round room at the wing's end.
- The quiet wing is the hall's west wing (y above 0, z about -35), down the west stairs. Its
  gate is at the foot of the stairs; Marrow stands on the stairs side of it. The four writ
  cells are the first cells past the gate. The warder's room is a cell halfway along, with the
  evidence chest and the lower doors, which Pell left open.
- The lower gallery is the wing's far half, past the lower doors. Pell is there with the two
  guards he bought.
- The drain is the round room at the wing's end, Dextren Ward's old room. The Hush is there,
  and the grate is in its floor.

The walking order is Dace, Marrow, Pell, the Hush, one boss per room and in level order.

## Bosses

- **Red Dace, 31, elite.** The Defias captain who took the deal, at the far side of the east
  hall with two Ringleaders. He calls two rioters out of each unlocked cell row every 20 s
  while he fights. He drops the purse, full of Twilight coin.
- **Bull Marrow, 32, elite.** The ogre the Defias kept for breaking doors, at the quiet wing's
  gate. A heavy swing and a shove. Health doubled.
- **Warder Pell, 33, elite.** The turned guard, in the lower gallery past the quiet wing with
  two guards he bought. A shield wall. He fights to the death. He drops the Sealed Ledger,
  which the player cannot open.
- **The Hush, 34, the boss with the Boss Bar.** A Twilight Silencer who came in through the
  drain, in the last room past the gallery. He is waiting: the writ cells are keyed upstairs,
  the ogre was to break them and the riot was to take the blame. He silences, fears and casts
  Shadow Bolt. He carries the Writ of Names, every prisoner in the quiet wing in a clerk's
  hand. The Stockade Sucker Pot halves his silence.

## The witnesses

Four cells, four lines, one thread each, in keeping with the bible's rumour rule.

- **Pim Gearspan**, a gnome tram engineer. Stopped the tram because the rails run into grey.
  Charged with sabotage. He plants the Deeprun Tram at 38 to 42.
- **Mara Fenn**, Defias, the last one out of Westfall. The hooded men left Moonbrook the day
  before the mist came. Her ledger page carries the hammer and black sun.
- **Hollis Dray**, a post-rider. Every letter to Ironforge comes back unopened, and there is a
  room in the city full of them. He never names whose.
- **Tam Orrel**, a canal boatman. Rowed out the sluice and found grey a mile out. The harbour
  knows.

## Levers

- A lever is a dungeon GameObject with `use = "lever"`. Pulling it is Run state: a portcullis
  with `after_used` drops, one with `until_used` rises, and a timed call with `until_used` stops.
  Levers reset with the Run.
- The Defias block has one lever per cell row. Each drops that row's portcullis and stops Red
  Dace's calls from it. Lock both and Dace fights with his two Ringleaders alone. A row is a
  dead-end gallery, so a dropped portcullis seals nothing the player needs. Pell left the rows
  unbarred; dropping them is the Warden's "get my cells back", and it is optional.
- The quiet wing's lever only works once Bull Marrow is dead (`needs_dead`), like a ward stone.
  It raises the gate's portcullis.

## Quests

Eleven quests, 97301 to 97311. The full text is in #580 and is the contract.

| # | Quest | Lvl | Giver | Task |
|---|---|---|---|---|
| 97301 | A Report for the Watch | 30 | Dughan | Carry his report to Thelwater at the Stockade door |
| 97302 | Quell the Riot | 30 | Thelwater | Kill 12 Defias Rioters |
| 97303 | Guards Below | 31 | Thelwater | Find Sergeant Ott (talk); he names Pell |
| 97304 | Red Dace | 31 | Thelwater | Kill Dace, bring the purse. Item quest |
| 97305 | The Quiet Wing | 32 | Thelwater | Kill Bull Marrow, pull the gate lever |
| 97306 | What They Saw | 32 | Gearspan | Talk to Fenn, Dray and Orrel |
| 97307 | The Last of Westfall | 33 | Fenn | Take her page from the evidence chest |
| 97308 | Warder Pell | 33 | Thelwater | Kill Pell, bring the Sealed Ledger. Item quest |
| 97309 | The Drain | 34 | the drain grate | Kill the Hush, bring the Writ of Names. Item quest |
| 97310 | Not Up the Hill | 34 | Thelwater | Carry the Ledger and the Writ to Dughan |
| 97311 | Twilight Coin | 32, repeatable | Thelwater | Bring 10 Twilight Coins |

## Loot

- A Stockade set of twenty blues, entries 905060 to 905079, item level 34, required level 31, one
  per line of the Hogger's camp set, stems Warder's, Hushed and Convict's. Dace, Marrow and Pell
  roll one at 50%, the Hush at 100%. No epics.
- The item quests choose among the set's four chests.
- Trash rolls the last Loot Band (27 to 30) until a later pass extends the bands past 30.

## Hooks forward

- The Writ of Names and "by writ from the Keep" are the 40s court chain's starting point. The
  Stockade never says which office.
- Pell's Sealed Ledger goes to Dughan, not up the hill. It is the paper that later traces the
  money to Cedric.
- Gearspan's stopped tram is the Deeprun Tram's door.
- Fenn's page is the cult's foreknowledge of the mist, a hint and never explained here.
- Spec #558 says Stockade inmates recognise a notorious player; the east block is where.
- The four witnesses stay in the quiet wing. Thelwater cannot open a writ cell; only the
  office that wrote the writ can, and that office sent the names. Their release is the court
  chain's payoff. He says so at the end of The Drain.

## Whose side

The player helps the Watch, never the Keep. Dughan sends them to Thelwater because he is
"Watch, not court". The riot is the real threat to the witnesses: the cult paid for it to kill
the quiet wing, so quelling it is saving them. The player learns what the quiet wing is only
once the gate has held (97305), and Thelwater, shown the Writ of Names, breaks his own rule and
sends the evidence to Dughan instead of up the hill (97310). The city splits into the Watch,
honest and in the dark, and the Keep, which wrote the writs.

## Reveal check

At 30 to 34 the player learns that the cult pays locals, that the cult knew the mist was coming,
and that the city locks up people who talk about it. Nobody names Prestor, Cedric, the Anchors or
the dragons.

## Readings

- Dungeon id 3, map 34, zone 717 (The Stockade). Creature guids 973001 up, GameObjects 983001
  up, templates 97200 to 97211, quests 97301 to 97311, gear 905060 to 905079.
- North is +x and west is +y, so the quest text's east block (the Defias) is y below 0 and the
  west block (the quiet wing) is y above 0. The first draft of this note called them south
  and north; the map and #578 fixed that on 2026-11-03.
- The first draft put Pell and the Hush in the Defias block, on the way to Dace. They moved
  below the quiet wing the same day so the player meets the bosses in level order.
- The map's walls are a schematic of the jail's shape, taken from a client map of the
  Stockade; the spawn points on it are measured, the walls are not. The first map drew each
  wing as a hall with deeper rooms behind it, read off the spawn points alone, and was wrong.
- Every Z on map 34 is written from the vanilla dump with `grounded = false`; the whole jail is
  one footprint box.
- Entrance and exit are vanilla's triggers 101 and 503. The Portal and the map mark stand at the
  city door, where Warden Thelwater already stands (`stormwind.toml` spawn 89325).
- The Dughan hook has no prerequisite beyond level 30. Witness talks are quest dialogue, not
  rumours.
- Sergeant Ott is the one talk target of Guards Below: a talk row counts an entry once.
- Bull Marrow uses the stock ogre display, so no scale field is needed.
- Silence counts as crowd control for `cc_half` and not for `fear_half`.
- Chases stay straight lines; a route graph like the Caverns' waits for play.
- Art: no new models. One icon row (the Twilight Coin) in `docs/art-assets.md`.

## Tickets

- #576 Phased Dungeons on an instance map: the Stockade shell on map 34 (ADR 0032)
- #577 Lockdown levers and the silence effect
- #578 The Stockade: the riot, the quiet wing, four bosses and the levers (blocked by #576 and
  #577)
- #579 The Stockade set: twenty blues at item level 34 and the bosses' rolls (blocked by #578)
- #580 The Stockade's eleven quests, Dughan's hook, Thelwater and the quest items (blocked by
  #578 and #579)
