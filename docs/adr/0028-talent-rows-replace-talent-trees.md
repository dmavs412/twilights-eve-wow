# Talent Rows replace Talent Trees and Talent Points

Every Calling had a vanilla-style Talent Tree: tabs, a grid, one Talent Point a level, prerequisites,
and a respec bought from a trainer. 346 talent rows in the master CSV served seven Callings, the
client drew them in the stock talent frame from `Talent.dbc`, and the realm read them through
`crates/twilight-realm/src/talents.rs`. Davis found the tree slow to read and the points a chore,
and no pick changed how a cast played. This ADR replaces the whole thing. Grilled 2026-10-28, spec
#498. Prototype: `docs/specs/talents/skill-variants-prototype.html`. Amended 2026-10-29 (see the
last section): rows open at fixed Row Levels shared by a Tier, and the Tier 1 Callings get six.

## Decision

- **A Calling has Talent Rows, each a row of three.** A row opens at one of its Tier's Row Levels,
  the way Mists of Pandaria's talent tiers open at fixed levels: 2, 4, 6 and 8 at Tier 0, and 4, 8,
  12, 16, 20 and 24 at Tier 1. The player sets one of its three choices. An empty row can be set anywhere, at no cost.
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
  ranks. Basic Warrior ships four Skill Choice rows; Acolyte's sixth row is the first Talent row.

## Consequences

- Row Levels are per Tier, written on each calling row as `row_levels` (`2|4|6|8`, or
  `4|8|12|16|20|24`). The build refuses a Calling whose rows do not fill that list exactly, one row
  a level, and two Callings of one Tier with different lists. A row still opens no earlier than
  its skill's `learn_level`.
- Every Tier 1 Calling has six rows, one per skill it learns at level 1. The Kick (level 10) and
  the utility (level 20) never get a row. Ranger has seven such skills, so one shot has none
  (Auto Shot never has one); Acolyte has five, so its sixth row is a Talent row. Their first
  choices are placeholders written by the Build lane; each Calling's rows get a grill of their
  own later.
- A Calling whose calling row has no `row_levels` has no rows: Zealot (Tier 2) for now. Tier 2
  and Tier 3 Row Levels are set when those Tiers are built.
- The 25 realm test files that learned talents to test a skill lose that step or go, and the
  tests of the tree itself go. The Skill Choice tests take their place.
- Balance work on a skill now covers four spells: the plain one and its three choices. The balance
  rules in `docs/balance/rules.md` read a choice as the skill with that strength added.
- `docs/specs/talents/README.md` holds the layout, the row vocabulary and Basic Warrior's twelve
  choices, with the prototype as the reference for the rules.

## Amended 2026-10-29

Davis asked for every Calling's old talents to go and for milestone choices in the Mists of
Pandaria style: four for Basic Warrior, six for each Tier 1 Calling. Only the row levels changed.
The first version let each skill pick its own `row_level`, and left the Tier 1 Callings with empty
rows until a grill each. Now a Tier shares one list of Row Levels, and the Tier 1 Callings get
placeholder rows at once. Row content, the innkeeper rule, the panel and Advancement are as above.
The talent trees go for every Calling, Zealot's TOML tree included. The balance model reads every
open row set to its most damaging choice, and nothing makes up the old trees' passive power,
since only Vanguard's talents ever worked in play (#187).
