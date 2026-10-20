# Twilight's Eve: first slice

Labels: `ready-for-agent`

## Problem Statement

The only game here is Survivors, a short top-down arcade run. I want to play a version of
*Twilight's Eve*, the Warcraft III custom RPG, on this WoW 1.12.1 engine. It plays like normal
WoW: I start as a Basic Warrior, level up by doing Repeatable Quests and running Dungeons, earn
gear, and slowly advance through Tiers of Callings. All of it should work offline in the
browser with no server to run. It should still be built so that a real server can replace
the simulation later without rewriting the game.

## Solution

A new Mode, Twilight's Eve, started with its own boot flag. The client runs in its normal-WoW
mode (stock character create and select, quest log, spellbook, talent frame, trainer, loot and
character sheet) against a **Local Realm**: a simulated server that runs inside the client and
speaks the same protocol a real server does. The first slice is the Warrior line, Tier 0
(Basic Warrior) and Tier 1 (Warrior), up to level 30. It is played in The Barrens, with the
Crossroads as the Hub and the Wailing Caverns as the one Dungeon. Vocabulary follows
`GLOSSARY.md`; the architecture follows ADR 0001.

## User Stories

### Starting out

1. As a player, I want to launch Twilight's Eve from its own launcher (`twilights-eve.exe`), so that it starts without a login screen.
2. As a player, I want the client to boot straight to the stock character select screen, so that it feels like logging in to a WoW realm.
3. As a player, I want to create a character on the stock create screen, picking name, race, gender and appearance, so that my hero is my own.
4. As a player, I want every race to be available, so that I'm not limited by faction.
5. As a player, I want Basic Warrior to be the only class I can pick, so that everyone starts the climb at Tier 0.
6. As a player, I want several characters (up to 10) on the select screen, showing level, zone and gear, so that I can keep more than one run going.
7. As a player, I want the select screen to show my Calling (e.g. "Level 7 Basic Warrior"), so that I can see where each character is on the advancement tree.
8. As a player, I want a new character to appear in the Crossroads, so that I start at the Hub.
9. As a player, I want to delete a character from the select screen, so that I can clear out old ones.

### Moving and the world

10. As a player, I want to move, jump and turn with the normal WoW controls and camera, so that it plays like normal WoW.
11. As a player, I want The Barrens to be filled with its real vanilla creature camps (plainstriders, quilboars, Kolkar, raptors, harpies…), so that the world feels like the place I remember.
12. As a new Basic Warrior, I want a few level 1–6 camps near the Crossroads, so that I have something I can actually fight at level 1.
13. As a player, I want mobs to respawn about a minute after I leave their area, so that the grind never runs dry.
14. As a player, I want a Hearthstone to the Hub on a short cooldown (about 5 minutes), so that long walks back don't eat my session.
15. As a player, I want friendly NPCs at the Hub and hostile mobs in the Hunting Grounds to show proper names, levels and models, so that the world reads as WoW.

### Combat

16. As a player, I want vanilla 1.12 melee combat (swing timer, rage from dealing and taking damage, the miss/dodge/parry/block/glancing/crit table, armour), so that fights feel like WoW.
17. As a player, I want mobs above my level to be harder to hit, so that pulling up has a real cost.
18. As a player, I want my character sheet to show accurate stats (attack power, crit, dodge, armour, health), so that I can judge gear with the stock UI.
19. As a player, I want floating combat text and the combat log to show my hits and misses, so that I can read fights.
20. As a player, I want mobs to aggro by vanilla's radius rules (bigger for higher-level mobs, smaller for grey ones), so that I can pick my pulls.
21. As a player, I want nearby mobs of the same group to join in (social aggro), so that camps fight like camps.
22. As a player, I want mobs to chase me and attack, so that combat has pressure.
23. As a player, I want mobs I drag too far from home to leash back, heal and reset, so that I can escape and they don't follow forever.

### Skills

