# Unattended runs

`scripts/overnight.ps1` works through open issues labelled `ready-for-agent` with no one at the
keyboard: every one labelled `bug` first, at any age, oldest first, then features oldest first.
Before each ticket the script reads its "Blocked by" sentences, any "Blocked by" heading and
GitHub's dependency links; while one names an open issue, including one that blocked earlier the
same night, the ticket is skipped and keeps its label for the next night. Each ticket gets a fresh `claude -p` session on Opus with
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
- **Dirty tree**: a tree with uncommitted work at the start is stashed (untracked files too) as
  `overnight <stamp>` and applied again once the night ends. If it no longer applies over the
  night's commits, the stash stays and the morning log's first line says so.
- **Push**: once the tickets are done, the script pushes `main`. Every commit on it passed its
  gate, because a failed gate is rolled back.
- **Logs**: `.claude/overnight/` and `.claude/smoke/` folders older than 14 days are deleted at
  the start of a run.
- **Morning log**: overnight posts one comment per night on the pinned "Overnight log" issue:
  tickets done, assumptions taken, the `play/` copy published, and failures. Its format:
  `## Night of <yyyy-MM-dd_HHmm>` (followed by the kept-stash line when there is one), then
  `### Tickets` (the summary table: issue, title, result, commits, cost; a skipped ticket names
  its open blockers), `### Assumptions` (every `Assumed:` line in the night's commits),
  `### Specs closed, unplayed` (each spec whose last ticket closed tonight),
  `### Play copy` (`play/<short-commit>` or none), `### Smoke` (`scripts/smoke.ps1` against
  that copy: `- clean, <n> spells`, one `- <calling> <spell> (<id>): <error>` per failed cast with
  its bug issue, a `- timed out at Calling <X>` line, or `- not run` with no copy) and
  `### Failures` (blocked or rolled-back tickets, a failed publish or push, a kept stash), each
  `- none` when empty.
  The smoke is report only: it never resets or blocks a commit. The script holds the build lock all
  night and names itself in `CLAUDE_BUILD_LOCK_HOLDER`, so the sessions it starts pass the hook.
- **`unplayed`**: a spec closes when its tickets land and gets `unplayed`; Davis's play note
  removes it. When the script closes a ticket, it closes the ticket's spec (its GitHub parent, or
  the `(#N)` that ends its title) once no ticket of it is open, adds `unplayed`, and removes the
  workflow labels. No feature waits on play. Bugs reference the spec they break.

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
  `tools/` changed: `cargo test -q -p twilight-realm 2>&1 | grep -E "FAILED|panicked|test result"`,
  which passes when every `test result:` line says `ok` and no line says `FAILED`. Don't add
  `${PIPESTATUS[0]}`: the night permission check denies it. `WOW_DATA` is already set for the
  night, so the data tests run; never put `WOW_DATA=...` in front of a command, because the
  prefix defeats the permission match.
  Before each commit, `rustfmt --edition 2021` the `.rs` files you touched; the pre-commit hook
  refuses unformatted ones.
- When a commit touches `crates/twilight-realm/content/`, also run
  `python3.15 -I tools/twilights-eve/ground_check.py`. A flagged row fails the gate, and
  `--fix` writes the answer. Never type a placed unit's Z by hand: write `0.0`, run `--fix`, and
  commit what it wrote. An interior unit takes its Z from the nearest existing row in the same
  building.
- Run every cargo command in the foreground and wait, with a 600000 ms timeout. Never
  `run_in_background`: background commands die at 10 minutes and leave corrupt rlibs.
- One plain command per Bash call. Headless permission matching refuses chains (`;`, `&&`),
  `$(...)`, heredocs and redirects into files, even when each part is allowed; only a trailing
  `2>&1 | tail -40` or `| head` is safe. Write scripts and comment bodies with the Write tool
  inside the repo's `target/agent/` folder, then run `python3.15 -I target/agent/x.py` or
  `gh issue comment <n> --body-file target/agent/c.md`.
- To undo your own edit to a file, use `git restore <path>`; `git checkout` is denied.
- Before `git commit`, run `rustfmt --edition 2021 <file>...` on every `.rs` file you changed.
  The pre-commit hook refuses code that is not rustfmt-clean; never `--no-verify`.
- Commit on `main` only when the gate passes. One commit per coherent change, message saying
  what changed, which check ran, and any `Assumed:` lines.
- After the session ends the script re-runs the gate on every crate the commits touched and
  resets `main` to the pre-ticket commit if it fails. A commit that skipped the gate is lost
  work.

## Finishing

- Done: `gh issue close -R dmavs412/twilights-eve-wow <n> --comment "..."` with what landed,
  the commit shas, and the assumptions, then
  `gh issue edit -R dmavs412/twilights-eve-wow <n> --remove-label ready-for-agent`. The script
  closes the spec when yours was its last ticket; leave the spec alone.
- Cannot finish, whether the gate will not pass, the ticket is mislabelled, or it needs a build
  or a game run: commit nothing partial, run
  `gh issue edit -R dmavs412/twilights-eve-wow <n> --add-label blocked`, comment with the
  error or the question, and stop. The script discards uncommitted changes.
- Builds, game runs, `run.ps1`, pushes, branches, worktrees, agents and the web are denied for
  this session. The one `cargo run` allowed is a spawn importer:
  `cargo run -q -p twilight-realm --example import_<name>`. A denial is a stop sign, not a
  puzzle: do not look for another way to do the denied thing.
