# The client's game definitions come from our data

Supersedes ADR 0003 (client-side spell overrides) and ADR 0004 (power from Class); amends ADR 0008
(creature spells).

benilla is our own client, so its spells, talents and Callings no longer borrow stock 1.12.1 rows.
Every spell a Calling or creature casts is built from scratch: the generated client patch inserts a
complete row into the client's in-memory `SpellCatalog`, under an id no stock spell uses, and the
client draws it on the same code path as a stock spell. A Calling stands on its own instead of
riding a vanilla Class.

- **Ids.** One `spell_id` column in `master.csv`, a whole number from 1,000,000 up, unique across
  the sheet and `creature_spells.toml`, never reused. It replaces `client_spell_id` and
  `realm_spell_id`. Saves store these ids; the save version was bumped once and the loader remaps
  pre-migration stock ids (action bar and talents) through a one-time old-to-new table, clearing
  any id with no match.
- **Look.** `visual=<stock spell id>` borrows only that spell's look: the generator resolves its
  SpellVisual from Spell.dbc. `icon` names a PNG in `data/twilights-eve/art/` (embedded in the exe,
  checked at build) or a stock icon path. Nothing else is read from the visual's row.
- **Behaviour.** Explicit columns (school, cast_style = instant/cast/channel/next_swing/auto_repeat,
  needs_facing, dispel, mechanic) with defaults for the rest. No effect cap: the client row carries
  every sheet effect's numbers, so `$sN` and spell mods reach any effect. The realm still decides
  what happens.
- **Talents.** Our own TalentTab and Talent rows, from the CSV, keyed to the Calling. The stock
  talent frame draws them.
- **Callings.** `data/twilights-eve/callings.csv`, one row per Calling: `power` (mana, rage, energy
  or focus), `stat_profile` (an extracted vanilla class table) with percentage multipliers,
  armor and weapon proficiencies, talent and spellbook tab names and art, and the Calling's icon.
  `class` leaves `master.csv`. The client reads no class byte: every stock read of Class is
  rewired to the Calling.
- **Creature spells** stay hand-written in `creature_spells.toml` and gain `spell_id`, `name`,
  `icon`, `visual` and `school`; the generator emits their client rows.

## Considered Options

- **Keep borrowing stock rows (ADR 0003).** Rejected: each borrowed row drags in reagents,
  targets, buff slots and ranks that disagree with the sheet, which #197 could only police.
- **A separate Twilight spell catalog in the client.** Rejected: every UI reader would check two
  catalogs. One catalog, rows of one shape.
- **Writing a real Spell.dbc into a patch MPQ.** Rejected: a file pipeline for what the patch
  already does in memory.
- **Custom power kinds now.** Deferred: the four existing bars cover the Callings; a new resource
  is its own decision.
- **BLP art in an MPQ.** Rejected: PNG needs no converter and embeds like the patch JSON.

## Consequences

- The client/realm seam checks of #197 that read a stock row go away with borrowing; the buff-slot
  and talent-rank checks stay.
- CLAUDE.md's three-effect rule is gone.
- A stock 1.12.1 client cannot play Twilight's Eve (already true since ADR 0010).
- Work: spec issue "Definitions from our data" and its six tickets.
