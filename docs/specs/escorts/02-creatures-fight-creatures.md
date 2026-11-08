Title: Defenders: a friendly creature that hostile creatures fight, that fights back and that players heal (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 2. Realm code only. Paths are under `crates/twilight-realm/` unless they
start with `docs/`.

## Why

An escortee is attacked by ambushers and by the canyon's own creatures, swings back, and can be
healed by the players walking him. Nothing in the realm lets a creature hit or be hit by a creature
today. `Mob.target` and the threat table hold player guids. The `Engaged` arm of `World::mob_tick`
(`src/world.rs` lines 1336 to 1460) looks targets up only among the session's own player and
`others`. `engage` (`src/world.rs` line 2083) is written for a player. A friendly creature is never
attackable (`Mob::attackable`, `src/mobs.rs` line 1866), and an ally skill lands only on players
(`World::ally`, `src/world/cast.rs` line 3646). This ticket adds one kind of creature that takes part
in fights: a **defender**. ADR draft 0035 (`docs/specs/escorts/adr-draft.md`) records it; copy it to
`docs/adr/0035-escorts.md` in this ticket if no other escort ticket has.

## The rules

**What a defender is.** `Mob` (`src/mobs.rs` line 1217) gains `defender: bool`. A defender is a
friendly creature (template `friendly = true`) that hostile creatures may fight. Players still
cannot attack it (`Mob::attackable` is unchanged). Nothing in content makes a defender; the escort
(03-escort-walker) sets the flag on its escortee. A defender never leashes (`leashes = false`).

**Who may fight whom.** Two creatures may fight only when some character is admitted by both
(`Mob::admits`, `src/mobs.rs` line 1871). A creature everyone sees may fight any defender. An
Audience creature (ADR 0014) may fight a defender whose Audience shares a character with its own.

**A hostile creature notices a defender.** In the notice loop of `World::tick`
(`src/world.rs` lines 876 to 896), the session that runs a hostile idle creature
(`self.runs(guid)`, line 742) also checks the living defenders in `self.mobs` that it may fight.
One within its aggro radius for the defender's level (`rules::aggro::radius`,
`src/rules/aggro.rs` line 7, with the creature's `detection_range`), in line of sight, is pulled
onto: the creature engages it and brings its camp, as a player's pull does. The player-only
cuts in `World::notice_radius` (`src/world.rs` line 959: Cover, stealth, a cart seat) do not apply.

**The threat table holds creatures too.** `engage(mob, guid, events)` and `rules::threat::Table`
take any unit guid. In the `Engaged` arm, the `reach` closure (`src/world.rs` around line 1340)
answers for a living creature in `self.mobs` that the mob may fight (its position, and whether it
is in melee reach), so `threat.retain` keeps it and `threat.pick` can choose it. The chase and the
swing (around lines 1400 to 1460) aim at the target's position, which is the creature's when the
target is a creature. A blow at a creature rolls a creature-on-creature swing: add
`combat::mob_swing_at_mob` beside `combat::mob_swing` (`src/combat.rs` line 109), using the
victim's armour and level in place of a player's stats, and land it with `wound`
(`src/world.rs` line 2112), which takes the attacker's guid. Report it as an `AttackerState` from the
attacker to the victim, as a swing at a player is reported.

**Being hit.** A creature hit by a creature adds the damage as threat for the attacker, 1:1. An
idle victim engages its attacker. This is how a defender fights back and how a hostile hit by a
defender turns on it.

**A defender's fight ends where it stands.** When a defender's threat table empties, it goes idle
on the spot and keeps its health. It does not evade or run home, and the `State::Evading` heal
(`src/world.rs` line 1326) never runs for it. The escort then walks it on.

**Deaths.**
- A hostile creature killed by a defender's blow dies through the usual `World::kill`
  (`src/world.rs` line 1706), so its `Kill` goes to `share_kill` (`src/lib.rs` line 1242). Every
  player within the credit range of the corpse is credited, as for a player's kill.
- A defender that dies makes no `Kill`: no XP, no loot, no money and no respawn timer. It stops
  with health 0, its attackers' tables drop it, and they turn on whoever is left on their table or
  evade. Its corpse stays until whatever made it removes it.

**Who runs the fight.** `Shard::owners` (`src/shard.rs` line 550) gives each creature to the
session of the player it fights. For a creature fighting a creature, the owner is the nearest player
admitted by both creatures. A fight with no such player present is run by nobody, and the
creatures stand still until one comes back.

**Players heal a defender.**
- `World::ally` (`src/world/cast.rs` line 3646) accepts a targeted living defender that the caster
  sees, in the skill's range.
- A direct heal (`Effect::Heal`, `src/world/cast.rs` line 5039, the branch that tests
  `crate::shard::is_player(target)`) heals the defender in `self.mobs` up to its maximum health,
  sends its new `UNIT_FIELD_HEALTH`, and sends a `SpellHealLog` with the defender as the target.
- A heal over time (`Effect::PeriodicHeal`, line 5074, the `None =>` arm that takes only players)
  goes on the defender and ticks on it.
- An area heal (`World::heal_allies`, line 3544) and a lowest-health pick
  (`World::lowest_health_ally`, line 3529) count defenders the caster sees, as they count players.
- Every other effect of an ally skill (a buff) skips a defender.
- **Heal threat.** `shard::note_threat` (`src/shard.rs` line 578) treats any event whose target is
  in `mobs` as damage to that creature, so a heal on a defender would put the healer on the
  defender's own table. A heal or heal tick on a defender goes to the heal branch (lines 609 to
  615): half its amount, split across the living creatures whose table holds the defender.

