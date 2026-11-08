Title: Templates in dungeon files, spell sections per dungeon, and a lore-bible test that accepts a filled dungeon (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 5. Blocked by RD:02-level-bands-code (it changes `src/mobs.rs`'s spawn
assembly, which this ticket also changes), RD:04-dungeon-levels-whelps (it edits the
`lore_bible_dungeons` test module in `src/phased.rs`, which this ticket rewrites) and
SO:03-milestone-gated-content (it changes `src/phased.rs` and `src/mobs.rs`).

Plumbing only, so the four monster tickets (04, 06, 08, 10) can run two at a time. Today every
creature template is a row of `content/mobs.toml` and every creature spell a row of
`content/creature_spells.toml`, so two builders filling two dungeons would both append to both
files, and both regenerate `data/twilights-eve/client_dbc_patch.json`. After this ticket, a
dungeon's templates live in its own file, and each dungeon's spells go in its own section of
`creature_spells.toml`, started by a row this ticket writes.

## Templates in a Phased Dungeon file

A Phased Dungeon file may hold `[[template]]` rows in `content/mobs.toml`'s exact schema
(`Template` in `src/mobs.rs`), as `content/wailing_caverns.toml` already does.

- `src/phased.rs`, `content()` (line 1109): the list of `include_str!` files moves into a
  `pub(crate) fn sources() -> &'static [(&'static str, &'static str)]` of (file name, text), and
  `content()` parses from it. `Dungeon` keeps ignoring the `template` key (it has no
  `deny_unknown_fields`; keep it that way).
- `src/mobs.rs`, where the Wailing Caverns templates join the list (line 911,
  `template.extend(dungeon.template)`): also parse each of `phased::sources()` with a small struct
  `{ #[serde(default)] template: Vec<Template> }` and extend the list, before `lay_kits` and the
  rolls and castable checks that follow, so a dungeon template is checked like any other.
- The load panics when two templates anywhere share an entry, naming both sources. Today a
  duplicate is silently shadowed.
- `tests/common/mod.rs`, `mob_template` (line 131): look in the Phased Dungeon files too, so a
  test can name a dungeon template by entry. Its doc comment says so.
- `content/mobs.toml`'s header: one sentence that a Phased Dungeon's own creatures may live in
  its file instead, with the entry block the spec gave it.

## Spell sections per dungeon

At the end of `content/creature_spells.toml`, after `crate_toss` (1900059), write four sections
in this order, each a header comment and one spell row. The rows are real spells the monster
tickets use; they also keep each dungeon's later rows apart in the generated patch, because
`build_spells.py` writes the client rows in file order. A monster ticket adds its rows under its
own section's first row and above the next section's header, never at the end of the file.

```toml
# ── Mistmantle Manor (#PARENT): spell ids 1900060 to 1900069 ─────────────────────────────────────
# A dungeon's rows go here, after this section's first row and before the next section's header.

# Stalvan Mistmantle's gaze (04). Baleful Gaze's look (visual 686). Placeholders: 900 fire damage
# to every character within 30 yd not behind a [[cover]] pillar of the dungeon file, after a 2 s
# warning cast, every 20 s.
[[spell]]
key = "candles_flare_stalvan"
spell_id = 1900060
name = "The Candles Flare"
icon = 'Interface\Icons\Spell_Fire_Fire'
visual = 686
school = "fire"
cast_time_ms = 2000
cooldown_ms = 20000
first_cast_ms = 8000
range_max = 30.0
chance = 100
warning = "The candles flare! Get behind something solid!"
effect = { kind = "gaze", damage = 900, radius = 30.0 }

# ── The Tower of Ilgalar (#PARENT): spell ids 1900070 to 1900079 ────────────────────────────────
# A dungeon's rows go here, after this section's first row and before the next section's header.

# Morganth's Tuning Pulse (06). Dark Glare's look (visual 8092). Placeholders: 1000 shadow damage
# to every character within 40 yd not behind one of the Anchor's pylons, after a 2.5 s warning
# cast, every 25 s.
[[spell]]
key = "tuning_pulse_morganth"
spell_id = 1900070
name = "Tuning Pulse"
icon = 'Interface\Icons\Spell_Shadow_UnholyFrenzy'
visual = 8092
school = "shadow"
cast_time_ms = 2500
cooldown_ms = 25000
first_cast_ms = 10000
range_max = 40.0
chance = 100
warning = "The hum rises to a scream. Get behind a pylon!"
effect = { kind = "gaze", damage = 1000, radius = 40.0 }

# ── Raven Hill Deeps (#PARENT): spell ids 1900080 to 1900089 ─────────────────────────────────────
# A dungeon's rows go here, after this section's first row and before the next section's header.

# The Grey Deacon's gaze from the shadow overhead (08). Dark Glare's look (visual 8092). No player
# text names what casts the shadow. Placeholders: 1300 shadow damage to every character within
# 40 yd not behind a rubble heap, after a 3 s warning cast, every 30 s.
[[spell]]
key = "crowned_shadow_deacon"
spell_id = 1900080
name = "Crowned Shadow"
icon = 'Interface\Icons\Spell_Shadow_UnholyFrenzy'
visual = 8092
school = "shadow"
cast_time_ms = 3000
cooldown_ms = 30000
first_cast_ms = 12000
range_max = 40.0
chance = 100
warning = "A vast shadow crosses the shaft above. Get behind the rubble!"
effect = { kind = "gaze", damage = 1300, radius = 40.0 }

# ── Stonewatch Keep (#PARENT): spell ids 1900090 to 1900099 ──────────────────────────────────────
# A dungeon's rows go here, after this section's first row and before the next section's header.

# Flame Breath, the Dragonspawn Envoy's and Singe's (10). Flame Patch's look (visual 2120).
# Placeholders: a 5 yd fire patch at a random character for 10 s, 13 a second.
[[spell]]
key = "flame_breath_dragonkin"
spell_id = 1900090
name = "Flame Breath"
icon = 'Interface\Icons\Spell_Fire_Fire'
visual = 2120
school = "fire"
cast_time_ms = 1500
cooldown_ms = 15000
range_max = 30.0
chance = 100
target = "random"
effect = { kind = "ground_patch", radius = 5.0, duration_ms = 10000, tick_ms = 1000, tick = { base = 13 } }
```

Then regenerate the patch: `python3.15 tools/twilights-eve/build_spells.py --data "$WOW_DATA"`
(`docs/spell_pipeline.md` line 252) and run the pipeline unittests
(`~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests`). No template names
these four yet; if a load check refuses a spell no template uses, give that check an exception
for these four keys and record it.

## The lore-bible test

`src/phased.rs`, module `lore_bible_dungeons` (line 1907),
`each_holds_only_friendly_creatures_and_its_portal_stands_at_the_door`: today it wants exactly
one friendly creature in each of dungeons 8 to 15 (three in Stormwind Keep). Rename it
`each_is_a_shell_or_filled_and_its_portal_stands_at_the_door` and make it accept either:

- a shell: every creature friendly, one of them (three in Stormwind Keep), as today; or
- a filled dungeon: no Phase Tester (entry 97100), at least one creature whose template has
  `boss = true`, and every creature inside the footprint.

The map mark and portal checks stay for both. Keep any test the arc's ticket 04 added to the
module (dungeon 11 at 40-44, dungeon 9 at 45-48). `no_open_world_creature_or_hub_npc_stands_inside_them`
is unchanged. After this ticket a monster ticket edits no line of `src/phased.rs`.

## Art list

`docs/art-assets.md`: a new section `## The 30-50 dungeons (#PARENT)` after "The Stockade"
section and before "The Deeprun Tram", so it does not touch the file's end. One table of stock
stand-ins, all placeholders, none needing new art:

| Asset | Where | Stand-in |
|---|---|---|
| Humming ore in Mistmantle's floor | 04 | GameObject display 2370 (`CaveMineCrystalFormation04`) |
| The Anchor's pylons | 06 | display 3080 (`UngoroCrystalPylon01`) |
| Dark Iron Tribute crates | 06, 10 | displays 1868 and 1869 (`DarkIronCrate01`, `02`) |
| Feed valves | 06 | display 755 (`DeadMineValve`) |
| Rubble heaps, the dropped side tunnel | 08 | display 2230 (`G_CaveIn`) |
| Brood cage and its lever | 10 | displays 676 (`G_Cage`) and 295 (`G_LeverMetal`) |
| Singe's Scale icon | 11 | a free stock scale icon that passes `icon_check.py` |
| Mourncrown | 12 | display 6678 (`World\Goober\G_Necropolis.m2`) |

## Failing tests first

- `src/mobs.rs` tests (or a new `tests/dungeon_templates.rs`): a dungeon file with a `[[template]]`
  row makes that template known to `crate::mobs::template`; two templates with one entry panic
  at load (a test seam with a second copy of a row, in the form the existing load-panic tests
  use).
- The renamed `lore_bible_dungeons` test passes for today's eight shells; a unit test of its rule
  fails for a dungeon with a Phase Tester and a hostile creature, and for a filled dungeon with no
  Boss Bar creature.
- `tests/creature_spells.rs` (or `src/creature_spells.rs`'s test module): the four new keys
  resolve with their ids.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass and
  `build_spells.py --check` reports the committed patch current; `ground_check.py` and
  `icon_check.py` are unchanged.

## Assumptions allowed

- Where `sources()` lives and its exact signature, if another shape keeps one list of the files.
- The anchor rows' numbers are placeholders; leave them as written.
