# The 30-50 dungeons: grill prep

A draft for a grill with Davis, written 2026-11-07. It covers the four 30-50 dungeons that are
still shells: Mistmantle Manor (32-36, Duskwood), the Tower of Ilgalar (34-38, Redridge), Raven
Hill Deeps (40-44, Duskwood) and Stonewatch Keep (45-48, Redridge). Nothing here is decided.
Every guess says "Assumed:". Names of new creatures are placeholders.

## 1. What exists today

### The shells

Each shell landed on 2026-10-27 (cd940cee) as a Phased Dungeon on map 0 (ADR 0006). Each has the
WMO's indoor groups as footprint boxes, a Portal in the doorway and one friendly Phase Tester
(entry 97100). The vanilla creatures inside the boxes are deleted through `removed_guids` in
`crates/twilight-realm/content/zones.toml`.

| Dungeon | File | Id | Levels in the file | Bible | WMO |
|---|---|---|---|---|---|
| Tower of Ilgalar | `content/ilgalar.toml` | 8 | 34-38 | 34-38 | `redridge_magetower.wmo`, 4 groups |
| Stonewatch Keep | `content/stonewatch_keep.toml` | 9 | 44-48 | 45-48 | `ruinedkeep.wmo`, 15 groups |
| Mistmantle Manor | `content/mistmantle_manor.toml` | 10 | 32-36 | 32-36 | `duskwoodabandoned_human_farm.wmo`, 1 group |
| Raven Hill Deeps | `content/raven_hill_deeps.toml` | 11 | 44-48 | 40-44 | `md_crypt_c.wmo`, 11 groups |

- Two files still carry the old 44-48 band. Lore bible rev 50 (2026-11-02) moved Raven Hill to
  40-44 and Stonewatch to 45-48, and Davis chose "lore bible only" at the time.
- `src/phased.rs`, module `lore_bible_dungeons`, asserts that dungeons 8 to 15 each hold exactly
  one friendly creature. Filling any of these dungeons edits that test.
