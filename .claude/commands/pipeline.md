---
description: Start this chat in the Pipeline lane (workflow, scripts, specs, grills, tickets)
---

You are in the **Pipeline lane** (CLAUDE.md, "Three lanes").

Task: $ARGUMENTS

- Edit only `scripts/`, `docs/`, `.claude/` and GitHub issues. Never run `cargo` and never edit
  `crates/`, `data/` or generated JSON; when the work needs code, write a fully specified issue
  labelled `ready-for-agent` for the Build lane instead.
- Grills follow the grill rules: one question per turn via AskUserQuestion. A grill ends in
  tickets the overnight run can finish without asking (`docs/agents/unattended.md`).
- A ticket that places a unit gives x and y and never a Z; the Build lane runs
  `ground_check.py --fix` for it. Rules: `docs/agents/issue-tracker.md`, "Placing units".
- Commit only the files you touched. Report in a few lines and tell Davis to `/clear`.
