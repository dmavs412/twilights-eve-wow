Title: Hidden trainers, hint lines and trials: what a Tier 2 Advancement Quest needs (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 12. Blocked by 01-calling-choice-per-tier (both edit
`src/quests/content.rs`, `src/characters.rs` and `src/save.rs`).

A Tier 2 Advancement Quest is a search, then a trial. Three things the realm cannot do today:

1. **Hidden NPCs.** A Hub NPC marked `hidden = true` makes no Living Map mark, neither a quest
   mark nor a service mark, for a character who has not met it. A character meets it the first
   time they open its gossip menu or its quest list. The save gains `met`, the entries of hidden
   NPCs met (empty writes nothing). After that it is marked as any Hub NPC is. Without this, the
   map's yellow `?` (the Advancement Quest asks only for a word, so it counts as complete) would
   show every hidden trainer the moment the quest is taken. Field Givers inside Phased Dungeons
   are never marked already, so a dungeon trainer needs no flag.
2. **Hint lines.** A quest may list `hint` rows, `{ npc, option, reply }`, as `talk` rows are
   written. While the character holds the quest, not yet handed in, NPC `npc` shows the gossip
   line `option`, and choosing it shows `reply`. A hint gives no credit and can be asked again.
   It works at a gossiping Hub NPC and at a friendly field creature, as `talk` lines do. An
   Advancement Quest may carry hints (it still may not carry objectives).
3. **Trials.** An Advancement Quest may name `trial = <quest id>`. Until the character has handed
   that quest in, the Advancement Quest is not complete: the ender shows a grey `?`, the map shows
   no ready mark, and the hand-in shows the quest's `progress` text and does not advance. Once
   it is handed in, the Advancement Quest completes as today.

The trial's own quests need nothing new: the first has `after_taken = <the Advancement Quest>`
(offered only while it is in the log or handed in), each later one `after` the one before, all
level 30 with `min_tier = 1` (`quests.toml` header, line 30).

## Where

- `crates/twilight-realm/src/hub.rs`: the `Npc` struct (lines 26 to 91) gains `hidden: bool`
  (serde default false). `gossip_hello` (line 269) is where a met hidden NPC is recorded, and
  where its hint lines join the options beside `talk_options` (line 297).
- The quest list opened at a Hub NPC also records a meeting: find where `CMSG_QUESTGIVER_HELLO`
  is answered in `src/quests/dialogs.rs`.
- `crates/twilight-realm/src/map_marks.rs`: `givers()` (line 27) builds the quest-mark list once;
  `marks()` (line 180) and its service-mark loop (about line 205) take the character, so filter
  there: skip a hidden Hub NPC the character has not met.
- `crates/twilight-realm/src/quests/content.rs`: `Talk` (line 289) is the form for a new `Hint`;
  `Quest` (line 340) gains `hint: Vec<Hint>`; `Advancement` (line 425) gains `trial:
  Option<u32>`. The checks (lines 755 to 850): a hint's `npc` must be a gossiping Hub NPC or a
  friendly spawn, as a talk's is (line 798); a `trial` must name a quest the content has, whose
  chain reaches back to this one through `after` and `after_taken`.
- `crates/twilight-realm/src/quests/mod.rs`: `talk_lines` (line 291) and `talk_options` (line
  307) show talk lines; hint lines need their own option indexes and reply text ids beside
  `TALK_OPTION` (line 280) and `REPLY_TEXT` (line 287), answered in `talk_text` (line 321).
- `crates/twilight-realm/src/quests/dialogs.rs`: `field_gossip_hello` (line 236) builds a field
  creature's menu: add hint lines there too, and let a creature with only hint lines open a menu.
  `is_complete` (line 499) and the private `complete` it calls are where the trial check goes,
  so the hand-in (line 186), the marks (`status`, line 508) and the map agree.
- `crates/twilight-realm/src/characters.rs` and `src/save.rs`: `met`, in the form ticket 01 gave
  `chosen_tiers`.
- `crates/twilight-realm/content/quests.toml` header (lines 25 to 60) and `content/hub.toml`
  header (lines 1 to 22): document `hint`, `trial` and `hidden`.
- `GLOSSARY.md`: add **Hidden Trainer** (a Tier 2 trainer the Living Map does not show until met),
  **Hint** (a gossip line an NPC says only while you hold a quest) and **Trial** (the quests a
  Tier 2 trainer sets before Advancement).

## Failing tests first

New file `crates/twilight-realm/tests/trainer_search.rs`. No shipped quest uses these fields
until ticket 10, so the test content comes from a fixture: add one test Hub NPC (`hidden = true`)
and two test quests in the form `content/ward_test.toml` and the Test Grounds keep test content
off every player's path (the Test Grounds box is at (-9650, 255) in Elwynn): an Advancement
Quest from `vanguard` to `zealot` with `calling_choice = false`, one `hint` at Lyria Du Lac (913,
a gossiping Hub NPC), `trial` naming the second quest, and the second quest with `after_taken`
the first and a `talk` objective at the test NPC. Mark both with a `min_tier` or level no player
reaches by play if the content allows, and say in the closing comment how the fixture is kept
away from players. Tests:

- A level 30 Vanguard holding the test Advancement Quest gets no map mark for the hidden NPC.
  After opening its gossip, `MapMarks` includes it, and a save round trip keeps `met`.
- Lyria shows the hint line only while the quest is held; choosing it shows the reply and changes
  no quest state; after the hand-in (or before the quest is taken) the line is gone.
- The hidden NPC's quest list shows the Advancement Quest incomplete until the trial quest is
  handed in; a hand-in attempt before that leaves the character a Vanguard; after, it advances
  them to Zealot.
- The content check refuses a `trial` naming a missing quest and a `hint` at an NPC that neither
  gossips nor is a friendly spawn (unit tests in `src/quests/content.rs`'s own test module, in
  the form of the checks already there).

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `ground_check.py` flags nothing for
  the fixture NPC.

Assumptions allowed: the save key's name, the option index and text id ranges, how the fixture is
kept off players' paths.
