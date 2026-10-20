# Unattended runs

`scripts/overnight.ps1` works through open issues labelled `ready-for-agent` with no one at the
keyboard: every one labelled `bug` first, at any age, oldest first, then features oldest first. Each ticket gets a fresh `claude -p` session on Sonnet with
`scripts/overnight-settings.json` layered over the shared settings. These rules apply to that
session on top of `CLAUDE.md`. Nobody will answer a question; a session that asks one has
failed.

## Two lanes

- **Build lane**: the main checkout, one session at a time, interactive or overnight. It is the
  only lane that builds or edits code, `data/`, or generated JSON. Overnight is a Build lane.
- **Lane 2**: a second session that writes no code. It smoke-runs from a `play/` copy, and it
  triages bugs: for each rough bug Davis files, it reproduces it from the note and the logs,
  writes the reproduction and a failing-test outline into the issue, then adds
  `ready-for-agent`. The Build lane then fixes it test-first, one bug per session.
- **Build lock**: a PreToolUse hook makes every `cargo` command take `target/.claude-build.lock`
  (session id, start time) and release it after. If someone else holds it, the hook refuses at
  once and names the holder; wait or stop, never work around it. A lock older than 2 h is
  stale and is taken over. `overnight.ps1` holds it for the whole run.
- **Play copies**: `scripts/publish-play.ps1` builds release under the lock and copies the exe
  and data patch to `play/<short-commit>/` (gitignored), keeping the newest 3 and never deleting
  one whose exe is running. It runs when Davis asks, and once at the end of a night that landed
  a commit. Davis plays only from `play/`.
- **Morning log**: overnight posts one comment per night on the pinned "Overnight log" issue:
  tickets done, assumptions taken, the `play/` copy published, and failures.
- **`unplayed`**: a spec closes when its tickets land and gets `unplayed`; Davis's play note
  removes it. No feature waits on play. Bugs reference the spec they break.

## The ticket is the spec

- Read the issue first: `gh issue view -R dmavs412/twilights-eve-wow <n> --json title,body,comments`.
  The comments may narrow the body; the latest comment wins.
- A ticket labelled `ready-for-agent` passed grilling. Do not reopen its design, and do not
  narrow it. Do the whole ticket.
- "Grill before architecture" does not apply here. If the ticket turns out to need an ADR, a
  new crate, a master CSV column, or a change across the client/realm seam, it was mislabelled:
  block it (below) instead of guessing.

## Assume, record, continue

- When the ticket allows two readings, take the one closest to the ticket text and to the
  pattern already in the code. Pick, then keep going.
- Every assumption is written down twice: in the commit message body as `Assumed: ...` and in
  the closing comment on the issue. Davis reads both in the morning.
- Do not write memory files. A lesson worth keeping goes in the issue comment.

## The gate

- After every edit: `cargo check -q -p <crate> 2>&1 | tail -40`. When the realm, `data/` or
  `tools/` changed: `cargo test -q -p twilight-realm 2>&1 | tail -40`.
- Commit on `main` only when the gate passes. One commit per coherent change, message saying
  what changed, which check ran, and any `Assumed:` lines.
- After the session ends the script re-runs the gate on every crate the commits touched and
  resets `main` to the pre-ticket commit if it fails. A commit that skipped the gate is lost
  work.

## Finishing

- Done: `gh issue close -R dmavs412/twilights-eve-wow <n> --comment "..."` with what landed,
  the commit shas, and the assumptions.
- Cannot finish, whether the gate will not pass, the ticket is mislabelled, or it needs a build
  or a game run: commit nothing partial, run
  `gh issue edit -R dmavs412/twilights-eve-wow <n> --add-label blocked`, comment with the
  error or the question, and stop. The script discards uncommitted changes.
- Builds, game runs, `run.ps1`, pushes, branches, worktrees, agents and the web are denied for
  this session. A denial is a stop sign, not a puzzle: do not look for another way to do the
  denied thing.
