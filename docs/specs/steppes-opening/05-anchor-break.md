Title: The break: an Encounter at the Ilgalar Anchor, and Toren's two quests (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 7, 8 and 11. Blocked by 04-tally-and-readouts (same `src/lib.rs`, and
it needs ticket 03's keys), RD:12-redridge-35-40 (Toren in Lakeshire, Hub NPC entry 190083, gives
both quests) and RD:03-quest-files (it changes `shipped()` in `src/quests/content.rs` and
`src/build_id.rs`, which this ticket also changes).

## What

A character at 48 takes a stand-in "translation complete" quest from Toren at Lakeshire's inn,
then the break quest. In the Tower of Ilgalar's Run, holding the quest at the Anchor on the top
floor starts an Encounter: four waves of the cult, 48 to 50, scaled by the Head Count. When the
last wave is dead, the realm reaches the Milestone `ilgalar_anchor_broken`: the whole Anchor gives
way to a broken one in every Run, the Tally starts (ticket 04), and the camp appears (ticket 06).
The land does not open.

## Encounters inside a Phased Dungeon

Encounters (`src/encounters.rs`, `content/encounters.toml`) run only in the open world today:
`tick_encounters` (`src/lib.rs` line 1662) hard-codes the open-world place, and `src/mobs.rs` builds
each wave creature's spawn row with `phase: Phase::OpenWorld` (line 1081). Add two optional keys to
an `[[encounter]]`:

- `dungeon`: a Phased Dungeon id. The encounter runs in that dungeon's Phase: its players are the
  characters whose shard place is that dungeon's Phase, its creatures go into that shard's mobs
  (the Run's), and its spawn rows carry `Phase::Dungeon(id)` on the dungeon's map. With no Run
  (nobody in the dungeon) it is idle. When the Run resets, a run under way is dropped. The Head
  Count (ADR 0029) scales its creatures as it scales any creature that spawns mid-Run, through
  `Shard::scale_to_head_count` (`src/shard.rs` line 310), with no code of its own.
- `records`: a Milestone key. When the encounter is won (its last wave dead, the branch at
  `src/encounters.rs` line 290) and the Milestone is not reached, the realm calls
  `reach_milestone`. Return the won encounter's key from `tick` (or an event the realm reads) so
  the Milestone code stays out of `encounters.rs`. A later win does nothing more.

The load panics on a `dungeon` that is no Phased Dungeon, a `records` key the catalogue lacks, and
an anchor outside that dungeon's footprint. Document both keys in `content/encounters.toml`'s
header. Every existing encounter keeps working: `tests/encounters.rs` must pass unchanged.

## The Anchor (`content/ilgalar.toml`)

Two GameObjects on the top floor, under a comment naming this ticket. Display 1667
(`FloatingPurpleCrystal01.mdx`) and 1687 (`FloatingPurpleCrystalBroken02.mdx`) are placeholders.

```toml
# The Anchor (#PARENT): whole until the break, broken after, in every Run.
[[gameobject]]
guid = 794101
display = 1667
position = [-9288.0, -3308.0, 150.09]
orientation = 1.47
sides = "inside"
until_milestone = "ilgalar_anchor_broken"

[[gameobject]]
guid = 794102
display = 1687
position = [-9288.0, -3308.0, 150.09]
orientation = 1.47
sides = "inside"
after_milestone = "ilgalar_anchor_broken"
```

z 150.09 is the floor `wmo_floor.py -9288 -3308` lists for group 2, the top floor (vanilla
Morganth stood at (-9291.5, -3309.9, 150.1)). If the dungeons spec has already put an Anchor
GameObject in this file, give that row `until_milestone` instead of adding 794101, keep 794102, and
record it.

## The waves (`content/encounters.toml` and `content/mobs.toml`)

Three templates in `content/mobs.toml` after the Twilight Cultist rows (entry 96024 onward), each
with a comment naming this ticket, in 96024's form:

| Entry | Name | Type | Display | Faction | Class | Levels | Rank | Extra |
|---|---|---|---|---|---|---|---|---|
| 97511 | Twilight Retuner | 7 | 2882 | 14 | mage | 48-50 | 0 | `spells = ["shadow_bolt_creature"]` |
| 97512 | Anchor Servant | 3 | 3916 | 14 | warrior | 49-50 | 0 | |
| 97513 | The Anchorwarden | 7 | 8772 | 14 | mage | 50 | 1 | `health_multiplier = 2.0`, `spells = ["shadow_bolt_creature", "shadow_word_pain_creature", "void_shield_creature"]` |

Display 3916 is the Servant of Ilgalar voidwalker the dungeons prep names; 8772 is the vanilla
Twilight Emissary. Elite templates carry no `rolls`.

```toml
# The break (#PARENT): four waves come up the stair to save the Anchor on the top floor.
[[encounter]]
key = "ilgalar_anchor"
quest = 99502
start = "accepted"
dungeon = 8
records = "ilgalar_anchor_broken"
anchor = [-9288.0, -3308.0, 150.09]
start_radius = 10.0
leash = 40.0
warning = "The Anchor shrieks as its tune slips. Footsteps on the stair!"
first_wave_after_s = 10.0
next_wave_after_s = 45.0

[[encounter.wave]]
creatures = [{ entry = 97511, level = 48, count = 3, position = [-9300.0, -3300.0, 150.09] }]

[[encounter.wave]]
creatures = [
  { entry = 97511, level = 49, count = 2, position = [-9300.0, -3300.0, 150.09] },
  { entry = 97512, level = 49, count = 2, position = [-9296.0, -3318.0, 150.09] },
]

[[encounter.wave]]
creatures = [
  { entry = 97512, level = 49, count = 3, position = [-9296.0, -3318.0, 150.09] },
  { entry = 97511, level = 50, count = 1, position = [-9300.0, -3300.0, 150.09] },
]

[[encounter.wave]]
creatures = [
  { entry = 97513, level = 50, count = 1, position = [-9300.0, -3300.0, 150.09] },
  { entry = 97512, level = 50, count = 2, position = [-9296.0, -3318.0, 150.09] },
]
```

Check each wave point with `python3.15 tools/twilights-eve/wmo_floor.py <x> <y>`: it must stand on
group 2's floor near z 150 and inside the footprint. If one does not, move it toward the Anchor
until it does, and record the point.

## The quests

A new file, `content/quests_blackrock_pass.toml`, loaded by `shipped()`
(`src/quests/content.rs` line 550) after the files already listed there, with the same schema, and
added to the `include_str!` list in `src/build_id.rs` beside the other quest files. Header: "The
Burning Steppes opening (#PARENT): Toren's break and the Blackrock Pass Watch camp. Same schema
as content/quests.toml." Both quests: `zone_or_sort = 44`, no em dashes, the never-name list of
the lore bible (no Prestor, Cedric, Sareth, Deathwing or the Lich King).

**99501 Translation Complete.** The stand-in for 98521 (decision 11); ticket 10 deletes it. Level
48. `kind = "story"`. Giver and ender Toren (190083). No `after`. No objectives. Money 0.
Objectives: "Hear Toren out at the inn in Lakeshire."
Details: "It's done. Every page I have, read through twice. I know how his stones fail now: where the casing gives, what the core does when the tune slips, and how long the mist takes to notice. I didn't think I'd ever say this. I know how to break the one on the hill."
Progress: "Sit down. This will take a minute."
Completion: "He built them to hold forever, and he wrote down every way they don't. That's Morganth: careful about everything except what it was for. Listen, and I'll tell you how we bring it down."

If the content check refuses a quest with no objective whose giver is its ender, give it one
`talk` objective to Toren instead, and record it.

**99502 Bring It Down.** Level 49. `kind = "story"`. Giver and ender Toren (190083).
`after = [99501]`. `until_milestone = "ilgalar_anchor_broken"`: offered only while the Anchor
stands, and a character who holds it after another group's break still finishes it (ticket 03's
rule). `kill = [[97513, 1]]`. Money 2450 (50 x level). No gear choice.
Objectives: "Climb the Tower of Ilgalar to the Anchor on its top floor, hold it while the cult comes to save it, kill the Anchorwarden, and return to Toren in Lakeshire."
Details: "The Anchor on the top floor still runs. Morganth's dead and it doesn't care. Here's how it breaks: you don't hit it, you make it work too hard. Stand at the stone and it pulls on everything near it to keep its tune. The cult will feel that the moment you start. They'll send tuners up the stair to steady it, and something worse after them. Kill them all, and the last one most of all. With no hands to hold the tune, the stone gives."
Progress: "Is it still humming? Some nights I can hear it from here. I want to stop hearing it."
Completion: "I felt it from here. The whole inn did; the cups jumped. It's done, then. It's really done. The grey won't go at once. He wrote that too. It thins, and what's been living behind it walks out first. Someone will have to hold the road while it goes."

## Art

`docs/art-assets.md`: a section `## The Burning Steppes opening (#PARENT)` after the Deeprun Tram
section, with a table row for "The Ilgalar Anchor, whole and broken": "two GameObject displays on
Ilgalar's top floor; a humming stone in a Dark Iron casing with a forge stamp, and the same stone
cracked open, its casing split. Until made, displays 1667 and 1687 (floating purple crystals)."

## Failing tests first

New `crates/twilight-realm/tests/steppes_opening.rs`, built like `tests/encounters.rs` and the
Phased Dungeon tests that walk a character into a dungeon's Phase (for example
`tests/northshire_abbey.rs`):

