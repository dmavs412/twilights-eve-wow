# 0035 Escorts: a private escortee per group, and creatures that fight creatures

Corporal Keeshan's "Missing in Action" (98391) is the first escort quest: the character walks him
out of Render's Camp, down the canyon road to Lakeshire, through Blackrock ambushes. The realm
had no escorts. Creatures fought players only, and ADR 0014's Audience could narrow who sees a
creature made at run time but could not hide a shared spawn from anyone. On the friends server
(ADR 0009) up to five players share one world, so one shared Keeshan would make two players'
escorts collide. Grilled 2026-11-08 (spec #596).

## Decision

- **A private escortee per group.** A word with the escortee at his start spot makes a copy of him
  for the starter and every character within the kill credit range (74 yards), fixed then, as a
  Rumour's Audience is. The group sees that copy, his walk and his ambushes; nobody else does.
  Another group can run its own escort at the same time.
- **Sight can hide, not only narrow.** A creature gains a run-time "hidden from" list, read where
  the Audience is read, so the escort hides the shared start creature from its group while their
  copy walks. Friendly creatures and Hub NPCs gain a sight rule by quest state: seen only after a
  quest is handed in, or only before. Keeshan stands at the Stonewatch Falls camp for those who
  rescued him and in Render's Camp for those who have not. Sight rules stay off hostile creatures,
  so aggro and threat never ask about quest state. This widens ADR 0014's "an Audience narrows and
  never hides".
- **Defenders: creatures that fight creatures.** A defender is a friendly creature that hostile
  creatures notice and attack, that fights back, and that players can heal. Threat tables hold
  creatures as well as players. Two creatures may fight only when some character is admitted by
  both. A defender's fight ends where it stands, with no evade and no heal. A defender's death pays
  nothing; a creature killed by a defender pays the players in range as any kill does (ADR 0009).
- **An escort is content.** `crates/twilight-realm/content/escorts.toml` holds one `[[escort]]` per
  quest: the start spawn, the escortee's level and health, the start line, the path of points with
  waits, lines and a run switch, the ambushes at points, and the distances and timers. The quest row
  needs no field. As with Encounters, nothing in the code names a quest.
- **Failure resets, it does not fail the quest.** The escort fails when the escortee dies or when no
  living group member has been near him for a few seconds. The quest stays in the log, the group sees
  him at his start spot again after a short delay, and a word with him starts a new walk. There is no
  time limit.
- **Credit for the whole group.** On arrival every group member holding the quest, on the map, alive
  or dead, wherever they stand, has it met. The log shows no counter; one hidden counter is saved in
  the quest's `kills`, and nothing else about an escort is saved.
- **No `bevy_ecs` yet.** The realm has no `bevy_ecs`, so an escort is a module with a realm-level
  tick beside Encounters, and a defender is a field on `Mob`, as `audience`, `hunts` and `frozen` are.

## Considered options

- **One shared escortee for the server.** Rejected by Davis: a second player waits for the first
  escort to end, or both pull the same Keeshan.
- **Escortees that cannot be hurt, failing only on a leash or a timer.** Rejected: the quest says
  "bring him back alive", and later escorts and guard fights need the same creature-on-creature rule.
- **Failing the quest, and a retake from the giver.** Rejected: Marris stands 532 yards from the
  hideout, and vanilla's retry was at the escortee.

## Consequences

- Every system that assumed a fight target is a player now meets creature targets: the threat
  table, the chase and swing, the owner of a fight, heal threat, and ally targeting.
- A character outside a group can see a shared hostile creature fight a group's escortee they
  cannot see. With five players at most, that is accepted.
- The next escort is a content row. A defence where creatures attack a defender at a fixed spot
  (vanilla's Belnistrasz) can reuse defenders and Encounters without new rules.
- Sight by quest state is open to every friendly creature and Hub NPC, so later quest chains can
  move a person from place to place by data alone.
