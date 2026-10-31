---
description: Start this chat in the Build lane and take one ticket
---

You are in the **Build lane** (CLAUDE.md, "Three lanes").

Task: $ARGUMENTS
If no task is given, take the oldest open issue labelled `ready-for-agent`
(`gh issue list -R dmavs412/twilights-eve-wow -l ready-for-agent`) and say which one you took.
This chat takes one ticket. A range of tickets runs through `scripts/overnight.ps1`, one fresh
session each.

1. Read the issue's state and labels, then the issue and only the code it touches. A closed
   issue, or one labelled `ready-for-human`, is not this lane's: say so in one line and stop.
2. If it hits a "Grill before architecture" trigger and is not labelled `ready-for-agent`, stop:
   write the open question as an issue comment and tell Davis to grill it in a pipeline chat.
3. A ticket's failing test that passes before any change is missing a game input: compare its
   inputs with what the app feeds (DBC rows, realm content, spawns) before searching the code,
   and write the gap in the issue (#434).
   Edit, then `cargo check -q -p <crate> 2>&1 | tail -40`; add the realm test line from
   CLAUDE.md when a realm rule changed, and `rustfmt --edition 2021` the `.rs` files you touched. A content edit (`crates/twilight-realm/content/`) also runs
   `python3.15 tools/twilights-eve/ground_check.py`; a flagged row is a failed gate, and `--fix`
   writes the Z. Wait for every command. A build-lock refusal means wait or stop.
4. Commit only the files you touched, saying what changed and which check ran, and push `main`.
   Comment on and close the issue, removing its `ready-for-agent` label (close its spec with
   `unplayed` when its last ticket lands).
5. Report in three lines and tell Davis to `/clear`.
