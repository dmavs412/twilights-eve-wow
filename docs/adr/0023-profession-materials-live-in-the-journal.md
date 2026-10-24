# Profession materials live in the Journal, not the bags

ADR 0022 moved the slotted pole and lure into the Fishing Journal, but every fish, every bait and
every spare lure or pole still lands in the bags. Fishing fills the bags fast, and herbs, meats
and oils from later professions would fill them faster. This ADR gives each profession a place in
the Journal that holds its materials, with no slot limit and no stack cap. Spec #390.

- **An item row names its profession.** An item row may set `journal = "fishing"`. Every way the
  character gets that item puts it in the Journal: a catch, mob loot, a vendor purchase, a quest
  reward and a trade. A later profession sets `journal = "herbalism"` and so on on its own rows.
  The key is `journal`, not `material`: `material` is already the stock item's sound material.
- **One Journal, one tab per profession.** The Fishing Journal becomes the Fishing tab of the
  Journal window. Each tab has a Satchel page: everything that profession's Journal holds.
  Fishing is the only tab today.
- **The Satchel lives in the inventory.** `Inventory` holds the Satchel beside the bags.
  `Inventory::store` sends a Journal item to the Satchel, `Inventory::count` adds the Satchel's
  count, and `equipment::take` takes from the Satchel before the bags. Every room check that
  clones the inventory and stores into the clone then passes for a Journal item without a change
  at its call site.
- **The Satchel keeps counts, and pieces for items with wear.** An item whose row has no
  durability is a count per entry, with no cap. An item whose row has durability (poles and
  lures) is kept as its own piece with its own durability, so a half-worn lure stays half worn.
- **Withdraw to sell or trade.** A Satchel item can be withdrawn to the bags, where it is an
  ordinary item: sold, traded, destroyed or, for a raw fish, eaten by right-click. Dragging it
  back onto the Journal returns it, and so does a right-click in the bags on any Journal Item that
  is not food or drink. A newly received Journal item always goes to the Satchel, never onto a withdrawn stack.
  A collect quest counts the Satchel and the bags and takes from the Satchel first.
- **A gear slot refills only with what it held.** The Pole, Lure and Bait slots fill from the
  Satchel. The player fills an empty slot by hand. When the slotted lure wears out or the chosen
  bait runs out, the slot keeps that entry and refills from the Satchel, now or when more of the
  same entry arrives. A different lure or bait never fills it.
- **A save from before moves its bag materials once.** Save version 5 adds the Satchel. Loading
  a version 4 save moves every bag item whose row names a profession into the Satchel. A version
  5 save never moves bag items, since a withdrawn item belongs in the bags.

## Considered options

- **Route only the fishing catch.** Rejected: Gnoll Ear bait drops from gnolls and the poles and
  lures come from vendors, so they would still fill the bags.
- **Route by stock item class.** Rejected: Trade Goods and Reagent mix professions, and a
  fishing pole is a weapon.
- **A slotted grid like a bag.** Rejected: the Journal could then be full, which is the problem
  being removed.
- **Lures as a plain count.** Rejected for the same reason ADR 0022 gave: swapping a lure out
  would throw its remaining bites away.
- **Sell straight from the Journal at a vendor.** Davis chose withdraw instead, so selling,
  buyback and trade stay stock.
- **Auto-fill an empty slot with the first pole, lure or bait that arrives.** Rejected by Davis:
  a slot never takes a different item than the one the player put there.
