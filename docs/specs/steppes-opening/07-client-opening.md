Title: The client opens a gated zone at run time and fades its mist (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 5 and 9. ADR 0034 and ADR 0015. Blocked by 02-gated-closed-zones (the
`gated` list in `closed_chunks.json` and its reader in `benilla-formats`). Ticket 01's
`SessionEvent::Milestones` is already on main by then. Client only: no realm change.

## What

The realm sends `SessionEvent::Milestones { reached }` at login and whenever a Milestone is reached
(ticket 01). Today the client treats every gated chunk as closed (ticket 02). Make it treat a
gated chunk as open once its key is in the reached set, and make an opening during play visible:
the border mist fades over a few seconds and the land appears behind it, with no relog.

## The reached set

- `crates/benilla-formats/src/closed_zones.rs`: the four readers (`closed_at`, `chunk_closed`,
  `closed_chunks_near`, `zone_closed`) answer for a given set of reached keys. A gated chunk is
  closed unless its key is in the set; a closed-for-good chunk is always closed. Shape it as you
  see fit (a parameter, or a small `Opened` value the callers hold), with one rule: no test may
  depend on state another test set, so no process-wide mutable set that tests share.
- `crates/benilla-app`: a resource holding the reached keys, updated from
  `SessionEvent::Milestones` wherever the app routes the realm's session events (start from
  `crates/benilla-app/src/twilight/mod.rs`). The ADT loader runs on the asset threads, so the
  loader and the resource share one handle (for example an `Arc<RwLock<..>>` the plugin creates
  and clones into `AdtLoader`).
- Every caller reads the set: the ADT cut (`cut_closed` in `crates/benilla-assets/src/adt.rs` line
  312), the horizon (`crates/benilla-world/src/wdl.rs` line 198), the mist curtains
  (`build_mist_curtains`, `crates/benilla-world/src/mist.rs` line 407, which calls `segments` with
  `chunk_closed` at line 419), the minimap's closed squares
  (`crates/benilla-app/src/minimap/mod.rs` line 625), the world map's misted zones
  (`crates/benilla-app/src/ui_world_map.rs` line 102), and the test helper in
  `crates/benilla-assets/src/lib.rs` line 509.

## At login

The event comes before the character's world (ticket 01 sends it before
`InWorld::enter`). The client applies it before the first tiles load, so an open Steppes loads
open, with no fade and no reload. If a tile did load before the event, the reload below fixes it.

## During play

When an event adds keys the client did not have:

1. **The mist fades.** Every loaded mist curtain segment that stands on an edge between an open
   chunk and a newly opened chunk fades to nothing over `OPEN_FADE_SECS` (a `pub const` in
   `mist.rs`, 6.0), then is despawned. A fading curtain needs its own material instance (or a
   per-instance value) so the other curtains keep their look; add a fade factor to `MistExt`
   (line 74) and multiply it into the alpha in `shaders/mist.wgsl`. Placed mist walls (ADR 0033)
   never fade.
2. **The land loads behind it.** Every loaded ADT tile that holds a newly opened chunk is loaded
   again through the terrain streamer (`crates/benilla-world/src/terrain_stream.rs`), so its
   terrain, water, clutter, doodads, WMOs and collision come in, and the opened chunks lose
   `impassable`. Start the reload when the fade starts. A curtain the old tile owned must survive
   the tile's despawn long enough to finish fading (detach it from the tile's root first).
3. **The rest follows.** Horizon cells over the opened chunks come back, the minimap stops drawing
   them closed, and the world map stops misting the zone. Curtains on the new border with zones
   still closed (the Burning Steppes against Searing Gorge, the Badlands, the Wetlands, the Swamp of
   Sorrows and Westfall) appear with the reloaded tiles, with no fade in.

## Failing tests first

- `crates/benilla-formats/src/closed_zones.rs` tests: Morgan's Vigil (-8378.5, -2748.9) is closed
  with no keys and open with `steppes_open`; Searing Gorge (-6679.9, -1194.4) and Westfall
  (-10508.8, 1045.23) stay closed with it; `zone_closed(0, 46)` follows the key;
  `closed_chunks_near` around Morgan's Vigil is empty with the key.
- `crates/benilla-world/src/mist.rs` tests, in the style of
  `closed_east_and_south_neighbours_give_two_segments_with_their_heights` (line 521): a border
  chunk whose neighbour is gated gives a segment while the key is missing and none once it is
  present.
- `mist.rs`: the fade factor is 1.0 at 0 s, 0.5 at 3 s and 0.0 at 6 s and after (a pure function).
- `crates/benilla-app`: a minimal-app test that sending `SessionEvent::Milestones` with
  `steppes_open` through the app's event path updates the reached-set resource, and that a second
  identical event starts no fade.
- `crates/benilla-assets/src/adt.rs` `closed_tests`: a Steppes tile is cut with no keys and uncut
  with `steppes_open` (data-gated like the module's other tests).

## Acceptance

- The tests pass: `cargo test -q -p benilla-formats closed_zones`, `cargo test -q -p benilla-world
  mist`, `cargo test -q -p benilla-assets closed`, and the benilla-app test with `--lib` and its
  filter.
- `cargo check -q -p benilla-app` is clean. CLAUDE.md and `docs/agents/unattended.md` say how to
  run the client builds.
- What the fade looks like in the game is not part of this ticket's acceptance: the spec's
  `unplayed` label covers it.

Assumptions allowed: the shape of the reached-set API; how the streamer reloads a tile (asset
reload or drop and re-request); whether a fading curtain is a copy or the original detached.
Record them as `Assumed:` lines.
