# Coding standards

Read at review time. Mechanical rules live in checks instead: rustfmt (`.githooks/pre-commit`) and
`cargo check` (CI). These are the judgement calls a check can't make.

## The Local Realm (`crates/twilight-realm`)

- **One module per concern.** A change extends the existing module: Hub NPCs in `hub`, merchants
  in `vendor`, casts in `world/cast`, bags and slots in `inventory`, the save in `save`. A second
  module doing the same job (a parallel NPC list, a second cast path) is a finding, even when the
  two were written on separate branches and only met at the merge.
- **Content is data.** A mob, item, quest, NPC, skill or talent is a row in `content/*.toml` or in
  `data/twilights-eve/master.csv`, not a branch in code (`docs/spell_pipeline.md`). A generated
  file has its generator in `tools/twilights-eve/` or `examples/` and a content test that fails
  when the two drift.
- **Formulas are rules.** A vanilla formula is a public function in `src/rules/` with an
  exact-value test in `tests/rules_*.rs` (ADR 0002). A formula inlined in a system is a finding.
- **Saves stay readable.** A new save field is optional with a default, within the current
  `SAVE_VERSION`, and a test loads a save written without it.
- **Test first.** A behaviour's test lands in the same commit as its code, or an earlier one.
  Player-script tests use commands, `tick` and events only, and assert on what the client sees.
- **One test harness.** A player-script helper (a view of the fields, a drain, a cast, a fight
  setup) lives in `tests/common/`, `gear_common/` or `quest_common/`; a test file defines only
  what is unique to it. A helper copied into a second test file is a finding.
- **Flow tests fight as the Champion.** A test that fights only to reach what it checks (a
  quest's credit, a loot roll, a call for help) plays `common::as_champion`. A test of a fight's
  outcome plays a shipped Calling. A flow test that raises a level or a stat to win its fight is a
  finding.

## Everywhere

- **Comments state what is, never how it came to be**:
  "saves from before the rename", "no kit yet" and "was X" belong in the commit message.

## Edits outside Twilight's Eve

- New work goes in new files. An edit to an existing client file is the smallest hook that calls
  into the new file (a `mod` line, one match arm), and its commit says why.
