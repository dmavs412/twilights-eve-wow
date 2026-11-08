Title: Frozen until a boss's health mark: a dungeon creature key, and the tram moved onto it (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 4. Blocked by 01-dungeon-templates (both edit `src/phased.rs` and
`src/mobs.rs`).

Raven Hill Deeps (ticket 08) needs dead that stand frozen in the grey and step out at the Grey
Deacon's 75, 50 and 25% health marks. The Deeprun Tram already does this for the Conductor's
passengers, in tram code (`src/tram.rs`, ADR 0033), and ADR 0033 said a second dungeon that needs
it is the time to make it general. This ticket makes it a Phased Dungeon creature key and moves
the tram onto the same code. No content dungeon uses the key yet; the Ward Test does, for the
tests.

## What exists

- `Mob::frozen` (`src/mobs.rs` line 1337): a frozen creature cannot be selected, takes no damage,
  is pulled by nothing and does nothing (`src/world.rs` lines 977, 1296, 2085 and 2121).
  `Mob::thaw(spell_id)` (line 1693) clears it, takes off the marker aura and returns the values
  the clients are sent.
- The tram: `Line::boss_stepping` (line 804) makes the passengers with `frozen: true` and the marker
  `frozen_spell` in `wears` (`src/tram.rs` around line 834); `Shard` copies `s.frozen` onto the
  mob (`src/shard.rs` line 406); `Line::run_boss` (line 927) thaws a group of passengers each time
  the Conductor's health reaches the next of `[tram] wake_at_pct`, groups split by `wake_group`
  (line 1526); `take_waking` and `Shard::wake_tram` turn a thawed passenger on the closest living
  player once `wake_s` has passed. Once he is dead, no more thaw.

## The key

A Phased Dungeon `[[creature]]` row may carry:

```toml
frozen_until = { guid = 974250, pct = 75 }
```

- `guid` is another creature row of the same dungeon (the boss), `pct` a percent of its maximum
  health, 1 to 99. An optional `spell` names the marker aura by Creature Spell key and defaults
  to `"frozen_in_the_mist"` (1900057, Freezing Trap's look).
- The creature starts each Run frozen and wears the marker, as a tram passenger does.
- The first time, in that Run, the creature of `guid` is alive with health at or below `pct`% of
  its maximum, the frozen one thaws (`Mob::thaw`) and turns on the closest living player in the
  Phase (`crate::world::engage`, as `Shard::wake_tram` does).
- If the creature of `guid` dies before reaching the mark, the frozen one stays frozen for the
  rest of the Run, as a passenger does after the Conductor dies. A Run reset makes it frozen
  again.
- The load panics on a `guid` that is no creature of the same dungeon, a `pct` outside 1 to 99,
  an unknown `spell`, or a frozen row that also has `asleep = true`.
- Document the key in the doc comment of `Creature` (`src/phased.rs` line 132) and in one comment
  line wherever the dungeon files' keys are listed (`content/ward_test.toml`'s header is enough).

## The code

- `src/phased.rs`: `Creature` gains `frozen_until: Option<FrozenUntil>`, a small struct of
  `guid`, `pct` and `spell`, with its serde default.
- `src/mobs.rs`: `Spawn` (and the spawn rows built from `phased::content()`, line 940) carries the
  row's `frozen_until`, as it carries `appears`.
- Where a Run's creatures are made from their spawn rows, a row with `frozen_until` makes a mob
  with `frozen = true`, the marker in `wears`, and a new `Mob::frozen_until: Option<(u64, u32,
  u32)>` (the boss's mob guid in this Run, the percent, the marker's spell id). Find that place
  from `Shard`'s Run start; name it in the commit.
- One generic system, run on the shard's tick beside the health calls (`call_for_help` in
  `src/world/call_for_help.rs` line 107 is the model): for each frozen mob with `frozen_until`,
  look up its boss by guid; if the boss is alive and at or below the percent, thaw it, clear
  `frozen_until`, and engage the closest living player. The events go out as `Mob::thaw`'s do.

## The tram moves onto it

- `Line::boss_stepping` gives each passenger `frozen_until` = (the Conductor's mob guid, the percent
  of the group `wake_group` puts it in, `frozen_spell`). `Line` needs the Conductor's guid when
  the passengers are made: they step off the same car, so take it from the boss state, or give
  the passengers their `frozen_until` in `Line::bring` once the Conductor is brought.
- `Line::run_boss` no longer thaws: delete its thaw loop (around lines 937 to 990). The line
  notices a thawed passenger by `!mob.frozen` in `run_creatures`, sets its `stir_ms` there, and
  `take_waking` turns it after `wake_s` as now. A tram passenger is not engaged by the generic
  system: the generic system skips mobs whose spawn row has `tram_of`, or leaves the engage to
  the line. Pick one and record it.
- `[tram]`'s keys in `content/deeprun_tram.toml` stay as they are (`wake_at_pct`,
  `frozen_spell`). Its comment on line 121 says the thaw now runs through the generic key.
- `docs/adr/0033-tram-waves-and-placed-mist-walls.md`: one line under the "Tram code is for the
  tram only" bullet: "Amended (#PARENT): the frozen passengers' thaw is now the generic
  `frozen_until` dungeon creature key; the rest stays tram code."

## Failing tests first

- `tests/phased_wards.rs`, with its harness (`party`, `go`, `kill`, `run`, line 319 has the
  pattern): add to `content/ward_test.toml` a boss row (a hostile template from `mobs.toml` with
  enough health, guid 800010) and two frozen rows (guids 800011 and 800012) with
  `frozen_until = { guid = 800010, pct = 75 }` and `{ guid = 800010, pct = 40 }`. Tests: both
  start frozen (not selectable, damage to them does nothing); bringing the boss to 70% thaws
  800011 only, and it engages a player; 40% thaws 800012; killing the boss from full health in one
  blow leaves both frozen; a Run reset puts both back frozen.
- A load test: a `frozen_until` naming a guid that is not in the file panics.
- `tests/deeprun_tram.rs` passes unchanged, and so does `tram.rs`'s
  `passengers_wake_in_three_groups_with_the_extra_ones_first`.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `ground_check.py` flags nothing.

## Assumptions allowed

- The exact field shapes and where the generic system lives, if the behaviour above holds.
- The Ward Test's boss template and positions inside its box (about z 47, unchecked ground).
