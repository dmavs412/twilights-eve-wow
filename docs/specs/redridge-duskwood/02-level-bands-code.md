Title: Level Bands for Redridge and Duskwood: a band file per zone (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 1. Code only: after this ticket the game plays exactly as before,
because the two new band files are empty. The four band tickets (06, 11, 16, 21) fill them.

## What

Elwynn's Level Bands (#228, `crates/twilight-realm/src/bands.rs`, `content/elwynn_bands.toml`)
give each sub-zone a level range. Redridge and Duskwood use a linear Level Stretch today
(`content/zones.toml`, `levels = { from, to }`, applied in `src/mobs.rs` line 739). Make the Level
Bands read one band file per zone, and let a Redridge or Duskwood spawn take its band's levels
where its zone's band file covers the point, and the Level Stretch everywhere else.

## The data

Two new hand-written files, same schema as `elwynn_bands.toml`:

- `crates/twilight-realm/content/redridge_bands.toml`
- `crates/twilight-realm/content/duskwood_bands.toml`

Each starts with a header comment in `elwynn_bands.toml`'s form that says: which zone file it
covers, that a spawn outside `bounds` keeps the zone's Level Stretch from `zones.toml`, that
friendly NPCs and critters keep their own levels, that a `level_overrides.toml` row beats a band,
and that on a tie between two boxes holding the same point the band listed first wins. Contents:
`bounds = []`, `exclude = []`, and no `[[band]]`. `BandFile.band` needs `#[serde(default)]` so a
file with no band parses.

Add both to `src/build_id.rs` beside `elwynn_bands.toml` (line 24). In `content/zones.toml`'s
header (lines 3 and 4), say that `levels` is the fallback for spawns outside the zone's band
file's `bounds`.

## The rule

- **Static spawns.** A spawn from `redridge.toml` takes `redridge_bands.toml`'s band for its point,
  a spawn from `duskwood.toml` takes `duskwood_bands.toml`'s, and an `elwynn.toml` spawn keeps
  using `elwynn_bands.toml` exactly as today. The band lookup is `band_for`'s (bounds, exclude,
  then the box with the smallest gap), run against that zone's file. When the zone's file gives
  no band (outside its bounds, or inside an exclude box), the spawn keeps the Level Stretch.
  Stormwind has no band file and keeps its stretch.
- **Who keeps their levels.** In Redridge and Duskwood, friendly NPCs and critters
  (CreatureType 8) keep their own levels, the same exception the stretch makes today (line 742).
  Elwynn's rule is unchanged.
- **Overrides win.** For a Redridge or Duskwood spawn, a `level_overrides.toml` creature row for
  its entry and point beats its band. Today the overrides are applied (line 859) before the
  Elwynn band loop (line 874) and the Elwynn loop overwrites only Elwynn vanilla spawns; keep
  that for Elwynn, and make sure the new zone bands are applied before the overrides, or skip a
  spawn that an override matched.
- **The level inside a band** comes from the same deterministic pick as Elwynn's
  (`bands::pick` / `population.rs`'s `level(guid, lo, hi)`), so streaming never rerolls it.
- **Runtime spawns.** `mobs::banded(entry, map, position)` (line 684), used by
  `world/call_for_help.rs` line 146, today returns a band only for Elwynn's vanilla templates.
  It now also returns one for a template that comes from `redridge.toml` or `duskwood.toml` and
  that `mobs.toml` does not shadow. The band files are tried in the order Duskwood, Redridge,
  Elwynn; the first whose `bounds` hold the point and whose `exclude` boxes do not decides.
- **Names.** `mobs::level_area` (line 693) names a band from any of the three files, in the same
  order. `src/map_marks.rs` (lines 106 to 111) reads it; its tests must still pass.

Suggested shape: `bands.rs` holds a small table of `(zone, include_str!)` and one `OnceLock`
per file, and `range_at`, `band_at` and `band_for` take the zone. Keep the public names Elwynn's
callers use, or update every caller.

## Failing tests first

In `src/bands.rs`'s test module:

- A test band file given as a string (two bands, one exclude box) parses into `BandFile`, and its
  lookup returns the right range inside a box, the nearest box's range for a point inside
  `bounds` but in no box, `None` inside the exclude box and `None` outside `bounds`. On two boxes
  that both hold a point, the band listed first wins.
- With the shipped empty files, every Redridge and Duskwood spawn keeps the level the stretch
  gives it: `stretch_maps_the_from_range_onto_the_to_range_and_clamps` still passes, and a new
  test checks a sample of hostile Redridge and Duskwood spawns against `stretch` of their
  `redridge.toml` / `duskwood.toml` row.
- `mobs::banded` for a Redridge vanilla entry at a Redridge point returns `None` with the empty
  file. A test that loads a one-band Redridge file through the same parsing path returns its
  range (use whatever seam the parsing code exposes; do not ship a band).
- Every existing Elwynn test in the module passes unchanged.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `cargo check -q -p twilight-realm` is
  clean; `ground_check.py` flags nothing (no unit moved).

Assumptions allowed: the module and function names, and the exact place the zone is threaded
through `mobs.rs`. Record them as `Assumed:` lines.
