# Starting outfits: one starter Chest per Calling

A new character wears its Calling's outfit (ADR 0007): one starter Chest, a weapon, a shield and
a stack of throwing axes, and carries a Hearthstone in the backpack. Every race starts the same;
a character is created a Basic Warrior, so that outfit is the one worn.

| file | what |
|---|---|
| `crates/twilight-realm/content/starting_outfits.toml` | Calling to items, by hand |
| `data/twilights-eve/gear.csv` | the items' rows (starter Chests from 900000) |
| `crates/twilight-realm/tests/content.rs` | `every_callings_starting_outfit_is_one_chest_and_its_weapons` |
| `crates/twilight-realm/tests/starting_outfit.rs` | what a new character wears |

## The outfits

| Calling | Chest | weapon | off hand | ranged |
|---|---|---|---|---|
| Basic Warrior | 900000 Recruit's Mail Vest | 25 Worn Shortsword | 2362 Worn Wooden Shield | 3111 Crude Throwing Axe (200) |
| Vanguard | 900001 Recruit's Plate Vest | 37 Worn Axe | 2362 Worn Wooden Shield | 3111 Crude Throwing Axe (200) |
| Zealot | 900001 Recruit's Plate Vest | 25 Worn Shortsword | 2362 Worn Wooden Shield | 3111 Crude Throwing Axe (200) |
| Cutthroat | 2108 Frostmane Leather Vest | 37 Worn Axe | 4565 Simple Dagger | 3111 Crude Throwing Axe (200) |

Callings that share an armour type share a starter Chest. Each Chest's look and Wardrobe come
from the vanilla Warrior's human outfit in the client's `CharStartOutfit.dbc` (class 1, sex 0):
the Recruit's Shirt's display (9891) on the torso, the Recruit's Pants' (9892) on the legs and the
Recruit's Boots' (10141) on the feet. The armour (12 mail, 20 plate) is a placeholder until the
Chest budget is tuned.

## Sources

- **The weapons**: vmangos `item_template` (patch 0, `db-sqlite-4641790`), seeded into the gear
  sheet with every other piece of Gear.
- **Before ADR 0007** each race wore its vanilla Warrior outfit (shirt, pants, boots, weapon,
  shield) from `CharStartOutfit.dbc`. The shirts, pants and boots are retired
  (`content/retired_items.toml`): a save holding one loads without it.
