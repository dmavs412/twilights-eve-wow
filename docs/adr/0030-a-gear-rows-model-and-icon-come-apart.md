# A Gear row's model and its icon come apart

A Gear row's `display_id` sets both what the item looks like worn and its bag icon, and no two
items may share an icon. The 80 Elwynn 11-30 Chests (#456) took Wardrobes from 32 vanilla
families, but none of those families' chest pieces has a free icon, so every Chest still wears
the chest of its old display, not its family's. Davis expects many more looks, all from
low-level items. Grilled 2026-10-31 (#464).

## Decision

- **`display_id` is the model, `icon` is the icon.** A new `gear.csv` column `icon` names the
  row's bag icon: a stock icon name such as `INV_Chest_Chain_05`, or a painted 128 x 128 PNG in
  `data/twilights-eve/art` (`chest_x.png`). A blank `icon` keeps the display's own icon, so no
  existing row changes. The column works on every slot, not only Chests.
- **A synthetic display per model and icon.** `build_gear.py` gives each distinct pair of
  `display_id` and `icon` a synthetic display id from 998000 up and writes it as the row's
  `display_id` in `gear.toml`, with `icon_base` (the model) and `icon` (a stock path or a painted
  stem). `build_spells.py` turns those into `item_displays` rows of the client DBC patch, as it
  does for painted items today, and also accepts a stock icon path. The client already copies a
  row's `base` display and swaps in its icon, so it needs no change.
- **The real Chest stays on the wire.** The worn Chest's own entry stays in the chest slot's
  visible-item field, so another player's inspect window and character select show the real
  Chest, its name and its stats.
- **A Line still shows one icon.** `build_gear.py` refuses a Line (ADR 0026) whose rows would show
  two icons: two different `icon` cells, or a blank `icon` on rows with different `display_id`s.
- **Icon names compare without an extension.** One stock display names `INV_Chest_Fur.tga`; the
  build writes icon names without one, and `icon_check.py` compares names with a trailing `.tga`
  or `.blp` removed.
- **A cloth Chest wears its family's robe** wherever the family has one, and a Chest named Vest
  that now wears a robe is renamed Robe. A Chest named Robe whose family has no robe keeps its
  name.

## Considered options

- A ninth Wardrobe cell, `wardrobe_chest`, as a look template written over the chest slot's
  visible-item field. It needed no client patch change, but inspecting another player would show a
  statless "Wardrobe chest" in place of their real Chest. Rejected.
- A `chest_model` column for Chests only, with the patch row naming the display to copy the icon
  from. The build reads no game data, so the client and `icon_check.py` would both have to resolve
  that display. Rejected for a column that names the icon directly.
- Re-picking the icons that name another material than their row (about 12 of the 40). Davis
  kept every current icon; only the model moves.

## Consequences

Each look's Chest wears its family's chest, and a new look costs one free icon per Chest, not one
family chest with a free icon. When the free stock chest icons run out (about 79 chest and robe
icons and 11 shirts were free on 2026-10-31), a painted PNG takes the `icon` cell. Synthetic
display ids renumber when rows are added, as look templates do; saves hold entries, not display
ids, so nothing stored goes stale.
