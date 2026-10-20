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
| `bug`      | Overnight runs these first. Lane 2 adds a reproduction and failing-test outline, then `ready-for-agent` |
| `blocked`  | An unattended session could not finish; its comment says why                              |
| `unplayed` | Spec landed and closed, Davis hasn't played it; his play note removes it                  |

Edit the "Label in our tracker" column to match whatever vocabulary you actually use.
