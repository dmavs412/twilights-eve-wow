Title: The Calling Choice window at Tier 1's cap, and a ten-row Talents panel (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 5 and 11. No blockers (the client reads whatever the realm sends).

At level 30 the realm (ticket 01) sends the same `SessionEvent::CallingChoice` with two options
instead of five. The window copes in part: rows past the count hide. Two things are wrong for
Tier 2, and one needs a check.

1. **The subtitle** is fixed text, "You have learned all a Basic Warrior can learn. Choose the
   path you will take." (`crates/benilla-app/assets/ui/CallingChoice.xml` line 149). Make it name
   the player's current Calling, read in Lua when the frame paints: "You have learned all a
   <Calling> can learn. Choose the path you will take." `UnitClass("player")` answers the
   Calling's name in this client (`docs/spell_pipeline.md`, "callings.csv": `UnitClass` answers
   the Calling's name), so a Basic Warrior still reads as today.
2. **The frame's height** is fixed for five rows (`<AbsDimension x="540" y="690"/>`, line 136;
   rows of 118 px, `BENILLA_CALLING_ROWS = 5`, line 14). With two options it shows three empty
   rows. In `BenillaCallingChoice_Paint` (line 18), size the frame to the rows shown: the top
   part above the first row, plus 118 px per shown row, plus the bottom margin, so five options
   give today's 690.
3. **Ten Talent Rows.** Tier 2 Callings have ten rows (`assets/ui/Talents.xml`). The panel
   already builds strips on demand (lines 118 to 145) inside a scroll frame of at most 640 px
   (line 34). Check that ten rows paint and scroll, and fix it if they do not.

## Failing tests first

- `crates/benilla-app/src/ui_script/calling_choice_tests.rs` (`two_options()` at line 68 already
  builds a two-option view): with the player's class reported as "Vanguard", the subtitle reads
  "You have learned all a Vanguard can learn. Choose the path you will take."; with "Basic
  Warrior" it reads today's text; with two options the frame's height is the five-option height
  less three rows of 118 px.
- `crates/benilla-app/src/ui_script/talent_rows_tests.rs`: pushing ten rows (levels 5 to 50, the
  first three open) paints ten strips, the tenth reachable in the scroll frame, and an eleventh
  strip is not made.

A client test build can pass the Bash tool's 10-minute cap: start it with `run_in_background`
and wait for the notification, with `--lib` and a filter (CLAUDE.md).

## Acceptance

- The tests pass: `cargo test -q -p benilla-app --lib calling_choice` and
  `cargo test -q -p benilla-app --lib talent_rows`.

Assumptions allowed: how the Lua reads the player's Calling name if `UnitClass` is not stubbed in
the test harness (stub it the way `calling_choice_tests.rs` stubs the tooltip, line 35).
