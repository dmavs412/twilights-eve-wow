Title: Escort quests (to grill) (#PARENT)
Labels:

A stub spec, not ready for an agent. Decision 11 of the Redridge and Duskwood arc: escort quests
are a new realm system. It gets its own short grill, then its own tickets. Keeshan's quest
(ticket 25, "Missing in Action") is blocked by this spec, and this issue closes when its tickets
land. The grill is required: an escort is a new ECS component family and a new objective kind
(CLAUDE.md, "Grill before architecture").

## What Keeshan's quest needs

- **Missing in Action** (98391, level 42). Marshal Marris (382) in Lakeshire sends the character
  to find Corporal Keeshan (349), missing a week while he scouted Render's Rock. He hides in
  Render's Camp at (-8756.8, -2187.1), a friendly vanilla spawn today. He asks to be walked out.
- **The route.** vmangos has Keeshan's vanilla escort: `creature_movement_template` entry 349,
  68 points, from his hideout through Render's Camp, south down the canyon road and over the
  bridge to Marris at (-9277.2, -2296.2). Point 25 waits 45 s (in vanilla he recovers his gear
  there) and point 43 runs a script (an ambush). The sqlite dump is at
  `C:\Users\davis\wow-data\vmangos\mangos.sqlite`.
- **The fights on the way.** Blackrock come after him: at least one ambush of 3 Blackrock
  Champions (435) or Trackers (615) near point 43, at the band's levels (41-44).
- **The end.** The quest is met when Keeshan reaches Marris alive. It fails if he dies.
- **After.** Keeshan joins the Stonewatch Falls camp (ticket 22's Hub NPCs) and gives
  "What Keeshan Saw" (98392, level 45): something winged over Stonewatch at night.

## Questions the grill must settle

1. **Who sees the escorted Keeshan?** One Keeshan for the server, so two players' escorts
   collide; or a copy per character or group made for its Audience (ADR 0014), as Rumour chests
   are; or the shared spawn while a second player waits for him to respawn.
2. **How does he move?** A fixed path of waypoints (the vmangos route, or one cut down), or he
   follows the character. What happens if the character runs ahead or falls behind: does he wait,
   and how far before the escort fails?
3. **Where does the path live?** A new content file of named paths, rows in the quest file, or
   the creature's row. Does `tools/twilights-eve/road_path.py` help draw it?
4. **What starts and ends it?** Accepting the quest starts the walk at once, or a word with him
   after accepting. The end is his reaching a point, or reaching the ender NPC.
5. **Failure and reset.** He dies, the character dies, logs out, leaves the zone, or a timer runs
   out. After a failure: when does he stand back at his hideout, and can the quest be taken again
   at once (the `fail_if_seen` quests' abandon-and-retake rule)?
6. **Ambushes.** Encounters (#238, `src/encounters.rs`) started by a path point, or a new
   "on reaching point N, spawn these" row. Who do the ambushers attack first?
7. **Does he fight?** Melee only, or a spell or two. His level and health while escorted, and
   whether Head Count (ADR 0029) or the Tier Step touches him.
8. **Group credit.** ADR 0009's rule (everyone within 74 yards when he arrives), or only the
   character who started it.
9. **The client.** Does the walk need anything new on the wire, or is it ordinary creature
   movement? Does the quest log show anything for an escort objective (vanilla shows nothing)?
10. **Saving.** A character logging out mid-escort: does the save hold any escort state, or does
    the escort simply fail?
11. **Keeshan at the camp afterwards.** A Hub NPC everyone sees, whatever their quest state, or one
    seen only by characters who rescued him (no Hub NPC is quest-gated today, so that is new
    code). If he is always at the camp, how is he also hiding in Render's Camp for a character
    who has not rescued him yet?
12. **Testing.** A realm test that walks an escort end to end on the realm clock, one that fails
    it by killing him, and one that checks the ambush fires once.
13. **Reuse.** Is this the wave-defence quest (94502) and Encounters' shape with a moving point,
    or a component family of its own? Which later quests would use it (Elwynn's unbuilt subdue
    and escort quests from spec 5)?
