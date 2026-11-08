Title: Resurrection, and a heal over time paid by the health a cast cost (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 10. Blocked by 07-trap-springs (same realm files) and
02-trainer-search (both may touch `src/lib.rs`).

The Blightcaller and the Bloodmender are the game's main healers (tickets 18 and 19). A group's
healer must bring a dead friend back, and the Bloodmender heals with its own blood.

## The kinds

1. **Resurrect.** Skill kind `resurrect`, on a row with `target=friend` and
   `requires=combat=out`: `health_pct=N` (1 to 100), or `pct_of=health_cost;pct=N` (the friend
   returns with N% of the health the cast cost the caster, at most their maximum, at least 1).
   The target must be a dead friendly player, not yet released, within range. The realm sends the
   target's session `SessionEvent::ResurrectRequest` (it exists in `benilla-protocol`,
   `src/events.rs` line 1583, and the client already shows the stock popup,
   `crates/benilla-app/src/death/mod.rs`). On accept, the target stands up where they lie with
   that health and a fresh power bar, as `release` (`src/world/death.rs` line 116) sets it but
   without moving them. On decline, or if they release first, nothing happens. A living target, a
   released one or an enemy is refused at no cost.
   - The answer reaches the realm as a new `RealmCommand::ResurrectResponse { caster, accept }`
     beside `RepopRequest` (`crates/twilight-realm/src/lib.rs` line 451, dispatched at line
     2286). The client already sends `ClientCommand::ResurrectResponse`
     (`crates/benilla-app/src/death/mod.rs` line 472); map it in
     `crates/benilla-app/src/twilight/mod.rs` beside line 835 (`ClientCommand::RepopRequest =>
     Some(RealmCommand::RepopRequest)`). That one line is the only client change.
2. **Heal over time from a health cost.** A skill's `periodic_heal` named by `apply_aura` on a
   `target=friend` row reads the aura row's `pct_of=health_cost;pct=N`: the friend heals N% of
   the health the cast cost, spread over the aura's ticks. The aura-row form exists for talent
   procs (Masochism); check whether a skill applying it already works. If it does, add only the
   test and say so.

## Where

- `crates/twilight-realm/src/callings.rs`: the `Effect` enum (line 1442).
- `crates/twilight-realm/src/world/cast.rs`: friendly targets; how a heal on another player
  reaches that player's session (search `overheal_burst` and the party code;
  `tests/common/party.rs` shows the test side).
- `crates/twilight-realm/src/world/death.rs`: `dead()` (line 54) and `release` (line 116).
- `tools/twilights-eve/build_spells.py`: `resurrect` needs `target=friend` and `combat=out` in
  `requires`, and exactly one of `health_pct` or the `pct_of`/`pct` pair. Tests in
  `tools/twilights-eve/tests/test_build_spells.py`.
- `docs/spell_pipeline.md` and `docs/agents/calling-script-guide.md`.

## Failing tests first

New file `crates/twilight-realm/tests/resurrect.rs`, with two players as `tests/common/party.rs`
makes them and a Priest-class test Calling in inline TOML (the form of `tests/ground_area.rs`).

- Player B dies to a mob and does not release; player A, out of combat, casts `resurrect
  health_pct=30` on B: B's session gets `ResurrectRequest` naming A; B accepts and stands where
  they lay at 30% health, alive.
- B declines: B stays dead. B releases before answering: the answer does nothing.
- Casting on a living B, or while A is in combat, is refused and costs nothing.
- With `cost_kind=health` 20 and `pct_of=health_cost;pct=200`, B returns with twice what A paid,
  capped at B's maximum.
- A `target=friend` skill with `cost_kind=health` 15 and `periodic_heal apply_aura=` an aura of
  `pct_of=health_cost;pct=150` over 6 s heals B 150% of the cost over the ticks.
- In `crates/benilla-app`, a unit test beside the mapping in `src/twilight/mod.rs` (its existing
  tests show the form) that `ClientCommand::ResurrectResponse` maps to the realm command. This
  is a client build: start it with `run_in_background` per CLAUDE.md, or keep it to `cargo check
  -q -p benilla-app` if the test module needs the whole app.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `cargo check -q -p benilla-app`
  passes; the pipeline unittests pass; `build_spells.py --check --no-dbc-check` is clean.

Assumptions allowed: the resurrect popup's timeout (the stock one), the power a resurrected
player gets (follow `release`).
