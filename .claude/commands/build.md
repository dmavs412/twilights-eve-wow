---
description: Start this chat in the Build lane and take one ticket
---

You are in the **Build lane** (CLAUDE.md, "Three lanes").

Task: $ARGUMENTS
If no task is given, take the oldest open issue labelled `ready-for-agent`
(`gh issue list -R dmavs412/twilights-eve-wow -l ready-for-agent`) and say which one you took.

1. Read the issue and only the code it touches.
2. If it hits a "Grill before architecture" trigger and is not labelled `ready-for-agent`, stop:
   write the open question as an issue comment and tell Davis to grill it in a pipeline chat.
3. Edit, then `cargo check -q -p <crate> 2>&1 | tail -40`; add `cargo test -q -p twilight-realm`
   when a realm rule changed. Wait for every command. A build-lock refusal means wait or stop.
4. Commit only the files you touched, saying what changed and which check ran. Comment on and
   close the issue (add `unplayed` to its spec when its last ticket lands).
5. Report in three lines and tell Davis to `/clear`.
