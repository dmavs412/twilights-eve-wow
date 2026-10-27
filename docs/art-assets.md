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

### Forager's Grid window, RGBA TGA

Files in `crates/benilla-app/assets/ui/herbalism/`, loaded as `Interface\Benilla\herbalism\<name>`.
`tools/twilights-eve/placeholder_herb_art.py` writes flat placeholders at these sizes. The window
frame reuses the Journal's `journal/window_bg.tga`.

| File | Size | What it shows |
|---|---|---|
| `bush_hidden.tga` | 64 x 64 | An unpulled tile, a leafy bush |
| `bush_glow.tga` | 64 x 64 | A hidden tile known to be a plant, glowing |
| `bush_wilted.tga` | 64 x 64 | A hidden tile known to be a weed |
| `bush_thorn.tga` | 64 x 64 | A hidden tile known to be a thorn, red |
| `soil.tga` | 64 x 64 | A pulled tile, bare soil |
| `soil_thorn.tga` | 64 x 64 | A pulled thorn, reddish soil |
| `stake.tga` | 32 x 40 | The wooden stake; the game prints the number on it |
| `tile_highlight.tga` | 64 x 64 | Gold ring over the tiles a stake counts, on hover |
| `tile_target.tga` | 64 x 64 | Dashed outline over the tiles a tool skill will cover |
| `part_weed.tga` | 32 x 32 | Weed glyph on a pulled weed |
| `part_thorn.tga` | 32 x 32 | Thorn glyph on a pulled thorn |

A pulled plant shows its own item icon, so it needs no glyph.

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

One icon per product type (ADR 0025): the five Strength Steps of a potion share its icon, and
the bands and qualities of a meal recipe share the recipe's icon. The tickets write the exact
list from the recipe rows; these are the spec's counts.

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
dishes, the 16 legendary meals, the 3 legendary plant meals and Hearty Mush. No category recipe
has an icon yet.

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

**Unassigned meals**, `meal_01.png` to `meal_35.png`: the 30 named fish dishes and five legends
(Bell-Toll Bream, Everstill Leviathan, Old Goldfin, Sorrowgill, Stockade Sucker). The cooking
tickets pick each one's dish and rename the file.

| File | Painting |
|---|---|
| `meal_01` | three fish on a skewer |
| `meal_02` | a fish on a plate with lemons |
| `meal_03` | four fish hung to dry |
| `meal_04` | a fish in a silver bowl set with a blue gem (a legend?) |
| `meal_05` | fish rolls on a gold plate |
| `meal_06` | a pot with a fish head and fin |
| `meal_07` | a pot of fish and vegetable stew |
| `meal_08` | a fish stuffed through its mouth with potatoes |
| `meal_09` | an orange fish in a skillet of green sauce |
| `meal_10` | a fish under herb sprigs |
| `meal_11` | a grilled fish steak |
| `meal_12` | a roast fish with lemon and potatoes |
| `meal_13` | a blackened fillet on a board |
| `meal_14` | a pie with a fish head through the crust |
| `meal_15` | a soup with a whole small fish |
| `meal_16` | fish balls in a bowl |
| `meal_17` | fried small fish in a paper cone (Brilliant Smallfish?) |
| `meal_18` | purple and pink fish cubes with a violet garnish |
| `meal_19` | a red fish stew |
| `meal_20` | a soup in a knot-patterned bowl (a legend?) |
| `meal_21` | an orange fish on leaves |
| `meal_22` | translucent raw slices (Dreaming Glassfin?) |
| `meal_23` | a grilled fillet on a plate |
| `meal_24` | a golden fish with lemon (Old Goldfin?) |
| `meal_25` | a fish with bread in a basket |
| `meal_26` | a paper parcel tied with string |
| `meal_27` | a pike head on a grill |
| `meal_28` | a soup with striped blue skin pieces (Slitherskin Mackerel?) |
| `meal_29` | a fillet on a big leaf |
| `meal_30` | a fish steak with lemon |
| `meal_31` | leaf-wrapped rice rolls |
| `meal_32` | a salt-crusted fish on paper |
| `meal_33` | pickled fish in a jar (Harbour Herring?) |
| `meal_34` | a pale fish soup with cabbage |
| `meal_35` | a braided pasty |

### Crafting window

The kitchen and alchemy table window (ADR 0025): a frame, a recipe list row, an ingredient slot
and a commit button. The tickets name the files.
