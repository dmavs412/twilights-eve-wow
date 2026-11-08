Title: The Blackrock Pass Watch camp: its captain, board, creatures, bounties and Writs (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 1. Blocked by 05-anchor-break (same `content/quests_blackrock_pass.toml`,
`content/mobs.toml`, `docs/art-assets.md` and `tests/steppes_opening.rs`; it also needs tickets 03
and 04's keys, `feeds` and `content/hub.toml` section) and RD:21-bands-45-50 (the last arc ticket
that edits Redridge's part of `content/zones.toml`).

## What

When the Anchor breaks, the mist over Blackrock Pass thins and things walk out of it. A Lakeshire
Watch camp appears on the Redridge side of the pass, with a captain, a Writ board and the creatures
it holds back. Its three bounties and its pool of three Writs are the only quests that feed the
Tally (decision 1). Everything here carries `after_milestone = "ilgalar_anchor_broken"` (ticket 03)
and stays after the Steppes open; once `steppes_open` is reached the quests still pay but feed
nothing (ticket 04's rule).

The camp stands at (-8735, -2590), on the road below the border at x -8667. The ground there is
open terrain, z about 132.6 (`wmo_floor.py` finds no building). Every unit below gets Z 0.0 and
then `python3.15 tools/twilights-eve/ground_check.py --fix`.

## The captain (`content/hub.toml`)

In the section ticket 04 added (`# ── The Burning Steppes Tally's readouts (#PARENT) ──`), after
its two rows, under a comment naming this ticket:

- guid 9300091, entry 190090, name "Captain Sela Marrow" (a placeholder), subname "Blackrock Pass
  Watch", level 52, `display_id = 3343`, `faction_template = 35`, `role = "giver"`,
  `npc_flags = 0x2`, `map = 0`, position (-8738.0, -2592.0, 0.0), `faces = [-8667.0, -2560.0]`,
  `after_milestone = "ilgalar_anchor_broken"`.
- greeting: "The grey's thinning, and whatever lived behind it is walking down this road. Every one you put down is a stretch of the pass we get back. The Watch pays, and Stormwind keeps count."

## The board (`content/writs.toml`)

A `[[gameobject]]` row after the Stormwind board, in its shape: guid 794001, entry 980601,
display 3053 (the stock Alliance wanted board, the Writ Board's placeholder), map 0, position
(-8730.0, -2585.0, 0.0), orientation 0.5, respawn 60.0,
`after_milestone = "ilgalar_anchor_broken"`. In the file's header, add the pool: "Burning Steppes
46, on the Blackrock Pass board (980601)". A pool is one board and one zone (`src/writs.rs`,
`pools`), so this pool never mixes with the Stormwind board's.

## Eight templates (`content/mobs.toml`)

After ticket 05's three (97511-97513), each with a comment naming this ticket, in the Twilight
Cultist's form (entry 96024). Elite templates carry no `rolls`.

| Entry | Name | Type | Display | Faction | Class | Levels | Rank | Extra |
|---|---|---|---|---|---|---|---|---|
| 97501 | Blackrock Pass Raider | 7 | 6045 | 40 | warrior | 48-49 | 0 | |
| 97502 | Blackrock Flamecaller | 7 | 6046 | 40 | mage | 49-50 | 0 | `spells = ["fire_bolt_creature"]` |
| 97503 | Pass Flamekin | 4 | 10815 | 90 | warrior | 48-50 | 0 | |
| 97504 | Ember Hound | 1 | 9371 | 38 | warrior | 48-49 | 0 | `spells = ["bite_wolf"]` |
| 97505 | Pass Broodling | 2 | 397 | 103 | warrior | 49-50 | 0 | |
| 97506 | Gorlak Ashfist | 7 | 6047 | 40 | warrior | 50 | 1 | `health_multiplier = 1.6` |
| 97507 | Kindlemaw | 4 | 10815 | 90 | warrior | 50 | 1 | `health_multiplier = 1.6`, `spells = ["flame_patch_creature"]` |
| 97508 | Cindertooth | 2 | 143 | 103 | warrior | 51 | 1 | `health_multiplier = 1.6` |

The displays and factions are the vanilla Blackrock Soldier (7025), Sorcerer (7026) and Slayer
(7027), Flamekin Spitter (9776), Ember Worg (9690), Black Broodling (7047) and Black Dragonspawn
(7040).

## The creatures (`content/zones.toml`)

Under the `redridge` zone, after its other `[[zone.added]]` rows, a comment naming this ticket and
30 rows, each with `after_milestone = "ilgalar_anchor_broken"`, Z 0.0 and orientation 0.0. All
stand on Redridge land between the camp and the mist (every point has x below -8670), at least 45
yards from the captain.

| Guids | Entry | Levels | Points (x, y) |
|---|---|---|---|
| 964001-964007 | 97501 | 48-49 | (-8690, -2560), (-8694, -2566), (-8690, -2520), (-8696, -2526), (-8760, -2660), (-8766, -2654), (-8705, -2540) |
| 964008-964011 | 97502 | 49-50 | (-8686, -2566), (-8686, -2516), (-8770, -2662), (-8700, -2545) |
| 964012-964016 | 97503 | 48-50 | (-8685, -2610), (-8681, -2616), (-8710, -2685), (-8690, -2600), (-8675, -2625) |
| 964017-964021 | 97504 | 48-49 | (-8689, -2604), (-8700, -2650), (-8706, -2646), (-8762, -2666), (-8680, -2640) |
| 964022-964027 | 97505 | 49-50 | (-8700, -2655), (-8714, -2690), (-8706, -2692), (-8720, -2680), (-8690, -2670), (-8696, -2676) |
| 964028 | 97506 | 50 | (-8680, -2585) |
| 964029 | 97507 | 50 | (-8695, -2635) |
| 964030 | 97508 | 51 | (-8705, -2670) |

That is 7 + 4 + 5 + 5 + 6 + 3 = 30 rows. Add the Great Goretusk at the camp, vanilla guid 10120
at (-8726, -2577), to Redridge's `removed_guids` under a comment naming this ticket.

## The bounties (`content/quests_blackrock_pass.toml`)

Under `# ── The Blackrock Pass Watch camp (#PARENT) ──`. Each `kind = "bounty"`, giver and ender
Captain Sela Marrow (190090), `zone_or_sort = 44`, money 30 x level. Family lines use the
`{ family, entries, count }` form (#572).

**99511 Blackrock at the Pass.** Level 48, money 1440.
`kill = [{ family = "Blackrock slain", entries = [97501, 97502], count = 12 }]`.
Objectives: "Kill 12 Blackrock raiders and flamecallers coming down Blackrock Pass, then return to Captain Sela Marrow at the Watch camp."
Details: "The orcs were the first through. They must have sat behind the grey for years, waiting for it to lift. Twelve of them, and the road stays ours another night."
Progress: "Still coming, are they?"
Completion: "That's twelve fewer. Stormwind counts every one, and so do I."

**99512 What Burns.** Level 49, money 1470.
`kill = [{ family = "Flamekin and ember hounds slain", entries = [97503, 97504], count = 12 }]`.
Objectives: "Kill 12 flamekin and ember hounds in Blackrock Pass, then return to Captain Sela Marrow."
Details: "There's fire walking in the pass, and dogs made of embers running with it. Whatever the mountain's doing beyond the grey, it's spilling over. Put twelve of them out."
Progress: "Mind your boots. The ground stays hot for an hour."
Completion: "Good. The grass on this side doesn't burn as easily as theirs, and I'd like to keep it that way."

**99513 Wings Out of the Grey.** Level 50, money 1500. `kill = [[97505, 10]]`.
Objectives: "Kill 10 Pass Broodlings in Blackrock Pass, then return to Captain Sela Marrow."
Details: "Broodlings. Black ones, the size of a hound, coming down off the ridge as the mist thins. Something bigger is laying them somewhere up there. Ten of them, before they learn to fly properly."
Progress: "Any bigger ones?"
Completion: "Ten. I'll write it up. The city won't want to read it, but I'll write it up."

## The Writs (`content/writs.toml`)

Under `# ── The Blackrock Pass Writs (#PARENT) ──` after the seven Writs. Each `kind = "writ"`,
`giver_object = 980601`, `ender = 190090`, `zone_or_sort = 46`, issued by the City Watch, money
140 x levels x level copper (the Writs spec's rule).

| id | level | levels | money | Family lines (entries) | Named (entry) |
|---|---|---|---|---|---|
| 99521 | 48 | 1 | 6720 | "Blackrock slain" 50 (97501, 97502); "Flamekin and ember hounds slain" 50 (97503, 97504) | Gorlak Ashfist (97506) |
| 99522 | 49 | 1.5 | 10290 | "Flamekin and ember hounds slain" 60 (97503, 97504); "Pass Broodlings slain" 40 (97505) | Kindlemaw (97507) |
| 99523 | 50 | 2 | 14000 | "Blackrock slain" 50 (97501, 97502); "Pass Broodlings slain" 50 (97505) | Cindertooth (97508), Gorlak Ashfist (97506) |

**99521 Writ of the Watch: Blackrock Pass.**
Objectives: "Slay 50 Blackrock and 50 flamekin and ember hounds in Blackrock Pass, and kill Gorlak Ashfist. Then return to Captain Sela Marrow at the Watch camp."
Details: "A writ of the City Watch, posted at the pass. Fifty Blackrock, fifty of the burning things, and the name at the bottom. Gorlak Ashfist leads the raiders down the road at dusk, and he's the reason the first camp we built here isn't standing. Bring him down."
Progress: "The writ's still open."
Completion: "Ashfist's dead? Then the road's quiet for a night. Here's the Watch's money, and the city's thanks, for what that's worth this far east."

**99522 Writ of the Watch: The Burning Rocks.**
Objectives: "Slay 60 flamekin and ember hounds and 40 Pass Broodlings in Blackrock Pass, and kill Kindlemaw. Then return to Captain Sela Marrow."
Details: "There's one flamekin bigger than the rest. The men call it Kindlemaw. It burned our first watchtower to the stones and walked back up the pass. The writ wants it put out, and everything that burns or hatches around it."
Progress: "Still burning out there?"
Completion: "Kindlemaw's gone out. The writ pays, and I'll buy the first round in Lakeshire myself."

**99523 Writ of the Watch: The Ridge.**
Objectives: "Slay 50 Blackrock and 50 Pass Broodlings in Blackrock Pass, and kill Cindertooth and Gorlak Ashfist. Then return to Captain Sela Marrow."
Details: "The broodlings come from somewhere, and the scouts say it's the ridge where the grey is thickest. A black dragonspawn they call Cindertooth keeps the nest there, and Ashfist's raiders guard it as if it were theirs. The writ wants the ridge clear: Cindertooth, the orcs, and every hatchling."
Progress: "Did you reach the ridge?"
Completion: "Cindertooth and Ashfist both. I didn't think we'd hold this camp a month, and look at it now. The Watch pays in full."

## The Tally's feeds (`content/milestones.toml`)

`steppes_open`'s `feeds` becomes:

```toml
feeds = [
  { quest = 99511, points = 15 },
  { quest = 99512, points = 15 },
  { quest = 99513, points = 15 },
  { quest = 99521, points = 120 },
  { quest = 99522, points = 120 },
  { quest = 99523, points = 120 },
]
```

## Art

In ticket 05's `docs/art-assets.md` section, one row: "Blackrock Pass Watch board: a GameObject
display for the camp's Writ board; a rough plank board on a post, Watch colours. Until made, the
stock Alliance wanted board (display 3053), as the Writ Board."

## Failing tests first

In `crates/twilight-realm/tests/steppes_opening.rs` (ticket 05's file):

- Before `reach_milestone("ilgalar_anchor_broken")`, a player at (-8745.0, -2600.0) gets no create
  for the captain, the board or any of the 30 creatures. After it, within a tick, they get the
  captain's and the board's, and the creatures stream in.
- The captain offers 99511, 99512 and 99513. The camp board offers exactly one of 99521 to 99523 in
  a window, and the Stormwind board (980201) offers none of them.
- After the break, handing in 99511 adds 15 to `steppes_open`'s count and a camp Writ adds 120.
  After `steppes_open` is reached, a hand-in pays as before and adds nothing.

In `crates/twilight-realm/tests/content_quests.rs`, beside the Writs' tests:

- The three camp Writs load with the level, `levels` and money of the table, in pool 46 of board
  980601. The three bounties load with their kill lines and money.
- Every creature entry in the six quests' kill lines has a `[[zone.added]]` spawn under `redridge`
  within 200 yards of the camp, and every one of the 30 rows stands on open land
  (`closed_for_good` false and not gated).
- The eight templates have the levels and ranks of the table.

## Acceptance

- The tests pass. `cargo test -q -p twilight-realm` passes. `cargo check -q -p twilight-realm` is
  clean.
- `python3.15 tools/twilights-eve/ground_check.py` and `python3.15 tools/twilights-eve/icon_check.py`
  pass.

Assumptions allowed: a creature point that `ground_check.py` flags (a cliff or water) moves to the
nearest open point within 15 yards that it passes; the camp board's Z if the terrain there is a
slope. Record them as `Assumed:` lines.
