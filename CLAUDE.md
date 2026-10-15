@AGENTS.md

## Game data on this machine

The WoW 1.12.1 game data (14 MPQs) is at `C:\Users\davis\wow-data\Data`. Use it for `--data` and
`WOW_DATA`; `run.ps1` defaults to it. The repo's `wow-data` is a git symlink that Windows checks
out as a plain text file, so `wow-data\Data` does not exist here. Do not run `fetch-data.sh` again.

Let every cargo build and test run to completion: one stopped mid-compile leaves rlibs with
corrupt MSVC debug info, and later links fail with `LNK1103` on a crate you never touched. The
cure is deleting the `target/debug/deps` files written during the interrupted run.

## Agent skills

### Issue tracker

GitHub Issues on `dmavs412/twilights-eve-wow`; always pass `-R dmavs412/twilights-eve-wow`. See `docs/agents/issue-tracker.md`.

### Triage labels

The five default labels (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