24. As a Basic Warrior, I want all eight of my Calling's skills in the spellbook from level 1, so that I don't have to train ranks.
25. As a player, I want my skills to get stronger as I level, with no ranks to buy, so that power comes with levels on its own.
26. As a player, I want to drag skills onto the action bar and cast them, with the stock cast bar, cooldowns and rage costs, so that it plays like WoW.
27. As a player, I want each skill's tooltip to show a 1.12 spell's name and icon, so that I can recognise it, even if the numbers aren't exact.
28. As a player, I want the placeholder Basic Warrior set (Heroic Strike, Charge, Rend, Thunder Clap, Battle Shout, Hamstring, Overpower, Bloodrage), so that I have a full toolkit from level 1.
29. As a Tier 1 Warrior, I want a stronger set of up to eight skills, so that Advancement feels like a step up.

### Talents

30. As a player, I want a Talent Point at every level from 2, so that I keep making build choices.
31. As a player, I want my Calling's Talent Tree in the stock talent frame, so that I can spend points with the familiar UI.
32. As a Basic Warrior, I want a small one-tab tree (about 10 points), so that I can try talents early.
33. As a Tier 1 Warrior, I want a 2–3 tab tree borrowed from Arms, Fury and Protection, sized so that about 29 points reach mid-tier talents, so that the build matters.
34. As a player, I want the talents I learn to change my combat numbers, so that spending points matters.
35. As a player, I want to respec at my trainer for gold through the stock confirmation dialog, so that I can fix a bad build.

### Quests and levelling

36. As a player, I want quest givers at the Hub, each tied to a Hunting Ground, so that every hand-in brings me back to town.
37. As a player, I want Repeatable Quests (kill N, or collect N drops) shown and tracked in the stock quest log, so that I know what I'm doing.
38. As a player, I want to retake a Repeatable Quest as soon as I hand it in, so that I can grind it as long as it's worth it.
39. As a player, I want a quest at my level to give about one level of XP, so that levelling is fast.
40. As a player, I want a quest's XP to shrink once I outlevel it, so that I'm pushed to the next Hunting Ground.
41. As a player, I want vanilla kill XP (more for mobs above me, none for grey mobs), so that hunting up pays.
42. As a player, I want quests to also give some gold and sometimes Tokens, so that quests feed gear.
43. As a player, I want the stock XP bar, level-up flash and level-up message, so that progress feels like WoW.
44. As a tester, I want `?xp=N` / `WOW_DEV_XP` to multiply XP, so that I can reach Tier Caps fast.

### Tiers and Advancement

45. As a player, I want my level to stop at the Tier Cap (10 for Tier 0, 30 for Tier 1), so that I know when to advance.
46. As a Basic Warrior at level 10, I want my trainer to offer an Advancement Quest for each Calling I can branch into (only Tier 1 Warrior in this slice), so that I choose my path.
47. As a player, I want finishing an Advancement Quest to make me level 1 of the new Calling, so that a new Tier begins.
48. As a player who just advanced, I want to keep my gear, bags, gold, Tokens and completed quests, so that the reset doesn't erase my work.
49. As a player who just advanced, I want my old skills and talents replaced by the new Calling's, and my Talent Points reset, so that I'm playing the new Calling.
50. As a player who just advanced, I want a Tier Bonus to my base stats, so that a Tier 1 level 1 is clearly stronger than a Tier 0 level 1.
51. As a Tier 1 Warrior at level 30, I want Champion's Advancement Quest to be offered but not completable, so that I can see what comes next.

### Gear and economy

52. As a player, I want mobs to drop real 1.12 items around their level, mostly grey and white with greens now and then, so that drops are my main gear source.
53. As a player, I want to loot through the stock loot window, so that it feels like WoW.
54. As a player, I want items limited by Class, using vanilla proficiency, so that gear choices are meaningful.
55. As a player, I want to equip gear on the stock character frame and see my stats change, so that upgrades are visible.
56. As a player, I want a Token vendor at the Hub selling greens by level band, so that the grind has a guaranteed payoff.
57. As a player, I want a goods vendor that buys my loot, repairs gear and sells food and drink, so that gold has a loop.
58. As a player, I want gear durability that wears and needs repair, and gear at zero durability to stop giving stats, so that death and repair matter.
59. As a player, I want the stock merchant, repair and bag UI, so that buying and selling feels like WoW.

