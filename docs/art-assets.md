# Art assets still to paint

The one list of art Davis still has to make. Each spec that ships placeholder art adds its
assets here, with the exact file name and size the game loads, so a painted file drops in under
the same name. Delete a row once its painted file is committed. When Davis asks "what art assets
do we need?", the answer is this file.

Item icons follow [no duplicate icons]: a painted 128 x 128 PNG in `data/twilights-eve/art/`,
named in the item row's `icon` field, with `icon_base` and a synthetic `display_id` (995000 up,
spec #395). Until it is painted, an item borrows a stock icon that no other item uses.

[no duplicate icons]: ../tools/twilights-eve/icon_check.py

## Open from earlier specs

| Asset | Where | Notes |
|---|---|---|
| Fishing pips, restyled | `crates/benilla-app/assets/ui/fishing/pip_on.tga`, `pip_off.tga` | The old fishing pips stand in on the Journal (#396). |

## Herbalism (spec #435)

The 37 herbalism items wear Davis's painted icons (2026-10-27). The eleven stock herbs (Peacebloom, Silverleaf, Earthroot, Mageroyal, Briarthorn, Swiftthistle,
Stranglekelp, Bruiseweed, Wild Steelbloom, Grave Moss, Kingsblood) keep their stock icons and
need no art.

### Forager's Grid window, RGBA TGA, placeholder

