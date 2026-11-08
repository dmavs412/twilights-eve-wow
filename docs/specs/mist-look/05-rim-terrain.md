Title: The rim row: closed terrain beside the edge draws and fades into the mist (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 3. Amends ADR 0015 (draft in the spec folder,
`docs/specs/mist-look/adr-0015-amendment.md`). Blocked by 01-mist-shots (so the "before" build
exists). Blocked by SO:07-client-opening, which gives `cut_closed` its reached set and edits
`terrain_stream.rs` and `crates/benilla-assets/src/lib.rs`. Client only; the realm does not
change. No mist file changes, so this runs beside tickets 02 to 04.

## Why

Today a closed chunk draws nothing (ADR 0015), so the land stops dead at the mist and the cut is
the first thing a player sees. Davis chose one row of closed land drawn inside the mist and fogged
to full mist (decision 3): roads and grass run on past the edge and sink into grey, as the lore
says ("roads end in grey"). The realm still refuses every move onto a closed chunk, and the client
keeps its fence, so only the look changes.

## The rim

A rim chunk is a closed chunk with at least one open chunk among its eight neighbours, judged by
the same closed predicate and reached set the cut uses. Record which neighbours are open as an
8-bit mask: bit 0 north, then clockwise (north-east, east, south-east, south, south-west, west,
north-west), north meaning +x and west +y in WoW coordinates. 0 means not a rim chunk.

### The cut (`crates/benilla-assets/src/adt.rs`)

`cut_closed` (line 312) treats a rim chunk differently from other closed chunks:

- It keeps its `indices` and `holes`, so the terrain draws.
- It still clears `liquids` (ticket 06 changes that), sets every `no_effect_doodad` (no clutter),
  sets `impassable` (the fence in `terrain_stream/collider.rs` line 233 stays), and drops doodads
  and WMOs whose position is on it, as today.
- A closed chunk that is not a rim chunk is emptied exactly as today.

It returns each chunk's mask, and the loader stores it on that chunk's `ChunkShading` (line 48) in
a new field `rim_open: u8` (0 by default). The loop that builds `shading` (line 205) must give a
rim chunk its real layers, alpha and shadow indices.

### Mist depth on the vertices

- `pub const ATTRIBUTE_MIST_DEPTH: MeshVertexAttribute`, `Float32`, beside the other custom
  attributes (`crates/benilla-assets/src/model.rs` lines 139 to 170), with its own unique id and a
  doc comment; export it from `lib.rs` as they are.
- `pub fn rim_depth(xy: [f32; 2], north_west: [f32; 2], open: u8) -> f32` in adt.rs: the distance in
  yards from `xy` to the nearest open neighbour's square, for a chunk whose north-west corner is
  `north_west` (`positions[0]`). For an open edge neighbour that is the distance to the shared
  side; for an open corner neighbour, the distance to the shared corner point. Returns 0 when
  `open` is 0.
- `chunks_to_mesh` (line 77): when any part's `rim_open` is non-zero, add `ATTRIBUTE_MIST_DEPTH` to
  every vertex of the cell, `rim_depth` of the vertex for a rim chunk and 0 for any other. A cell
  with no rim chunk gets no attribute, so its pipeline is unchanged.
- `pub const RIM_FOG_YD: f32 = 30.0;` in adt.rs: the depth at which the land is pure mist.
- Export `rim_depth` and `RIM_FOG_YD` from `crates/benilla-assets/src/lib.rs` (line 40) beside
  `chunks_to_mesh`; ticket 06 uses both from `benilla-world`.

### The terrain shader

- `TerrainExtension` (`crates/benilla-assets/src/materials.rs` line 398) gains a `specialize`. When
  the mesh layout contains `ATTRIBUTE_MIST_DEPTH`, push shader def `TERRAIN_MIST_RIM` into the
  vertex and fragment stages and rebuild the vertex buffer layout with the standard attributes at
  their usual locations plus the depth at location 13, following `WowModelExt::specialize`'s
  `WOW_MERGED_FADE` rebuild (materials.rs lines 164 to 190).
- `crates/benilla-assets/src/shaders/terrain.wgsl`: under `TERRAIN_MIST_RIM` the vertex stage reads
  the depth at location 13 and passes it to the fragment in a new `TerrainVsOut` slot. The vertex
  stage takes `forward_io::Vertex` today (line 135), which has no slot 13, so under the def it needs
  its own input struct.
- In the fragment, after the `terrainp_s` combine (line 229) and before the scene fog (line 233):
  `rim = smoothstep(0.0, t.params.y, depth)`, the foot colour
  `foot = mix(fog, vec3(luma(fog)), t.params.z) * t.params.w` with `fog = wow_light.fog_color.rgb`
  and luma weights (0.299, 0.587, 0.114), and `tuned = mix(tuned, foot, rim)`. The scene fog then
  applies on top, as it does to the mist after ticket 02.
- `crates/benilla-world/src/terrain_stream.rs` line 734 fills the spare lanes:
  `params: Vec4::new(TERRAIN_LAYER_TILES, RIM_FOG_YD, crate::mist::FOOT_DESAT, crate::mist::FOOT_VALUE)`,
  today 0.6 and 0.35, the mist's own foot. Update `TerrainParams`' comment (terrain.wgsl line 25)
  and the field's doc comment.

### What stays as it is

- The horizon (`crates/benilla-world/src/wdl.rs` line 198) stays cut over every closed chunk.
- The ground collider (`terrain_collider_data`) now welds the rim's triangles. That is fine: the
  fence still stops the walk, and the camera may rest on the rim.
- The minimap and world map.

### The ADR

Append the section between the `---` lines of `docs/specs/mist-look/adr-0015-amendment.md` to
`docs/adr/0015-closed-zones.md`, after its Consequences, with `#PARENT` replaced by this spec's
number. Its line about water is ticket 06's; leave it in, since 06 follows this ticket.

## Failing tests first

- `crates/benilla-assets/src/adt.rs` `closed_tests` (line 334), in the style of
  `a_closed_chunk_is_emptied_and_only_the_open_doodad_stays` (line 396): with a closed chunk beside
  an open one, the closed one keeps its indices, is impassable, has no liquids, has every
  `no_effect_doodad` set, and loses its doodad; its mask has the open side's bit. A closed chunk
  with no open neighbour is emptied as before.
- `rim_depth`: with only the east neighbour open, a vertex on the east side is 0, a vertex on the
  west side is 33.33 and the middle 16.67; with only the south-east neighbour open, the north-west
  corner is 47.14 (33.33 times the square root of 2) and the south-east corner 0; with `open` 0,
  every vertex is 0.
- `chunks_to_mesh`: a cell of one rim chunk and one open chunk carries `ATTRIBUTE_MIST_DEPTH` on all
  its vertices, 0 on the open chunk's; a cell with no rim chunk has no such attribute.
- The data test in `crates/benilla-assets/src/lib.rs` (line 509) still finds an open tile and
  loads; add a data-gated test that a Westfall-edge tile (the one holding (-9700, 880)) has rim
  chunks drawn on the closed side of y 866.7.

## Acceptance

- `cargo test -q -p benilla-assets --lib 2>&1 | grep -E "FAILED|panicked|test result"` passes.
- `cargo check -q -p benilla-assets`, `-p benilla-world` and `-p benilla-app` are clean. CLAUDE.md and
  `docs/agents/unattended.md` say how to run client builds.
- `docs/adr/0015-closed-zones.md` carries the amendment.
- The gate cannot compile WGSL. Read `terrain.wgsl` once more for names declared only under the def
  and used outside it. Ticket 11 compiles it.
- What it looks like is ticket 11's and the spec's `unplayed` label's.

Assumptions allowed: the attribute's id, how the mask reaches `ChunkShading`, the names of the new
shader structs. Record them as `Assumed:` lines.
