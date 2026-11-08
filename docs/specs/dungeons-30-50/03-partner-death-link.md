Title: A partner-death link: the survivor enrages (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 15. Blocked by 02-frozen-until (both edit `src/phased.rs` and
`content/ward_test.toml`).

Gath'Ilzogg and Singe fight together on Stonewatch Keep's top storey (ticket 10), and when one
dies the other enrages. Today no creature reacts to another's death except through the Appears
keys (`after_dead`, `until_dead`), which bring a creature in or take it out. This ticket adds one
generic Phased Dungeon creature key.

## The key

A Phased Dungeon `[[creature]]` row may carry:

```toml
on_partner_death = { guid = 974350, spell = "enraged_partner" }
```

- `guid` is another creature row of the same dungeon; `spell` a Creature Spell key whose effect is
  `stance` (`content/creature_spells.toml`, the `stance` kind).
- While the creature of `guid` is dead in this Run, and this creature is alive and engaged and
  not already in that spell's stance, it puts the stance up at once: no cast bar, no cooldown, no
  `chance` roll, and any other stance it holds comes down first (the usual rule that a stance
  already up blocks a cast does not apply here). The spell's `warning`, if any, goes out to the
  Phase as a cast's warning does. The buff shows as a stance's buff does.
- The stance's own rules end it: with `duration_ms = 0` it holds until the creature leaves combat
  or dies. So a survivor that leashes and is pulled again enrages again, because its partner is
  still dead.
- The spell is otherwise never cast: a template that lists it in `spells` is refused at load, as
  a marker is. Give such a spell `chance = 0` and cast columns of 0, and document in
  `creature_spells.toml`'s header, under `stance`, that a stance spell may be forced by
  `on_partner_death`.
- The load panics on a `guid` that is no creature of the same dungeon, a `spell` that is not a
  `stance`, or a row naming itself.
- Two rows may name each other: each is the other's partner.

## The code

- `src/phased.rs`: `Creature` gains `on_partner_death: Option<PartnerDeath>` (`guid`, `spell`),
  documented on the struct.
- `src/mobs.rs`: the spawn rows built from `phased::content()` carry it, as they carry `appears`
  and (after 02) `frozen_until`.
- A Run's mob made from such a row records its partner's mob guid in this Run and the stance's
  spell id.
- One generic system on the shard's tick, beside 02's thaw system: for each mob with a partner
  link, if the partner is dead and the mob is alive, engaged and not in that stance, put the
  stance up the way `stance` effects are applied in `src/world/creature_cast.rs` (search for the
  `stance` effect), with the warning and the buff's events.

## Failing tests first

`tests/phased_wards.rs`, with its harness: add to `content/ward_test.toml` two hostile rows that
share a camp number (guids 800020 and 800021), each with `on_partner_death` naming the other and
a test stance spell. That spell is a new row at the end of `creature_spells.toml`'s Ward Test or
test rows, or, if there is no such section, the first row of a new `# ── Test rows ──` section
after Stonewatch Keep's section: key `test_enrage`, a `stance { damage_dealt_pct = 50,
damage_taken_pct = 0, duration_ms = 0 }`, the next free id after 1900099 (1900100), chance 0.
Regenerate the patch with `build_spells.py` and run the pipeline unittests.

- Kill 800020 while 800021 fights: 800021 is in the stance within a tick, its swings land 50%
  harder, and the warning reaches the party.
- Kill 800021 first instead: 800020 enrages.
- With neither dead, neither is ever in the stance.
- The survivor leaves combat (leash) and the stance ends; pulled again, it enrages again.
- A template with `test_enrage` in `spells` panics at load.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests pass;
  `ground_check.py` flags nothing.

## Assumptions allowed

- Where the system lives and the field shapes, if the behaviour holds.
- The test rows' templates and positions inside the Ward Test's box.
