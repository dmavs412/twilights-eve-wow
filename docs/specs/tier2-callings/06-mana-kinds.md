Title: Mana kinds: damage that grows with a full bar, and damage paid by the mana a cast spends (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 8. Blocked by 05-aura-copies (same files).

The Leyweaver (ticket 16) hits hardest on a full mana bar and spends its mana for burst.
`school_damage` already takes `crit_pct_at_full_mana` (Ley-Spark), and `cost_kind=all_remaining`
already pays the cost then the rest of the bar up to `cost_extra_max`. Two params are missing.

## The params

1. **`pct_at_full_mana=N`** on `school_damage`: the hit deals up to N% more, scaled by the
   caster's mana fill after the cost is paid, exactly as `crit_pct_at_full_mana` scales its crit
   points (a bar 80% full after the cost gives 0.8 N). On a channel (`cast_kind=channel`) each
   tick reads the fill at that tick. It sums with `controlled_pct`, `bleeding_pct`, `vs_aura_pct`
   and `vs_type_pct`.
2. **`per_mana_pct=N`** on `school_damage`, only on a row with `cost_kind=all_remaining`: the hit
   deals N% of the mana the cast actually spent (the cost plus the extra taken) as extra damage
   in the row's school, before crits and resistances, added to `amount`.

## Where

- `crates/twilight-realm/src/world/cast.rs`: find `crit_pct_at_full_mana` and the
  `all_remaining` cost payment; the new params read the same numbers.
- `crates/twilight-realm/src/callings.rs`: `SchoolDamage`'s params in the `Effect` enum (line
  1442).
- `tools/twilights-eve/build_spells.py`: accept both on `school_damage`; refuse `per_mana_pct`
  on a row whose `cost_kind` is not `all_remaining`, and either below 1. Tests in
  `tools/twilights-eve/tests/test_build_spells.py`.
- `docs/spell_pipeline.md` (beside `crit_pct_at_full_mana`, line 188 onward) and
  `docs/agents/calling-script-guide.md`.

## Failing tests first

New file `crates/twilight-realm/tests/mana_kinds.rs`, in the form of `tests/initiate_spells.rs`
(a Mage-class test Calling in inline TOML).

- `pct_at_full_mana=50`, crits and misses ruled out as that file rules them out: at a full bar
  after the cost the hit is 1.5 times the base; at half a bar after the cost, 1.25 times.
- On a channel of three ticks with the bar draining, each tick's bonus follows the fill at that
  tick.
- `per_mana_pct=50` with `cost_kind=all_remaining` and `cost_extra_max` 400: from a bar with 600
  above the base cost the hit adds half of (cost plus 400); from a bar with 100 above, half of
  (cost plus 100).

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `build_spells.py --check --no-dbc-check` is clean.

Assumptions allowed: rounding of the scaled bonus (follow `crit_pct_at_full_mana`).
