# A spawn spot follows written rules, and the realm asks its host whether a body fits

Davis kept finding units in the ground (#167, #261, #333). The realm already grounds every
placed unit (`crate::ground`, #253 and #254), and `ground_check.py` gates the
Z of every content TOML row that writes one. Three gaps were left. The gate never read a point
with x and y only (Rumour givers, Rumour step spawns). Nothing checked water, walls, doodads or
roofs. And spots picked at run time near a moving character (a Bounty Hunter and its adds) were
grounded loosely or not at all: the adds took the hunter's own Z. Spec #350 closes the gaps.
This ADR fixes the rules and the one new question the realm asks its host.

- **The rules apply to units we placed.** That covers every content TOML row with a position,
  `elwynn_overrides.toml`'s added and moved rows, the Rumour points, and every run-time spot. A
  spawn imported from the vmangos world DB (`elwynn_spawns.json`, `barrens_spawns.json`, the
  Wailing Caverns files) is Blizzard's layout and is not checked, unless Davis names one.
- **A good spot passes six rules.** The unit stands on the ground or on a floor, within the
  limits `ground_check.py` already uses. A land unit is not in water deeper than 1.5 yd.
  Its body is not inside a WMO wall or a doodad. It is not on a roof. Its model loads. A player
  can reach it. The body is a vertical capsule 0.4 yd in radius and 2 yd tall, standing on the
  spot.
- **Swimmers.** A creature may stand in deep water when its template says `swims = true` (our
  templates) or its vmangos `InhabitType` includes water (imported templates). A GameObject is
  exempt from the water rule. An NPC never is.
- **Roofs.** A floor in a WMO group with the indoor flag (0x2000) is a storey. A floor in an
  outdoor group more than 3 yd above the terrain is a roof. A row that belongs up there (a tower
  top, a bridge) says `on_top = true`.
- **The static gate checks everything it can read from the game data.** `ground_check.py`
  checks ground, floors, water, WMO walls, roofs, doodads, models and reach. Its `--fix` nudges a
  flagged row to the nearest passing spot within 5 yd, in rings 1 yd apart with 8 directions
  each. Beyond 5 yd it flags the row, and a session picks the spot from the row's comment.
- **The realm asks `Host::body_fits(map, at)`.** It answers whether the capsule at `at` touches
  nothing solid: terrain, a WMO or a doodad. The default is `true`, so a test host needs no
  change. The app answers from its collision world, and `twilight-server` from its own. With no
  collision streamed in at `at`, the answer is `true`, and the unit settles as it does today.
- **A run-time spot is tried, then nudged, then dropped.** `crate::ground::spot` grounds the
  wanted point and tests it against the run-time rules: ground found, water, `body_fits`. On a
  failure it tries the same rings as `--fix`, out to 5 yd. With none passing it returns `None`,
  and the caller makes no unit. The realm does not log. The audit probe is how a dropped spawn
  is found.
- **The in-game audit is a probe Davis runs on demand.** `WOW_PROBE=spawn_audit` walks our
  spawns tile by tile with the real collision streamed in and writes a report. It is no gate:
  nothing runs it on a schedule.

## Considered options

- **Reuse `line_of_sight` for walls.** Rejected: short sight lines around a point miss a body
  standing inside a thin rock or a tree trunk, and the probe would have to second-guess them.
- **Check walls and doodads statically only.** Rejected: a Bounty Hunter's spot is rolled near
  a moving character, so no content row exists for the gate to read.
- **Place a run-time spawn anyway and only report it.** Rejected: a Bounty Hunter inside a rock
  is the bug Davis sees. A nearby passing spot is almost always within 5 yd.
- **A reachability walk on a navmesh.** Rejected for now: no navmesh exists. The roof rule and
  the reach rule cover the cases Davis has hit.
- **Check the imported vanilla spawns too.** Rejected by Davis: Blizzard's layout is trusted.
