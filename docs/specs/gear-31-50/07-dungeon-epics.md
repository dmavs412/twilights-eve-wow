Title: Dungeon epics: twelve epics on the four 30-50 final bosses at 5% (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 5 and 8. Blocked by 05-band-gear-41-50 (same files: `gear.csv`,
`gear.toml`, `client_dbc_patch.json`), DUNGEONS:05-mistmantle-loot, DUNGEONS:07-ilgalar-loot,
DUNGEONS:09-raven-hill-loot and DUNGEONS:11-stonewatch-loot (each writes its dungeon's set, its
bosses' rolls and `tests/dungeon_gear.rs`, and the dungeon's monsters ticket before it writes the
boss templates this ticket edits).

Three Flavoured epics per final boss, one each of Might, Swift and Mind, in the shape of Elwynn's
nine (905900-905908). Each final boss rolls 5% for one of its three, on top of its 100% set roll.
12 rows, 12 new stock icons.

## The rows

Write each with `python3.15 tools/twilights-eve/gear_sheet.py scale <source> <entry>
item_level=L required_level=R name=... icon=<icon> --display <model>` from the Elwynn epic of the
same shape. Quality Epic, `bonding = Pickup`, no `line`: each is its own item with its own icon.
Required level is three below the item level, the dungeon sets' rule. Names are placeholders.

| Entry | Dungeon | Name | Slot, type | Item level / required | Source | Points |
|---|---|---|---|---|---|---|
| 905909 | Mistmantle Manor | Stalvan's Iron Torc | Amulet, Might | 36 / 33 | 905903 | str 86, sta 43 |
| 905910 | Mistmantle Manor | Stalvan's Running Talisman | Amulet, Swift | 36 / 33 | 905904 | agi 86, sta 43 |
| 905911 | Mistmantle Manor | Stalvan's Candlewax Pendant | Amulet, Mind | 36 / 33 | 905905 | int 65, spi 32, sta 32 |
| 905912 | Tower of Ilgalar | Tower-Tuned Blade | Weapon, Sword, 1H, Might | 38 / 35 | 905900 | str 91, sta 46; DPS 72.2 |
| 905913 | Tower of Ilgalar | Valvekey Dirk | Weapon, Dagger, 1H, Swift | 38 / 35 | 905901 | agi 91, sta 46; DPS 72.2 |
| 905914 | Tower of Ilgalar | Staff of the Humming Tower | Weapon, Staff, 2H, Mind | 38 / 35 | 905902 | int 137, spell power 274; DPS 96.1 |
| 905915 | Raven Hill Deeps | Grey Deacon's Bulwark | Offhand, Shield, Might | 44 / 41 | 905906 | str 109, sta 54; armour about 1197, block about 17 |
| 905916 | Raven Hill Deeps | Gravewatch Longbow | Ranged, Bow, Swift | 44 / 41 | 905907 | agi 109, sta 54; DPS 65.3 |
| 905917 | Raven Hill Deeps | Censer of the Stilled | Offhand, Held, Mind | 44 / 41 | 905908 | int 81, spell power 164 |
| 905918 | Stonewatch Keep | Gath'Ilzogg's Warblade | Weapon, Sword, 1H, Might | 48 / 45 | 905900 | str 123, sta 61; DPS 97.0 |
| 905919 | Stonewatch Keep | Singe-Scorched Fang | Weapon, Dagger, 1H, Swift | 48 / 45 | 905901 | agi 123, sta 61; DPS 97.0 |
| 905920 | Stonewatch Keep | Emberroof Staff | Weapon, Staff, 2H, Mind | 48 / 45 | 905902 | int 184, spell power 368; DPS 129.2 |

Budgets: epic base 36 x P(L), a two-hander doubled: 129 at 36, 137 (274) at 38, 163 at 44, 184
(368) at 48. Epic DPS anchors x P(L): one-hander 19.0, staff 25.3, bow 14.4. `build_gear.py`
refuses a row more than 4 points or 10% of DPS off; the points column above is the Flavour split
(#448: Might 2/3 Strength and 1/3 Stamina; Swift 2/3 Agility and 1/3 Stamina; Mind 1/2 Intellect,
1/4 Spirit, 1/4 Stamina, and a Mind weapon or held item spends half its points as Spell Power at
2 per point, the rest on Intellect). The tool lands close to it; the build is the judge. Speeds
copy the source (sword 2.0, dagger 1.5, staff 3.0, bow 2.4).

## Icons and models

- **Icons.** 12 stock icons that `python3.15 -I tools/twilights-eve/icon_check.py` accepts and no
  item uses, checked by icon name, not display id: 3 amulets (22 free after the arc), 2 swords
  (27), 2 daggers (20), 2 staves (9), 1 shield (19), 1 held item (43) and 1 bow. Bows have at most
  one free icon; if none is left, take a free crossbow or gun icon for the Gravewatch Longbow, keep
  its type Bow and a bow model, and record it. Prefer icons whose vanilla item is from item level
  31 to 65. Write each in the row's `icon` cell.
- **Models** (decision 8). Each weapon, shield, held item and bow takes a model from `gear_sheet.py
  looks <slot> <type> <item level>`, written with `--display`: a vanilla item of the same type
  within 5 levels whose display no other row uses, a rare or epic one where the list has it. The
  amulets keep any display; their icon cell decides what shows.

## The rolls

Each final boss's template (in its dungeon file since DUNGEONS:01) gains
`extra_rolls = [{ chance = 5, one_of = [<its three>] }]`, written out in full, with a comment
naming this ticket. A creature row's `rolls` stops at the first success and the set roll is at
100%, so the epic goes in `extra_rolls`, which rolls on its own (`src/world.rs` line 1865, Morgan's
Loaded Die's form in `content/mobs.toml` line 972).

| Template | File | Epics |
|---|---|---|
| 97229 Stalvan Mistmantle | `content/mistmantle_manor.toml` | 905909, 905910, 905911 |
| 97244 Morganth | `content/ilgalar.toml` | 905912, 905913, 905914 |
| 97265 The Grey Deacon | `content/raven_hill_deeps.toml` | 905915, 905916, 905917 |
| 97287 Gath'Ilzogg | `content/stonewatch_keep.toml` | 905918, 905919, 905920 |

Singe (97288) gets none: one 5% roll on the warlord keeps Stonewatch's epic rate the same as the
other three. Leave every creature row's `rolls` and `drop` alone.

## Failing tests first

`crates/twilight-realm/tests/dungeon_gear.rs` gains:

- 905909 to 905920 exist at quality 4, at the table's item and required levels, `bonding` Pickup
  (2 in `gear.toml`), with no `line`;
- each of the four templates above has exactly one `extra_rolls` entry, chance 5, listing exactly
  its three epics (`tests/common/mod.rs`'s `mob_template` finds a dungeon template);
- Singe's template and every other creature template of the four files has no `extra_rolls`;
- each final boss's creature row still rolls its set at 100.

## Acceptance

- `build_gear.py` passes; `build_spells.py --data "$WOW_DATA"` regenerates the patch;
  `icon_check.py` reports no new clash.
- `python3.15 tools/twilights-eve/gear_sheet.py look-check 905912-905920` exits 0.
- The tests pass; `cargo test -q -p twilight-realm` passes; `python3.15 -I
  tools/twilights-eve/ground_check.py` flags nothing; the pipeline unittests pass.

## Assumptions allowed

- Names, if `build_gear.py` refuses one.
- The icon and model picked for each row, named in the commit.
- If a dungeon's final boss template has another entry than the table's when this runs, use the
  final boss's template (Stalvan, Morganth, the Grey Deacon, Gath'Ilzogg, whatever their entries)
  and record it.
