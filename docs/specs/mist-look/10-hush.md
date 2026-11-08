Title: The Hush: the zone's ambience fades near the edge of the mist (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 6. Blocked by 09-edge-fog (its `closed_distance`, `edge_pull` and `ease`).
Client only: `crates/benilla-app/src/sound/zone.rs`.

## Why

The lore's mist cuts sound out; ordinary people call its silence the Hush (`docs/lore-bible.md`,
lines 19 and 71). Davis wants the ambient sound to fade as a player nears the edge, in its own
ticket (decision 6). Ticket 09 already measures how near the edge is and eases the result; this
ticket turns the zone's ambience bed down by the same curve.

## What to change

| Constant | Value | Meaning |
|---|---|---|
| `HUSH_FLOOR` | 0.15 | the ambience bed's share of its volume at the edge |

In `zone_audio` (zone.rs line 271):

- Take `Res<super::AudioListener>` (its `pos` is in Bevy space; `bevy_to_wow` gives WoW
  coordinates) and the current map (`benilla_world::world_map::CurrentMap`, map 0 when absent),
  and the reached set the way SO:07-client-opening (#633) exposes it in `benilla-app`.
- `target = benilla_world::lighting::edge::edge_pull(closed_distance(map, x, y, EDGE_NEAR_YD,
  reached))`. Keep the applied value on `ZoneAudio` (a new `hush: f32` field, 0 by default) and move
  it with `edge::ease` each frame using `time.delta_secs()`.
- The per-frame ambience volume (lines 408 to 418) multiplies in
  `hush_gain = 1 - (1 - HUSH_FLOOR) * hush`, so the bed plays at 15% at the edge and full volume from
  60 yd out.
- When the loading cover tears the soundscape down (`config.world_hold`, line 298) or the world is
  left (`leave_world`, line 1062), reset `hush` to 0.

Only the ambience bed hushes. Music, footsteps, creature, spell and UI sounds, liquid loops and
positional emitters keep their volume. Submerged, the bed is the underwater one and still hushes;
that is fine.

## Failing tests first

zone.rs's tests: split the gain out as a pure `fn hush_gain(hush: f32) -> f32` and test that it is
1.0 at 0, `HUSH_FLOOR` at 1 and 0.575 at 0.5. If zone.rs's tests already run `zone_audio` in a
minimal app with a fake output, add one that a listener at (-9700, 856.67) on map 0, after enough
frames for the ease to settle, sets the bed's volume to the floor share of a listener at
(-9700, 760); if they do not, the pure test is enough.

## Acceptance

- `cargo test -q -p benilla-app --lib hush 2>&1 | grep -E "FAILED|panicked|test result"` passes.
- `cargo check -q -p benilla-app` is clean. CLAUDE.md and `docs/agents/unattended.md` say how to run
  client builds.
- How it sounds is Davis's play pass and the spec's `unplayed` label's; a capture is silent.

Assumptions allowed: where the listener and map are read, the field's name. Record them as
`Assumed:` lines.
