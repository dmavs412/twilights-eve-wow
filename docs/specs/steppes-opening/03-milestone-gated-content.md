Title: Content that waits for a Milestone: after_milestone and until_milestone (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 1 and 7. ADR 0034. Blocked by 02-gated-closed-zones (same streaming
code in `src/hub.rs`, `src/population.rs`, `src/world/phase.rs` and `src/lib.rs`),
RD:02-level-bands-code (`src/mobs.rs`) and RD:03-quest-files (`src/quests/content.rs`). Code
only: no shipped row uses the new keys until tickets 05 and 06.

## What

The break makes things appear for the whole server: the Blackrock Pass camp's people, board and
creatures (ticket 06) and the broken Anchor (ticket 05), and it takes away the whole Anchor and
Toren's offer of the break quest. Give five kinds of content two optional keys, each naming a
Milestone key from `content/milestones.toml`:

- `after_milestone`: absent until that Milestone is reached, then present for good.
- `until_milestone`: present until that Milestone is reached, then gone for good.

A row may carry both. The content load panics on a key the catalogue does not have, naming the
row and the key. Every check reads the realm's reached set (ticket 01's `Milestones`), never a
cached flag, so a Milestone reached mid-session takes effect on the next tick with no restart.

## The five kinds

1. **Hub NPCs** (`content/hub.toml`, `Npc` in `src/hub.rs` line 26). `InView::update` (line 377)
   keeps an NPC out while its keys are not met, and lets go one that stops being met
   (`ObjectsRemoved`, as for an NPC that leaves view). `find` (line 223) and every interaction it
   serves (gossip, quests, vendor, inn, kitchen) refuse an NPC that is not met, as they refuse one
   out of reach. Its map marks (`src/map_marks.rs`, lines 31 and 206) follow.
2. **Quest GameObjects** (`[[gameobject]]` rows in the quest files, `QuestObject` in
   `src/quests/content.rs` line 298). Their creates (`src/world/phase.rs` line 749) and their use
   (lines 799 and 864) follow the keys, as do their marks (`src/map_marks.rs` line 59).
3. **Hand-placed zone creatures** (`[[zone.added]]` in `content/zones.toml`, `Added` in
   `src/mobs.rs` line 619). `src/population.rs` (`update`, line 89) brings such a spawn in only
   while its keys are met. One that stops being met leaves at the next look, alive or dead, in or
   out of a fight.
4. **Quest offers** (`[[quest]]` rows, `Quest` in `src/quests/content.rs`). `offered`
   (`src/quests/dialogs.rs` line 462) refuses a quest whose keys are not met, so its giver neither
   lists it nor shows its `!`. A character who already holds it keeps it, and its ender takes it
   back whatever the Milestones say. Check that the hand-in path does not call `offered`.
5. **Phased Dungeon rows** (`Appears` in `src/phased.rs` line 211, flattened into creatures and
   GameObjects). `met`, `comes_later` and `gone` (lines 231 to 250) take the reached set.
   `after_milestone` counts as coming later. In a Run under way, a row whose `after_milestone` is
   reached comes in on the next tick, as an `after_dead` row does (`src/shard.rs` line 235), and a
   row whose `until_milestone` is reached leaves on the next tick. A Run that starts after the
   Milestone starts with the right rows. The client-side shard view (`src/world/phase.rs` lines 719
   and 911) follows the same rule.

## Headers

Document the two keys in each file's header: `content/hub.toml` (the paragraph that lists the row
fields), `content/zones.toml` (the `[[zone.added]]` sentence on line 5), `content/quests.toml` (the
quest fields), the `[[gameobject]]` paragraph of `content/quests_elwynn.toml`'s header if it has
one, and the Appears keys wherever `after_used` is documented (`content/ward_test.toml`,
`content/stockade_riot.toml`). Say that the key names a row of `content/milestones.toml`.

## Failing tests first

In `crates/twilight-realm/tests/milestones.rs` (ticket 01's file), using the seams the existing
tests use: `realm.with_quests(toml)` for quests and quest GameObjects (as
`clicking_a_giver_object_offers_its_quest_and_accepting_puts_it_in_the_log` in
`tests/quest_mechanics.rs` places one), and a test dungeon (`content/ward_test.toml` or
`content/test_grounds.toml`) or a test-only content string for Phased Dungeon rows. Where no seam
reaches a kind (Hub NPCs, `[[zone.added]]`), unit-test the filter in its module instead.

- A quest with `until_milestone = "ilgalar_anchor_broken"` is offered before
  `reach_milestone("ilgalar_anchor_broken")` and not after. A character who accepted it before can
  still hand it in after.
- A quest with `after_milestone = "ilgalar_anchor_broken"` is not offered before and is after.
- A quest GameObject with `after_milestone` sends no create to a player in view before the
  Milestone, and its create arrives within a tick after `reach_milestone`, with no relog.
- A Phased Dungeon GameObject with `until_milestone` and its twin with `after_milestone` swap in a
  Run under way when the Milestone is reached: the first is removed and the second created for a
  player in the Phase, within a tick.
- A Hub NPC row with `after_milestone` is not present before and is present after; one with
  `until_milestone` is removed after (unit test on `InView::update` with a test row if no seam).
- An added zone creature with `after_milestone` is not brought in before and is after (unit test
  on the population filter if no seam).
- A quest row with `after_milestone = "nope"` fails the content load, naming the quest and `nope`.

## Acceptance

- The tests pass. `cargo test -q -p twilight-realm` passes. `cargo check -q -p twilight-realm` is
  clean.
- No shipped row changes behaviour: none uses the keys yet.

Assumptions allowed: whether the two keys live in one small shared struct (`#[serde(flatten)]`)
or on each row type; how the reached set reaches each check; the wording of the headers. Record
them as `Assumed:` lines.
