Title: Per-viewer sight: creatures and Hub NPCs that only some characters see (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 1. Realm code and two test rows. No shipped creature or Hub NPC changes
here: the escort (03-escort-walker) and Keeshan's data ticket (#620) are the first users. Paths
are under `crates/twilight-realm/` unless they start with `docs/`.

## Why

An escort gives each group a private escortee. While their copy walks, the group must not also see
the shared one standing at the hideout. After the rescue, Keeshan stands at the Stonewatch Falls
camp, but only for characters who handed in his quest, and the hideout Keeshan is gone for them.
ADR 0014's Audience can only narrow who sees a creature made at run time
(`Mob::for_audience`, `src/mobs.rs` line 1413). It cannot hide a shared spawn from a few
characters, and no Hub NPC is gated by anything (`hub::in_view`, `src/hub.rs` line 124, takes only
a position). This ticket adds both rules. ADR draft 0035 (`docs/specs/escorts/adr-draft.md`)
records them; copy it to `docs/adr/0035-escorts.md` in this ticket if no other escort ticket has.

## The rules

**Sight by quest state.** A sight rule has `after` and `before`, each an optional quest id.
- `after = Q`: seen only by a character with Q in `quests.completed` (handed in).
- `before = Q`: seen only by a character without Q in `quests.completed`.
- Both may be set. A thing with no rule is seen as today.

Where the rules are written:
- A Hub NPC row in `content/hub.toml` gains an optional `seen = { after = ..., before = ... }`
  (a new `Npc` field, `src/hub.rs` line 26).
- A spawn row in the generated zone files cannot be edited, so `content/zones.toml`'s
  `[[zone]]` gains an optional `seen = [{ guid = ..., after = ..., before = ... }, ...]`, keyed by
  the spawn's guid in that zone's file or in its `[[zone.added]]` rows (`ZoneOverrides`,
  `src/mobs.rs` line 587, applied where the zone files load, around line 738). The rule goes on
  the `Spawn` (`src/mobs.rs` line 366) as a new field, and from there on each `Mob` made from it.
  A rule naming a guid the file does not have is a load panic, as other content mistakes are.
- Only a friendly creature (template `friendly = true`) may carry a rule. A rule on a hostile
  spawn is a load panic. This keeps sight out of aggro and threat.

**Hidden from, at run time.** `Mob` (`src/mobs.rs` line 1217) gains `hidden_from: Vec<u64>`, the
characters it is hidden from while everyone else sees it. `Mob::admits` (`src/mobs.rs` line 1871)
returns false for them, so every caller of `admits` treats them as outside the creature's
Audience. Add `Shard::hide_from(guid, &[u64])` and `Shard::unhide(guid, &[u64])` next to
`Shard::spawn_for` (`src/shard.rs` line 118). Nothing in this ticket calls them except tests. The
escort calls them every tick for its start creature, because the Population can stream a spawn
out and back in as a new `Mob`, which starts with an empty list.

## Where sight is read

- **The view.** `Shard::in_view` (`src/shard.rs` line 518) and `hub::InView::in_view`
  (`src/hub.rs` line 433) gain the viewer's handed-in quests and drop what the viewer does not
  see. Carry them on `shard::OnShard` (`src/shard.rs` line 46) as a new `completed: Vec<u32>`,
  filled from `c.quests.completed` where the players are gathered (`src/lib.rs` line 1392), and
  pass them at the view call (`src/lib.rs` line 1466). The view diff right below it already sends
  the removal of what left the view and the create of what entered it, so a hand-in shows and
  hides things on the next tick with no more code. `World` also checks Hub NPCs in view at
  `src/world/phase.rs` line 677; give it the same check.
- **Using a Hub NPC.** A character cannot use a Hub NPC they do not see: the gossip menu
  (`Realm::near`, `src/hub.rs` line 330), the quest giver dialogs (`hub_speaker`,
  `src/quests/dialogs.rs` line 310, and the dialog path that calls it), the vendor
  (`src/vendor.rs` line 338) and the innkeeper (`src/inn.rs` line 93).
- **Using a creature with a rule.** `field_gossip_hello` and `field_gossip_select`
  (`src/quests/dialogs.rs` lines 237 and 262) and the Field Giver's quest dialogs refuse a creature
  whose rule hides it from the character.
- **Map marks.** `src/map_marks.rs` line 31 lists quest givers' marks. Skip a Hub NPC the character
  does not see. A creature-giver mark, if the file has one, follows the creature's rule.

## Headers

- `content/hub.toml`: one paragraph on `seen` in the header's field list.
- `content/zones.toml`: one paragraph on `seen` in the header (lines 1 to 6), saying it is keyed by
  spawn guid and allowed only on friendly creatures.

## Failing tests first

New `tests/sight.rs`, in the style of `tests/audience.rs` and `tests/encounters.rs` (a `Party` from
`common::party`, characters edited with `Party::edited`, a handed-in quest written into the blob as
`completed = [99811]`, the way `tests/crystal_lake.rs` line 52 does it). Test content, marked in a
comment as belonging to `tests/sight.rs` the way `content/encounters.toml` marks its test rows:

- A `[[zone.added]]` row in `content/zones.toml`'s redridge `[[zone]]` (suggested guid 960811) of a
  friendly creature template that no shipped quest names, at a Redridge spot with no other spawn or
  Hub NPC within 120 yards, with `seen = [{ guid = 960811, before = 99811 }]`.
- A Hub NPC row in `content/hub.toml` (a guid clear of every other row's, entry 199811,
  `role = "ambient"`, `map = 0`) 10 yards from it, with `seen = { after = 99811 }`.
- Test quest 99811, added with `Realm::with_quests`: no objectives, given and ended by the
  zone row's creature (so it is a Field Giver).

Tests:
1. Two characters stand by the two test rows. The one without 99811 handed in is sent the creature
   and not the Hub NPC; the one with it is sent the Hub NPC and not the creature.
2. A character holding 99811 hands it in at the creature. Within a few ticks their client is sent
   the creature's removal and the Hub NPC's create. The other character's view does not change.
3. A character who does not see the Hub NPC gets no gossip menu from `GossipHello` on it, and one who
   does not see the creature gets none from it.
4. A unit test in `src/shard.rs`: `hide_from` takes a creature out of one character's
   `in_view` and leaves it in another's, `admits` answers false for the hidden one, and `unhide`
   puts it back.

## Acceptance

- The tests pass, and `cargo test -q -p twilight-realm` passes.
- `python3.15 -I tools/twilights-eve/ground_check.py` flags nothing after `--fix` (the two test
  rows are placed units: write their Z as `0.0` and let `--fix` write it).

Assumptions allowed: the test rows' ids, spot and template; whether `before`/`after` sit in one
`Sight` struct or two fields.
