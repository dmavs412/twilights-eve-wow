Title: Escort ambushes and the escortee's assist (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 2 and 5. Blocked by 03-escort-walker.

Realm code and test content. Paths are under `crates/twilight-realm/` unless they start with
`docs/`. ADR draft 0035 (`docs/specs/escorts/adr-draft.md`) records the design.

## What it does

03-escort-walker made `content/escorts.toml` and `src/escorts.rs`: a private escortee walks a path
for his group, waits, fails and resets. This ticket adds the scripted ambushes along the path (two or
three per escort in the shipped data) and makes the escortee join the fight of a creature that
attacks a group member near him. An ambush comes the first time the escortee reaches its point in a
run, is made for the group only, and opens on the escortee.

## The content

Add to `content/escorts.toml`'s header, and to its parsing in `src/escorts.rs`:

```
[[escort]]
assist_yd     optional, 0 by default; see "Assist" below

[[escort.ambush]]   any number, each at one point
point         the point's number, counting the escort's [[escort.point]] tables from 1 in file order
say           optional; the escortee says it when the ambush comes

[[escort.ambush.creature]]   one or more per ambush
entry         a creature template
level         its level (the Level Bands leave it alone, as an encounter's creatures)
count         optional, 1 by default; that many at the same spot
position      [x, y, z] on the escort's map; z is written 0.0 and set by ground_check.py --fix
```

Each creature table has `entry` and `position` on lines of their own, so `ground_check.py` checks
every ambush creature as a placed creature (a table whose `entry` names a template is a creature,
`tools/twilights-eve/ground_check.py` lines 94 to 100). Load checks (panics): `point` names one of the
escort's points, and every `entry` names a template.

## The rules

**An ambush comes.** When the escortee reaches point N for the first time in this run, each ambush at
N comes:
- Its creatures are made with `Shard::spawn_for` (`src/shard.rs` line 118) for the group's Audience,
  each at the ground spot for its position, `count` times. Nobody outside the group sees them or can
  be hit by them (ADR 0014).
- The escortee says the ambush's `say`.
- On the next tick, once every group client has them in view, each one still idle engages the
  escortee: a creature target, as 02-creatures-fight-creatures allows. `src/encounters.rs` lines 228
  to 246 wait one tick the same way before an encounter's creatures turn on a character. Reuse that
  code if it moves out cleanly.
- The escort keeps the guids. An ambush comes once per run; a new run after a failure brings it again.

**The walk and the ambush.** Nothing new: while the escortee fights, 03's walker waits, and when his
table empties he walks on. Ambushers that turn on players instead (threat) fight on where they are.

**Clearing.** When the escort ends (success or failure), its ambushers, alive or dead, are removed at
its reset, when the group sees the start creature again. They are removed by guid, and
`Shard::despawn_audience` leaves them, as 03 made it leave the escortee. They make no respawn timer.
A killed ambusher's `Kill` goes to `share_kill` as any kill does (02).

**Assist.** Each tick, an escortee that is not engaged attacks the nearest hostile creature within
`assist_yd` of him that is engaged on a living group member and that he may fight (02's rule: some
character admitted by both). He engages it as 02's creature fight does and fights it by threat from
there. The creature can be one of his ambush or one everyone sees. With `assist_yd = 0` he never
assists.

## Test content

The test escort in `content/escorts.toml` (`test_escort`, quest 99821, made by 03) gains
`assist_yd = 20.0` and one ambush at point 2: two Prairie Wolves (2958) at level 20, `count = 2`, at
one spot 15 yards off the path beside point 2, and `say = "Wolves!"`. Mark them in the same test
comment.

## Failing tests first

Extend `tests/escorts.rs` (made by 03; level 60 characters, `NoGround`, the realm clock):

1. **It comes at its point.** Following the escortee, the holder's client is sent two wolves when he
   reaches point 2, and not before. The escortee's "Wolves!" line arrives. Each wolf is sent attacking
   the escortee (`AttackStart` from the wolf to the escortee), not the holder, and the escortee swings
   back.
2. **Only the group sees it.** A third character who was 150 yards away at the start (so outside the
   group) and stands 40 yards from point 2 when the wolves come is never sent them.
3. **Once per run.** With the wolves dead, the escortee walks on to the end and no more wolves come.
   In a second test, the holder leaves him behind after the ambush, the escort fails, and once the
   start creature is back the holder starts again. Reaching point 2 brings two new wolves (new guids).
4. **Cleared at the reset.** After that failure, the group's clients are sent the removal of both
   wolves, alive or dead, by the time the start creature is created again.
5. **Assist.** A level 20 wolf made with `Realm::spawn_for` for the group, attacking the holder 10
   yards from the waiting escortee: the escortee attacks it (`AttackStart` from the escortee to the
   wolf). With the wolf 40 yards from him, he does not.

## Acceptance

- The tests pass, and `cargo test -q -p twilight-realm` passes.
- `python3.15 -I tools/twilights-eve/ground_check.py` flags nothing after `--fix` (the test ambush
  creatures are placed units: write their Z as `0.0` and let `--fix` write it).

Assumptions allowed: the test wolves' spot; whether the ambush's creatures share code with
`src/encounters.rs` or only its pattern.
