# Twilight's Eve: Callings

A single-player offline WoW ORPG prototype on a from-scratch 1.12.1 client in Rust + Bevy. The
client (`crates/benilla-*`) renders the game and speaks the 1.12.1 protocol. The Local Realm
(`crates/twilight-realm`) is the game's rules: in-process today, behind a socket for friends
later. The Survivors mode and the browser build are being removed, so `crates/warcraft-survivors`
(still the launcher binary), `crates/wenilla*` and `web/` are dead weight, never a target.

Vocabulary: `GLOSSARY.md`. Decisions: `docs/adr/`. Specs: `docs/specs/`. Vanilla reference notes:
`docs/reference/`. Judgement-call coding rules: `CODING_STANDARDS.md`.

## How work runs

- **Verify with `cargo check -p <crate>`** in the foreground after every edit. When a realm rule
  changes, also `cargo test -p twilight-realm` (seconds, no game data). That is the whole gate; a
  commit needs nothing else. A full build or a game run happens only when Davis asks for one.
- **Every command runs in the foreground, and you wait for it.** Let each cargo command finish:
  one stopped mid-compile leaves rlibs with corrupt MSVC debug info, and later links fail with
  `LNK1103` on a crate you never touched. The cure is `cargo clean -p <crate>` for each crate the
  linker names.
- **Do the work in this session.** Search with Grep, read with Read. Spawning any agent, through
  the Agent tool or a skill that fans out (implement-spec, wayfinder, code-review, research),
  needs Davis's explicit yes for that task. When asking, name the task and ask which model it
  should run on. Sonnet is the default (`CLAUDE_CODE_SUBAGENT_MODEL` in `.claude/settings.json`);
  Haiku for a mechanical search or conversion, Opus or Fable only when Davis says so.
- **Keep context small.** Read the lines you need. Report in a few lines. Davis runs `/compact`
  between tasks; say so when a task is done and the next one starts cold.
- **Grill before architecture.** Run `/grill-me` first for: a new crate or a new `twilight-realm`
  dependency; a new ECS component family or system ordering in the realm; a column added or
  changed in the master CSV, or a schema change in a generated file; anything crossing the
  client/realm seam or changing how the DBC patch applies; any decision that would need an ADR.
  A bug fix, a new data row, or a component that follows an existing pattern goes straight to
  code.
- **Land as small commits on `main`**, each saying what changed and which check ran. A branch
  and a pull request only when Davis asks. CI (`.github/workflows/ci.yml`) repeats the check and
  the realm tests on GitHub's runners; it is a backstop you never wait on.

## Architecture rules

- **Content is data.** Spell ids, talent trees, costs, formulas and ability effects live in
  `data/twilights-eve/master.csv` and flow through `tools/twilights-eve/build_spells.py` into
  `server_spells.json` for the realm and `client_dbc_patch.json` for the client. A spell id or a
  talent's numbers in a `.rs` file is a bug. `docs/spell_pipeline.md`.
- **Compose in `bevy_ecs`.** The realm uses `bevy_ecs` alone: no Bevy app, renderer or window.
  An ability is a bundle of generic components (`ProcOnCrit`, `CleaveOnHit`, `PeriodicDamage`)
  driven by generic systems. A system or match arm named after one spell is a bug.
  `docs/bevy_guidelines.md`.
- **The client's DBCs and the realm are separate worlds.** Spell.dbc holds three effects per
  spell. The client shows and gates a cast from its row; the realm decides what happens. A talent
  that needs more than three effects sends effects 1 to 3 to the DBC patch and the rest to
  realm-only components the client never sees. `docs/dbc_structure.md`.

## This machine

- Game data: `C:\Users\davis\wow-data\Data`, for `WOW_DATA` and `--data`; `run.ps1` defaults to
  it. The repo's `wow-data` is a git symlink that Windows checks out as a text file.
- Python is `~/.local/bin/python3.15`; `python3` on PATH is the Microsoft Store stub. No Node.
- Build in this checkout's `target` only; a worktree's own `target` is 18 GB and fills C:.
  `CARGO_BUILD_JOBS=4` is in the user environment. Davis plays from
  `.claude/worktrees/integration/target/release/warcraft-survivors.exe`, and a running exe cannot
  be overwritten.
- `WOW_TWILIGHT=1` picks Twilight's Eve in the launcher. `WOW_PROBE=list` lists the variables
  that drive a run with no one at the keyboard.

## Issue tracker

GitHub Issues on `dmavs412/twilights-eve-wow`; always pass `-R dmavs412/twilights-eve-wow`.
Conventions: `docs/agents/issue-tracker.md`. Labels: `docs/agents/triage-labels.md`. Domain
docs: `docs/agents/domain.md`.
