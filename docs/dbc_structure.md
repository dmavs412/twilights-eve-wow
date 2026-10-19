# DBC structure: what the client reads, and where the realm takes over

The 1.12.1 client reads its spell data from `Spell.dbc`, `Talent.dbc` and `TalentTab.dbc` in the
game's MPQ patch chain. The client shows a spell and decides whether to send a cast from those
rows alone. The Local Realm never reads them at runtime: it reads `server_spells.json`. The two
meet only through `client_dbc_patch.json`, which the client applies to its rows in memory at load
(ADR 0003). Keeping the two apart is what lets the realm become a server later without the client
changing.

## Spell.dbc

A record is 173 columns of 4 bytes (692 bytes). The column numbers the loader uses, verified
against the client binary and vmangos's `spell_template`, live in
`crates/benilla-formats/src/spells/mod.rs`; read that file for the full list rather than a copy
here. The columns the client gates a cast on, and so the ones the CSV's `client_*` columns
mirror:

| column | field | what the client does with it |
|---|---|---|
| 1 | School | damage school, for resist text |
| 2 | Category | shared cooldown group |
| 11, 12 | Stances, StancesNot | refuses the cast in any form the mask excludes, before sending |
| 16, 17 | CasterAuraState, TargetAuraState | greys the button until the state holds (Overpower after a dodge) |
| 19, 20 | RecoveryTime, CategoryRecoveryTime | the cooldown it shows |
| 31, 32 | PowerType, ManaCost | greys a spell whose power the bar does not hold; rage costs are stored in tenths |
| 36 | RangeIndex | into `SpellRange.dbc`; refuses an out-of-range target |
| 58 to 60 | EquippedItemClass, SubclassMask, InventoryTypeMask | refuses without the weapon kind |
| 61 to 63 | Effect[3] | the three effect slots |
| 157, 158 | StartRecoveryCategory, StartRecoveryTime | the global cooldown |
| 160 to 162 | SpellFamilyName, SpellFamilyFlags | which talent spellmods reach the spell; Twilight adds op 29 `MIN_RANGE` past vanilla's ops (ADR 0010) |
| 165 | PreventionType | silence or pacify |

**Effects.** Every per-effect field is a `[3]` array in consecutive columns from `Effect` at 61:
die sides, base dice, dice per level, real points per level, base points, effect mechanic (79),
implicit target A (82) and B (85), radius, apply-aura name, amplitude, chain target, item type,
misc value (106), trigger spell (109), points per combo point. Three slots is the whole row.

**The three-effect limit.** A passive talent rank is a spell whose effects are `APPLY_AURA` with
an aura name per slot, so a talent can carry at most three auras the client knows about. The
client needs those only for what it shows or gates itself: a cost, a crit chance on the character
sheet, a tooltip number. Everything a talent does beyond that is the realm's business and needs no
slot at all. Hence the rule in `CLAUDE.md`: effects 1 to 3 may go to the patch, effects 4 and 5
are realm-only components, and the `description` text carries them to the tooltip.

**Borrowed spells.** A Calling that casts another class's spell (Zealot's Seals) gets that spell's
row rewritten in memory: power type, cost, stances cleared, family name set to the character's
class, family flags set to one bit no spell of that class uses. The CSV's `borrowed_*` columns
and the `client_*` columns are the source; the spell id stays the real one.

## Talent.dbc and TalentTab.dbc

`Talent.dbc`: id, tab id, row, column, the rank spells (five slots), prerequisite talents (three)
with their ranks, flags, required spell. `TalentTab.dbc`: id, name, icon, race mask, class mask,
order, background texture. The loader's columns are in `crates/benilla-formats/src/talents.rs`.

A Calling's tree is real `Talent.dbc` rows re-seated on a grid of the Calling's own: the CSV's
`talent_id` and `spell_id` per rank are the real records, and `tab`, `tab_row`, `tab_column` are
the seat. The client's talent catalog is built from the patch file, so the stock talent frame
shows the Calling's tabs. A row opens at five points per row above it, as in vanilla.

## What the client reads from the realm instead

Everything the client cannot learn from a row comes over the protocol: aura slots, cooldown
events, spell start and go, power values, the character sheet's numbers. The realm sends them
from its components. A hidden (realm-only) effect never appears in an aura slot, so it never
needs a `Spell.dbc` aura row, and the client sees it only through what it changes.
