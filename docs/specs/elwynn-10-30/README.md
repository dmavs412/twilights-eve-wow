# Elwynn Forest 10–30

Davis's quest and dungeon design for Elwynn from level 10 to 30, grilled 2026-10-20. Read the
decisions below first. Where they disagree with the design files, the decisions win.

- `ELWYNN_10-30_QUEST_DESIGN.md`: every story quest, bounty and dungeon repeatable, by sub-zone.
- `elwynn_quests.json`: the same quests as structured data.
- `elwynn_map.html`: the zone map, with sub-zones in play order, level bands and dungeon markers.

Already built elsewhere: Northshire Valley 1–10 (94001–94009), the Northshire Abbey dungeon, and
Hogger's camp at Forest's Edge (spec #169), which moves from 22 to 28–30 here.

## Decisions (grill 2026-10-20)

1. **The doc's dungeon order wins.** Jasperlode is 15–18, Tower of Azora 19–22 and Hogger's
   Camp 28–30. This replaces spec #169's ladder (mine 15, Forest's Edge 22, Azora 28). The camp
   built in #170–#174 is relevelled to 28–30.
2. **Hogger's camp keeps #169's Noise siege.** The Westbrook items work the Noise meter. Tainted
   Meat (94904) stops the kennel sleepers from adding Noise. Ragtooth's horn (94902) pulls one
   gate's gnolls away once per Run. The sketch map (94903) and the sentry rotation (94905) are
   flavour that point to the hiding spots.
3. **Five specs, one per band, in play order.**
   1. Goldshire and the Farmlands, 10–14.
   2. Crystal Lake, Fargodeep and Jasperlode, 13–18.
   3. Stone Cairn Lake and the Tower of Azora, 16–22.
   4. Eastvale, Ridgepoint, Mirror Lake and Jerod's Landing, 20–26.
   5. Westbrook, Hogger's Camp and the exit quests, 25–30.
4. **Azora's four portal stops are phased and Run-only.** Each one is a Phased Dungeon
   footprint at its real Redridge spot, so it leaves nothing in Redridge's open world.
5. **No story branches.** Grik's camp in Jasperlode is friendly and can't be wiped. Toren always
   runs at 94602, and 94704 always opens after Ridgepoint and always ends with Toren freed.
6. **The Jasperlode boss is the Eye Below**, on a C'Thun eye-stalk model. The Tribute crate
   (95002) holds a shard of the same buried thing that Morganth is collecting.
7. **Hogger is kill only.** There is no Stockades capture turn-in.
8. **The new quest mechanics are all built**, each in the band spec that first needs it.
   - Spec 1: several prerequisites per quest, talk-to-NPC objectives and use-GameObject
     objectives.
   - Spec 3: wave defense (94502).
   - Spec 4: several explore points, and "fails if seen" (94702).
   - Spec 5: escort (94901) and subdue alive (94905).
9. **Item quests ship with placeholder rewards.** They pay their silver plus 3 Tokens and have
   no item choice until a later gear pass. The "zone title" on 95005 is dropped. All copper and
   silver amounts are placeholders.
10. **Only Hogger's camp is at night.** The realm sends the client a night game time while a
    character is inside that phase and restores the real time on leaving. This crosses the
    client/realm seam, so spec 5 starts with an ADR. Every other "dusk" or "night" is in the
    text only, and quest steps trigger those events.
11. **The whole zone is relevelled by sub-zone bands.** A data table gives each Elwynn sub-zone
    a level band, and every vanilla creature that spawns in a sub-zone takes a level from that
    band, including creatures spawned at runtime. Northshire Valley keeps its vanilla levels.
    Creatures we define in `mobs.toml` keep their own level. Phased Dungeon footprints are
    excluded.
12. **The Jasperlode and Azora interiors get their own grills** at the start of specs 2 and 3,
    before their tickets are written.
