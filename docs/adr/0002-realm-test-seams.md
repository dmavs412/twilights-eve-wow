# The Local Realm is tested through two seams

The spec asks for realm tests that drive only the public interface: commands in, `tick`, events out, plus save export and import. That works for behaviour ("attack the mob until it dies, expect XP and loot"), but vanilla's formulas are dice-driven. A hit table, an armour reduction or a talent's percentage can't be pinned to the reference values through events without statistical tests.

So the realm has a second public seam: `twilight_realm::rules`, pure functions of vanilla's formulas. `tests/rules_*.rs` check them against exact values from `docs/reference/vanilla-combat-rules.md` and vmangos. Every other test file is a player script that uses commands, `tick` and events only. It may seed the dice (`Realm::with_seed`) so a fight replays exactly.

## Consequences

- A formula lives in `src/rules/` as a public function, and the systems call it. A formula buried in a system can be tested only through a fight.
- The rules API is part of the crate's public surface. A future server reuses it unchanged.
- Player scripts never reach into realm structs. They assert on what a client would see.
