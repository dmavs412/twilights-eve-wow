Title: Escort quests (spec)
Labels:

Grilled with Davis on 2026-11-08 from `docs/specs/prep/escort-quests.md`. The grill's answers
are below and override the prep draft wherever they differ. Escorts are a new realm system for
decision 11 of the Redridge and Duskwood arc (#595). Corporal Keeshan's "Missing in Action"
(98391, #620) is the first escort and the reason for this spec. ADR draft: 0035, "Escorts: a
private escortee per group, and creatures that fight creatures"
(`docs/specs/escorts/adr-draft.md`; the first ticket to land copies it to `docs/adr/`).

## What an escort is

A character holding the quest finds the escortee at his hideout and picks a gossip line. A private
copy of him sets off along a fixed path for that character and everyone near them, waits when they
fall behind, fights what attacks him, takes heals from players, and walks into two or three
scripted ambushes. When he reaches the end, every member of the group holding the quest has it met.
If he dies, or nobody stays near him for a few seconds, the escort ends, the quest stays in the log,
and he is back at his hideout a few seconds later for another try.

## Decisions (Davis, 2026-11-08)

1. **Own copy each.** Each character, with the characters near them, gets a private escortee. Nobody
   else sees him, his walk or his ambushes, and another group can run its own escort at the same
   time. This needs per-viewer sight in the realm: a creature or Hub NPC seen by some characters
   only. The same rule shows Keeshan at the Stonewatch Falls camp only to characters who rescued
   him, and hides the hideout Keeshan from them.
2. **He fights, and players can heal him.** Hostile creatures attack him, he swings back, and a
   player's heal lands on him. Nothing in the realm lets a creature hit a creature today
   (`src/world.rs`, `engage` line 2083 takes a player), so this is the largest new piece.
3. **Failure resets him.** The escort fails when he dies, or when no living escorter has been near
   him for a few seconds. The quest does not fail and stays in the log. He stands at his hideout
   again after a short delay, and a word with him starts the walk again. There is no trip back to
   the quest giver and no time limit.
4. **The group shares it.** Group members see that escortee and his ambushes and can fight. Every
   member holding the quest is credited when he arrives.
5. **The walk.** A fixed path with two or three scripted ambushes along it, plus whatever canyon
   creatures join in. He waits when the escorters fall behind.

## Claude's readings, taken in the tickets

- A gossip line on the escortee starts the walk (prep decision 6a). Accepting the quest starts
  nothing.
- The path, waits, lines and ambushes live in a new hand-written `content/escorts.toml`, one
  `[[escort]]` per quest, beside `content/encounters.toml` (prep 7a). The quest row needs no new
  field: the escort names its quest, as an encounter does.
- Vanilla's 68 points for Keeshan are thinned to about 30, and the 45 s rest at point 25 becomes
  10 s (prep 8b).
- Keeshan walks at level 42 with extra health (prep 10a, with heals from players added by the
  grill).
- The quest log shows no counter (prep 12a). Arrival sets one hidden log counter to 1, saved in
  the quest's `kills` as a talk objective's is, so the log reads "Complete".
- "The group" is the starter plus every character on the map within the kill credit range
  (`content/group.toml` `credit_range_yd`, 74 yards) of the escortee when the walk starts, fixed
  then. This is how a Rumour's Audience is chosen (ADR 0014, GLOSSARY "Audience"). ADR 0009 has no
  party invites, so range is the only group there is.
- "An escorter" is a living group member, holding the quest or not. He waits while no escorter is
  within 20 yards, and the escort fails once none has been within 60 yards for 10 seconds (prep
  5a). Keeshan's numbers are data in #620.
- Credit on arrival goes to every group member holding the quest who is on the map, alive or dead,
  wherever they stand. Decision 4 names no range.
- The ambushers open on the escortee (prep 9a). He also joins the fight of any hostile creature
  that attacks a group member near him, as vanilla's escort AI assisted its players.
- A heal (direct or over time) lands on a defender. A buff does not; that is left for later.
- Hostile creatures everyone sees (the canyon's Blackrock) notice and attack the escortee as they
  would a player. A character outside the group then sees such a creature fight something they
  cannot see. That is accepted.
- Nothing about an escort is saved. A logout counts as leaving, and the 20 s logout rule of ADR
  0009 applies. A realm restart drops every running escort.
- Head Count and the Tier Step do not touch the escortee (open world).

## Tickets

- 01-per-viewer-sight: sight rules by quest state for friendly creatures and Hub NPCs, and a
  runtime "hidden from" list on a creature.
- 02-creatures-fight-creatures: defenders. A friendly creature that hostile creatures notice and
  fight, that fights back and that players can heal.
- 03-escort-walker: `content/escorts.toml` and `src/escorts.rs`. The start line, the private copy,
  the walk, the waits, the two failures, the reset and the credit. Needs 01 and 02.
- 04-escort-ambushes: ambushes at path points, made for the group and opened on the escortee, and
  the escortee's assist. Needs 03.

Client: none. Movement is `MonsterMove`, lines are creature speech, and failure is a system line.
The client already handles all three. Assumed: the client draws a friendly creature fighting a
hostile one without changes, since both are ordinary units.

## The data ticket

#620 (Keeshan: Missing in Action) is the data ticket. It must be Blocked by 04-escort-ambushes, the
last realm ticket. Its data shape differs from what #620 says in a few places (the start, the
failure, two or three ambushes, the camp Keeshan's sight rule). `docs/specs/escorts/620-comment.md`
is the comment to post on #620 that tells its builder what changed. The latest comment wins
(`docs/agents/unattended.md`).

## Out of scope

- Elwynn's unbuilt supply wagon escort (94901). It may suit a Cart better than a walker.
- Buffs on creatures, escortees that cast spells, and escorts inside Phased Dungeons.
- A follow mode (an escortee that follows a character instead of a path).
