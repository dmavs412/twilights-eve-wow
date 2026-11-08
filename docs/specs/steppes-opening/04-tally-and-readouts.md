Title: The Tally, its two readout NPCs, and the opening's line on /1 and Toren's yell (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 1, 9 and 10. ADR 0034. Blocked by 03-milestone-gated-content (same
`src/hub.rs`, `src/lib.rs`, `src/quests/dialogs.rs` and `tests/milestones.rs`) and
RD:12-redridge-35-40 (it adds Toren in Lakeshire, Hub NPC entry 190083, who yells).

## What

`steppes_open` becomes a Tally: a count that the camp's quests feed after the Anchor breaks, with
a fixed goal. Reaching the goal reaches the Milestone, which opens the Burning Steppes (ticket 02's
wall reads the reached set) and sends a line on `/1` and Toren's yell to every character in the
world. Two NPCs say in words how far the Tally has come. Ticket 06 fills `feeds` with the camp's
quests.

## The catalogue

In `content/milestones.toml`, document four optional fields in the header and give them to
`steppes_open`:

```toml
# goal       optional: the count that reaches this Milestone. It makes the Milestone a Tally.
# feeds      with a goal: [{ quest, points }]. Each hand-in of `quest` by any character adds
#            `points` to the count while `after` is reached and this Milestone is not.
# announce   optional: a line every character in the world sees on /1 when it is reached
# yell       optional: { npc, text }: Hub NPC `npc` yells `text`, heard by every character in the
#            world, when it is reached

[[milestone]]
key = "steppes_open"
name = "The Burning Steppes are open"
after = "ilgalar_anchor_broken"
# Points are roughly solo minutes: a camp bounty 15, a camp Writ 120 (ticket 06). 540 is nine
# hours of bounties alone (#PARENT decision 1: "about 8 to 10 hours").
goal = 540
feeds = []
announce = "The mist has rolled off Blackrock Pass. The road into the Burning Steppes is open."
yell = { npc = 190083, text = "The pass is clear! I can see the rocks above Render's Camp from the inn. That was his stone, and it held twelve years. It doesn't hold now." }
```

The load panics on `feeds` without a `goal`, a fed quest the content lacks, a quest fed twice, a
`goal` below 1, and a `yell.npc` that is no Hub NPC entry.

## The count

- In `src/milestones.rs`, a `feed(quest_id)` that the hand-in calls. For each Milestone that feeds
  the quest, whose `after` is reached and which is not reached, it adds the points to `count` and
  marks the state dirty (ticket 01 writes the file at most once a tick). When `count` reaches
  `goal`, it calls the realm's `reach_milestone` path. The count stops at the goal: hand-ins after
  add nothing.
- The hand-in is `reward` in `src/quests/dialogs.rs` (line 707), after the quest is recorded as
  done. Each character's own hand-in counts once. Kill credit shared under ADR 0009 does not add a
  hand-in.
- A state file whose `count` is already at or past `goal` with no `reached` reaches the Milestone
  when it is read (`set_milestones`), with the wall clock's date, and writes the file.

## The opening

`reach_milestone` (ticket 01) gains, for a Milestone with `announce` or `yell`:

- `announce`: one `CHAT_MSG_CHANNEL` line on the world channel (`chat::WORLD_CHANNEL`, see
  `src/chat.rs` line 112) to every character in the world. Its sender is the Stormwind herald
  (Hub NPC guid 9300089, below), and the realm answers a name query for that guid with his name if
  it does not already. If the client cannot show a channel line from a non-player guid, send it
  as `CHAT_MSG_SYSTEM` text `[1. World] <line>` instead and record that.
- `yell`: one `CHAT_MSG_MONSTER_YELL` line from that Hub NPC's guid and name to every character in
  the world, wherever they stand, built the way `src/chat.rs`'s creature lines are (`due_speech`,
  line 233).
- Both go out on the tick the Milestone is reached, after `SessionEvent::Milestones`.

## The readout NPCs

A Hub NPC row may carry a `[npc.readout]` table. Its gossip greeting then comes from the Tally's
stage instead of `greeting`/`text_id`, which it does not need:

```toml
[npc.readout]
milestone = "steppes_open"
before = { text_id = 64301, text = "..." }     # the Milestone's `after` is not reached
progress = [                                   # after reached, this one not: the last row whose
  { from_pct = 0, text_id = 64302, text = "..." },   # from_pct <= count x 100 / goal, rounded down
  { from_pct = 25, text_id = 64303, text = "..." },
  { from_pct = 50, text_id = 64304, text = "..." },
  { from_pct = 75, text_id = 64305, text = "..." },
  { from_pct = 90, text_id = 64306, text = "..." },
]
reached = { text_id = 64307, text = "..." }
```

- `gossip_hello` (`src/hub.rs` line 269) sends the stage's `text_id`, and `greeting` (line 251)
  answers each of a readout's text ids with its text. Each stage has its own id because the
  client asks for a text only on its first visit to that id and keeps it (`gossip_menu` in
  `crates/benilla-app/src/ui_gossip/net.rs`).
- The load panics on a readout whose Milestone has no `goal`, whose `progress` does not start at
  0 or does not rise, or whose text ids clash with any other Hub NPC's.
- Document `readout` in `content/hub.toml`'s header.

