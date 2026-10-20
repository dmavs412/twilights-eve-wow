# Twilight's Eve: Callings

A single-player offline WoW ORPG prototype on a from-scratch 1.12.1 client in Rust + Bevy. The
client (`crates/benilla-*`) renders the game and speaks the 1.12.1 protocol. The Local Realm
(`crates/twilight-realm`) is the game's rules: in-process today, behind a socket for friends
later. The Survivors mode and the browser build are being removed, so `crates/warcraft-survivors`
(still the launcher binary), `crates/wenilla*` and `web/` are dead weight, never a target.

Vocabulary: `GLOSSARY.md`. Decisions: `docs/adr/`. Specs: `docs/specs/`. Vanilla reference notes:
`docs/reference/`. Judgement-call coding rules: `CODING_STANDARDS.md`.

## How work runs

- **Verify with `cargo check -q -p <crate> 2>&1 | tail -40`** in the foreground after every
  edit; the pipe keeps warnings out of the context. When a realm rule changes, also
  `cargo test -q -p twilight-realm` (seconds, no game data). That is the whole gate; a commit
  needs nothing else. A full build or a game run happens only when Davis asks for one.
- **Every command runs in the foreground, and you wait for it.** Let each cargo command finish:
  one stopped mid-compile leaves rlibs with corrupt MSVC debug info, and later links fail with
  `LNK1103` on a crate you never touched. The cure is `cargo clean -p <crate>` for each crate the
  linker names.
- **Do the work in this session.** Search with Grep, read with Read. The Agent tool is denied
  in `.claude/settings.json`, so no subagents and no skills that fan out (implement-spec,
  wayfinder, code-review, research). If a task seems to need one, say so and do it inline.
- **Keep context small.** Read the lines you need. Report in a few lines. Davis runs `/clear` or
  `/compact` between tasks; say so when a task is done and the next one starts cold.
- **Unattended runs.** `scripts/overnight.ps1` works through open issues labelled
  `ready-for-agent`, oldest first, one fresh Sonnet session each, with no one at the keyboard;
  rules in `docs/agents/unattended.md`. An evening grill ends by writing those tickets: each
  one fully specified, with its assumptions allowed and its acceptance stated, so the night
  session can pick a reading and record it instead of asking.
- **Two lanes.** The **Build** lane (this checkout, interactive or overnight) is the only one
  that builds or edits code, the CSV or generated JSON. **Lane 2** writes no code: it smoke-runs
  from a `play/` copy, reproduces Davis's bugs and writes issues. Every `cargo` command takes
  the build lock `target/.claude-build.lock`; a refusal names the holder, so wait or stop.
  Davis plays from `play/<short-commit>/`, made by `scripts/publish-play.ps1`, never from
  `target`. Rules: `docs/agents/unattended.md`.
- **`unplayed`.** A spec closes when its tickets land and gets the `unplayed` label; Davis's
  play note removes it. Bugs name the spec they break.
- **Grill before architecture.** Run `/grill-me` first for: a new crate or a new `twilight-realm`
  dependency; a new ECS component family or system ordering in the realm; a column added or
  changed in the master CSV, or a schema change in a generated file; anything crossing the
  client/realm seam or changing how the DBC patch applies; any decision that would need an ADR.
  A bug fix, a new data row, or a component that follows an existing pattern goes straight to
  code. A ticket labelled `ready-for-agent` has already passed grilling.
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
  `CARGO_BUILD_JOBS=4` is in the user environment. Davis plays from a `play/` copy
  (gitignored, newest 3 kept), because a running exe cannot be overwritten.
- `WOW_TWILIGHT=1` picks Twilight's Eve in the launcher. `WOW_PROBE=list` lists the variables
  that drive a run with no one at the keyboard.

## Issue tracker

GitHub Issues on `dmavs412/twilights-eve-wow`; always pass `-R dmavs412/twilights-eve-wow`.
Conventions: `docs/agents/issue-tracker.md`. Labels: `docs/agents/triage-labels.md`. Domain
docs: `docs/agents/domain.md`.
