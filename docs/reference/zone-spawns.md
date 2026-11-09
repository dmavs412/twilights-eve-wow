# Duskwood, Redridge, Stormwind City and Burning Steppes spawns (Twilight's Eve, spec #466, #634)

`data/twilights-eve/duskwood_spawns.json`, `redridge_spawns.json`, `stormwind_spawns.json` and
`burning_steppes_spawns.json` hold every visible creature spawn in Duskwood (map 0, zone 10),
Redridge Mountains (zone 44), Stormwind City (zone 1519) and the Burning Steppes (zone 46) at
patch 1.12. They come from the same importer, the same source and
the same `--all-creatures` rules as Elwynn Forest (`docs/reference/elwynn-spawns.md`), which in
turn points at `docs/reference/barrens-spawns.md` for the source DB, the zone-membership rule and
the cave/mine WMO lookup.

## Re-running

From the repo root, with the vmangos sqlite dump and the game data (CLAUDE.md, "This machine"):

```bash
DB="C:/Users/davis/wow-data/vmangos/mangos.sqlite"
DATA="C:/Users/davis/wow-data/Data"
python3.15 -I tools/twilights-eve/import_barrens_spawns.py --db "$DB" --data "$DATA" --source-asset db-sqlite-4641790.zip --map 0 --zone 10 --box -12000 -9500 -2500 1500 --all-creatures --out data/twilights-eve/duskwood_spawns.json
python3.15 -I tools/twilights-eve/import_barrens_spawns.py --db "$DB" --data "$DATA" --source-asset db-sqlite-4641790.zip --map 0 --zone 44 --box -10500 -8300 -4500 -1000 --all-creatures --out data/twilights-eve/redridge_spawns.json
python3.15 -I tools/twilights-eve/import_barrens_spawns.py --db "$DB" --data "$DATA" --source-asset db-sqlite-4641790.zip --map 0 --zone 1519 --box -9500 -8000 -200 1800 --all-creatures --out data/twilights-eve/stormwind_spawns.json
python3.15 -I tools/twilights-eve/import_barrens_spawns.py --db "$DB" --data "$DATA" --source-asset db-sqlite-4641790.zip --map 0 --zone 46 --box -9000 -6800 -4300 1100 --all-creatures --out data/twilights-eve/burning_steppes_spawns.json
```

Each box has margin on every side; the exact zone test does the rest.

## Counts (vmangos 4641790)

| Zone | Spawns | Entries | Hostile | Neutral | Friendly | Game-event spawns dropped | Invisible or unselectable dropped |
|---|---|---|---|---|---|---|---|
| Duskwood | 842 | 120 | 716 | 60 | 66 | 3 | 6 |
| Redridge Mountains | 796 | 122 | 637 | 96 | 63 | 2 | 0 |
| Stormwind City | 333 | 239 | 6 | 25 | 302 | 110 | 1 |
| Burning Steppes | 500 | 67 | 418 | 64 | 18 | 181 | 0 |

Interiors come from the WMO rule. Duskwood: Dawning Wood Catacombs 61 (27 under Forlorn Rowe, 18
under Raven Hill Cemetery, 16 under the Darkened Bank), Roland's Doom 7, The Stockpile 2 and
Vul'Gol Ogre Mound 6. Redridge: Render's Rock 30 and Rethban Caverns 27. Stormwind City has none.
Burning Steppes: Blackrock Mountain 39, Dustbelch Grotto 3 and Flame Crest 3.

The Burning Steppes' dropped game-event spawns are the Scourge Invasion's 179 (163 Minion
finders and 16 Necropolis parts) and the Lunar Festival's Elder Dawnstrider and Elder Rumblerock;
the dump keeps none.

No guid clashes with Elwynn or between the zones. The Burning Steppes shares one entry with the
others, 6491 Spirit Healer. Seven entries also spawn in Elwynn: 68 Stormwind City Guard, 385
Horse, 620 Chicken, 721 Rabbit, 1933 Sheep, 2442 Cow and 6491 Spirit Healer.

Spot checks: Mor'Ladim (522, guid 5086) in Duskwood, a Blackrock Gladiator (4464, guid 18385) in
Redridge, a Sewer Beast (3581, guid 86301) in Stormwind's canals and Highlord Bolvar Fordragon
(1748, guid 10495) in Stormwind Keep.

## The realm's copies: `crates/twilight-realm/content/{duskwood,redridge,stormwind,burning_steppes}.toml`

`cargo run -p twilight-realm --example import_zone` converts the four dumps, and
`tests/content_zones.rs` fails when a file and its dump disagree. The conversion is the one
`import_elwynn.rs` uses (`import_zone::convert`), so every spawn keeps its vmangos position, facing
and level range and is a camp of its own. Friendly NPCs get `friendly = true` and faction template
35, with no menus, as Elwynn's do.

`mobs::content` loads the four files on map 0 beside `elwynn.toml`. A template whose entry the
hand or Elwynn already defines is not loaded twice. The new templates are not Elwynn's vanilla
templates, and Elwynn's Level Bands and `elwynn_overrides.toml` moves touch Elwynn's own spawns
only, so a Duskwood spawn inside the bands' bounds (the Darkened Bank reaches x -10200) keeps its
own levels. Levels stay vanilla until the Level Stretch (#468).

## `content/zones.toml`

Hand-written changes laid over the generated files, which are never edited, one `[[zone]]` per
file. `removed` deletes every spawn of an entry:

- Duskwood: Unseen (3094), 28 level-49 spawns, invisible in vanilla (aura 4986).
- Stormwind City: Rift Spawn (6492), six spawns, invisible until a quest item shows them (aura
  9093).

`removed_guids` deletes single spawns: the creatures inside the lore bible's Phased Dungeons'
footprints, and the ground of camps the content puts there. The Burning Steppes loses 20 spawns
inside Blackrock Stronghold and 13 inside Dreadmaul Rock. Its eight Morgan's Vigil NPCs are Hub
NPCs (`content/hub.toml`, `import_zone.rs`'s `hub_npcs`), so `burning_steppes.toml` leaves out
their spawns.

After the removals the realm loads 814 Duskwood spawns, 796 Redridge spawns, 327 Stormwind City
spawns and 459 Burning Steppes spawns.
