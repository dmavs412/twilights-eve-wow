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

### Item icons, 128 x 128 PNG, 37 items

Each one is a row in `crates/twilight-realm/content/herbs.toml`. The name in brackets is the
`icon` name to paint it under.

Forage and finds:

| Entry | Item | Icon name |
|---|---|---|
| 961001 | Dandelion Greens | `dandelion_greens` |
| 961002 | Wild Onion | `wild_onion` |
| 961003 | Blackberries | `blackberries` |
| 961004 | Elwynn Chanterelle | `elwynn_chanterelle` |
| 961005 | Hazelnuts | `hazelnuts` |
| 961006 | Wild Strawberries | `wild_strawberries` |
| 961007 | Wild Thyme | `wild_thyme` |
| 961008 | Sweetroot | `sweetroot` |
| 961009 | Moonpetal (legendary) | `moonpetal` |
| 961010 | Wild Garlic | `wild_garlic` |
| 961011 | Redridge Pepper | `redridge_pepper` |
| 961012 | Rock Sage | `rock_sage` |
| 961013 | Lakeshore Watercress | `lakeshore_watercress` |
| 961014 | Redridge Morel | `redridge_morel` |
| 961015 | Hill Truffle | `hill_truffle` |
| 961016 | Emberbloom (legendary) | `emberbloom` |
| 961017 | Ghostcap | `ghostcap` |
| 961018 | Raven's Bramble | `ravens_bramble` |
| 961019 | Black Morel | `black_morel` |
| 961020 | Duskroot | `duskroot` |
| 961021 | Gravecap | `gravecap` |
| 961022 | Black Truffle | `black_truffle` |
| 961023 | Widow's Lily (legendary) | `widows_lily` |
| 961024 | Wild Honeycomb | `wild_honeycomb` |
| 961025 | Amber Sap | `amber_sap` |
| 961026 | Bird's Nest | `birds_nest` |
| 961027 | Lost Coin Purse | `lost_coin_purse` |

Tools:

| Entry | Item | Icon name |
|---|---|---|
| 961101 | Herbalist's Sickle | `herbalists_sickle` |
| 961102 | Herbalist's Trowel | `herbalists_trowel` |
| 961103 | Herbalist's Shears | `herbalists_shears` |

Consumables:

| Entry | Item | Icon name |
|---|---|---|
| 961201 | Thyme Tea | `thyme_tea` |
| 961202 | Elixir of Herb Sight | `elixir_of_herb_sight` |
| 961203 | Thorn Salve | `thorn_salve` |
| 961204 | Bloom Draught | `bloom_draught` |
| 961205 | Weedcutter's Twine | `weedcutters_twine` |
| 961206 | Forager's Charm | `foragers_charm` |
| 961207 | Moonwater | `moonwater` |

The eleven stock herbs (Peacebloom, Silverleaf, Earthroot, Mageroyal, Briarthorn, Swiftthistle,
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

### Item icons, 128 x 128 PNG

| Group | Count | Notes |
|---|---|---|
| Potions | 6 | one per Property |
| Weapon oils | 6 | one per Property |
| Tinctures | 6 | one per Property |
| Essences | 6 | one per Property |
| Flasks | 21 | one per Essence pair |
| Fish Oil, Murky Sludge | 2 | |
| Meals | about 60 | one per recipe: category recipes and the 30 named fish dishes |
| Legendary meals | 19 | 16 Legendary Fish, 3 legendary plants |
| Cut bait | 22 | one per common and uncommon fish |
| Hearty Mush | 1 | |

### Crafting window

The kitchen and alchemy table window (ADR 0025): a frame, a recipe list row, an ingredient slot
and a commit button. The tickets name the files.
