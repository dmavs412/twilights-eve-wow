# Starting outfits: each race's Warrior gear

A new character wears its race's vanilla Warrior outfit (shirt, pants, boots where the race has
them, a weapon, and a shield where the race gets one) and carries a Hearthstone in the backpack.

| file | what |
|---|---|
| `crates/twilight-realm/content/starting_outfits.toml` | race to items, generated |
| `crates/twilight-realm/examples/gen_outfits.rs` | the generator |
| `crates/twilight-realm/content/given_items.toml` | the items' templates, by hand |
| `crates/twilight-realm/tests/content.rs` | `every_starting_outfit_item_has_a_template_that_agrees_with_the_client` |

## Sources

- **Which items, and how they look**: the client's `DBFilesClient\CharStartOutfit.dbc` (1.12.1,
  build 5875), class 1, sex 0. Each row gives `ItemId[12]`, `DisplayId[12]` and
  `InventoryType[12]`. The 1.12.1 MPQs ship no `Item.dbc`, so this row is the client's only record
  of an item's display id and inventory type. Slots with display or inventory type 0 (food, bags)
  are dropped. Regenerate with:

  ```bash
  cargo run -p twilight-realm --example gen_outfits -- --data C:/Users/davis/wow-data/Data
  ```

- **Stats** (armour, block, damage, speed, durability, prices, quality, levels, sheath and
  material): vmangos `item_template`, the highest `patch <= 10` row (all are patch 0), from the
  `db-sqlite-4641790.zip` asset of the `db_latest` release on
  [vmangos/core](https://github.com/vmangos/core/releases/tag/db_latest), the same snapshot as
  `items-and-vendors.md`.
- **Cross-check**: vmangos `playercreateinfo_item` (class 1) lists the same worn items for all
  eight races. It also gives four of the race's starting food and a Hearthstone, which the realm
  does not hand out except for the Hearthstone. The Troll's Crude Throwing Axe comes as a stack of
  200 there; the realm gives one, since it has no ranged attack.

The content test checks each template's display id and inventory type against the generated
table, so a hand-typed template that drifts from the client fails.

## The outfits

| race | shirt | legs | feet | weapon | off hand / ranged |
|---|---|---|---|---|---|
| Human | 38 Recruit's Shirt | 39 Recruit's Pants | 40 Recruit's Boots | 25 Worn Shortsword | 2362 Worn Wooden Shield |
| Orc | 6125 Brawler's Harness | 139 Brawler's Pants | 140 Brawler's Boots | 12282 Worn Battleaxe (2H) | |
| Dwarf | 38 Recruit's Shirt | 39 Recruit's Pants | 40 Recruit's Boots | 12282 Worn Battleaxe (2H) | |
| Night Elf | 6120 Recruit's Shirt | 6121 Recruit's Pants | 6122 Recruit's Boots | 25 Worn Shortsword | 2362 Worn Wooden Shield |
| Undead | 6125 Brawler's Harness | 139 Brawler's Pants | 140 Brawler's Boots | 25 Worn Shortsword | 2362 Worn Wooden Shield |
| Tauren | 6125 Brawler's Harness | 139 Brawler's Pants | | 2361 Battleworn Hammer (2H) | |
| Gnome | 38 Recruit's Shirt | 39 Recruit's Pants | 40 Recruit's Boots | 25 Worn Shortsword | 2362 Worn Wooden Shield |
| Troll | 6125 Brawler's Harness | 139 Brawler's Pants | | 37 Worn Axe | 2362 Worn Wooden Shield, 3111 Crude Throwing Axe |

## The item stats (vmangos `item_template`)

| entry | name | q | ilvl | req | class/sub | inv | display | armour | block | damage | speed | dur |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 25 | Worn Shortsword | 1 | 2 | 1 | 2/7 | 21 | 1542 | | | 1-3 | 1900 | 20 |
| 37 | Worn Axe | 1 | 2 | 1 | 2/0 | 21 | 14029 | | | 1-3 | 2000 | 20 |
| 38 | Recruit's Shirt | 1 | 1 | 0 | 4/0 | 4 | 9891 | | | | | |
| 39 | Recruit's Pants | 0 | 1 | 1 | 4/1 | 7 | 9892 | 2 | | | | 25 |
| 40 | Recruit's Boots | 1 | 1 | 0 | 4/0 | 8 | 10141 | | | | | |
| 139 | Brawler's Pants | 0 | 1 | 1 | 4/1 | 7 | 9988 | 2 | | | | 25 |
| 140 | Brawler's Boots | 1 | 1 | 0 | 4/0 | 8 | 9992 | | | | | |
| 2361 | Battleworn Hammer | 1 | 2 | 1 | 2/5 | 17 | 8690 | | | 3-5 | 2900 | 25 |
| 2362 | Worn Wooden Shield | 0 | 1 | 1 | 4/6 | 14 | 18730 | 5 | 1 | | | 20 |
| 3111 | Crude Throwing Axe | 1 | 3 | 1 | 2/16 | 25 | 20777 | | | 1-3 | 2000 | |
| 6120 | Recruit's Shirt | 1 | 1 | 0 | 4/0 | 4 | 9983 | | | | | |
| 6121 | Recruit's Pants | 0 | 1 | 1 | 4/1 | 7 | 9984 | 2 | | | | 25 |
| 6122 | Recruit's Boots | 1 | 1 | 0 | 4/0 | 8 | 9985 | | | | | |
| 6125 | Brawler's Harness | 1 | 1 | 0 | 4/0 | 4 | 9995 | | | | | |
| 12282 | Worn Battleaxe | 1 | 2 | 1 | 2/1 | 17 | 22291 | | | 3-5 | 2900 | 25 |

Existing saves keep the items they hold; only characters created from now on get the outfit.
