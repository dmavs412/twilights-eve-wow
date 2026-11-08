Title: The Calling Choice at every Tier Cap: a per-Tier record, Tier 1 options, and Zealot in the popup (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 1, 4, 5 and 11. No blockers.

Today the Calling Choice popup (#414) opens only for a Tier 0 Calling at its Tier Cap, and one
saved bool remembers the pick. This ticket opens it at the Tier 1 Cap too, listing the Tier 2
Callings of the character's own Line, and writes the docs the Tier 2 tickets lean on.

## Behaviour

- The popup is due when the character is at its Calling's Tier Cap, has not picked at this Tier,
  and at least one quest with `calling_choice = true` has `advancement.from` equal to the
  character's Calling key. Its options are exactly those quests. A Tier 1 Calling with no such
  quest (Cutthroat until ticket 13 lands) gets no popup.
- The save replaces `calling_chosen: bool` with `chosen_tiers`, the Tiers picked at, as a list
  (`chosen_tiers = [0]`, then `[0, 1]`). An old save's `calling_chosen = true` loads as `[0]`.
  Nothing is written when the list is empty, as `calling_chosen` was skipped when false.
- `choose` records the character's current Tier. The system line stays "Your path is chosen.
  Speak with {giver} to begin." for a Tier 0 pick. For a Tier 1 pick it says "Your path is
  chosen. Your quest log tells you where to begin looking." and names nobody, because Tier 2
  trainers are hidden.
- The login rule stays: no popup while an Advancement Quest is in the log.
- The content check allows a Calling Choice quest whose `from` is a Calling of Tier 0 or Tier 1
  (today it must be `basic_warrior`). "Two Calling Choice quests advance into X" and "X has no
  blurb" stay.

## Data in this ticket

- `crates/twilight-realm/content/quests.toml`, 64003 "The Zealot's Vow" (line 1521): add
  `calling_choice = true`. Its text and giver stay until ticket 12 rewrites them. 64002 stays
  `completable = false` and not a choice until ticket 11.
- `data/twilights-eve/callings.csv`, the `zealot` row: blurb "A holy fury in plate. Stands in its
  own Consecration, judges its Seal to mend itself, and is the dead's worst enemy." and armor
  `cloth|leather|mail|plate|shield`. The kit stays the TOML's until ticket 12.
- Run `~/.local/bin/python3.15 tools/twilights-eve/build_spells.py --data "$WOW_DATA"` and commit
  `callings.csv` with both generated files.

## Docs in this ticket

- `docs/adr/0028-talent-rows-replace-talent-trees.md`: add a section "Amended 2026-11-08: Tier 2
  rows". Tier 2's Row Levels are `5|10|15|20|25|30|35|40|45|50`. Every Tier 2 skill gets a Skill
  Choice row, the Kick and the utility included, and the two rows left are Talent rows. A Tier 2
  kit learns the two carried skills, the Kick and two new skills at level 1, then one new skill at
  10, one at 20 and the utility at 30 (ADR 0020's "Kicks at 10, utilities at 20" is Tier 1's). No
  Tier 1 choice carries into Tier 2.
- `GLOSSARY.md`, the entry that names the Vanguard Line (Vanguard, Ironclad, Zealot, Bastion,
  Templar): add each Line's Tier 2 pair: Cutthroat to Bloodletter and Nightblade, Initiate to
  Pyromancer and Leyweaver, Ranger to Sharpshooter and Trapper, Acolyte to Blightcaller and
  Bloodmender. Keep the entry's form.

## Where

- `crates/twilight-realm/src/calling_choice.rs`: `calling_choice` (line 11) gates on `tier == 0`
  and `!c.calling_chosen`; `choose` (line 54) sets the bool and pushes the system line (lines 66
  to 73). `calling_choices()` is in `src/quests/content.rs` line 894.
- `crates/twilight-realm/src/characters.rs`: the field (line 107) and its default (line 369).
- `crates/twilight-realm/src/save.rs`: the saved field (line 252) and both conversions (lines 537
  and 662).
- `crates/twilight-realm/src/quests/content.rs`: the Calling Choice checks at lines 583 to 600
  and 764 to 775.

## Failing tests first

Extend `crates/twilight-realm/tests/calling_choice.rs` (its setup brings a character to a Tier
Cap; read it once):

- A level 30 Vanguard gets `SessionEvent::CallingChoice` at login and at the level-up to 30,
  listing 64003 (Zealot) alone.
- Choosing 64003 puts it in the log, pushes the Tier 1 system line naming nobody, and saves
  `chosen_tiers = [0, 1]` for a character advanced from Basic Warrior with the popup.
- After the pick, the popup does not come back at login, even with 64003 abandoned.
- A level 30 Cutthroat gets no popup.
- A Basic Warrior at 10 still gets the five Tier 1 options and the old system line.
- In `tests/saving_quests.rs` (lines 166 to 174 test the old bool): an old blob with
  `calling_chosen = true` loads as having picked at Tier 0, and saves back as
  `chosen_tiers = [0]`; a blob with neither writes neither.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; the pipeline unittests
  (`~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests`) pass;
  `build_spells.py --check --no-dbc-check` is clean after the rebuild.

Assumptions allowed: the save key's name, the exact wording of the Tier 1 system line.
