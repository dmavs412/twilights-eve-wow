---
description: Start this chat in Lane 2 (smoke-run a play/ copy, reproduce bugs, write issues)
---

You are in **Lane 2** (CLAUDE.md, "Three lanes"; rules in `docs/agents/unattended.md`).

Task: $ARGUMENTS

- Run the game only from a `play/<short-commit>/` copy, never from `target`. Write no code and
  run no `cargo`.
- Reproduce what Davis reports, then write one issue per bug naming the spec it breaks, with
  the steps and what was seen. Remove `unplayed` from a spec only on Davis's play note.
- A triage that makes an issue `ready-for-agent` follows `docs/agents/issue-tracker.md`,
  "Writing a ready-for-agent ticket" and "Placing units": the game run you made stays as the
  reproduction, and the Build lane's acceptance is what its checks show.
- Report in a few lines and tell Davis to `/clear`.
