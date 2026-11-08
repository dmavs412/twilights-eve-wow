Not a ticket. Post this as a comment on #620 (Keeshan: Missing in Action), after the escort spec
#596 is filed; the latest comment wins (`docs/agents/unattended.md`). Replace each NN-name below
with its issue number from `docs/specs/escorts/.filed.json` before posting.

---

The escort spec #596 was grilled on 2026-11-08 and its tickets (01-per-viewer-sight,
02-creatures-fight-creatures, 03-escort-walker, 04-escort-ambushes) built the escort system. Where
this ticket and the spec differ, the spec wins, as the body says. These are the differences, and
the data they mean for this ticket. Everything not listed here stands as the body says (the two
quests' rows and text, the camp Keeshan's Hub row, the gear choices).

## What changed

1. **The start.** Accepting 98391 from Marris starts nothing. A character holding 98391 picks a
   gossip line on Keeshan at his hideout (spawn 17874 in Render's Camp), and a private copy of him
   sets off for that character and everyone within 74 yards of him.
2. **The failure.** Keeshan dying, or no living group member within `leash_yd` of him for `leash_s`,
   ends the escort. 98391 does not fail and stays in the log. He stands at his hideout again after
   `reset_s`, and the same line starts a new walk.
3. **The ambushes.** Two, not one (the grill asked for two or three along the path).
4. **The camp Keeshan.** He is seen only by characters who handed in 98391, and the Render's Camp
   Keeshan only by characters who have not. 01-per-viewer-sight made both rules.
5. **No quest field.** 98391's row needs nothing escort-specific. The escort row names the quest.

## The data

**`content/escorts.toml`**, one `[[escort]]` in the schema its header gives (03 and 04 wrote it):
- `key = "keeshan"`, `quest = 98391`, `spawn = 17874`, `level = 42`, `health_mul = 3.0`.
- `start_option = "I'm here to get you out. Lead the way."`
- `start_say = "About time. Stay close, they patrol the ridge."`
- `end_say = "Marshal. I've got a report you won't like."`
- `failed_text = "Corporal Keeshan has slipped back to his hideout above Render's Camp."`
- `wait_yd = 20.0`, `leash_yd = 60.0`, `leash_s = 10.0`, `reset_s = 10.0`, `linger_s = 10.0`,
  `assist_yd = 20.0`.
- **The path.** vmangos's `creature_movement_template` rows for entry 349 (68 points) in the sqlite
  dump (`C:\Users\davis\AppData\Local\Temp\vmangos\sqlite-dump\mangos.sqlite`), thinned to about 30
  `[[escort.point]]` tables with `at = [x, y]` (no Z). Keep a point wherever the vanilla path turns
  by more than about 20 degrees, and keep these four with their extras:
  - vanilla point 25: `wait_s = 10.0`, `say = "I need a moment to rest."` (vanilla sat 45 s here);
  - vanilla point 26: `say = "Let's go!"`;
  - vanilla point 43 (-8914.7, -2579.3): `run = true`, `say = "So close to blessed Lakeshire."`;
  - the last point, vanilla 67, by Marshal Marris (-9277.2, -2296.2).
  Thin the rows with a script under `target/agent/`; no new tool.
- **Two ambushes** (`[[escort.ambush]]`, `point` counted in the thinned list):
  - at the point that was vanilla 25: `say = "Blackrock! Behind us!"`, two Blackrock Trackers (615)
    at levels 41 and 42, 10 to 15 yards back along the path;
  - at the point that was vanilla 43: `say = "Here they come!"`, three Blackrock Champions (435) at
    levels 42, 42 and 43, 10 to 15 yards ahead of him on the road.
  Each creature's `position` is `[x, y, 0.0]`; `ground_check.py --fix` writes the Z.

**`content/zones.toml`**, the redridge `[[zone]]`: `seen = [{ guid = 17874, before = 98391 }]`.

**`content/hub.toml`**, the camp Keeshan's row (guid 9300088): `seen = { after = 98391 }`.

## The tests, in place of the body's "Failing tests first" list

`tests/redridge_30_50.rs`, in the form `tests/escorts.rs` uses:
- Accepting 98391 from Marris starts no walk. With 98391 in the log, Keeshan's start line at Render's
  Camp starts it.
- Keeshan reaching Marris meets 98391 for every group member holding it.
- Keeshan dying ends the escort: 98391 stays in the log and is not failed, he is at his hideout again
  after `reset_s`, and the line starts a new walk.
- Each ambush comes once per escort.
- 98392 is offered by the camp Keeshan only once 98391 and 98401 are handed in, and reaching the rocks
  credits it. A character who has not handed in 98391 is not sent the camp Keeshan; one who has is
  not sent the Render's Camp Keeshan.
- Every story quest in the gear table offers exactly the `choices` it names (unchanged).

Assumptions allowed, on top of the body's: the thinned points, `health_mul`, the ambush spots and
levels within 41 to 44, the start line and the failed text.