Two rows in `content/hub.toml`, under a new section header
`# ── The Burning Steppes Tally's readouts (#PARENT) ──` placed directly after the Goldshire inn
and shops section (the one that starts "Goldshire's inn and shops (#459)"), not at the end of the
file. Both are `role = "ambient"`, `npc_flags = 0x1`, `faction_template = 35`, `level = 30`,
`map = 0`, `display_id = 3343` (the Stormwind guard body Sergeant Keane wears), with no `text_id`
and no `greeting`. Names and bodies are placeholders.

**Herald Osric Penn**, subname "Stormwind Herald", guid 9300089, entry 190088, at
(-8824.0, 667.0) beside the Writ Board, facing it (`faces = [-8821.0, 663.0]`). He stands inside
Stormwind's city model on the street floor: `wmo_floor.py` lists group 277 at z 95.43 there.
Write that Z, then run `ground_check.py`. His texts:

- 64301 before: "No word from the east worth crying, $C. Redridge says the grey over Blackrock Pass hasn't moved in twelve years. I'll cry it the day it does."
- 64302 from 0: "Word from Redridge: the tower on the hill is down, and the grey over Blackrock Pass has begun to thin. The Watch has a camp at the pass. Things are walking out of the mist, and the captain needs swords to put them down. They've barely started."
- 64303 from 25: "The camp at Blackrock Pass holds, and the mist keeps thinning. A quarter of the way, the captain writes. Perhaps a little more."
- 64304 from 50: "Half the pass is clear, by the captain's count. You can see the road climbing into the rocks now, where there was only grey."
- 64305 from 75: "Three parts done, the captain writes. The grey over the pass is a curtain now, not a wall."
- 64306 from 90: "Any day now. The last report says the mist over the pass is a rag. One more good push."
- 64307 reached: "The pass is open, $C. The Burning Steppes lie beyond it, the first land the Hush has given back. Marshal Maxwell holds Morgan's Vigil on the far side."

**Watchman Hollis Pell**, subname "Lakeshire Watch", guid 9300090, entry 190089, at
(-9255.0, -2205.0) in Lakeshire between the town hall and the inn, outdoors (`wmo_floor.py` finds
only terrain there, z 63.93), orientation 4.7. His texts:

- 64311 before: "The pass? Grey as ever. Nothing goes up that road and nothing comes down it. Ask me again when that changes."
- 64312 from 0: "Since the tower fell, the grey over the pass has started to thin, and what lived behind it is walking out. The captain's camp up the road counts every one put down. We've hardly started."
- 64313 from 25: "A quarter of the way, by the captain's tally. The road shows through the grey for the first stretch."
- 64314 from 50: "Halfway. On a clear morning you can see rocks past the border stones now."
- 64315 from 75: "Three parts of the way. The grey's thin enough to see torches behind it at night. Theirs, not ours."
- 64316 from 90: "Nearly there. The captain says one more good week. Or one good night with enough swords."
- 64317 reached: "The pass is open. Follow the road up and you'll come to Morgan's Vigil. The orcs at Flame Crest fly Blackrock colours now, so give them a wide berth."

## Glossary

`GLOSSARY.md`: a **Tally** entry after **Milestone**: "A Milestone's count toward its goal (ADR
0034): each hand-in of a quest it feeds adds points, for the whole server, and reaching the goal
reaches the Milestone. Shown by NPCs in words, never on screen." `_Avoid_: progress bar, meter`.

## Failing tests first

In `crates/twilight-realm/tests/milestones.rs`, with
`Realm::new().with_milestone_catalogue(text)` where `text` is the shipped catalogue with
`goal = 30` and `feeds = [{ quest = <q>, points = 15 }]`, and `<q>` a `bounty` quest added with
`realm.with_quests(toml)` from a Northshire Hub NPC, as `tests/quest_mechanics.rs` builds quests:

- Before `ilgalar_anchor_broken`, handing in `<q>` adds nothing.
- After `reach_milestone("ilgalar_anchor_broken")`, one hand-in makes the count 15 and writes a
  state file with `count = 15`; the second reaches `steppes_open`.
- On that tick every character in the world (two players) gets `SessionEvent::Milestones` with
  both keys, the `/1` line and Toren's yell, and the step from (-8690.0, -2560.0, 132.0) to
  (-8650.0, -2560.0, 132.0) is no longer refused (ticket 02's test, now through the Tally).
- A third hand-in changes nothing and writes nothing.
- A state file with `[steppes_open] count = 600` and the Anchor broken reaches `steppes_open` at
  `set_milestones`.
- Gossip at the herald: before the break the menu carries 64301; after the break with count 0,
  64302; with count 134 of 540 (24%), 64302; with 135 (25%), 64303; with 486 (90%), 64306; once
  reached, 64307. `CMSG_NPC_TEXT_QUERY` for each id answers its text. Set the count through a
  state file read with `set_milestones`.
- In `src/milestones.rs`'s tests: `feeds` without `goal` panics, and a quest fed twice panics.

## Acceptance

- The tests pass. `cargo test -q -p twilight-realm` passes. `cargo check -q -p twilight-realm` is
  clean.
- `python3.15 tools/twilights-eve/ground_check.py` passes.

Assumptions allowed: the sender of the `/1` line if the herald's guid does not work (record what
you chose); inner names; the hub row field layout if `[npc.readout]` clashes with an existing
field name. Record them as `Assumed:` lines.
