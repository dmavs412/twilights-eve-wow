Title: The escort walker: a private escortee walks a path for his group (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 1, 3 and 4. Blocked by 01-per-viewer-sight and 02-creatures-fight-creatures.

Realm code, a new content file with one test escort, and one test row. Ambushes and the escortee's
assist are 04-escort-ambushes; leave them out. Paths are under `crates/twilight-realm/` unless they
start with `docs/`. ADR draft 0035 (`docs/specs/escorts/adr-draft.md`) records the design; copy it to
`docs/adr/0035-escorts.md` in this ticket if no other escort ticket has.

## What it does

A character holding the quest picks a gossip line on the escortee at his start spot. A private copy
of him, seen by that character and everyone near them, walks a fixed path, waits when they fall
behind, and fights what attacks him (02 made him a defender). When he reaches the end, every member
of the group holding the quest has it met. If he dies, or no living group member has been near him
for a few seconds, the escort fails: the quest stays in the log and is not failed, and he is back at
his start spot a few seconds later for another try. Nothing names a quest in the code; the module
reads `content/escorts.toml`, as `src/encounters.rs` reads `content/encounters.toml`.

## The content file

New hand-written `content/escorts.toml`, with a header in `content/encounters.toml`'s style that
lists every field below. Add it to `src/build_id.rs` beside `encounters.toml` (line 21).

```
[[escort]]
key           the escort's name, unique
quest         the quest whose holders may start it; met for every group member holding it on arrival
map           the map, 0 (Eastern Kingdoms) by default; open world only
spawn         the low guid of the friendly spawn row at the start: the creature players talk to.
              Its entry is the escortee's.
level         the escortee's level while escorted
health_mul    multiplies the escortee's health while escorted; 1.0 by default
start_option  the gossip line that starts the walk
start_say     optional; the escortee says it as he sets off
end_say       optional; he says it on reaching the last point
failed_text   sent to every group member as a system line when the escort fails
wait_yd       he stops and waits while no living group member is within this
leash_yd      the escort fails once no living group member has been within this for `leash_s`
leash_s       seconds
reset_s       seconds after the escort ends (success or failure) before the group sees the start
              creature again
linger_s      seconds he stands at the last point before he goes

[[escort.point]]  in order; the first is his first step from the start spot, the last is the end
at            [x, y]; the Z comes from the host's ground when he walks there
wait_s        optional; seconds he stands here
say           optional; he says it on reaching the point
run           optional; true makes him run from here on
```

The `[[escort.point]]` tables carry `at`, not `position`, on purpose: `ground_check.py` treats any
`position = [x, y, z]` line as a placed unit, and a path point is not one. Load checks (panics, as
`encounters::content`, `src/encounters.rs` line 65, does): keys unique, at least two points, `spawn`
names a friendly spawn row on `map`, `quest` is a quest the realm has, and no two escorts share a
quest or a spawn.

## The rules

**The start line.** `field_gossip_hello` (`src/quests/dialogs.rs` line 237) adds an option for an
escort's start creature (the creature of spawn row `spawn`, found by `Mob::spawn_row`,
`src/mobs.rs` line 1765) when the character holds the quest, it has not failed, its escort counter
is 0, and the character is in no running escort's group. Its index is a new `ESCORT_OPTION = 90`,
clear of the hint lines (`OPTION_HINT` 80 to 89, `src/world/fishing.rs` line 473) and the talk lines
(`TALK_OPTION` 100, `src/quests/mod.rs` line 280); its text is `start_option`. Add it before the
early return for an empty menu. The start creature needs the gossip flag in its create
(`src/mobs.rs` lines 1910 to 1914): an escort's start entry shows `UNIT_NPC_FLAG_GOSSIP` as a talk
target does. `field_gossip_select` (`src/quests/dialogs.rs` line 262) on `ESCORT_OPTION` closes the
menu (`SessionEvent::GossipComplete`) and starts the escort if the same checks still pass.

**Starting.**
- The group is the starter plus every character on the escort's map within `content/group.toml`'s
  `credit_range_yd` (74 yards) of the start creature, alive or dead, who is in no running escort's
  group. It is fixed then, as an Audience is (ADR 0014).
