# Issue tracker: GitHub

Issues and specs for this repo live as GitHub issues. Use the `gh` CLI for all operations.

## Conventions

- **Create an issue**: `gh issue create -R dmavs412/twilights-eve-wow --title "..." --body "..."`. Use a heredoc for multi-line bodies.
- **Read an issue**: `gh issue view -R dmavs412/twilights-eve-wow <number> --json number,title,body,labels,comments`.
- **List issues**: `gh issue list -R dmavs412/twilights-eve-wow --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'` with appropriate `--label` and `--state` filters.
- **Make an issue a sub-issue of a parent**: `gh issue create -R dmavs412/twilights-eve-wow --parent <parent> ...`, or `gh issue edit -R dmavs412/twilights-eve-wow <parent> --add-sub-issue <child>` afterwards (`gh` 2.94+). Older `gh`: `gh api --method POST repos/dmavs412/twilights-eve-wow/issues/<parent>/sub_issues -F sub_issue_id=<child-db-id>` (database id, as in **Blocking** below). Without sub-issues, put `Part of #<parent>` at the top of the child body.
- **Comment on an issue**: `gh issue comment -R dmavs412/twilights-eve-wow <number> --body "..."`
- **Apply / remove labels**: `gh issue edit -R dmavs412/twilights-eve-wow <number> --add-label "..."` / `--remove-label "..."`
- **Close**: `gh issue close -R dmavs412/twilights-eve-wow <number> --comment "..."`

**Always pass `-R dmavs412/twilights-eve-wow`.** This clone is a fork of `OutThisLife/azeroth-survivors-re`; without `-R`, `gh` targets the parent repository, not ours.

**A ticket's numbers come from its own lists.** Count the rows a ticket summarises before writing the count in its title or body, and compute every worked example from the rule the ticket states. A Build lane session takes the ticket's words as the contract; a count that disagrees with the list, or an example that disagrees with the rule, costs it a cycle and a judgement call it then records in the issue.

## Placing units

A ticket gives x and y for every placed unit, and the map when it is not 0. It never gives a Z.
The Build lane writes `0.0` and runs `python3.15 tools/twilights-eve/ground_check.py --fix`,
which computes the Z. A ticket for an interior unit names the building and the floor it stands
on (the nave, the tower top). `python3.15 tools/twilights-eve/wmo_floor.py <x> <y>` lists every
floor of the building over a point; the session writes that floor's Z, and `ground_check.py`
fails a row inside a building that stands on no floor. A building's bounding box holds the
ground around its walls too, so a point inside the box can still be outdoors: pick the point
with `wmo_floor.py`, never from the box.

## Writing a ready-for-agent ticket

- Name every place the change touches by file, function and line ("`effect_row` in
  `crates/benilla-formats/src/spells/twilight.rs`, line 172"). The Build session goes straight to
  code from these. A ticket without them sends the session exploring.
- A bug ticket also names the game data in play where the bug happens, as the reproduction
  found it: the nearby DBC rows, realm content rows and spawns. #420's cause was two stock
  `AreaPOI.dbc` flags under the dungeon portals, while its ticket named only the drawing code.
- Acceptance lists only what the Build lane's checks show: a test, `ground_check.py`, the
  pipeline unittests. The Build lane makes no game run (CLAUDE.md), so a screenshot, a probe
  run, or "casting X shows Y" belongs to the spec's `unplayed` label or a Lane 2 follow-up, not
  to the ticket's acceptance.
- A "Failing tests first" step names the test file to extend, and is written to match that
  file's setup. Read the file once: if its tests run on a test Calling with its own spell ids,
  the step uses that Calling, not the shipped row's id.

## Pull requests as a triage surface

**PRs as a request surface: no.** _(Set to `yes` if this repo treats external PRs as feature requests; `/triage` reads this flag.)_

When set to `yes`, PRs run through the same labels and states as issues, using the `gh pr` equivalents:

- **Read a PR**: `gh pr view -R dmavs412/twilights-eve-wow <number> --comments` and `gh pr diff -R dmavs412/twilights-eve-wow <number>` for the diff.
- **List external PRs for triage**: `gh api --paginate 'repos/dmavs412/twilights-eve-wow/pulls?state=open' --jq '.[] | select(.author_association | IN("OWNER","MEMBER","COLLABORATOR") | not) | {number, title, author: .user.login, author_association, labels: [.labels[].name]}'`.
- **Comment / label / close**: `gh pr comment -R dmavs412/twilights-eve-wow`, `gh pr edit -R dmavs412/twilights-eve-wow --add-label`/`--remove-label`, `gh pr close -R dmavs412/twilights-eve-wow`.

GitHub shares one number space across issues and PRs, so a bare `#42` may be either: resolve with `gh pr view -R dmavs412/twilights-eve-wow 42` and fall back to `gh issue view -R dmavs412/twilights-eve-wow 42`.

## When a skill says "publish to the issue tracker"

Create a GitHub issue.

## When a skill says "fetch the relevant ticket"

Read it as in **Read an issue** above.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a single issue with **child** issues as tickets.

- **Map**: a single issue labelled `wayfinder:map`, holding the Notes / Decisions-so-far / Fog body. `gh issue create -R dmavs412/twilights-eve-wow --label wayfinder:map`.
- **Child ticket**: an issue linked to the map as a GitHub sub-issue (see **Make an issue a sub-issue of a parent**). Where sub-issues aren't enabled, add the child to a task list in the map body and put `Part of #<map>` at the top of the child body. Labels: `wayfinder:<type>` (`research`/`prototype`/`grilling`/`task`). Once claimed, the ticket is assigned to the driving dev.
- **Blocking**: GitHub's **native issue dependencies**, the canonical, UI-visible representation. Add an edge with `gh api --method POST repos/dmavs412/twilights-eve-wow/issues/<child>/dependencies/blocked_by -F issue_id=<blocker-db-id>`, where `<blocker-db-id>` is the blocker's numeric **database id** (`gh api repos/dmavs412/twilights-eve-wow/issues/<n> --jq .id`, _not_ the `#number` or `node_id`). GitHub reports `issue_dependencies_summary.blocked_by` (open blockers only, the live gate). Where dependencies aren't available, fall back to a `Blocked by: #<n>, #<n>` line at the top of the child body. A ticket is unblocked when every blocker is closed.
- **Frontier query**: list the map's open children (`gh issue list -R dmavs412/twilights-eve-wow --state open`, scoped to the map's sub-issues / task list), drop any with an open blocker (`issue_dependencies_summary.blocked_by > 0`, or an open issue in the `Blocked by` line) or an assignee; first in map order wins.
- **Claim**: `gh issue edit -R dmavs412/twilights-eve-wow <n> --add-assignee @me`, the session's first write.
- **Resolve**: `gh issue comment -R dmavs412/twilights-eve-wow <n> --body "<answer>"`, then `gh issue close -R dmavs412/twilights-eve-wow <n>`, then append a context pointer (gist + link) to the map's Decisions-so-far.
