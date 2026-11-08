Title: Blade kinds: bleed damage taken, ticking bleeds now, spending your own stacks, spilling droplets and Vanish (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 7. Blocked by 03-holy-tank-kinds (same files: `src/callings.rs`,
`src/world/cast.rs`, `build_spells.py`, `docs/spell_pipeline.md`).

The Bloodletter (ticket 13) and the Nightblade (ticket 14) need six things the realm and the
build cannot run yet. Each is generic.

## The kinds

1. **Periodic damage taken.** Aura kind `mod_periodic_taken_pct` (`pct=N`, 1 to 100, optional
   `school=`, default physical): the unit wearing it takes N% more from the aura's caster's
   periodic damage of that school. A skill puts it on with an effect of the same kind and
   `apply_aura=<key>`, as `mod_run_speed` does. (`mod_damage_taken_pct` on a mob changes nothing,
   since it counts only other casters.) Hemorrhage uses it.
2. **Tick now.** Skill kind `tick_now`: every periodic damage aura the caster has on the target,
   of `school=` (default physical, so bleeds), deals `ticks` of its ticks at once (default 1),
   leaving its timer and stacks. `ticks_per_combo=N` makes it a finisher: N ticks per combo
   point, spending the points. Optional `heal_pct=N` heals the caster N% of what the forced ticks
   dealt, and `drop_chance=N` (1 to 100) spills the Calling's `bleed_drop` droplet at N% per
   forced physical tick, as `periodic_damage`'s `drop_chance` does. With no such aura on the
   target the cast fails and costs nothing. Open Veins uses it.
3. **Spend your own stacks.** Skill kind `consume_self_stacks`: `aura=<key>` names a stacking aura
   row of the Calling worn by the caster (Bloodlust), and `grant=<key>` an aura row with
   `stack_rule=stack`. The cast takes every stack of `aura` and puts `grant` up with that many
   stacks, for `grant`'s duration. With none the cast is refused. (`consume_aura_stacks` takes
   stacks off the target, not the caster.) Red Frenzy uses it.
4. **Spill droplets.** Skill kind `spill_droplets`: `count=N` of the Calling's `bleed_drop`
   droplets land around the caster, scattered as the pickup row's `scatter_min_yd` and
   `scatter_max_yd` say, centred on the caster. Optional `duration_ms=` overrides the pickup row's
   duration for these. A Calling with no `bleed_drop` fails the build. Blood Pool uses it.
5. **Vanish.** Skill kind `drop_threat`: every creature's threat on the caster goes to 0; a
   creature left with no one on its list evades and goes home, as when its target dies. The
   caster leaves combat. A `grant_aura` of a `stealth` aura row in the same cast, after
   `drop_threat`, then puts Stealth up even though the cast began in combat (today Stealth needs
   `combat=out`). A creature that is attacked again re-enters combat as usual.
6. **Any skill's next crit.** An aura row's `skill_crit_pct` takes `skill=any`: the caster's next
   landed hit with any skill uses the bonus, spending one `charges`. Check first: if it already
   works on an aura row (the talent form does), leave it and say so. Cold Blood uses it.

## Where

- `crates/twilight-realm/src/callings.rs`: the `Effect` enum (line 1442) and the aura effects
  (the `AuraEffect` enum near the stealth arm, `src/world/cast.rs` line 7888, shows the aura
  side).
- `crates/twilight-realm/src/world/cast.rs`: effect application (`Effect::Stealth`, line 4641;
  `Effect::Taunt`, line 4547, shows how a target's threat list is reached); periodic ticks;
  `bleed_drop` spilling (search `bleed_drop`).
- Threat: `crates/twilight-realm/src/rules/threat.rs` and `src/shard.rs` line 578 (`note_threat`).
- `tools/twilights-eve/build_spells.py`: `KNOWN_KINDS` (line 48) and the param checks; refuse a
  `grant` that is not `stack_rule=stack`, a `count` below 1, a `ticks_per_combo` below 1, and an
  unknown aura key. Tests in `tools/twilights-eve/tests/test_build_spells.py`.
- `docs/spell_pipeline.md`: a paragraph after the one ticket 03 added ("The Cutthroat Line's Tier
  2 rows add six"); `docs/agents/calling-script-guide.md`'s vocabulary list.

## Failing tests first

New file `crates/twilight-realm/tests/blade_kinds.rs`, in the form of `tests/ground_area.rs` (a
test Calling in inline TOML; read `tests/cutthroat_blood.rs` for how droplets and Bloodlust are
set up on a Rogue-class test Calling).

- With `mod_periodic_taken_pct` 20 on a mob, the caster's bleed ticks deal 20% more; another test
  Calling's bleed on the same mob does not.
- `tick_now` with `ticks_per_combo=1` at 3 combo points makes a bleed deal three ticks at once,
  leaves its time left alone and spends the points; with no bleed on the target it is refused at
  no cost.
- `consume_self_stacks` with 4 Bloodlust stacks puts the granted aura up at 4 stacks and leaves
  no Bloodlust; with none it is refused.
- `spill_droplets` `count=3` puts three droplets within the scatter ring around the caster;
  walking over one grants the droplet's aura.
- `drop_threat` then `grant_aura` of a stealth aura, cast while two mobs are attacking: both lose
  the caster from their lists and evade, the caster is out of combat and in Stealth.
- An aura with `skill_crit_pct skill=any;pct=100` and one charge makes the next skill hit a crit
  and is then gone.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `build_spells.py --check --no-dbc-check` is clean.

Assumptions allowed: whether evading creatures heal on the way home (follow what a creature does
when its target dies), the order of riders on `tick_now`.