- The start creature is hidden from the group with 01's `Shard::hide_from`. The escort applies the
  hide again every tick while it runs and until its reset, because the Population can stream the
  spawn out and back in as a new `Mob` with an empty `hidden_from`.
- The escortee is made with `Shard::spawn_for` (`src/shard.rs` line 118) at the start creature's
  position: the spawn's entry at `level`, with the group as its Audience. Its maximum health and
  health are multiplied by `health_mul`. It is a defender (02), never leashes, never strolls, and
  faces as the start creature does. He says `start_say`.
- Everyone outside the group still sees the start creature and can start an escort of their own.
  Several escorts of one `[[escort]]` row may run at once, one per group.

**The tick.** A new `Realm::tick_escorts(dt, host)` runs beside `tick_encounters` (`src/lib.rs`
line 2354), built the same way (`src/lib.rs` lines 1662 to 1702): it gathers every character in the
open world of the escort's map from every session (guid, position, alive, their active quests),
works on the shard's mobs, and hands what it says to each session. The escortee's moves and lines
go to every session whose world has him in view (`w.seen`, as the shard tick hands out its `made`
events at `src/lib.rs` lines 1440 to 1462). The `Realm` holds a new `escorts: escorts::Escorts`
beside `encounters` (`src/lib.rs` line 731). Per running escort, each tick:

1. **Gone or dead.** An escortee missing from the shard's mobs, or dead, fails the escort.
2. **Leash.** If no living group member is within `leash_yd` of the escortee, a timer runs; it
   resets when one comes back. At `leash_s` the escort fails.
3. **Fighting.** While the escortee is engaged, the walker does nothing. When the fight ends, 02
   leaves him idle where he stopped, and the walk goes on from there.
4. **Waiting.** While no living group member is within `wait_yd`, he stops where he is
   (`Mob::halt`, `src/mobs.rs` line 2043) and waits. He goes on when one comes back.
5. **Walking.** Otherwise he walks to the next point: `Mob::walk_path` (`src/mobs.rs` line 2010) at
   100 percent, or `Mob::run_path` (line 1978) once a point with `run = true` is behind him, to the
   point's `at` with the Z from `crate::ground::ground_under` (his own Z where the host has no
   ground). The tram already walks porters this way (`walk_toward` and `there`, `src/tram.rs` lines
   1498 and 1504, private today); make them shared if that fits. The owning session's `mob_tick`
   moves him along the spline (`Mob::advance`, `src/world.rs` line 1268).
6. **A point reached.** His `home` becomes the point, so nothing ever sends him back to the start
   spot. He says the point's `say`, stands `wait_s`, then goes on.
7. **The end.** On reaching the last point: credit (below), `end_say`, `linger_s` standing, then he
   is removed.

He speaks as a creature does: a `CHAT_MSG_MONSTER_SAY` from his guid under his template's name
(build it as `due_speech` does, `src/chat.rs` line 233), to every group member within `SAY_YD`
(25 yards, `src/chat.rs` line 15).

**Failing.** The escortee is removed (by guid, not with `despawn_audience`), and `failed_text` goes
to every group member online as a system line. The quest is untouched: still in the log, not failed,
its escort counter 0, and no `QuestFailed` is sent. After `reset_s`, 01's `Shard::unhide` shows the
start creature to the group again and the escort is dropped. Any group member who holds the quest
can then start a new one. A dead member does not fail it, and a member who logs out counts as gone.

**Credit.** On arrival, every group member who holds the quest (in the log, not failed) and is on the
map, alive or dead, wherever they stand, gets the escort counter set to 1. Add
`quests::credit_escort(c, quest, events)` beside `credit_talk` (`src/quests/mod.rs` line 340): it
sets the counter, calls `update_slot` (line 526) so the client's log turns complete, and the realm
saves the character. After `reset_s` from the arrival the start creature is shown to the group again.

**The quest objective.** An escort's quest is met when its escort counter is 1.
- Add `Quest::escort_slot` after `explore_slot` (`src/quests/content.rs` line 471): the slot after
  the listed explore spots.