## Test seam

Add `pub fn spawn_defender_for(...)` to `Realm`, the same as `Realm::spawn_for`
(`src/lib.rs` line 2789) with the defender flag set on the creature it makes. ADR 0002's player
scripts use it as they use `spawn_for`.

## Failing tests first

New `tests/defenders.rs`, set up as `tests/audience.rs` is: far out past The Barrens' camps
(`FAR_AWAY`), characters placed with `party.go`, creatures made with `Realm::spawn_for` and
`Realm::spawn_defender_for` for an Audience holding the test characters. The defender is Corporal
Keeshan's template (349). The hostile is a Prairie Wolf (2958). Pick levels close enough that both
live through several swings (both 30, say) unless a test says otherwise.

1. A wolf made 10 yards from a defender attacks it within a few seconds, and the defender swings
   back: the Audience character's client is sent `AttackerState` from the wolf to the defender and
   from the defender to the wolf.
2. The character stands 60 yards off and does nothing. Neither creature turns on them
   (no `AttackStart` with the character as victim) while the fight lasts.
3. A level 42 defender against a level 5 wolf made 3 yards from it (the aggro radius never goes
   under 5 yards, `src/rules/aggro.rs` line 7): the wolf dies, and the character, within 74 yards,
   gets the kill credit (a test kill quest added with `Realm::with_quests` counts it).
4. A level 5 defender against a level 30 wolf. A level 30 test healer (as in test 5) stands 30
   yards off, outside the wolf's aggro radius, and heals the defender once during the fight. The
   defender dies, the wolf then turns on the healer (heal threat), and the healer gets no kill
   credit or loot for the defender.
5. A character of a test healer Calling (as `tests/shared_world.rs` lines 280 to 300 make
   `test_healer` with `mend`, spell 2061) targets a hurt defender and casts: the defender's health
   goes up and a `SpellHealLog` names it. The defender does not turn on the healer.
6. A wolf made for one character's Audience never notices a defender made for a different
   character's Audience 10 yards away.
7. A defender left with an empty threat table stays where it stopped and keeps its health (no
   evade move, no heal to full).

## Acceptance

- The tests pass, and `cargo test -q -p twilight-realm` passes. The existing fight, threat and
  Audience tests still pass unchanged.

Assumptions allowed: the creature-on-creature swing's hit table (vanilla's mob-versus-mob rules,
or the mob-versus-player table with the victim's armour), the test levels, and how the threat table
tells a creature guid from a player guid if it needs to.

Blocked by 01-per-viewer-sight, which edits the same files (`mobs.rs`, `shard.rs`, `lib.rs`).
