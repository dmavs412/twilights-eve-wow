@AGENTS.md

## Game data on this machine

The WoW 1.12.1 game data (14 MPQs) is at `C:\Users\davis\wow-data\Data`. Use it for `--data` and
`WOW_DATA`; `run.ps1` defaults to it. The repo's `wow-data` is a git symlink that Windows checks
out as a plain text file, so `wow-data\Data` does not exist here. Do not run `fetch-data.sh` again.

Let every cargo build and test run to completion: one stopped mid-compile leaves rlibs with
corrupt MSVC debug info, and later links fail with `LNK1103` on a crate you never touched. The
cure is deleting the `target/debug/deps` files written during the interrupted run.

A worktree builds its own `target`, about 18 GB, and several building at once run rustc out of
memory (which also ends in `LNK1103`). Delete a worktree's `target` once its branch is merged.
To skip a cold build, point `CARGO_TARGET_DIR` at another worktree's `target` (builds sharing
one wait on its lock), or copy one. `lua-src` bakes in the folder of the worktree that first
built it, so once that worktree is gone the build fails with "Cannot read
…\third_party\lua-src\lua-5.1.5"; `cargo clean -p lua-src -p mlua-sys` in that `target` cures
it. Build `--release` only in your own `target`: the user plays from
`.claude/worktrees/integration/target/release/warcraft-survivors.exe`, and a running exe cannot
be overwritten.
`CARGO_BUILD_JOBS=4` and `WASI_SDK` (the main checkout's `tools\wasi-sdk`, which worktrees lack)
are set in the user environment; if a shell lacks them, set them by hand.

## Agent skills

### Issue tracker

GitHub Issues on `dmavs412/twilights-eve-wow`; always pass `-R dmavs412/twilights-eve-wow`. See `docs/agents/issue-tracker.md`.

### Triage labels

The five default labels (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
