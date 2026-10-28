# Talent Rows replace Talent Trees and Talent Points

Every Calling had a vanilla-style Talent Tree: tabs, a grid, one Talent Point a level, prerequisites,
and a respec bought from a trainer. 346 talent rows in the master CSV served seven Callings, the
client drew them in the stock talent frame from `Talent.dbc`, and the realm read them through
`crates/twilight-realm/src/talents.rs`. Davis found the tree slow to read and the points a chore,
and no pick changed how a cast played. This ADR replaces the whole thing. Grilled 2026-10-28, spec
#498. Prototype: `docs/specs/talents/skill-variants-prototype.html`.

## Decision

- **A Calling has Talent Rows, each a row of three.** A row opens at a level (`row_level` in the
  sheet). The player sets one of its three choices. An empty row can be set anywhere, at no cost.
  A set row changes only while an innkeeper is within service range, at no cost, reached by the
  innkeeper's gossip line "I want to rethink my talents". A set row never clears back to empty: a
  choice is a strict upgrade, so there is nothing to go back to. Advancement clears every row, as
  it replaces the skills.
- **Two kinds of row.** A Skill Choice row belongs to one skill: its three choices are variants of
  that skill, each a strict upgrade (nothing taken away, one strength added), each its own spell
  with its own id, name, icon and description. While set, the choice replaces the plain skill in
  the spell book and on the action bar. A Talent row belongs to no skill: its three choices are
  passives (one-rank effects in the old talent vocabulary) or actives (a skill of their own,
  learned while set). Both kinds share the panel, the levels, the set rule and the innkeeper.
- **A Skill Choice is an overlay row in the sheet.** A `talent` row with `variant_of=<skill key>`
  fills only the cells that differ from the skill's row; `build_spells.py` merges it over the
  skill row and emits a full spell under the choice's own id. A base-skill retune edits one row.
  A cell of `-` blanks the inherited cell. The choice takes the skill's `SpellFamilyFlags` bit, so
  a spell mod aimed at the skill reaches the choice too.
- **The realm decides, the client draws (ADR 0013).** The realm sends `SessionEvent::TalentRows`:
  every row of the current Calling with its level, whether it is open, which choice is set, and
  whether an innkeeper is in range. The client's Talents panel (`assets/ui/Talents.xml`, in place
  of the stock talent frame) draws rows top to bottom in the Mists of Pandaria layout and sends a
  click back as `RealmCommand::SetTalentRow`. The client reads a choice's icon and description from
  its own Spell.dbc row, patched like any skill. `Talent.dbc` and `TalentTab.dbc` are no longer
  patched, and the client's talent catalog goes.
- **Talent Points, the tree columns and the respec go.** `PLAYER_CHARACTER_POINTS1` stays 0. The
  tree columns of the sheet (`talent_id`, `source_tab_id`, `tab`, `tab_row`, `tab_column`,
  `prereq_key`, `prereq_rank`, `max_rank`, `tab_id`, `background`, `tab_order`) stay in the header
  and empty; the build refuses a filled one. The `talent_tab` kind goes. The save drops `talents`
  and `talent_wipes` and gains `rows`, the set choice per row key.
- **The talent effect machinery stays for Talent rows.** `talents.rs`'s queries (`on_hit`, `procs`,
  `stats`, `spell_mods` and the rest) read the set Talent choices at rank one instead of learned
  ranks. No shipped Calling has a Talent row yet; Basic Warrior ships four Skill Choice rows.

## Consequences

- A Tier 1 Calling plays with empty rows, so its plain skills, until its own follow-up ticket set
  writes its choices. Each Calling's rows are one ticket set, grilled from this ADR, not a new one.
- Row levels are per Calling. A row opens no earlier than its skill's `learn_level` and no later
  than the Tier Cap; the build enforces both.
- The 25 realm test files that learned talents to test a skill lose that step or go, and the
  tests of the tree itself go. The Skill Choice tests take their place.
- Balance work on a skill now covers four spells: the plain one and its three choices. The balance
  rules in `docs/balance/rules.md` read a choice as the skill with that strength added.
- `docs/specs/talents/README.md` holds the layout, the row vocabulary and Basic Warrior's twelve
  choices, with the prototype as the reference for the rules.