- `complete` (`src/quests/mod.rs` line 176) asks for that counter when an escort names the quest.
- The four-counter check (`src/quests/content.rs` line 847) counts one more for such a quest.
- The quest's template response (`template_response`, `src/quests/dialogs.rs` line 17) already sends
  an empty objective for a slot past the explore spots, so the log shows no counter line. Keep it so.
- The counter is saved in `kills` as a talk objective's is (`src/quests/mod.rs` line 112). No save
  change, and no escort state is saved.
- One line in `content/quests.toml`'s header: an escort in `content/escorts.toml` can name a quest,
  and that quest is met when the escortee arrives.

**A Rumour ending.** `Shard::despawn_audience` (`src/shard.rs` line 141) removes every creature made
for exactly an Audience. A solo character's Rumour and that character's escort have the same
Audience, so a Rumour ending would take the escortee. It must leave the creatures an escort made.

**Glossary.** Add **Escort** and **Defender** to `GLOSSARY.md`, two or three sentences each, in its
style.

## Test content

In `content/escorts.toml`, one test escort, marked in a comment as belonging to `tests/escorts.rs`
the way `content/encounters.toml` marks its test rows: key `test_escort`, quest 99821, `level = 30`,
`health_mul = 1.0`, `wait_yd = 20.0`, `leash_yd = 60.0`, `leash_s = 10.0`, `reset_s = 5.0`,
`linger_s = 3.0`, short texts, and four points 20 yards apart in a straight line from the start
spot. Its `spawn` is a new `[[zone.added]]` row in `content/zones.toml`'s redridge `[[zone]]`
(suggested guid 960821) of a friendly template that no shipped quest names, at a Redridge spot with
no other spawn within 150 yards of the start or the path. Quest 99821 is added in the test with
`Realm::with_quests` and put in the log through the blob, as `tests/encounters.rs` lines 30 to 85 do.

## Failing tests first

New `tests/escorts.rs`, set up as `tests/encounters.rs` is (`Party::edited`, `NoGround`, the realm
clock advanced with `party.run`). The characters follow the escortee with `party.go` when a test
needs them to keep up.

1. **Start to finish.** The holder's `GossipHello` on the start creature lists option 90 with the
   start text. Choosing it: their client is sent the start creature's removal and a new creature of
   the same entry (the escortee), then `MonsterMove`s for him. Following him, he reaches the last
   point, and the client is sent the quest's log slot in the complete state. After `linger_s` he is
   removed; after `reset_s` from the arrival the start creature is created again for them.
2. **The group.** A second character 30 yards away at the start, holding the quest, is sent the
   escortee and is credited on arrival without standing near the end. A third character 150 yards
   away, holding the quest, is never sent that escortee, still sees the start creature, and can start
   an escort of their own while the first runs (a different escortee guid).
3. **Waiting.** The holder stops and stays 30 yards behind him (outside `wait_yd`, inside
   `leash_yd`): within a second he is sent a stop and stays put; when they come within 20 yards he
   walks on.
4. **Left behind.** The holder goes 100 yards away and stays. After 10 seconds the failed text
   arrives and the escortee is removed. The quest is still in the log and not failed (no
   `QuestFailed`). After 5 more seconds the start creature is created again, and choosing the line
   starts a new escort.
5. **Killed.** A level 60 Prairie Wolf (2958) made with `Realm::spawn_for` for the group, beside the
   escortee, kills him. The failed text arrives, the quest is in the log and not failed, and after
   `reset_s` a new escort can start.
6. **Who is offered the line.** A character without the quest, one with the escort already met, and a
   group member while their escort runs get no option 90.

## Acceptance

- The tests pass, and `cargo test -q -p twilight-realm` passes.
- `python3.15 -I tools/twilights-eve/ground_check.py` flags nothing after `--fix` (the test
  `[[zone.added]]` row is a placed unit: write its Z as `0.0` and let `--fix` write it).

Assumptions allowed: the test row's id, spot and template; whether the escort's lines and moves
reach the clients through the realm tick or through the shard's `made` events; the shape of the
running-escort state in `src/escorts.rs`.