### Dungeon

60. As a player, I want to enter the Wailing Caverns through its real entrance, so that the Dungeon sits in the world.
61. As a player, I want the Wailing Caverns to be a solo run of about ten mobs and then a boss, so that a run is short.
62. As a player, I want the Dungeon boss to always drop one blue item, so that every clear pays out.
63. As a player, I want mobs in the Dungeon to follow its tunnels instead of walking through walls, so that fights look right.
64. As a player, I want to run the Dungeon again without limit, fresh each time I re-enter, so that it's a levelling option.
65. As a player, I want mobs I killed in a Dungeon to stay dead if I die and walk back in, until I leave, so that a wipe doesn't erase my progress.

### Death

66. As a player, I want to respawn at the Hub after dying in the world, so that death is a setback rather than a game over.
67. As a player, I want to respawn at the Dungeon entrance after dying inside it, so that I can try again.
68. As a player, I want death to cost 10% of my gold and 10% durability, so that risky pulls cost something.

### Saving

69. As a player, I want my character saved automatically (on level-up, quest hand-in, loot, zoning and about once a minute), so that I never lose much.
70. As a player, I want to reload the page and find my characters exactly where I left them, so that the game is something to come back to.
71. As a player, I want saves to work in the browser and natively, so that both builds are real.

### Testing and future

72. As a tester, I want `?speed=N` / `WOW_DEV_SPEED` to multiply my run speed, so that I can cross The Barrens fast.
73. As a tester, I want the dev multipliers off by default, read only from the boot flags and never saved, so that they can't leak into a real save.
74. As the developer, I want the game rules in a crate free of Bevy, with tests that need no window, GPU or game data, so that I can grow content with confidence.
75. As the developer, I want a future real server to reuse the Local Realm's rules behind a socket, so that multiplayer grouping becomes possible without a rewrite.

## Implementation Decisions

- **Architecture (ADR 0001).** The Local Realm sits behind the same command and event seam a
  real server feeds. The client runs its normal-WoW mode. Survivors' capture mode (FrameXML
  off, player controller off) is **not** used.
- **Two modules.**
  - **The Local Realm crate** (new, free of Bevy, no client types). It owns all game state and
    rules: characters, Callings, Advancement, combat, mob AI, quests, loot, vendors, talents,
    death and saving. It depends on the protocol crate for event types.
  - **The Twilight's Eve glue module** (new, inside the app crate). It holds the boot flag,
    turns client commands into realm commands and realm events into the net layer's event
    stream, ticks the realm once per frame on the main thread, answers
    the host trait from the client's terrain and collision, and reads and writes save blobs
    through the existing local-state module.
- **The realm's interface** is the main seam, roughly:
  - Construct with the static content and the host trait.
  - `handle(RealmCommand)`, `tick(dt)`, `drain_events() -> SessionEvent…`.
  - Export and import of a character's save state.
- **`RealmCommand` is the realm's own enum.** The app's client-command type is internal to the
  app crate, and moving it into the protocol crate would edit upstream files. `RealmCommand`
  covers only what this slice needs: character list, create and delete; enter world; movement
  updates; set target and start/stop attack; cast; gossip hello and option; quest giver hello,
  accept, complete and choose reward; learn talent and talent wipe; list inventory, buy, sell
  and repair; loot open, take and release; equip, unequip and swap; use item (Hearthstone);
  logout. The glue maps client commands to these and drops the rest.
- **Events are the protocol's `SessionEvent`s**, so a future server sends the same thing. The
  realm emits the object and value updates the client needs: player fields (Class, level, XP,
  stats, Talent Points), units, items, auras, spell start and go, the combat log, quest and
  gossip dialogs, the trainer and vendor lists, loot, the spellbook, and XP and level-up.
