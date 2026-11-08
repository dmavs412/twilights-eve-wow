Title: Trap springs: a trap that catches a group, and a shot that sets off your traps (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 9. Blocked by 06-mana-kinds (same files).

The Trapper (ticket 17) lays traps on the move. `ground_area` already has what a Snare Trap
needs (`src/callings.rs` lines 1821 to 1850): `arm_ms` (no one counts as inside until it has
passed), `first_only` (the first enemy inside takes the enemy aura and spends the area; the aura
outlasts it), `object_display`, and one area per caster and skill (`src/world/cast.rs` line
4976), so several different traps stand at once. Two things are missing.

## The params and kind

1. **`spring_radius_yd=N`** on a `first_only` `ground_area`: when the first enemy steps inside,
   every living enemy within N yd of the area's centre takes the enemy aura, not only the first,
   and the area is spent. Bosses are still left out, as `first_only` leaves them (line 2907).
   Without it, `first_only` works as today.
2. **Skill kind `detonate_traps`**: `radius_yd=N`. When the cast lands, every armed, unspent
   `first_only` area of the caster whose centre is within N yd of the cast's point (its ground
   target with `target=dest`, else its unit target) springs at once, as if an enemy had stepped
   in: with `spring_radius_yd` it catches everyone within that radius, without it the nearest
   enemy inside its own radius, if any. An area still arming does not spring.

## Where

- `crates/twilight-realm/src/callings.rs`: `GroundArea` (line 1827) gains `spring_radius_yd`;
  the `Effect` enum gains `DetonateTraps`.
- `crates/twilight-realm/src/world/cast.rs`: the `first_only` check (lines 2907 to 2925) and the
  area's creation (lines 4936 to 4985).
- `tools/twilights-eve/build_spells.py`: the `ground_area` checks (line 1782); refuse
  `spring_radius_yd` without `first_only=yes` or at 0; accept `detonate_traps` with `radius_yd`
  above 0. Tests in `tools/twilights-eve/tests/test_build_spells.py`.
- `docs/spell_pipeline.md` (the `ground_area` text) and `docs/agents/calling-script-guide.md`.

## Failing tests first

Extend `crates/twilight-realm/tests/ground_area.rs` (its test Priest Calling; add a Hunter-class
test Calling beside it if a ranged weapon is needed, as `tests/ranger_*.rs` files do).

- A `first_only` area with `spring_radius_yd=6` and a 3 s damage aura: three mobs within 6 yd and
  one at 9 yd; the first to walk in springs it, the three take the aura, the fourth does not, and
  the area is gone.
- A boss within the radius takes nothing.
- Two different trap skills stand at once; casting the first again replaces only the first.
- `detonate_traps radius_yd=10` at a point 5 yd from an armed trap with a mob beside it springs
  the trap; a trap still arming, and one 15 yd away, stay.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `build_spells.py --check --no-dbc-check` is clean.

Assumptions allowed: whether a sprung trap's object stays for its aura's time or goes at once
(follow Snare Trap).
