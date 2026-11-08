Title: Holy tank kinds: judging Seal of Iron, a threat aura, creature-type bonuses and per-target stacks (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 6. No blockers. Tickets 04 to 08 follow it, since all six edit
`src/callings.rs`, `src/world/cast.rs`, `tools/twilights-eve/build_spells.py` and
`docs/spell_pipeline.md`.

The Ironclad (ticket 11) and the rebuilt Zealot (ticket 12) need four things the realm and the
build cannot run yet. Each is generic: no system or match arm names a Calling or a spell.

## The kinds

1. **Judging a sheet Seal.** `judgement` today judges only the TOML's Seal of Command and Seal of
   the Crusader (`Effect::Judgement`, `src/callings.rs` line 1442 onward; cast at
   `src/world/cast.rs` line 5276, gated at line 3849). Add params: `seal=<aura key>` names an aura
   row of the Calling (Seal of Iron's `aura_seal_of_iron` pattern: a `grant_aura` buff). The cast
   needs the caster to wear that aura; it takes it off, deals `amount` (a formula block) in the
   row's `school` to the target, and heals the caster `heal_pct` percent of their maximum health.
   With the aura absent the cast is refused, as Judgement is with no Seal today. The TOML
   Judgement keeps working.
2. **A threat aura.** Aura kind `mod_threat_pct` (`pct=N`, a whole number from 1 up, optional
   `school=`): while worn, the wearer's damage of that school (every school without one) makes N%
   more threat. Today `threat_pct()` (`src/world/cast.rs` line 1177) takes the lowest
   `threat_pct` from an absorb (Steady Stance); combine them: absorb percent first, then this
   aura's increase multiplied on top.
3. **Creature type.** On `school_damage` and `weapon_strike`: `vs_type=<name>;vs_type_pct=N`, N%
   more against a creature of that type, summed with `controlled_pct`, `bleeding_pct` and
   `below_bonus_pct` as those already sum. On `stun`, `incapacitate`, `root` and `knockback`:
   `only_type=<name>`, so the control lands only on that type and does nothing to others. Names:
   `beast`, `dragonkin`, `demon`, `elemental`, `giant`, `undead`, `humanoid`, `critter`,
   `mechanical` (1.12's CreatureType ids 1 to 9). The mob's type is `creature_type` on its
   template (`src/mobs.rs` line 80). A player target is never of a creature type.
4. **Per-target stacks.** `add_aura_stacks` takes `per_target=yes`: on an area row (Bulwark
   Taunt's `target=self_area` form), it adds `stacks` once per enemy the cast landed on, up to
   the aura row's `max_stacks`. Without it, an area cast adds `stacks` once, as today. Check first
   whether `taunt` on a `self_area` row already taunts every enemy hit (Bulwark Taunt says it
   does); if it does, leave it.

## The build

`tools/twilights-eve/build_spells.py`: accept the new params on modelled rows and refuse bad ones
(the form of the checks near line 1156 and line 1782): `seal` must name an aura row of the
Calling; `vs_type` and `only_type` must be one of the nine names, and `vs_type` needs
`vs_type_pct` (a whole number of at least 1) and the other way round; `per_target` is `yes` or
`no`; `mod_threat_pct` is an aura-row kind with a whole `pct` of at least 1 and an optional
school. Add `mod_threat_pct` to `KNOWN_KINDS` (line 48) and to the server JSON as other aura
kinds are. Add unit tests to `tools/twilights-eve/tests/test_build_spells.py` for each refusal.

## Docs

`docs/spell_pipeline.md`: a paragraph after "Acolyte's rows (#508) add more" (line 146) titled by
use ("The Vanguard Line's Tier 2 rows add four"), in the voice of those paragraphs. Add the new
kinds to `docs/agents/calling-script-guide.md`'s vocabulary list.

## Failing tests first

New file `crates/twilight-realm/tests/holy_tank_kinds.rs`, written in the form of
`tests/ground_area.rs`: a test Calling in an inline TOML string with its own spell ids, played
through the realm's seam with `tests/common/fight.rs`.

- A test Calling with a Seal-like `grant_aura` buff and a `judgement` with `seal=`: judging with
  the buff up removes it, damages the target in holy and heals the caster the percent of max
  health; with no buff the cast is refused and costs nothing.
- With a `mod_threat_pct` aura of 50 for holy up, a holy hit adds 1.5 times the threat it adds
  without; a physical hit adds the same as without (read with `Realm::creature_threat`,
  `src/lib.rs` line 940).
- A `school_damage` with `vs_type=undead;vs_type_pct=100` deals double to an undead test mob and
  normal to a humanoid one; a `stun` with `only_type=undead` stuns the undead mob and not the
  humanoid.
- A `self_area` taunt with `add_aura_stacks` `per_target=yes` against three mobs gives three
  stacks, capped at `max_stacks`.
- `cargo test -q -p twilight-realm` still passes the Zealot TOML Judgement tests.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `build_spells.py --check --no-dbc-check` is clean (no sheet row changes in this ticket).

Assumptions allowed: how the two threat modifiers combine if the code suggests a better order,
the creature type names' spelling.