- **The host trait** gives the realm ground height at a point and line of sight between two
  points. Inside the client these come from terrain streaming and collision. Tests use a flat
  fake, with walls where needed.
- **Static content is data inside the realm crate:** Callings (Tier, Class, branches, skills,
  Talent Tree, Tier Cap, Tier Bonus), skill formulas, mob templates and spawns, Hub NPCs,
  Repeatable Quests, vendor lists, loot tables by level and the Dungeon definition. Callings
  and quests are data, not code, so new ones need no new systems.
- **Spawn import.** A one-off offline script pulls The Barrens' creature spawns (position,
  display ID, level, faction, wander radius) from a public vmangos world database into a data
  file. It filters out friendly NPCs and quest NPCs. The low-level Crossroads camps, Hub NPCs
  and Wailing Caverns packs are written by hand.
- **Tiers:** caps at 10, 30, 50 and 60, with Tier 3 the last. The slice builds Tier 0 (Basic
  Warrior) and Tier 1 (Warrior). Champion (Tier 2) exists only as an Advancement Quest that
  can't be completed.
- **Advancement:** at the Tier Cap, completing a branch's Advancement Quest sets the Calling,
  resets level to 1 and XP to 0, replaces skills, wipes talents and points, and applies the new
  Tier's flat **Tier Bonus** to base stats. Gear, bags, gold, Tokens and completed quests carry
  over. XP past the cap is not banked.
- **Skills:** one spell ID per skill, no ranks, given whenever a Calling becomes current. Effect
  = a formula of level, attack or spell power and weapon, owned by the realm. Names, icons and
  tooltips come from existing 1.12 spells.
- **Talents:** a point every level from 2. On Advancement the client's talent catalog resource
  is replaced with the current Calling's tabs, built from existing 1.12 talent spells. The
  realm decides each talent's effect. Respec costs gold at the trainer, through the stock
  talent-wipe flow.
- **Combat:** vanilla 1.12 formulas (swing timer, rage, hit table by level and weapon skill,
  armour mitigation, stat conversions), with vmangos and cmangos as the reference. The Tier
  Bonus only raises base stats.
- **XP:** vanilla kill XP and level table. A Repeatable Quest's reward = the XP needed for the
  player's next level, reduced by vanilla's quest-level falloff when the player is above the
  quest's level. Quests can be retaken straight away.
- **Mob AI:** aggro radius by level difference, social aggro within a group, straight-line
  chase sent as move splines over host ground height, leash and evade at about 40–60 yd from
  home, respawn about 60 s after the player leaves. Inside the Dungeon, chase follows a
  hand-written waypoint route graph.
- **Loot:** loot tables owned by the realm, picking real 1.12 item IDs near the mob's level:
  mostly grey and white, sometimes green. The Dungeon boss always drops one blue. Class limits
  follow vanilla proficiency.
- **Death:** respawn at the Hub, or at the Dungeon entrance if inside it, at full health, minus
  10% gold and 10% durability on every equipped item. Items at zero durability give no stats.
  Inside the Dungeon, killed mobs stay dead until the player leaves.
- **Dungeon:** the Wailing Caverns on its real instance map, reached by its real entrance. About
  ten mobs, then a boss. A fresh copy each time the player enters from outside, with no limit
  on runs.
- **Saving:** one save per character through the existing local-state module (a file natively,
  `localStorage` in the browser). It holds Calling, level, XP, Talent Points and talents, gear,
  bags, gold, Tokens, completed and active quests, and position. Writes happen on level-up,
  quest hand-in, loot, zoning, logout and about once a minute. The format is versioned.
- **Boot:** the Twilight's Eve flag starts the client at character select with no login, the
  net layer's IO threads off, and the glue attached. The dev multipliers `speed` and `xp` are
  read only at boot, default to 1 and are never saved.
