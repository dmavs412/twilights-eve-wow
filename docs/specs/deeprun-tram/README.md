# The Deeprun Tram

Grilled with Davis on 2026-11-06. A wave dungeon for levels 38 to 42 on vanilla's tram map
(369). Trams roll out of the mist from the Ironforge side, unload cultists onto the Stormwind
platform, and back out again. Porters try to carry the cult's tribute away on them. The run
ends with the Conductor, who comes on the tram that froze in the mist. ADR 0033 records the
choices that reach past this dungeon.

## Where it is

- **Map 369 behind a loading screen** (ADR 0032). `AreaTrigger.dbc` 2173 in the Dwarven
  District station (map 0, (-8346.46, 514.03, 96.60), radius 10) worldports to (67.76, 2490.98,
  -4.30) facing 3.14. Trigger 2171 inside (80.16, 2490.88, -1.86), radius 10, sends the
  character back to (-8354.23, 524.07, 91.80) facing 2.36. Area 2257, "Deeprun Tram".
- **Dungeon 18, "The Deeprun Tram"**, levels 38 to 42. Its footprint is the Stormwind station
  and the tunnel up to the mist wall: x -90 to 50, y 2400 to 2530, z -25 to 20. The arrival
  point and the exit trigger stand east of it, open world on map 369. The Portal is at the
  footprint's east edge, (50.0, 2491.0), facing west.
- **The station.** Three platform strips run along y: the west strip (benches at x -80 to -61),
  the centre strip (benches at x -20 to -17) and the east strip (benches at x 20 to 40). Track A
  runs at x -45.4 between the west and centre strips, track B at x 4.53 between the centre and
  east strips. The platform floor is z -4.2965 (the vanilla benches and the arrival point). The
  tunnel leaves the station toward lower y; Ironforge is at y 10.
- **The mist wall** stands across both tracks at y 2400, 60 yards down the tunnel from the
  platform's south end, from x -75 to x 35 and z -20 to 30. The client draws it with the outdoor
  mist's shader and makes it solid. The realm refuses any step past it with "You cannot go that
  way." as it does for a Closed Zone (ADR 0015). Players may walk the platforms and both tracks
  up to the wall.

## Before the run

- **Station Master Brannoc**, a dwarf Hub NPC in the Dwarven District station hall on map 0
  ((-8350.5, 519.5), on the hall floor at z 91.80), gives "Down the Line" (level 38). He has
  heard trams in the tunnel at night since Cogg was jailed. Wender Cogg's reply in the quiet wing
  ends by sending the player to Brannoc.
- **The Tribute Pile** is 12 crates (display 31) on the centre strip, in a 3 by 4 grid at x
  -19.7, -18.5, -17.3 and y 2488.7, 2489.9, 2491.1, 2492.3. Each crate is its own object, so the
  pile's count is the score. There is no score bar.
- **The station bell** stands beside the pile at (-18.5, 2484.0). Ringing it starts the run;
  the first tram arrives 10 seconds later. Before the bell, nothing comes.

## The trams

- **A car** is vanilla's Subway car (GameObject entry 176080's display). It stops at one of
  three points on its track: the lead stop at y 2512.2, then 2492.8, then 2472.9, the three
  vanilla cars' places. A tram of n cars fills the first n stops. Car origins stand at z 6.905,
  the vanilla cars' own.
- **The run in.** Each car starts 150 yards before its stop, behind the mist wall, and rolls at
  30 yards a second, so it stops 5 seconds later. As the cars start rolling, the bell rings (sound
  7234, BellTollDwarfGnome) and that track's lamp lights: a lantern (display 6038) at the
  platform's south end, (-45.4, 2458.0) for track A and (4.53, 2458.0) for track B, shown only
  while a warning lasts. The cars face the platform both ways and never turn round.
- **At the platform** the cars stand 20 seconds. At 17 seconds the departure whistle sounds
  (6757, SubwayStart) and the lamp lights; at 20 they back out along the same track at 30 yards a
  second and are gone once they pass the wall.
- **Lane hits.** A lane is a track's strip, 3 yards either side of its x, from the wall to the
  station's north end (y 2525). A moving car hits every living player in its lane within 10
  yards of the car's centre along y and within 5 yards of the platform floor: 40% of the
  player's maximum health, unaffected by armour, and 2 seconds rooted. A car hits a player at most
  once a pass. This holds for every moving car: arrivals, departures and runaway cars.
- **The timetable.** Tram 1 arrives 10 seconds after the bell, with 1 car. The gap after it is
  40 seconds. At each later tram's warning the realm checks for any living creature that a tram
  brought (fighters and porters). If there is none, this tram gets one car more than the last (at
  most 3) and the gap after it is 25 seconds. If there is one, it gets one car fewer (at least 1)
  and the gap after it is 40 seconds. Odd trams use track A and even trams track B.
- **Six wave trams, then the boss tram** one gap after tram 6. A group that clears every wave
  sees arrivals at 0:10, 0:50, 1:15, 1:40, 2:05 and 2:30, with 1, 2, 3, 3, 3 and 3 cars, and
  the boss at 2:55. A group that never clears one sees 0:10, 0:50, 1:30, 2:10, 2:50 and 3:30,
  one car each, and the boss at 4:10.

## What a car carries

- **Two fighters and one porter.** Each fighter slot rolls Hauler, Listener or Lampman with
  equal odds, and a car holds at most one Lampman. Fighters are levels 38 to 40, porters 38.
