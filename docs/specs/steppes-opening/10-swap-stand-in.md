Title: The break quest follows The Unsent Letter, and the stand-in goes (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 11. Blocked by 06-pass-camp (same `content/quests_blackrock_pass.toml`
and `tests/steppes_opening.rs`; 05 wrote both quests) and RD:30-toren-unsent-letter (quest 98521
"The Unsent Letter", given by Sergeant Crane and handed to Toren, 190083).

## What

Ticket 05 shipped the break quest 99502 "Bring It Down" behind a stand-in, 99501 "Translation
Complete", because the Redridge and Duskwood arc's last quest did not exist yet. Now it does.

- In `content/quests_blackrock_pass.toml`, 99502's `after` becomes `[98521]`.
- Delete 99501's row and its comment.
- A character who already handed in 99501 keeps it in `completed`. A save that holds 99501 in the
  log must still load: check how the realm treats a quest id the content no longer has
  (`content/retired_items.toml` is the items' version of this). If the save load refuses or panics
  on it, drop unknown quest ids from the log at load with a log line, and record it. A character who
  handed in 99501 but not 98521 loses the offer of 99502 until they finish 98521: say so in the
  closing comment.
- Change the file's header line that names the stand-in, if any.

## Failing tests first

In `crates/twilight-realm/tests/steppes_opening.rs`:

- Change ticket 05's offer test: Toren offers 99502 only once 98521 is handed in, and not to a
  level 49 character who has not done it.
- The content has no quest 99501.
- A save whose quest log holds 99501 loads, and the character enters the world.

## Acceptance

- The tests pass. `cargo test -q -p twilight-realm` passes. `cargo check -q -p twilight-realm` is
  clean.

Assumptions allowed: how an unknown quest id in a save is handled if the realm does not handle it
already. Record it as an `Assumed:` line.
