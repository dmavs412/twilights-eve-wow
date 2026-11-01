# Cooking recipes name their items, alchemy reads properties, and the realm runs one crafting window

Cooking and alchemy turn the fish and herbs that fishing and herbalism gather into food,
potions, weapon oils and flasks. Neither craft has a level. Neither fits the stock tradeskill
window, which lists fixed reagents and has no way to try a guess. This ADR settles how an
ingredient is read, where a product's strength comes from, how a product exists as an item, and
how the player crafts. Spec #447 (cooking #445 and alchemy #446 grilled together).

**Amended 2026-10-27: cooking follows Davis's Kitchen prototype** (artifact
`claude.ai/artifact/ENPoEXoRt3uiqNf3pVPU8B`). The 2026-10-25 grill had cooking read herb
categories, give each meal a Plain, Fine or Exquisite quality from the ingredients' rarity, and
lift it with a fish-and-herb pairing. All three are gone.

**Amended 2026-11-01: alchemy follows Davis's Alchemy prototype** (artifact
`claude.ai/artifact/YXnqKyyTNE1UGvmiigRbBZ`, source and staged data in `docs/specs/alchemy/`).
The 2026-10-25 grill had six properties with Toxic among them, a recipe per product with caps,
Fish Oil, and Tinctures stepped against the weakest herb. Toxic is now a risk, not a product:
it makes nothing, it can spoil any craft, and it pays points into the product when the craft
holds. Purifying cancels it. Every family has one ladder of four grades. Fish Oil is gone: an
oil takes two fish of any kind. Ten hybrid potions join the five single ones.

- **A cooking recipe names exact items; an alchemy product is a grade on a ladder of property
  points.** Every slot of a cooking recipe is one item ("Goldshire Perch, Wild Onion, Sweetroot"),
  and only those items cook it, in any order. Categories (mushroom, greens) still name orders and
  group the pantry. An alchemy herb carries points in five properties (Restorative, Fortifying,
  Volatile, Arcane, Calming) and may carry Toxic or Purifying. A make sums its materials, its
  main property leads (or the player's target), and it gives the highest grade of its family's
  ladder that the points meet: Minor, Lesser, the plain grade, or Greater (bands 1, 10, 20, 30).
  Two properties that both clear the hybrid ladder make that pair's hybrid potion. Six items
  carry both crafts: the three legendary plants, the two truffles and Bird's Nest. Every other
  item keeps the single `use` that spec #435 gave it.
- **Toxic is one rule for every craft.** Net Toxic is Toxic in less Purifying in. A craft fails
  into Murky Sludge at net / (net + main points); when it holds, each net point adds a point to
  the product. A Tincture or Essence carries half its net Toxic, rounded down, and rolls again in
  the next craft. A make whose body is Tinctures or Essences may add up to two herbs as a wash,
  whose Toxic and Purifying alone count.
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
- **Every grade and step of an alchemy product is its own item row, generated at load.** A 1.12
  item is a fixed template with a fixed tooltip. A make's strength (its points over its grade's
  threshold, x1 to x2) rounds down to x1, x1.25, x1.5, x1.75 or x2, so a potion kind is 20 rows
  (four grades, five steps), each with its real numbers in its tooltip. A Tincture or Essence row
  also names the Toxic it carries, and a flask row names the grade and step of each of its two
  Essences. The realm builds all 8,551 rows at load from `alchemy.toml`'s product kinds, by a
  fixed entry formula, so a save's item entry never moves. Rows stack only with themselves.
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
- **Alchemy is learned by making.** Every craft counts its herbs in their Herb Notes: one use
  names a herb's values, five show their numbers. A grade is learned the first time a make gives
  it, and its page then shows its threshold; an alchemist's Rumour tells one threshold for
  copper. The Still opens after the first potion, Essences and the Oil Tin after the first
  Tincture, the Flask Rack after the first Essence. A make that holds nothing, or fails its roll,
  makes **Murky Sludge** and uses its materials. The Brew Log keeps each make's materials,
  summed values and roll.
- **Alchemy has a Journal tab.** Tinctures, Essences and Murky Sludge set `journal = "alchemy"`
  and live in the Alchemy tab's Satchel (ADR 0023). The tab also holds the Herb Notes, a ladder
  page per family, the flask list and the Brew Log, readable anywhere. Potions, oils and flasks
  go to the bags.
- **Each craft opens a custom window, and the realm decides the result.** A kitchen opens the
  Kitchen window: the recipe book on the left, the pot on the right. A known recipe fills its
  slots from the bags and Satchels and takes nothing else. An unknown recipe's revealed slots
  fill and its `???` slots take guesses. Experiment takes two to four of anything. An alchemy
  table opens the Alchemy window: the five makes to pick from, the property vials, a target, and
  a tray of the materials the picked make takes, from the bags and Satchels. The client shows the
  realm's prediction before the player commits. The realm checks the materials, removes them,
  and answers with the product, Hearty Mush or Murky Sludge. The client never computes a result.
- **Crafting happens at named stations.** Cooking happens only at the Lion's Pride and Abbey
  kitchens, opened from their cooks' gossip (Tomas, Brother Hollis). Every alchemy make happens
  only at an alchemy table, opened from its alchemist's gossip: Sister Marla at the Abbey, Alwen
  Carrow in Goldshire, Quartermaster Ilsa Venn in Stormwind's Alchemy Needs. Each sells Rumours
  for the grades of her table. Cutting bait happens at a kitchen.
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
- **Toxic as a sixth product property, with Toxic potions, oils and flasks** (the 2026-10-25
  grill). Replaced by the prototype: Toxic is the risk every craft takes. The eleven Toxic and
  Fish Oil icons Davis painted stay in the art folder, unused.
- **Fish Oil rendered from two fish.** Dropped with the prototype: an oil takes the two fish.
- **Tinctures, Essences and Murky Sludge in the bags, with the alchemy pages only at a table**
  (the prototype). Rejected by Davis: they go to an Alchemy Journal tab, readable anywhere.
- **One grade and step for a whole flask.** Fewer rows (300), but a flask gives each Essence's
  stat at that Essence's own grade and step in the prototype, so each pair is its own row.
- **Hybrid potions later.** Davis chose to build all ten now: a brew that clears two ladders
  already makes a hybrid in the prototype's rules.