- **Stepping off.** When the car stops, its three creatures appear at its door on the centre
  strip side (x -40.0 for track A, x 0.0 for track B, at the car's stop y, 1.5 yards apart along
  y). Each waits 1 second, then the fighters attack the closest living player. Head Count (ADR
  0029) scales every one of them, porters included, as a creature made into a fight.
- **The Twilight Hauler** is melee in dock clothes (Defias Dockworker's display, 2357).
- **The Twilight Listener** is the Stockade's caster (template 97207, Shadow Bolt and Cold
  Word).
- **The Twilight Lampman** (Twilight Aquamancer's display, 2886) sets a mist lamp where he
  stands 1 second after stepping off and every 12 seconds after. A lamp is a lantern (display
  6038) that slows every player within 6 yards by 30%. All his lamps go out when he dies.
- **The Twilight Porter** (Dabyrie Laborer's display, 3958, with a gold glow aura) never
  attacks, ignores taunts and has half a normal's health. It walks to the pile, takes 2 seconds
  to lift a crate, and walks at 70% speed to the door of the closest car standing at the platform.
  If none stands there, it waits at the door point of the track it last walked to, and goes to
  the next car that stops on either track. A porter that reaches a standing car's door boards
  it: it leaves the fight and its crate has escaped. A porter killed while carrying drops its
  crate back onto the pile at once. A porter that finds the pile bare waits beside it until a
  crate comes back. Once every crate has escaped, cars carry no porters.

## The Conductor

- **The boss tram** is one car on track A at the lead stop, with the usual warning. When it
  stops, the Conductor and his frozen passengers step off at its door and the empty car leaves on
  the usual 20-second timetable. Porters still alive can board it.
- **The Conductor** is the dwarf who drove the last tram before Cogg stopped the line, a level
  42 elite boss (Durtham Greldon's display, 9817; the generic boss's multipliers, 2.0 health and
  1.1 damage). He waits 1 second and attacks the closest player. He shows on the Boss Bar (ADR
  0013).
- **Tribute.** His Tribute aura carries one stack per crate escaped by the time he steps off.
  Each stack gives +4% maximum health and +3% damage.
- **Passengers.** 3, plus 1 for every 3 escaped crates (at most 7), stand frozen mid-step in
  Freezing Trap's ice shell: stock Stormwind and Ironforge citizens, level 40 normals. Their six
  templates wear displays 1541, 1440 and 1489 (humans), 6062 (a dwarf), and 9821 and 9818
  (gnomes); a seventh passenger takes the first again. They wake in three groups as the
  Conductor's health reaches 75%, 50% and 25%, split as evenly as possible with the extra ones in
  the earlier groups (4 passengers wake 2, 1, 1). A woken passenger waits 1 second and attacks
  the closest player.
- **His moves** start when he does, one every 10 seconds, alternating. At 10 seconds and every
  20 after, the **runaway car**: the bell and a random track's lamp, and 3 seconds later an
  empty car runs in from behind the wall to the lead stop and straight back out, hitting its lane
  both ways. At 20 seconds and every 20 after, the **crate toss**: if the pile holds a crate, he
  takes one and a 3-yard circle marks the ground under the living player farthest from him for 3
  seconds. If any living player stands in it when it lands, the crate is caught and goes back on
  the pile, and every player in the circle takes 15% of their maximum health. If nobody does, the
  crate is lost. With the pile bare he does not toss.
- **No time limit.** The fight lasts until he dies or the group leaves. The 6 minutes is the
  target length of a run, not a cap.

## Loot

- **The Tribute Pile chest.** When the Conductor dies, the crates on the pile turn into one
  chest at the pile's centre holding 100 copper per crate and one Deeprun green per 3 crates
  (rounded down). All 12 saved pay 12 silver and 4 greens.
- **The gold chest.** If every porter that stepped off a tram died, a second chest stands beside
  the Conductor's corpse with one Deeprun blue.
- **The Conductor** always drops one Deeprun blue, as Captain Rook does.
- **Two new sets** in `gear.csv`, after the Stockade's: 20 blues (905100 to 905119, item level
  42, required level 39) and 20 greens (905120 to 905139, item level 40, required level 37),
  each copying the slot, line, look and stat shape of Hogger's camp rows (905040 to 905059).

## Readings taken

- A porter's crate escapes when the porter boards, not when the car leaves. Once aboard it is out
  of reach, so "escaped at boarding" and "escaped as the car leaves" never disagree.
- Headway is judged at each tram's warning and changes that tram's cars and the gap after it.
  That makes a cleared wave show at once in the next tram's length.
- Tribute counts crates escaped when the Conductor steps off. A porter that escapes on the boss
  car still costs the gold chest and the pile, but not a stack.
- Wipes and resets follow ADR 0006 and ADR 0032: release brings the character out at the
  station, and the Run resets once the last living player leaves the footprint.
- Unattended test runs cannot click a GameObject, so a `WOW_PROBE` variable rings the bell.
- The tram code is for this dungeon only (Davis, 2026-11-06). Its numbers live in the dungeon's
  content file, not in Rust.

## Tickets

Spec issue #586. In build order:

- #587 The shell: map 369, the station, the bell and the mist wall's realm half.
- #588 The client draws and blocks placed mist walls (after #587).
- #589 The tram line: cars, the timetable with Headway, warnings and lane hits (after #587).
- #590 The tram creatures and the Tribute Pile (after #589).
- #591 The Conductor (after #590).
- #592 The loot (after #591).
- #593 Brannoc, Down the Line and Cogg's pointer (after #591 and #592).
- #594 `WOW_PROBE_USE`, so an unattended run can ring the bell (after #587).