- Toren offers 99501 to a level 48 character and not 99502; after 99501 is handed in, he offers
  99502. After `reach_milestone("ilgalar_anchor_broken")`, he no longer offers 99502.
- A character holding 99502 who walks into Ilgalar's Phase and within 10 yards of the Anchor starts
  the encounter: the first wave's three Retuners come into that Run's shard after 10 s, and a
  character in the open world outside the tower sees none of them.
- With two characters in the Phase, a wave creature's health is the Head Count's 1.5 times what it
  is with one (ADR 0029).
- Killing every wave reaches `ilgalar_anchor_broken`: `milestone_reached` is true, a
  `Storage::Milestones` is pushed, and a character in the Phase is sent the removal of 794101 and
  the create of 794102 within a tick. A second character in the Phase within 74 yards of the
  Anchorwarden's death gets the kill credit for 99502.
- After the break, a character holding 99502 can still start the encounter and finish the quest,
  and the Milestone is not written again.
- Leaving the Phase with the encounter under way drops it ("You have abandoned the defense.").
- `content/encounters.toml`: an encounter whose `records` names no Milestone fails the load.

## Acceptance

- The tests pass. `cargo test -q -p twilight-realm` passes, including `tests/encounters.rs`
  unchanged. `cargo check -q -p twilight-realm` is clean.
- `python3.15 tools/twilights-eve/ground_check.py` and `python3.15 tools/twilights-eve/icon_check.py`
  pass.

Assumptions allowed: how a won encounter reports to the realm; whether `dungeon` encounters keep
their own slot list; the wave points if `wmo_floor.py` moves them; the stand-in's objective form.
Record them as `Assumed:` lines.

Blocked by DUNGEONS:06-ilgalar-monsters, which places the Anchor object (guid 794101) this ticket gates.
