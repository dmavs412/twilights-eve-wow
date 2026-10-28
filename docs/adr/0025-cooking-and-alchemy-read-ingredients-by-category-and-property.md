# Cooking recipes name their items, alchemy reads properties, and the realm runs one crafting window

Cooking and alchemy turn the fish and herbs that fishing and herbalism gather into food,
potions, weapon oils and flasks. Neither craft has a level. Neither fits the stock tradeskill
window, which lists fixed reagents and has no way to try a guess. This ADR settles how an
ingredient is read, where a product's strength comes from, how a product exists as an item, and
how the player crafts. Spec #447 (cooking #445 and alchemy #446 grilled together).

**Amended 2026-10-27: cooking follows Davis's Kitchen prototype** (artifact
`claude.ai/artifact/ENPoEXoRt3uiqNf3pVPU8B`). The 2026-10-25 grill had cooking read herb
categories, give each meal a Plain, Fine or Exquisite quality from the ingredients' rarity, and
lift it with a fish-and-herb pairing. All three are gone. Alchemy is unchanged.

- **A cooking recipe names exact items; an alchemy recipe is a threshold over properties.** Every
  slot of a cooking recipe is one item ("Goldshire Perch, Wild Onion, Sweetroot"), and only those
  items cook it, in any order. Categories (mushroom, greens) still name orders and group the
  pantry. An alchemy herb carries one to three properties, each with a strength, and an alchemy
  recipe is a threshold over the summed properties ("Restorative 4, Toxic at most 1"). Seven
  items carry both and can go either way: the three legendary plants, the two truffles, Bird's
  Nest and Fish Oil. Every other item keeps the single `use` that spec #435 gave it.
- **Every fish and cooking herb is in at least two cooking recipes**, a fish's own dish counted;
  a Legendary Fish and a legendary plant are in one. No two recipes share an ingredient set. The
  content loader checks both.
- **A cooking recipe's strength is its tier.** The Well Fed effects come in four tiers, one per
  band (1, 10, 20, 30), and each tier is its own recipe with its own ingredients, drawn from the
  band's levels. The tier sets the strength and the item's quality colour: white, green, blue,
  purple. A higher band is a different recipe, never a better cook of the same one. A fish's own
  dish takes the band of the fish's level. A legendary meal has one fixed strength and the
  legendary colour. Alchemy's strength still comes from the ingredients: a herb's property points
  are a budget from its derived level and rarity, split by hand in its data row, and a build
  check fails a row that misses its budget.
- **A scaled alchemy product is five item rows.** A 1.12 item is a fixed template with a fixed
  tooltip. A brew's strength (its main property over the recipe's threshold, x1 to x2) rounds
  down to x1, x1.25, x1.5, x1.75 or x2. Each step is its own item row with its real numbers in
  its tooltip. Tinctures and Essences are stepped the same way. Steps stack only with themselves.
- **A meal is one item row per recipe, in the bags.** Meals go to the bags like stock food, not
  to a Satchel. Since a recipe's ingredients are fixed, its meal is too: one row, one icon. The
  no-duplicate-icons rule holds for every meal. Alchemy keeps one icon per product type, the
  five steps of a potion sharing it, and `icon_check.py` treats rows generated from one alchemy
  recipe as one item.
- **Unknown recipes show, with their ingredients hidden until tried.** The recipe book lists every
  tier recipe from the start: an unknown one shows its effect and band and a `???` for each slot.
  Cooking with an ingredient once, in any dish, reveals it in every unknown recipe that uses it.
  A recipe is learned four ways: the kitchen's cook teaches a starter set on a character's first
  cook there, an order's Rumour bonus teaches one, an exact guess teaches it, and a fish's own
  dish or a legend's meal is learned on its first catch (a legendary plant's meal on its first
  pick). A guess that matches nothing makes **Hearty Mush** and uses the ingredients.
- **One window serves both crafts, and the realm decides the result.** A kitchen or an alchemy
  table opens a custom window: the recipe book on the left, the pot on the right. A known recipe
  fills its slots from the bags and Satchels and takes nothing else. An unknown recipe's
  revealed slots fill and its `???` slots take guesses. Experiment takes two to four of anything.
  The client shows the realm's prediction before the player commits. The realm checks the
  ingredients, removes them, and answers with the product or Hearty Mush. The client never
  computes a result.
- **Crafting happens at named stations.** Cooking happens only at the Lion's Pride and Abbey
  kitchens, opened from their cooks' gossip (Tomas, Brother Hollis). Brewing, distilling and
  condensing happen only at alchemy tables (the Abbey, Goldshire, Stormwind's Alchemy Needs).
  Cutting bait happens at a kitchen and rendering Fish Oil at a table.
- **One food buff at a time.** Every meal shares one Well Fed slot; eating any meal replaces the
  last. A meal lasts 30 minutes and ends on death; a legendary meal lasts 60 and survives it.
  Out-of-combat effects switch off in combat. Alchemy keeps vanilla's own slots: one flask, one
  oil per weapon, and a shared potion cooldown. A flask stacks with food.

## Considered options

- **Category recipes, quality from rarity, and pairings** (the 2026-10-25 grill). Replaced by
  Davis's prototype: exact recipes make the `???` reveal a puzzle, and one row per recipe drops
  the twelve banded rows a combat recipe needed.
- **Properties for cooking too.** Rejected by Davis: alchemy's sums are not what cooking is for.
- **Per-item strength through the random-property fields.** Rejected: it changes how items cross
  the client/realm seam, and potions would stop stacking.
- **The stock tradeskill window.** Rejected: it lists fixed reagents and cannot take a guess.
- **A Cooking Level or Alchemy Level.** Rejected by Davis. Tiers and ingredient level already
  give the power curve, and a level would bring back a grind.
- **Meals in a Satchel, eaten from the Journal.** Rejected by Davis: meals go to the bags.
- **Every alchemy row its own icon.** Davis chose it first, then dropped it: about 700 icons.
