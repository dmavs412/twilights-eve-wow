Title: Aura copies: copy an aura with its stacks to nearby foes, and spread it when its wearer dies (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 8 and 10. Blocked by 04-blade-kinds (same files).

The Pyromancer's Kindle (ticket 15) and the Blightcaller's Spread Blight and Contagion (ticket
18) copy a damage over time with its stacks. Today `consume_aura`'s `spread_yd` and the proc
action `spread_aura` move one fresh stack; `tick_spread_chance` copies stacks but only by chance
after a tick.

## The kinds

1. **Copy an aura.** Skill kind `copy_aura`: `aura=<key>` (an aura row of the Calling),
   `targets=N` (1 to 10) and `radius_yd=M`. When the cast lands, the caster's aura of that row on
   the cast's target is copied, with its stacks, its time left and its params, onto up to N of
   the nearest other living enemies within M yd of the target that do not wear the caster's aura
   of that row. With none on the target the cast fails and costs nothing. The copies are the
   caster's own, so they tick, grow and spread as the original does.
2. **Spread on death.** Proc trigger `on=wearer_death` with `aura=<key>`: when an enemy wearing
   the caster's aura of that row dies, by anyone's hand or by the aura's own tick, the proc
   fires once. Its action `spread_aura=N` takes the param `spread_stacks=all`: the copy carries
   every stack the dead wearer had and the full duration of the row, onto the nearest other
   living enemy within N yd that lacks it. Without `spread_stacks`, `spread_aura` keeps its
   one-stack meaning. The trigger works on a Talent row (passive) and on an aura row's proc.

## Where

- `crates/twilight-realm/src/callings.rs`: the `Effect` enum (line 1442); the proc triggers and
  actions are listed in `docs/spell_pipeline.md` line 188 onward and parsed in the same file.
- `crates/twilight-realm/src/world/cast.rs`: `target_died` (line 1953) is where a death is seen;
  the existing `spread_aura` and `tick_spread_chance` code shows how a copy is made.
- `tools/twilights-eve/build_spells.py`: `KNOWN_KINDS` (line 48), `copy_aura`'s params (`aura`
  must name an aura row of the Calling with a `periodic_damage` effect; `targets` 1 to 10;
  `radius_yd` above 0), the `wearer_death` trigger (needs `aura`), `spread_stacks` (only `all`,
  only with `spread_aura`). Tests in `tools/twilights-eve/tests/test_build_spells.py`.
- `docs/spell_pipeline.md` and `docs/agents/calling-script-guide.md`: document both.

## Failing tests first

New file `crates/twilight-realm/tests/aura_copies.rs`, in the form of `tests/ground_area.rs` (a
Priest-class test Calling in inline TOML with a stacking damage over time, as
`tests/acolyte_rot.rs` sets up Creeping Blight).

- Target A wears the DoT at 6 stacks with 8 s left; `copy_aura targets=2 radius_yd=10` puts it
  at 6 stacks and 8 s on the two nearest of three mobs within 10 yd, skips one that already wears
  it, and leaves A's untouched.
- With no DoT on the target the cast is refused and costs nothing.
- A passive Talent proc `on=wearer_death;aura=<dot>;spread_aura=10;spread_stacks=all`: killing A
  at 6 stacks puts the DoT at 6 stacks and full duration on the nearest mob within 10 yd; a mob
  dying without the DoT spreads nothing; with `spread_stacks` absent, one stack goes, as today.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `build_spells.py --check --no-dbc-check` is clean.

Assumptions allowed: tie-breaking between equally near enemies.