- **Repository rules:** edits to upstream
  app files are kept minimal and gated so upstream merges stay clean. Twilight's Eve lives in
  new files.

## Testing Decisions

- **A good test** drives the realm only through its public interface: commands in, `tick`,
  events out, plus save export and import. It asserts on the events a client would see and on
  the state those events describe, never on the realm's internal structs. Tests read as player
  scripts, e.g. "create a Basic Warrior → enter world → target a level 3 mob → attack → tick
  until it dies → expect an XP gain and loot".
- **Tested module:** the Local Realm crate, with plain `cargo test`. No window, no GPU and no
  1.12 game data, so it runs anywhere, including CI. The host trait is a flat-ground fake, with
  walls where a test needs them (the Dungeon route).
- **Areas to cover:**
  - character create and list
  - XP and levelling, including about one level per quest at its level and falloff above it
  - the Tier Cap
  - Advancement: level reset, skill and talent swap, Tier Bonus, what carries over
  - talents and respec
  - combat formulas against known vanilla values
  - aggro, social aggro, leash and evade
  - quest accept, progress, hand-in and retake
  - loot by level and the guaranteed boss blue
  - vendor buy, sell and repair, and Tokens
  - death penalty and respawn location
  - Dungeon kill state persisting across a death and resetting on leave
  - save round trip
- **Not unit-tested:** the glue module. It's covered by `cargo check` and by playing the
  full loop by hand with `WOW_DEV_SPEED` and `WOW_DEV_XP`.
- **Prior art:** the protocol crate's integration tests (one per message family: character
  enum, gossip and vendor, chat, group…) for asserting on protocol-shaped values, and the app's
  talent-window code, whose comments already cite vmangos behaviour, as the model for
  checking rules against a reference server.
- **Done** means this loop works in the browser with `?twilight=1`:
  1. Create a Basic Warrior, choose them at select, and appear in the Crossroads.
  2. Kill mobs with the 8 skills under vanilla combat; loot, sell and repair.
  3. Do Repeatable Quests (about one level each), earn Tokens and buy a green.
  4. Spend a Talent Point at level 2 in the stock talent frame.
  5. Die and respawn at the Hub minus 10% gold and durability.
  6. At 10, advance into Tier 1 Warrior at level 1, with the Tier Bonus, new skills and a new tree.
  7. Clear the Wailing Caverns solo and get the blue.
  8. Reload the page and find the character intact. At 30, Champion's quest is offered but
     can't be finished.

## Out of Scope

- Classes other than Warrior. Tier 2 and Tier 3 Callings beyond Champion's locked quest.
- New spell or talent records (custom tooltip text). Borrowed 1.12 spells are used throughout.
- A real server, networking and grouping or co-op. NPC companions.
- Zones other than The Barrens, travel across the continent, and flight paths.
- A navmesh or general pathfinding.
- Tuning for balance beyond "playable to 30". Content beyond the first slice's quests, vendors
  and one Dungeon.
- Any change to Survivors.

## Further Notes

- The domain vocabulary (Mode, Local Realm, Class, Tier, Calling, Advancement, Advancement
  Quest, Tier Cap, Tier Bonus, Talent Point, Talent Tree, Hub, Hunting Ground, Repeatable Quest,
  Token, Dungeon) is defined in `GLOSSARY.md`. ADR 0001 records the Local Realm decision and the
  split into a crate.
- The Barrens has nothing below level 6–8. The hand-placed Crossroads camps exist only to cover
  levels 1–5. Levels 25–30 are thin and are covered by the Wailing Caverns and the Tier Bonus.
- Protocol fidelity is the main risk: object update fields, GUIDs, spell start and go, aura
  slots, and getting the stock FrameXML screens (character select, quest log, talent frame,
  merchant) to accept events that came from the realm. Prove the thin path early:
  character select → enter world → one mob → one kill, before building broad systems.
- The fork's invariant that the shipped files are what the web build copies applies if the
  slice adds any page-side file.