Files in `crates/benilla-app/assets/ui/herbalism/`, loaded as `Interface\Benilla\herbalism\<name>`.
Every size is a power of two, as the 1.12 UI needs. `tools/twilights-eve/placeholder_herb_art.py`
writes flat placeholders at these sizes (#481, spec #479) and checks each size. Paint each file at
twice its shipped size, as a PNG under `data/twilights-eve/art/herbalism/<name>.png`;
`tools/twilights-eve/import_art.py --size` shrinks it to the shipped size. The prototype's CSS
(`.bed`, `.bush`, `.glint`, `.stake`, `.pip`) and its `GLYPHS` table give the shapes and colours.

All 34 rows are placeholders waiting for Davis's painting.

| File | Shipped size | What it shows | State |
|---|---|---|---|
| `bush_hidden_1` to `_3` | 128 x 128 | Three shapes of an unpulled bush, leafy green, base at the bottom centre | placeholder |
| `bush_wilted_1` to `_3` | 128 x 128 | The same three shapes, wilted brown-grey: a hidden tile known to be a weed | placeholder |
| `bush_thorn_1` to `_3` | 128 x 128 | The same three shapes, red: a hidden tile known to be a thorn | placeholder |
| `bush_glow` | 128 x 128 | Overlay: warm gold glow and four sparkles, transparent elsewhere, laid over any bush known to be a plant | placeholder |
| `mound` | 128 x 128 | A pulled tile: a small dark dirt mound | placeholder |
| `mound_thorn` | 128 x 128 | A pulled thorn: a reddish mound | placeholder |
| `bed_corner_tl`, `_tr`, `_bl`, `_br` | 64 x 64 | The hedge border's rounded corners | placeholder |
| `bed_edge_top`, `_bottom` | 64 x 64 | Hedge edge, tiled across | placeholder |
| `bed_edge_left`, `_right` | 64 x 64 | Hedge edge, tiled down | placeholder |
| `bed_soil` | 128 x 128 | Dark soil with faint specks, tiling seamlessly in both directions | placeholder |
| `stake` | 32 x 64 | A wooden board on a post; the game prints the number on the board | placeholder |
| `pip_on`, `pip_off` | 16 x 16 | The gold diamond pull pip, lit and dark | placeholder |
| `glyph_flower`, `_leaf`, `_root`, `_berry`, `_mushroom`, `_nest`, `_nut`, `_honeycomb`, `_sap`, `_purse` | 64 x 64 | A plant part's glyph in white on transparent; the game tints it by rarity | placeholder |

These earlier placeholders stay as they are:

| File | Size | What it shows |
|---|---|---|
| `tile_highlight.tga` | 64 x 64 | Gold ring over the tiles a stake counts, on hover |
| `tile_target.tga` | 64 x 64 | Dashed outline over the tiles a tool skill will cover |
| `part_weed.tga` | 32 x 32 | Weed glyph on a pulled weed |
| `part_thorn.tga` | 32 x 32 | Thorn glyph on a pulled thorn |

The window's panel, the pull log's panel, chips, XP bar and tour outline are drawn from colour and
need no art. The old 64 x 64 `bush_hidden`, `bush_wilted`, `bush_thorn`, `soil` and `soil_thorn`
were deleted in #482, when the rebuilt window stopped loading them.

### Minimap

| File | Size | What it shows |
|---|---|---|
| `crates/benilla-app/assets/ui/herbalism/minimap_leaf.tga` | 32 x 32 | The leaf drawn on the minimap at each live Herb Patch while Find Herbs is on |

### Journal

The Herbalism bookmark (`journal/prof_herbalism.tga`) and the Elwynn, Redridge and Duskwood zone
scenes already exist. The Herbalism tab reuses every other Journal texture, so it needs nothing
new.

### World model

A Herb Patch uses a stock 1.12 bush model, so it needs no art.

## Cooking and alchemy (spec #447)

Alchemy keeps one icon per product type (ADR 0025): the five Strength Steps of a potion share
its icon. Cooking has one item row per recipe, so every recipe has its own icon.

### Alchemy item icons, painted

Davis painted 47 alchemy icons for the 2026-10-25 design. They are 128 x 128 PNGs in
`data/twilights-eve/art/`. `<p>` is one of `arcane`, `calming`, `fortifying`, `restorative`,
`toxic`, `volatile`. The Alchemy prototype (ADR 0025, amended 2026-11-01) uses 36 of them:
every row below whose `<p>` is not `toxic`, and Murky Sludge. Its `[[product]]` rows in
`docs/specs/alchemy/alchemy.toml` name each one. The 11 others (`potion_toxic`, `oil_toxic`,
`tincture_toxic`, `essence_toxic`, the six flasks with `toxic` in their name, and `fish_oil`)
stay in the folder, unused.

| Group | Files | Painting |
|---|---|---|
| Potions | `potion_<p>.png` | a round bottle; Volatile is a lit bomb |
| Weapon oils | `oil_<p>.png` | an open tin with a rag |
| Tinctures | `tincture_<p>.png` | a dropper bottle |
| Essences | `essence_<p>.png` | a crystal in an iron cage |
| Flasks of two Essences | `flask_<p1>_<p2>.png`, the two in alphabetical order | a gold flask, one colour per Essence |
| Flasks of one Essence twice | `flask_<p>_<p>.png` | a tall bottle with a gem stopper |
| Fish Oil, Murky Sludge | `fish_oil.png`, `murky_sludge.png` | |

Each Property has one colour: Arcane purple, Calming cyan, Fortifying amber, Restorative red,
Toxic green, Volatile orange.

### Hybrid potion icons, placeholders (spec #522)

Ten 128 x 128 PNGs in `data/twilights-eve/art/`, `hybrid_<p1>_<p2>.png` with the two Properties
in alphabetical order, as the flasks are: `hybrid_fortifying_restorative` (Trollblood Draught),
`hybrid_restorative_volatile` (Bloodfire Bomb), `hybrid_arcane_restorative` (Rejuvenating
Draught), `hybrid_calming_restorative` (Deepsleep Potion), `hybrid_fortifying_volatile` (Ember
Shield), `hybrid_arcane_fortifying` (Mana Shield), `hybrid_calming_fortifying` (Steady Stance),
`hybrid_arcane_volatile` (Spellfire Bomb), `hybrid_calming_volatile` (Smoke Bomb),
`hybrid_arcane_calming` (Invisibility Draught). The placeholder is the round potion bottle
split down the middle in the two Properties' colours, painted by
`tools/twilights-eve/placeholder_alchemy_art.py` (#541); Davis's icons replace them under the
same names.

### Alchemy Journal bookmark, placeholder

`crates/benilla-app/assets/ui/journal/prof_alchemy.tga`, 128 x 128 RGBA TGA: the Journal's
Alchemy bookmark, beside `prof_cooking`. The placeholder is a round flask of green liquid on
brown, painted by #546; Davis's icon replaces it under the same name.

### Cooking item icons, painted

Davis painted 72 cooking icons: 22 cut baits and 50 meals. They are 128 x 128 PNGs in
`data/twilights-eve/art/`, waiting for the cooking item rows. The 50 meals are the 30 named fish
dishes, the 16 legendary meals, the 3 legendary plant meals and Hearty Mush. The 44 grid
recipes have placeholders (below).

**Cut baits**, `cut_<fish>.png`, one per common and uncommon fish: `brilliant_smallfish`,
`bristle_whisker_catfish`, `canal_eel`, `darkshire_bleak`, `everstill_perch`, `goldshire_perch`,
`gravemoss_loach`, `harbour_herring`, `hushed_bank_grayling`, `ironjaw_pike`, `lakeshire_sunfish`,
`longjaw_mud_snapper`, `millrace_trout`, `mirror_lake_shiner`, `mithril_head_trout`,
`northshire_brook_trout`, `oily_blackmouth`, `rainbow_fin_albacore`, `ravenwood_chub`,
`redridge_bluegill`, `slitherskin_mackerel`, `stonefield_carp`. The paintings arrived in this
order, and each skin matches its fish.

**Named meals.** The painting shows which meal it is.

| File | Painting |
|---|---|
| `hearty_mush.png` | a plain bowl of mush |
| `meal_emberbloom.png` | chili with a red flower in a gold-trimmed bowl |
| `meal_moonpetal.png` | a petal tart with a crescent moon |
| `meal_widows_lily.png` | a teacup with a white lily |
| `meal_blackrock_bloodfin.png` | a red fin in an iron skillet |
| `meal_brother_bass.png` | a fish with a pastry cross |
| `meal_gnoll_tooth_snapper.png` | a red fish with a tooth tied on |
| `meal_moonless_muskie.png` | a dark fish on a dark plate with violets |
| `meal_moonlit_gar.png` | a gar with lemons and blue flowers |
| `meal_stitchjaw_eel.png` | a stew with a toothed jaw |
| `meal_the_harbourmasters_eel.png` | an eel on a pie with an anchor |
| `meal_the_lightscale.png` | a pale shining fish with white flowers |
| `meal_the_millstone_carp.png` | a fish between millstones |
| `meal_the_twilight_koi.png` | a purple fish on a crescent-moon stand |
| `meal_worgfang_pike.png` | a fanged pike in a fang-trimmed bowl |

**The 35 paintings that arrived unnamed**, matched to the dish each shows on 2026-10-27 and renamed
to `meal_<fish>.png` (the fish dish of that fish, or the legend's meal). A match Davis disagrees
with is a rename of two files.

| File | Was | Fish dish or legend | Painting |
|---|---|---|---|
| `meal_canal_eel.png` | `meal_01` | Canal Eel | three fish on a skewer |
| `meal_goldshire_perch.png` | `meal_02` | Goldshire Perch | a fish on a plate with lemons |
| `meal_oily_blackmouth.png` | `meal_03` | Oily Blackmouth | four fish hung to dry |
| `meal_bell_toll_bream.png` | `meal_04` | Bell-Toll Bream | a fish in a silver bowl set with a blue gem (a legend?) |
| `meal_gilded_koi.png` | `meal_05` | Gilded Koi | fish rolls on a gold plate |
| `meal_dockside_gulper.png` | `meal_06` | Dockside Gulper | a pot with a fish head and fin |
| `meal_longjaw_mud_snapper.png` | `meal_07` | Longjaw Mud Snapper | a pot of fish and vegetable stew |
| `meal_stockade_sucker.png` | `meal_08` | Stockade Sucker | a fish stuffed through its mouth with potatoes |
| `meal_bristle_whisker_catfish.png` | `meal_09` | Bristle Whisker Catfish | an orange fish in a skillet of green sauce |
| `meal_ghostscale_carp.png` | `meal_10` | Ghostscale Carp | a fish under herb sprigs |
| `meal_rainbow_fin_albacore.png` | `meal_11` | Rainbow Fin Albacore | a grilled fish steak |
| `meal_millrace_trout.png` | `meal_12` | Millrace Trout | a roast fish with lemon and potatoes |
| `meal_emberscale_darter.png` | `meal_13` | Emberscale Darter | a blackened fillet on a board |
| `meal_mirror_lake_shiner.png` | `meal_14` | Mirror Lake Shiner | a pie with a fish head through the crust |
| `meal_darkshire_bleak.png` | `meal_15` | Darkshire Bleak | a soup with a whole small fish |
| `meal_stonefield_carp.png` | `meal_16` | Stonefield Carp | fish balls in a bowl |
| `meal_brilliant_smallfish.png` | `meal_17` | Brilliant Smallfish | fried small fish in a paper cone (Brilliant Smallfish?) |
| `meal_duskfin.png` | `meal_18` | Duskfin | purple and pink fish cubes with a violet garnish |
| `meal_redridge_bluegill.png` | `meal_19` | Redridge Bluegill | a red fish stew |
| `meal_sorrowgill.png` | `meal_20` | Sorrowgill | a soup in a knot-patterned bowl (a legend?) |
| `meal_everstill_perch.png` | `meal_21` | Everstill Perch | an orange fish on leaves |
| `meal_dreaming_glassfin.png` | `meal_22` | Dreaming Glassfin | translucent raw slices (Dreaming Glassfin?) |
| `meal_mithril_head_trout.png` | `meal_23` | Mithril Head Trout | a grilled fillet on a plate |
| `meal_old_goldfin.png` | `meal_24` | Old Goldfin | a golden fish with lemon (Old Goldfin?) |
| `meal_ravenwood_chub.png` | `meal_25` | Ravenwood Chub | a fish with bread in a basket |
| `meal_everstill_whitefish.png` | `meal_26` | Everstill Whitefish | a paper parcel tied with string |
| `meal_ironjaw_pike.png` | `meal_27` | Ironjaw Pike | a pike head on a grill |
| `meal_slitherskin_mackerel.png` | `meal_28` | Slitherskin Mackerel | a soup with striped blue skin pieces (Slitherskin Mackerel?) |
| `meal_hushed_bank_grayling.png` | `meal_29` | Hushed Bank Grayling | a fillet on a big leaf |
| `meal_lakeshire_sunfish.png` | `meal_30` | Lakeshire Sunfish | a fish steak with lemon |
| `meal_gravemoss_loach.png` | `meal_31` | Gravemoss Loach | leaf-wrapped rice rolls |
| `meal_everstill_leviathan.png` | `meal_32` | Everstill Leviathan | a salt-crusted fish on paper |
| `meal_harbour_herring.png` | `meal_33` | Harbour Herring | pickled fish in a jar (Harbour Herring?) |
| `meal_northshire_brook_trout.png` | `meal_34` | Northshire Brook Trout | a pale fish soup with cabbage |
| `meal_crystal_lake_glimmerfish.png` | `meal_35` | Crystal Lake Glimmerfish | a braided pasty |

### Cooking grid recipe icons, placeholder

The 44 grid recipes (11 effects, four band tiers each, spec #447) wear placeholders painted on
2026-10-27: a bowl on the effect's colour, a border in the tier's quality colour (white, green,
blue, purple) and one to four pips for band 1, 10, 20 or 30. 128 x 128 PNGs in
`data/twilights-eve/art/`, named `meal_<recipe>.png`, band 1 first in each row. Paint over them
under the same names.

| Effect | Files |
|---|---|
| stamina | `meal_hunters_fish_stew`, `meal_woodsmans_hotpot`, `meal_darkshire_gulper_pot`, `meal_ghostlight_stew` |
| strength | `meal_spiced_fish_skewer`, `meal_brawlers_skewer`, `meal_ironjaw_kebab`, `meal_ember_truffle_kebab` |
| agility | `meal_brookside_fry`, `meal_millrace_fry`, `meal_emberscale_fry`, `meal_truffled_grayling` |
| intellect | `meal_abbey_mushroom_pie`, `meal_scholars_supper`, `meal_koi_and_ghostcap_pie`, `meal_dreaming_truffle_pie` |
| spirit | `meal_monks_honey_broth`, `meal_vigil_broth`, `meal_mithril_honey_broth`, `meal_twilight_rite` |
| xp | `meal_hearthside_fish_cakes`, `meal_travellers_supper`, `meal_lakeshire_fish_cakes`, `meal_whitefish_truffle_cakes` |
| regen | `meal_lions_pride_chowder`, `meal_lakeshore_chowder`, `meal_hushed_bank_chowder`, `meal_gravemoss_chowder` |
| speed | `meal_trail_fish_wrap`, `meal_ravenwood_wrap`, `meal_millrace_wrap`, `meal_runners_glassfin_wrap` |
| pull | `meal_foragers_stew`, `meal_redridge_foragers_pot`, `meal_gravemoss_foragers_pot`, `meal_truffle_hunters_pot` |
| rarity | `meal_anglers_soup`, `meal_ironjaw_anglers_soup`, `meal_lakeshire_anglers_soup`, `meal_hushed_bank_anglers_soup` |
| lure | `meal_smoked_fish_jerky`, `meal_blackmouth_jerky`, `meal_ironjaw_jerky`, `meal_gravemoss_jerky` |

### Cooking Journal bookmark, placeholder

`crates/benilla-app/assets/ui/journal/prof_cooking.tga`, 128 x 128 RGBA TGA: the Journal's
Cooking bookmark, beside `prof_fishing` and `prof_herbalism`. The placeholder is a bowl on brown.

### Crafting window

The Kitchen window reuses the Journal's frame art (`journal\window_bg`, `slot`, `title_plaque`,
`bottom_tab`, `divider`), so it needs no file of its own yet. The Alchemy window (spec #522)
reuses the same frame art. Its six property vials are drawn from two placeholder files in
`crates/benilla-app/assets/ui/alchemy/`: `vial_glass.tga` (64 x 256 RGBA, the empty glass
with its neck and cork, transparent inside) and `vial_liquid.tga` (64 x 256 RGBA, a white
liquid column the window tints with the Property's colour and crops to the fill). #545 painted
both as placeholders with `tools/twilights-eve/placeholder_alchemy_vials.py`: a pale-rimmed glass
with a brown cork, its inside from x 13 to 50 and y 51 to 242 (the window's
`BENILLA_ALCHEMY_GLASS` reads that inside at 40 x 160), and a grey column with a white top edge.
Real art keeps the same inside.

| Asset | Where | Notes |
|---|---|---|
| Vial glass | `crates/benilla-app/assets/ui/alchemy/vial_glass.tga`, loaded as `Interface\Benilla\alchemy\vial_glass` | 64 x 256 RGBA. Placeholder. |
| Vial liquid | `crates/benilla-app/assets/ui/alchemy/vial_liquid.tga`, loaded as `Interface\Benilla\alchemy\vial_liquid` | 64 x 256 RGBA, white or pale grey so the tint reads. Placeholder. |

## Talent Rows (spec #498)

### Talents panel

The Talents panel (`assets/ui/Talents.xml`) uses the Journal's frame art (`journal\window_bg`,
`title_plaque`, `divider`, the close button), so it needs no file of its own yet.

## Forest's Edge Noise Bar (spec #533)

| Asset | Where | Notes |
|---|---|---|
| Hogger's head | `crates/benilla-app/assets/ui/noise/hogger_head.tga`, loaded as `Interface\Benilla\noise\hogger_head` | 64 x 64 RGBA TGA at the right end of the Noise Bar, where full Noise wakes Hogger. #537 writes a flat placeholder. |

The In Cover and Sleeping auras borrow stock icons (`Ability_Stealth`, `Spell_Nature_Sleep`)
and need no art.

## Travel (spec #560)

| Asset | Where | Notes |
|---|---|---|
| Cart stop map mark | the texture `ui_world_map.rs` draws for `MapIcon::CartStop`, at the size the flight master mark uses | A wagon or a signpost. Until painted the stop borrows the flight master icon, and the locked state is the same icon desaturated. |
| Wagon map blip | the texture the world map draws for a moving wagon | A small wagon seen from above. Until painted it is the flight master icon at 70% scale. |

The Open Road buff borrows a stock icon (`Ability_Tracking`); the wagon, horse and driver are
stock models (Gypsy Wagon 5392, Caravan Packhorse 14550, a townsfolk display) and need no art.

## Trinkets (spec #458)

| Asset | Where | Notes |
|---|---|---|
| Morgan's Loaded Die icon | `data/twilights-eve/art/`, 128 x 128 | A bone die showing a six. No ItemDisplayInfo.dbc row shows a die (`INV_Misc_Dice_01` and `_02` are spell icons only), so `INV_Misc_Bone_09` stands in on the item (#567). The spell row keeps `INV_Misc_Dice_01`. |
| Apprentice's Hourglass icon | the same folder, 128 x 128 | An hourglass with sand running. Vanilla has no hourglass icon; until painted it borrows a pocket watch other than `INV_Misc_PocketWatch_02`, which another item holds. |
| Vael's Echo Shard icon | the same folder, 128 x 128, only if no stock shard icon is free | A violet crystal shard with rings around it. A free stock shard or crystal icon stands in. |

The buffs (Quick Hands, Blood Luck, Sixes, Falling Sand, Echo) borrow stock spell icons and need
no art.

## Writs (spec #557)

| Asset | Where | Notes |
|---|---|---|
| Writ Board model | a GameObject display for the board in the Trade District | A notice board with sealed papers and the Stormwind lion. Until made it is the stock Alliance wanted board (display 3053). |

The City Watch officer is a stock Stormwind City Guard display and needs no art.

## The Stockade (spec #575)

| Asset | Where | Notes |
|---|---|---|
| Twilight Coin icon | `data/twilights-eve/art/`, 128 x 128, only if no free stock coin icon passes `icon_check.py` | A dull coin with a hammer over a black sun. A free stock coin icon stands in until then. |

Every creature (the Defias, the Watch and its captain, the Keep's warders and scribe, the
Twilight Listeners, the Hushed, the Hush, the turnkey and the four witnesses), the levers, the
portcullises, the sealed door and the drain grate are stock displays and need no art. The jail
is vanilla's map 34.

## The 30-50 dungeons (#637)

Stock stand-ins for Mistmantle Manor, the Tower of Ilgalar, Raven Hill Deeps and Stonewatch Keep.
All are placeholders and none needs new art. "Where" names the spec's ticket.

| Asset | Where | Stand-in |
|---|---|---|
| Humming ore in Mistmantle's floor | 04 | GameObject display 2370 (`CaveMineCrystalFormation04`) |
| The Anchor's pylons | 06 | display 3080 (`UngoroCrystalPylon01`) |
| Dark Iron Tribute crates | 06, 10 | displays 1868 and 1869 (`DarkIronCrate01`, `02`) |
| Feed valves | 06 | display 755 (`DeadMineValve`) |
| The emptied strongbox, its lock cut | 06 | display 259 (`TreasureChest01`) |
| Rubble heaps, the dropped side tunnel | 08 | display 2230 (`G_CaveIn`) |
| Brood cage and its lever | 10 | displays 676 (`G_Cage`) and 295 (`G_LeverMetal`) |
| Singe's Scale icon | 11 | a free stock scale icon that passes `icon_check.py` |
| Mourncrown | 12 | display 6678 (`World\Goober\G_Necropolis.m2`) |
| The four Bonus Chests | #659 (spec #650) | display 259 (`TreasureChest01`), vanilla's small wooden chest, until each dungeon's chest has a look of its own |

## The Deeprun Tram (spec in `docs/specs/deeprun-tram/README.md`)

| Asset | Where | Notes |
|---|---|---|
| Station bell | a GameObject display, stock until painted | Stands in as vanilla's Horde Bell (display 3972). Wanted: a dwarven brass platform bell on a post. |
| Gold chest look | a GameObject display, stock until painted | Stands in as the small wooden chest (259), the same as the Tribute Pile's. Wanted: the same chest, gilded, so a clean sweep reads at a glance. |
| Porter's gold glow | the porter's marker aura visual | Stands in as Holy Strength's look (visual 4345, the Crusader's gold glow) on the Gilded marker (1900056, #590). Wanted: a faint gold shimmer on the porter and the crate it carries. |
| Crate toss circle | the toss's ground marker visual | Stands in as Flare's look (stock spell 1543) on the Crate Toss circle (1900059, #591). Wanted: a crate's shadow growing on the floor. |

Every creature (Haulers, Listeners, Lampmen, porters, the Conductor and the passengers), the
tram cars, the crates, the lamps and the frozen passengers' ice are stock displays and need no
art. The station is vanilla's map 369. The mist wall uses the Closed Zones' mist shader.

## The Burning Steppes opening (#626)

| Asset | Where | Notes |
|---|---|---|
| The Ilgalar Anchor, whole and broken | two GameObject displays on Ilgalar's top floor | A humming stone in a Dark Iron casing with a forge stamp, and the same stone cracked open, its casing split. Until made, displays 1667 and 1687 (floating purple crystals). |
| Blackrock Pass Watch board | a GameObject display for the camp's Writ board | A rough plank board on a post, Watch colours. Until made, the stock Alliance wanted board (display 3053), as the Writ Board. |

## Redridge and Duskwood 30-50 (spec #595)

| Asset | Where | Notes |
|---|---|---|
| Grey Robe look | creature display, stock until painted | Stands in as display 2492 (vmangos's Scarlet Abbot, a robed human). Wanted: a grey hooded robe with a small crowned mark on the breast, for the Grey Robe Diggers, Lantern-bearers, Surveyors, Agents, Wardens and the Grey Captain. |
| Crowned mark | quest item icons and the agents' camp banners | Stock icons stand in. Wanted: a plain crown over a closed eye, in the same hand on every item, so a player tells it from the cult's hammer and black sun. |
| Lakeshire Watch camp | GameObject displays, stock | Tents (6738) and a campfire (192), as Sergeant Keane's outpost. No new art needed. |
| Writ Board poster | GameObject display, stock | Stands in as vanilla's Wanted Poster (display 17). Wanted: a smaller notice pinned beside the board, "Hands wanted east". |

Every creature (the Blackrock, the gnolls, the worgen, the dead, the ogres, the Defias, the
Twilight courier and outriders) and every quest item but the crowned ones use stock displays and
need no art.

## Tier 2 Callings (spec #674)

Each Tier 2 Calling's trainer borrows a stock look, and its sheet rows borrow stock icons. Every
one is a placeholder.

| Asset | Where | Stand-in |
|---|---|---|
| Wren Ashcombe, the Sharpshooter trainer (#684) | `content/hub.toml`, entry 190116 | display 2384, Watcher Dodds's look. Wanted: a weathered woman archer in leather and a hooded cloak, a longbow at rest. |
| Sharpshooter Calling icon (#684) | `callings.csv` | `Ability_Hunter_SniperShot` |
| Steady Shot, Steady Aim (#684) | master CSV | `Ability_Marksmanship`, `Ability_Hunter_Pathfinding` |
| Aimed Shot, Sunder Arrow, Sundering (#684) | master CSV | `Ability_PierceDamage`, `Ability_Hunter_CriticalShot`, `Ability_Warrior_Sunder` |
| Hold Your Ground and its buffs (#684) | master CSV | `Ability_Defend` |
| Pinning Shot and its root (#684) | master CSV | `Spell_Frost_Stun`, `Spell_Nature_StrangleVines` |
| Crippling Aim, Bleeding Arrow, Barbed Pin (#684) | master CSV, aura rows | `Ability_Rogue_Trip`, `Ability_BackStab`, `Ability_Hunter_Quickshot` |
| The Sharpshooter's Talents (#684) | master CSV, rows 25 and 50 | `Spell_Nature_StoneClawTotem`, `Ability_Hunter_EagleEye`, `Spell_Nature_StoneSkinTotem`, `Ability_TrueShot`, `INV_Ammo_Arrow_02`, `Spell_Nature_Sleep` |
| Sergeant Maera Holt, the Ironclad trainer (#685) | `content/hub.toml`, entry 190110 | display 3287, Ilsa Corbin's look. Wanted: a grey-haired woman sergeant in old, dented plate, a heavy shield on her arm. |
| Ironclad Calling icon (#685) | `callings.csv` | `INV_Shield_06` |
| Shield Slam and Stunning Slam's stun (#685) | master CSV | `INV_Shield_05` |
| Retort (#685) | master CSV | `Ability_Warrior_Revenge` |
| Iron Wall and its buffs (#685) | master CSV | `Ability_Warrior_ShieldWall` |
| Steel Resolve and its buffs (#685) | master CSV | `Spell_Holy_AshesToAshes` |
| Hold the Line and its buff (#685) | master CSV | `Ability_Warrior_Challange` |
| The Ironclad's Talents (#685) | master CSV, rows 25 and 50 | `INV_Chest_Plate04`, `INV_Shield_04`, `Spell_Holy_BlessingOfStamina`, `Spell_Holy_Heal`, `Spell_Holy_RighteousFury`, `Spell_Holy_SealOfValor` |
| Blackrock Shield, the trial's quest item (#685) | `content/quest_items.toml`, entry 980811 | display 17943, `INV_Shield_20` |
| Brother-Knight Aldren Vale, the Zealot trainer (#686) | `content/hub.toml`, entry 190111 | display 4885, Duthorian Rall's look. Wanted: a knight in white-and-gold plate, bareheaded, a warhammer at his belt, kneeling among graves. |
| Zealot Calling icon (#686) | `callings.csv` | `Spell_Holy_RighteousFury` |
| Judgement (#686) | master CSV | `Spell_Holy_RighteousFury` |
| Hallowed Ground and its heal auras (#686) | master CSV | `Spell_Holy_InnerFire` |
| Righteous Fury and its buffs (#686) | master CSV | `Spell_Holy_SealOfFury` |
| Holy Wrath and its stuns (#686) | master CSV | `Spell_Holy_Excorcism` |
| Hammer of Justice and its stuns (#686) | master CSV | `Spell_Holy_SealOfMight` |
| The Zealot's Talents (#686) | master CSV, rows 25 and 50 | `Spell_Holy_FlashHeal`, `INV_Chest_Plate02`, `Ability_Warrior_InnerRage`, `Spell_Holy_PrayerOfHealing02`, `Spell_Holy_BlessedRecovery`, `INV_Hammer_04` |
| Hallowed Bone, the trial's quest item (#686) | `content/quest_items.toml`, entry 980821 | display 1273, `INV_Misc_Bone_DwarfSkull_01` |
| Sabine Redmark, the Bloodletter trainer (#687) | `content/hub.toml`, entry 190112 | display 1297, Keryn Sylvius's look. Wanted: a lean woman in stained leathers by a campfire, a skinning knife in her lap and salt sacks at her feet. |
| Bloodletter Calling icon (#687) | `callings.csv` | `Ability_Rogue_Rupture` |
| Hemorrhage, its debuff and Open Wound's (#687) | master CSV | `Spell_Shadow_LifeDrain` |
| Open Veins (#687) | master CSV | `Ability_Druid_Disembowel` |
| Blood Fan and its bleeds (#687) | master CSV | `Ability_Warrior_PunishingBlow` |
| Red Frenzy and its buffs (#687) | master CSV | `Ability_Rogue_SliceDice` |
| Blood Pool (#687) | master CSV | `Spell_Shadow_BloodBoil` |
| The Bloodletter's Talents (#687) | master CSV, rows 25 and 50 | `Spell_Shadow_SiphonMana`, `Ability_Rogue_Sprint`, `Ability_BackStab`, `Ability_Druid_Rake`, `Ability_CriticalStrike`, `Spell_Shadow_DeathPact` |
| Torn Mongrel Hide, the trial's quest item (#687) | `content/quest_items.toml`, entry 980831 | display 8952, `INV_Misc_Pelt_Bear_Ruin_01` |
| Silas Dunmere, the Nightblade trainer (#688) | `content/mobs.toml`, template 97290 | display 1507, Osborne the Night Man's look. Wanted: a thin man in dark leathers sitting on the bunk of an open cell, half in shadow, a knife turning in his fingers. |
| Nightblade Calling icon (#688) | `callings.csv` | `Ability_Stealth` |
| Backstab (#688) | master CSV | `INV_Weapon_ShortBlade_14` (`Ability_BackStab` is the Bloodletter's Deep Wounds) |
| Ambush and Lingering Ambush's bleed (#688) | master CSV | `Ability_Ambush` |
| Vanish and Fleeting Vanish's sprint (#688) | master CSV | `Ability_Vanish` |
| Cold Blood and its buffs (#688) | master CSV | `Spell_Ice_Lament` |
| Kidney Shot and its stun (#688) | master CSV | `Ability_Rogue_KidneyShot` |
| The Nightblade's Talents (#688) | master CSV, rows 25 and 50 | `Spell_Shadow_Twilight`, `Spell_Shadow_Curse`, `Spell_Shadow_UnholyStrength`, `Spell_Shadow_DeathScream`, `Spell_Shadow_Possession`, `Ability_Rogue_FeignDeath` |
| Magus Ilyra Vance, the Pyromancer trainer (#689) | `content/hub.toml`, entry 190114 | display 3293, Elsharin's look. Wanted: a grey-haired woman magus in scorched red robes, sleeves rolled, beside an old courtyard fire. |
| Pyromancer Calling icon (#689) | `callings.csv` | `Spell_Fire_FlameBolt` |
| Fireball (#689) | master CSV | `Spell_Fire_FireBolt02` |
| Pyroblast and its burns (#689) | master CSV | `Spell_Fire_FireBolt` (`Spell_Fire_Fireball02` is the Ranger's Explosive Shot) |
| Kindle (#689) | master CSV | `Spell_Fire_SoulBurn` |
| Flamestrike (#689) | master CSV | `Spell_Fire_Fire` (`Spell_Fire_SelfDestruct` is the Acolyte's Miasma) |
| Blast Wave and its slows (#689) | master CSV | `Spell_Fire_MeteorStorm` (`Spell_Holy_Excorcism_02` is the Initiate's Ember Blast) |
| Hot Streak's buff (#689) | master CSV, aura row | `Spell_Fire_Incinerate` |
| The Pyromancer's Talents (#689) | master CSV, rows 25 and 50 | `Spell_Fire_Incinerate`, `Spell_Fire_FlameTounge`, `Spell_Fire_FireArmor`, `Spell_Fire_MasterOfElements`, `Spell_Fire_SearingTotem`, `Spell_Fire_Volcano` |
| Gnoll Tallow, the trial's quest item (#689) | `content/quest_items.toml`, entry 980851 | display 3341, `INV_Misc_Organ_05` |
| Teodric Lumen, the Leyweaver trainer (#690) | `content/hub.toml`, entry 190115 | display 1470, Larimaine Purdue's look. Wanted: a lean, sleepless man in faded blue robes, barefoot by a glowing spring on a dark riverbank. |
| Leyweaver Calling icon (#690) | `callings.csv` | `Spell_Nature_StarFall` |
| Arcane Missiles (#690) | master CSV | `Spell_Nature_WispSplode` (`Spell_Nature_StarFall` is the Initiate's Catalyst) |
| Arcane Surge (#690) | master CSV | `Spell_Lightning_LightningBolt01` |
| Ley Tap (#690) | master CSV | `Spell_Nature_Purge` |
| Attunement and its buffs (#690) | master CSV | `Spell_Arcane_ArcaneResilience` |
| Ley Bind and its binds (#690) | master CSV | `Spell_Nature_Polymorph` |
| The Leyweaver's Talents (#690) | master CSV, rows 25 and 50 | `Spell_Shadow_Charm`, `Spell_Holy_MagicalSentry`, `Spell_Holy_Devotion`, `Spell_Holy_PowerInfusion`, `Spell_Frost_ManaRecharge`, `Spell_Nature_ManaRegenTotem` |
| Hale Brannock, the Trapper trainer (#691) | `content/hub.toml`, entry 190117 | display 4327, Sven Yorgen's look. Wanted: a wiry woodsman in patched leathers kneeling at the tree line, coils of wire and a trap jaw at his belt. |
| Trapper Calling icon (#691) | `callings.csv` | `Spell_Fire_SelfDestruct` |
| Immolation Trap and its burns (#691) | master CSV | `Spell_Fire_FlameShock`; the trap shows the stock Immolation Trap GameObject (display 3074) |
| Frost Trap and its chills (#691) | master CSV | `Spell_Frost_Glacier` (`Spell_Frost_FreezingBreath` is the Initiate's Shattered Blink); the trap shows the stock Frost Trap GameObject (display 3073) |
| Caltrops and its auras (#691) | master CSV | `Spell_Nature_Thorns` |
| Spring the Traps (#691) | master CSV | `INV_Misc_Bomb_05` |
| Tripwire and its roots (#691) | master CSV | `Spell_Nature_Earthbind`; the trap shows the stock Freezing Trap GameObject (display 3071) |
| Quieting Snare's snare (#691) | master CSV, aura row | `Ability_GolemStormBolt` |
| The Trapper's Talents and their buffs (#691) | master CSV, rows 25 and 50 | `INV_Misc_Gear_02`, `Ability_Hunter_Harass`, `Spell_Nature_Swiftness`, `Ability_Hunter_BeastSoothe`, `Ability_Mount_JungleTiger`, `Ability_Hunter_AimedShot` |
| Tarantula Silk, the trial's quest item (#691) | `content/quest_items.toml`, entry 980881 | display 4815, `INV_Fabric_Silk_03` |
| Mother Ysolde Grey, the Blightcaller trainer (#692) | `content/hub.toml`, entry 190118 | display 4272, Morbent Fel's look. Wanted: an old woman in a mud-stained grey robe and a gardener's apron, kneeling among raised beds of black rot, a trowel in one hand. |
| Blightcaller Calling icon (#692) | `callings.csv` | `Spell_Shadow_AbominationExplosion` |
| Plague Tithe (#692) | master CSV | `Spell_Shadow_DarkRitual` |
| Harvest (#692) | master CSV | `Spell_Shadow_Haunting` |
| Spread Blight (#692) | master CSV | `Spell_Shadow_CreepingPlague` |
| Wasting Communion and its heals (#692) | master CSV | `Spell_Shadow_GatherShadows` |
| Rise Again (#692) | master CSV | `Spell_Shadow_AnimateDead` |
| The Blightcaller's Talents (#692) | master CSV, rows 25 and 50 | `Spell_Shadow_CorpseExplode`, `Spell_Nature_NullifyDisease`, `Spell_Shadow_ChillTouch`, `Spell_Shadow_DeathCoil`, `Spell_Shadow_ScourgeBuild`, `Spell_Shadow_CurseOfAchimonde` |
| Rotting Heart, the trial's quest item (#692) | `content/quest_items.toml`, entry 980891 | display 10923, `INV_Misc_Organ_03` |
| Sister Elowen Marr, the Bloodmender trainer (#693) | `content/hub.toml`, entry 190119 | display 7008, Brother Sarno's look. Wanted: a young sister in a plain grey habit with bandaged forearms, sitting on the cathedral steps among sleeping orphans, a bowl of bread beside her. |
| Bloodmender Calling icon (#693) | `callings.csv` | `Spell_Shadow_LifeDrain` |
| Blood Feast (#693) | master CSV | `Ability_Racial_Cannibalize` |
| Transfusion and its heals (#693) | master CSV | `Spell_Holy_SealOfSacrifice` |
| Blood Ward (#693) | master CSV | `Spell_Shadow_SacrificialShield` |
| Circle of Blood and its heals (#693) | master CSV | `Spell_Holy_PrayerOfHealing` |
| Last Rites (#693) | master CSV | `Spell_Holy_Resurrection` |
| The Bloodmender's Talents and their buffs (#693) | master CSV, rows 25 and 50 | `Spell_Shadow_BurningSpirit`, `Spell_Nature_BloodLust`, `Spell_Shadow_DeadofNight`, `Spell_Holy_Heal02`, `Spell_Holy_HolyProtection`, `Spell_Holy_ElunesGrace` |

The Sharpshooter's carried skills (Auto Shot, Puncturing Shaft, Rapid Fire, Silencing Shot) keep
the Ranger's icons. Its trial has no quest items. The Ironclad's carried skills (Righteous Smash,
Shield of the Penitent, Rebuke) keep the Vanguard's icons, and Shield Rebuke's stun shows
`Ability_Warrior_ShieldBash`. The Zealot's carried skills (Bulwark Charge and its Consecration,
Seal of Iron and its armour spike, Rebuke) keep the Vanguard's icons. The Bloodletter's carried
skills (Rupture, Garrote and its bleeds, Kick, and Bleeding Kick's bleed) keep the Cutthroat's
icons, as do its copies of Bloodlust and the blood droplet. The Nightblade's carried skills
(Eviscerate, Stealth and its stealth auras, Kick) keep the Cutthroat's icons, and its trial has no
quest items. The Pyromancer's carried skills (Searing Ember and its Ember, Catalyst, Counterspell)
keep the Initiate's icons. The Leyweaver's carried skills (Ley-Spark, Ley Ward, Counterspell) keep
the Initiate's icons, and its trial has no quest items. The Trapper's carried skills (Auto Shot,
Snapshot, Vault, Silencing Shot) keep the Ranger's icons. The Blightcaller's carried skills
(Creeping Blight, Miasma and its auras, Hush) keep the Acolyte's icons, and Healing Miasma's aura
keeps Miasma's. The Bloodmender's carried skills (Blood Lance, Dark Mending and its aura, Hush)
keep the Acolyte's icons, the auras of Transfusion and Circle of Blood and their Skill Choices
keep their skill's, and its trial has no quest items.
