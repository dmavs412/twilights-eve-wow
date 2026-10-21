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