- No Redridge or Duskwood quest exists. The Elwynn exit quests 95101 to 95103 are designed in
  `docs/specs/elwynn-10-30/ELWYNN_10-30_QUEST_DESIGN.md` but not in content. Solomon, the
  Ebonlockes and the other town NPCs are friendly spawns with no menus (spec #466), so quest
  givers would be Field Givers.

### Parts to build bosses from

- Creature rows (`src/phased.rs`, `Creature`): `drop`, `rolls`, `on_health` adds, `every` adds on
  a clock (stopped by a lever with `until_used`), the Appears keys (`after_dead`, `until_dead`,
  `after_used`, `until_used`, `after_pinned`), and `asleep`, `camp`, `wake_radius_mod`, `wander`.
- Template keys (`content/mobs.toml`): `boss` (the Boss Bar, ADR 0013, up to three at once),
  `rank`, `say`, `flee_at_pct`, `melee = false`, `shield`, `never_fights`, `inert`,
  `hunts_at_meter_max`, `wakes_sleepers`.
- Creature Spell effects (`content/creature_spells.toml`, ADR 0008): `periodic_damage`, `heal`,
  `hit`, `snare`, `charge`, `gaze` (stopped by `[[cover]]` pillars), `nova`, `fear`, `silence`,
  `root`, `ground_patch`, `absorb`, `armor_debuff`, `stance`. Options: `warning`, `target =
  "random"`, `below_health_pct` (a second phase), `first_cast_ms`, `corpse_within`. The next free
  spell id is 1900060.
- Dungeon-level parts. Levers that drop a portcullis and stop calls (`stockade_riot.toml`). A
  sealed Portal door that opens for a key item (`stockade_quiet_wing.toml`). Locked inner doors
  with a key rolled per Run among holders, and a hint NPC (`northshire_abbey.toml`). Stops behind
  inner portals, pinned with ward stones (`azora.toml`). A Meter with Cover spots and sleepers
  (`forest_edge.toml`, whose #533 rework landed but is unplayed). `[[cover]]` against a gaze
  (`jasperlode.toml`). `[wall]` round an open-air box (`forest_edge.toml`). Placed mist walls in
  `data/twilights-eve/mist_walls.json` (ADR 0033).
- These apply on their own: Head Count scaling (ADR 0029), control rules on bosses (ADR 0020) and
  spawn spot rules with `ground_check.py` (ADR 0021).
- Tram only (`src/tram.rs`, ADR 0033): frozen passengers that thaw at the Conductor's 75, 50 and
  25% marks, Tribute stacks, and porters that walk to a point. ADR 0033 keeps them tram code until
  a second dungeon needs them.

The spec shape to copy is the Stockade (#575, `docs/specs/stockade/README.md`): one spec, a
monsters ticket per wing, then loot (20 blues a wing), then quests. A wing has about 25 creatures
and two bosses.

### Findings from the game data

The Level Stretch (spec #466) maps Redridge's vanilla 14-27 and Duskwood's 18-35 onto 30-50. The
open world around the two early dungeons comes out well above them:

| Place | Vanilla creatures near the door | After the stretch |
|---|---|---|
| Ilgalar's hill | Shadowhide gnolls and Servants of Ilgalar (23-26) | about 44 to 48 |
| Mistmantle's yard | 10 Fetid Corpses about 25 yd east of the house (29-30) | about 43 to 44 |
| Raven Hill Cemetery | Skeletal Healer, Warder, Raider (26-29); Mor'Ladim (35) | 39 to 43; Mor'Ladim 50 |
| Stonewatch Keep | Blackrock Sentry, Shadowcaster, Hunter, Gladiator (21-25) | 41 to 47 |

- Ilgalar and Stonewatch Keep stand about 320 yd apart in eastern Redridge. Mistmantle Manor is
  about 150 yd north of Darkshire's inn.
- `azora.toml`'s shadow stop says it stands "at the foot of the Tower of Ilgalar, about (-9235,
  -2700)". The tower's door is at (-9282, -3328), about 630 yd south. Fix or reword it before
  Ilgalar ships.
- Free ids: gear 905140 up, templates 97225 up, creature guids 974001 up, GameObject guids 983029
  up. Assumed: each dungeon takes 100 guids and 20 templates.

## 2. The dungeons

Coordinates are world coordinates on map 0, rounded. Box centres come from the footprints.

### Mistmantle Manor (32-36, Duskwood)

**Where.** The house at Manor Mistmantle, (-10370, -1256, 36). The Portal is the front door at
(-10362.6, -1257.0, 35.9). The footprint is one indoor group, 14 by 16 by 13 yd. Vanilla had
Stalvan Mistmantle (35, ghost, display 1562) inside, now deleted, and 10 Fetid Corpses in the yard.

**Story beat.** Rung 5, first half: "Anchors can go wrong." The cult buried its untuned test
Anchor under the manor twelve years ago. It made Duskwood's night and woke the dead, and the
household went silent that night. The candles never went out, because the air near an Anchor
doesn't move. Assumed: the bible's "first look at what the mist does to people" means what an
Anchor does to people, silence and stillness. The manor shows no grey, since the test Anchor made
night, not mist.

**Layout.** Assumed: the footprint grows to an open-air box over the yard and outbuildings with a
`[wall]`, the Forest's Edge form (M1). The house is the last room.

1. The yard (around (-10346, -1250)). The farmhands lie where they fell, 32-33 normal, asleep in
   camps of 3. The Stillness meter starts (M2). It gains stacks on the Run clock and on each
   death, and Cover spots at draughty places (a broken window, the well) drain it.
2. The steward, a 34 elite at the kitchen door. Hush, a 4 s `silence` on a random character every
   15 s. At 50% (`on_health`) he calls two servants.
3. The ground floor. Servants stand at their tasks, 33-34, asleep with a negative
   `wake_radius_mod`, so a careful player slips past. The lady of the house, a 35 elite on the
   stair, casts `fear` every 25 s and a shadow `periodic_damage`.
4. The master, 36 elite, on the Boss Bar. Assumed: Stalvan Mistmantle (M3). He stands over the
   cellar hatch where the test Anchor hums. With `hunts_at_meter_max`, a full meter sends him to
   the players, waking every sleeper on his way. In his fight: "The candles flare", a `gaze` with
   a warning every 20 s, with the chimney stack and the stair post as `[[cover]]`; `fear` every
   30 s; a `nova` below 30% health.

**New parts.** None if the meter, sleepers and Cover are reused. Assumed: the Noise Bar shows the
meter's own `name` and `end_icon`, so "Stillness" needs no client work.

**Quests in.** The Duskwood arc picks the givers. Assumed slots: a Darkshire hook about lights in
a house nobody lives in, the master's kill, and an item quest for the cult surveyor's site record,
which says "the stone" and never "Anchor" (S1). The record points to Raven Hill and to Ilgalar.

**Loot.** 20 blues, 905140 to 905159, item level about 36. The steward rolls one at 50%, the
master at 100%. The site record is a `drop`. No failure-note page here, since the notebooks are
torn later, at Ilgalar.

### Tower of Ilgalar (34-38, Redridge)

**Where.** Morganth's tower on its hill in eastern Redridge, (-9280, -3300). The Portal is the
south door at (-9282.2, -3327.6, 115.7). Three stacked boxes: the ground floor at z about 116, a
middle storey, and the top floor at z about 150, where vanilla Morganth (display 3383) stood at
(-9291.5, -3309.9, 150.1). Shadowhide gnolls and Servants of Ilgalar (voidwalkers, display 3916)
hold the hill. Assumed: a ramp joins the floors, from where the vanilla Servants stood at z 121 to
150.

**Story beat.** Rung 4: "It's building machines, and the mist comes from them." The tower is a
working Anchor, with the forged core from the Elwynn Tribute crate at its heart. Morganth dies
and the Anchor keeps running. Sareth's letters lie among his notebooks, which are torn three
ways: the cult escapes with the tuning method, Sareth's agents take the casing specs, and the
players keep the failure notes and the unsent letter to Toren. A Dark Iron forge stamp on the
casing is the clue for the Steppes. Blackrock orcs beside the cult hint at the dragons, and stay a
hint.

**Layout.**

1. The ground floor. Tribute crates under the hammer and black sun. Twilight Tuners (35, casters)
   each paired with a Servant of Ilgalar (35) that takes 50% less damage while its Tuner lives.
   Assumed: two Servant rows, one `until_dead` of the Tuner and its twin `after_dead`.
2. The ramp's feed valves. Three levers, each guarded by a Tuner and a Blackrock Porter (36).
   Each pull stops one of Morganth's calls (`every` with `until_used`), Varl Cutter's form. All
   optional: skip them and the boss is harder.
3. The overseer, a 37 elite Blackrock orc on the middle storey, guarding the core's crates.
   `stance` (shield wall every 30 s), a 200% `hit`, and Sunder (`armor_debuff`). He never says who
   sent him.
4. Morganth, 38 elite, on the Boss Bar, beside the Anchor. Tuning Pulse: a `gaze` with a warning
   every 25 s that hits everyone not behind the Anchor's pylons (`[[cover]]`). A shadow
   `periodic_damage`. Void Barrier, an `absorb` every 40 s. Every 20 s a Servant comes up for each
   valve still open. At his death two figures appear by the far window (I2), a Twilight tuner and
   a pale Scourge courier, say a line each and leave. His corpse holds the failure notes, the
   letter and Sareth's letters.

**New parts.** An Appears key `for_s`: a creature stays in the Run that many seconds after it
appears. With `never_fights`, nobody can pull it. The Cathedral can reuse it for Prestor's agents.

**Quests in.** From Elwynn, 95101 to 95103 (Solomon, Theocritus's challenge, the Tribute crate).
Assumed slots in Redridge: Solomon's quest to clear the hill, Theocritus's kill quest on Morganth,
and the failure notes to Toren in Lakeshire's inn, which starts the page race through the 40s.
The 48-50 return to break the Anchor is later work (I1).

**Loot.** 20 blues, 905160 to 905179, item level about 38. The overseer rolls at 50%, Morganth at
100%. The notes and letters are quest items.

### Raven Hill Deeps (40-44, Duskwood)

**Where.** The crypt under Raven Hill Cemetery in western Duskwood. The Portal is on the mausoleum
floor beside the stairwell, (-10418.6, 410.8, 47.8). The crypt is large: 11 groups running about
260 yd east and south under the cemetery, down to z about 0. Vanilla had skeletons inside (deleted)
and Commander Felstrom (32, rare, display 7848) near the foot of the stairs at (-10399, 347, 25).
Mor'Ladim (50 after the stretch) walks the hill outside and stays open world.

Assumed room order, since the WMO's portals were not read: the stairs (group 3), the upper vaults
(groups 0, 5, 11, 12, around (-10380, 400, 28)), the hall (group 1, (-10324, 448, 19)), vault01
(group 2, (-10265, 385, 18)), vault06 (group 4, (-10207, 325, 10)), XBox (group 9, (-10229, 259,
10)), exterior02 (group 8, (-10239, 170, 7)) and the deepest room, middle (group 7, (-10337, 143,
16)).

**Story beat.** Rung 5, second half: "the dead aren't silenced." Sareth's cultists dig the seam
for his own Anchor. The dig breaks into an old pocket of grey where Scourge dead stand frozen,
preserved and not gone. Mourncrown's shadow shows up close. Sareth is glimpsed only (R2). Nobody
says the Lich King is frozen.

**Layout.**

1. The upper vaults. Skeletal Warders (40) and cultist acolytes (41, the Necropolis Acolyte look,
   display 16160), who raise a skeleton from the wall niches every 15 s (`every`). The dead speak
   here: aggro lines on the undead, unlike the quiet wing.
2. Commander Felstrom, 42 elite, the vanilla name, a dead Stormwind officer. `fear` every 25 s and
   a `stance` below 50% (`below_health_pct`).
3. The dig face (hall and vault01). Ghoul diggers in camps, ore carts, and the dig overseer, a 43
   elite necromancer. Feast, a `heal` that needs a corpse within 10 yd (`corpse_within`). Blight,
   a `ground_patch` on a random character. Two ghouls every 20 s until the shoring lever drops the
   side tunnel.
4. The seam vaults (vault06, XBox). Survey marks in a crowned seal, not the cult's mark. Paired
   bone constructs, 43 elite, with `root` (radius 8) and `armor_debuff`.
5. The grey pocket (exterior02 and middle). A placed mist wall closes the far end, with frozen
   ghouls half inside it. The deacon (placeholder), 44 elite, on the Boss Bar. Draw from the Grey:
   at 75, 50 and 25% of his health a group of the frozen dead steps out and wakes (S4). Shadow of
   Mourncrown: a `gaze` with a warning every 30 s through a roof shaft, with the crypt pillars as
   `[[cover]]`. Bone Armor, an `absorb` every 40 s. He carries orders under the crowned seal.

**New parts.** Frozen until a boss's health mark: the tram's passengers as a dungeon creature key,
for example `frozen_until = { guid, pct }` (S4). The mist wall is a data row. Assumed: the
Necropolis GameObject (display 6678, vmangos entry 181154) loads in the 1.12.1 client (R1).

**Quests in.** Assumed slots: a Night Watch hook about grave robbing at Raven Hill, the deacon's
kill, the crowned orders to someone who can read a Scourge seal, and a repeatable on a Scourge
token like the Stockade's Twilight Coin. One failure-note page can drop here for Toren.

**Loot.** 20 blues, 905180 to 905199, item level about 44. Felstrom and the overseer roll at 50%,
the deacon at 100%.

### Stonewatch Keep (45-48, Redridge)

**Where.** The ruined keep in eastern Redridge, (-9388, -3065). The Portal is at the gate,
(-9386.3, -3046.8, 139.2). 15 small rooms: entry, staging areas, right halls, rear storage, war
room, the left towers and the top storey at z about 158 to 167. Vanilla had Gath'Ilzogg (orc
elite) and Singe (elite dragonkin, display 497) together up top at (-9383, -3082, 158), now
deleted. Blackrock (41 to 44) hold the yard outside.

**Story beat.** Rung 8: "The dragons are in it." The Blackrock hold the keep with black dragonkin
among them, which proves the orcs and the cult answer to the same master. Nothing here reveals
rung 7 (the Cathedral's) or Prestor.

**Layout.**

1. The gate hall and staging areas (z about 145). Blackrock Sentries and Shadowcasters (45) in
   camps, so damage to one wakes the hall. A Twilight paymaster stands among them.
2. The right halls and rear storage. Tribute crates under the cult's mark, and Blackrock
   Gladiators (46 elite) in pairs. The quartermaster pair: an orc with `stance`, and a paymaster
   who heals him (a 3 s `heal` that a Kick stops).
3. The war room (z about 157). A black dragonspawn envoy, 47 elite (stock Black Dragonspawn,
   display 143). Flame Breath, a `ground_patch` on a random character. Roar, a `fear` every 30 s.
   `armor_debuff`. Black Broodlings (display 397) come every 20 s until the brood cage lever is
   pulled. He carries orders for both the warlord and the paymaster, in one hand that is neither
   orc nor cult (St2).
4. The roof. Gath'Ilzogg, 48 elite, on the Boss Bar: `stance`, a 200% `hit` and `charge`. Then
   Singe, 48 elite, on the Boss Bar (St1): Flame Breath, Wing Buffet (a `nova` of radius 10) and
   `fear`.

**New parts.** None if Singe comes after Gath'Ilzogg dies. A partner link ("when guid X dies, this
one enrages") is new code if they fight together.

**Quests in.** Assumed slots: Solomon or the Lakeshire guard captain on Blackrock raids, the
envoy's orders, which go to someone outside the court (Dughan's channel, as in the Stockade), and
the roof kill. One failure-note page can drop here.

**Loot.** 20 blues, 905200 to 905219, item level about 48. The envoy rolls at 50%, each roof boss
at 100%.

### Later work, for scale only

- Altar of Storms (50-52) has no shell. It is open-air in the Forest's Edge form: a box over the
  bowl round `altarofstorms.wmo` at (-7614, -756, 182), a `[wall]`, and the Portal in the one
  approach near (-7705, -715). The Burning Steppes has no spawn file and stays closed until
  Ilgalar's Anchor breaks.
- Blackrock Stronghold (52-55, id 14) and Dreadmaul Rock (55-58, id 15) are shells that need the
  Steppes populated first.
- The Cathedral of Light (48-50, id 12) is a shell. Cedric's confession and death need a scripted
  scene, which `for_s` helps with. The 48-50 return to Ilgalar belongs with it.
- Stormwind Keep (60, id 13) is a shell with Prestor and Anduin (#559). The reveal is 60 content.

## 3. Open decisions, most important first

### Shared

S1. A player can finish Mistmantle (32-36) before Ilgalar (34-38) teaches what an Anchor is. How
do we keep the reveal ladder in order?
- (a) Mistmantle shows a buried humming stone and never says "Anchor". (b) Gate Mistmantle's last
  quest behind Ilgalar's. (c) Swap the bands.
- Recommend (a): both stay open, and the writer's rules allow a detail without its name.

S2. The open world round Ilgalar (44-48) and Mistmantle (43-44) is far above the dungeons. What
do we do?
- (a) A Level Band area over each approach. (b) Fold the approach into the footprint as dungeon
  trash. (c) Leave it; it suits the 48-50 return. (d) A Level Stretch per subzone.
- Recommend (b) for Mistmantle, whose yard joins the dungeon anyway (M1), and (a) for Ilgalar.

S3. How does loot work past level 30?
- (a) 20 blues per dungeon, the Stockade form, with trash on the 27-30 Loot Band. (b) 20 blues and
  20 greens, the Tram form. (c) (a) plus a small spec first that extends the Loot Bands to 50.
- Recommend (c): trash in a 45-48 keep rolling level-30 greens will feel wrong.

S4. Raven Hill's frozen dead reuse the tram's passengers. Do we generalise them now?
- (a) A dungeon creature key, with the tram moved onto it. (b) Copy the logic for Raven Hill.
  (c) Use plain `on_health` adds.
- Recommend (a): ADR 0033 named the second user as the time, and Raven Hill is it.

S5. Do we want two build agents filling dungeons at once?
- (a) No: monster and loot tickets go one at a time, realm code runs beside content. (b) Yes: a
  realm ticket first lets each dungeon file carry its own `[[template]]` rows (the Wailing
  Caverns form) and each zone its own quests file.
- Recommend (a), unless Davis plans nights of parallel runs. (b) costs a ticket of plumbing.

S6. How long is a run?
- (a) About 15 minutes, 3 encounters. (b) About 20 to 25 minutes, 4 to 5 encounters and about 25
  creatures, like a Stockade wing. (c) Longer for the 40s pair.
- Recommend (b): it matches what was just built, and Head Count scaling covers groups.

### Tower of Ilgalar

I1. How does the 48-50 return to break the Anchor work?
- (a) Quest-gated Run content in the same tower (a new Appears condition). (b) A second small
  dungeon on the top floor. (c) An open-world Encounter at the tower's foot. (d) Leave it to the
  Cathedral and Steppes spec; this one only leaves the Anchor standing and the hook in the notes.
- Recommend (d): the return gets designed with what it opens.

I2. How are the notebooks torn three ways in the fight?
- (a) At Morganth's death a tuner and a Scourge courier appear, speak and leave (new `for_s`).
  (b) Adds at 50% that flee (`flee_at_pct`) and drop nothing if killed. (c) Text only, in the item
  and Toren's turn-in.
- Recommend (a): it shows the theft, needs little code, and a fast group can't break the story.

I3. Which mechanics carry Morganth's fight?
- (a) The gaze and the Anchor's pylons. (b) The feed valves that stop his calls. (c) Both.
- Recommend (c): the valves give a choice before the boss, and the gaze makes the Anchor felt.

### Mistmantle Manor

M1. The house is one 14 by 16 yd room. Where does the rest of the dungeon go?
- (a) An open-air box over the yard and outbuildings with a `[wall]`, the house last. (b) The
  house alone, two encounters. (c) An Azora-style stop to a cellar elsewhere. (d) Move to a bigger
  building.
- Recommend (a): it keeps the bible's place. Assumed: the yard is walkable; a survey confirms it.

M2. Does Mistmantle reuse Forest's Edge's meter, sleepers and Cover as a quiet run?
- (a) Yes, as Stillness, with a master who walks when it fills. (b) No, a plain fight dungeon.
  (c) A meter with no hunter.
- Recommend (a), unless Davis's first play of the unplayed Forest's Edge rework says no.

M3. Who is the master, and is the test Anchor seen?
- (a) Stalvan Mistmantle, vanilla's name and spot, with cracked ore humming under the hatch.
  (b) A new name, the Anchor unseen. (c) Stalvan, the Anchor unseen.
- Recommend (a): the name is already in the world, and a seen but unnamed stone fits S1.

### Raven Hill Deeps

R1. How do players see Mourncrown "up close"?
- (a) The Shadow of Mourncrown gaze in the last room. (b) The Necropolis model over the cemetery.
  (c) Both. (d) Text only.
- Recommend (c) if display 6678 loads in the client, otherwise (a).

R2. How much of Sareth shows?
- (a) A crowned seal and "the Master" in orders, never a name. (b) His name in a letter, no goal.
  (c) Nothing.
- Recommend (a): a glimpse is allowed, and a name raises his goal before rung 10.

### Stonewatch Keep

St1. How do Gath'Ilzogg and Singe fight on the roof?
- (a) Both at once, and the survivor enrages (a new partner link). (b) Gath'Ilzogg first, then
  Singe lands (`after_dead`, no new code). (c) Singe alone.
- Recommend (b): two clear fights, no new code, and the dragon's arrival is the reveal.

St2. What dragon proof does the player carry out?
- (a) The envoy's orders to both the warlord and the paymaster. (b) A scale from Singe, nothing
  written. (c) Both.
- Recommend (a): it proves a shared master without naming Nefarian or Prestor.

## 4. Ticket sketch

Shared files: `content/mobs.toml`, `content/creature_spells.toml` (and the `client_dbc_patch.json`
that `build_spells.py` writes from it), `data/twilights-eve/gear.csv` (and the generated
`content/gear.toml`), the quests files, `content/zones.toml`, the `lore_bible_dungeons` test in
`src/phased.rs`, `docs/art-assets.md` and `mist_walls.json`. Two tickets that write one of these
can't run at once in the one checkout. Lane A and Lane B mark tickets with no file in common.

Shared:
- X1, realm (Lane A). The frozen-until key, with the tram moved onto it (`src/phased.rs`,
  `src/tram.rs`, `src/world*`, `tests/`).
- X2, realm (Lane A, after X1, since both edit `phased.rs`). The Appears key `for_s`, with a test.
- X3, content (Lane B, beside X1). Levels 40-44 and 45-48 in the two files; fix Azora's shadow
  stop.
- X4, content (Lane B). Loot Bands past 30, if S3 is (c) (`content/loot_bands.toml`).
- X5, realm (Lane A), only if S5 is (b). Templates in dungeon files, quests files per zone.

Mistmantle Manor:
- M-1, content (Lane B, beside X1 or X2). Footprint over the yard, `[wall]`, Portal, and
  `removed_guids` for the Fetid Corpses (`mistmantle_manor.toml`, `zones.toml`).
- M-2, content, after M-1. Monsters, meter, Cover, steward, lady, master.
- M-3, data, after M-2: 20 blues and rolls. M-4, quests, after M-3 and the Duskwood arc grill.
- Client: none. Assumed: the Noise Bar shows the meter's name.

Tower of Ilgalar:
- I-1, content. Monsters, valves, pylons, overseer, Morganth. The two figures wait for X2, or go
  in a later I-1b.
- I-2, content (Lane B). A Level Band over the hill, if S2 says so (`level_overrides.toml`).
- I-3, data, after I-1: 20 blues. I-4, quests 95101 to 95103 and the Redridge slots, after I-3
  and the Redridge arc grill.
- Client: none.

Raven Hill Deeps:
- R-1, content (Lane B). Survey the WMO portals for the room order, and add the mist wall row
  (`raven_hill_deeps.toml`, `mist_walls.json`).
- R-2, content, after X1 and R-1. Monsters, Felstrom, overseer, shoring lever, constructs, deacon.
- R-3, content (Lane B), if R1 says so. The Necropolis object over the cemetery, after checking
  display 6678 loads. Assumed: it is an open-world row in `zones.toml`'s Duskwood additions.
- R-4, data: 20 blues. R-5, quests. Both after R-2.
- Client: none; the client already draws `mist_walls.json`.

Stonewatch Keep:
- S-1, content, after X3. Monsters, camps, quartermaster pair, envoy and brood lever, roof. If St1
  is (a), a partner-link realm ticket (Lane A) comes first.
- S-2, data: 20 blues. S-3, quests. Both after S-1.
- Client: none.

A two-lane order:
1. Lane A takes X1 while Lane B takes X3, M-1 and R-1.
2. Lane A takes X2 while Lane B takes X4 and I-2.
3. The monster tickets (M-2, I-1, R-2, S-1) all write `mobs.toml` and `creature_spells.toml`, so
   they go one at a time unless X5 lands. Meanwhile, Lane B writes the loot rows for whichever
   dungeon finished before.
4. Quests wait for the Redridge and Duskwood arc grill.

## Grill answers (Davis, 2026-11-08)

- S1 Reveal order: (a) Mistmantle shows a buried humming stone; nobody says "Anchor".
- S2 Out-of-band approaches (Claude, follows the quest arc's decision 1): sub-zone Level Bands over each approach; Mistmantle's yard joins the footprint as trash (M1).
- S3 Loot past 30: dungeon sets are 20 blues per dungeon, the Stockade form (Claude's reading; the gear 31-60 grill may amend). Trash rolls the new 31+ Loot Bands once the gear spec lands them. Each dungeon set must include four Chest rows, one per armour type, because the quest arc's dungeon-ending quests pay them.
- S4 (Claude): (a) generalise the tram's frozen passengers into a dungeon creature key, tram moved onto it.
- S5 (Claude): (b) a plumbing ticket first so each dungeon file carries its own template rows, so two builders can fill dungeons at once.
- S6 Run length: ABOUT 15 MINUTES, 3 encounters (not the 4-5 drafted). Trim each layout.
- I1 Ilgalar's 48-50 return: owned by the Steppes opening spec (the break is a quest-started Encounter inside the Ilgalar Run). This spec leaves the Anchor standing, plus room for that Encounter.
- I2 Notebooks: ALREADY SPLIT before the fight. Morganth carries only the failure notes; the theft happened earlier (a clue in the tower can show it). No appearing thieves, no for_s key.
- I3 Morganth: (c) feed valves before the boss, then the gaze and the Anchor's pylons in the fight.
- M1 Mistmantle: (a) open-air box over the yard and outbuildings with a wall, the house last.
- M2: (b) PLAIN FIGHTS, no meter, no sleepers.
- M3: (a) Stalvan Mistmantle; cracked humming ore under a hatch, never named.
- R1 Mourncrown: Davis: Mourncrown hangs over TWILIGHT GROVE, replacing the whole Emerald Dream portal area (not over the cemetery). Raven Hill's last room has the Shadow of Mourncrown gaze. Separately, one ticket places the Necropolis model over Twilight Grove, gated on a WOW_CAPTURE screenshot that shows it looks right; if it looks wrong, it does not ship. (Lore bible update for Davis: say where Mourncrown hangs.)
- R2 Sareth: (a) a crowned seal and "the Master", never his name.
- St1 Roof: (a) BOTH AT ONCE: Gath'Ilzogg and Singe fight together, and the survivor enrages (a new partner-death link).
- St2 Proof: (b) a scale from Singe, nothing written.
