# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker.

| Label in mattpocock/skills | Label in our tracker | Meaning                                  |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `needs-triage`       | Maintainer needs to evaluate this issue  |
| `needs-info`               | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `ready-for-human`    | Requires human implementation            |
| `wontfix`                  | `wontfix`            | Will not be actioned                     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

Repo-only labels:

| Label      | Meaning                                                                                   |
| ---------- | ----------------------------------------------------------------------------------------- |
| `bug`      | Overnight runs these first. An open `bug` without `ready-for-agent` is Lane 2's queue: Lane 2 adds a reproduction and failing-test outline, then `ready-for-agent` |
| `blocked`  | An unattended session could not finish; its comment says why                              |
| `on-hold`  | Parked for later; not picked by overnight runs                                            |
| `unplayed` | Spec landed and closed, Davis hasn't played it; his play note removes it                  |

A closed issue keeps no workflow label (`needs-triage`, `needs-info`, `ready-for-agent`,
`blocked`): whoever closes it removes them, and `overnight.ps1` does so for the tickets and specs
it closes. `bug` and `unplayed` stay on closed issues.

Edit the "Label in our tracker" column to match whatever vocabulary you actually use.
