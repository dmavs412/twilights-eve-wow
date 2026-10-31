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

Davis painted all 47 alchemy icons. They are 128 x 128 PNGs in `data/twilights-eve/art/`,
waiting for the alchemy item rows. `<p>` is one of `arcane`, `calming`, `fortifying`,
`restorative`, `toxic`, `volatile`.

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
`bottom_tab`, `divider`), so it needs no file of its own yet. The alchemy table's window is
still to be specified.

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
