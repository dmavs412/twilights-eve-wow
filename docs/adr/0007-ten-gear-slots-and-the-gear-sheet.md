# Ten Gear Slots, the Chest's Wardrobe, and the gear sheet

Twilight's Eve keeps ten Gear Slots: Weapon, Offhand, Ranged, Ammo, two Rings, two Trinkets,
Chest and Amulet. Items for every other body part (head, shoulders, back, shirt, tabard, wrists,
hands, waist, legs, feet) do not exist: they never drop, sell or start in an outfit, and the realm
refuses to equip them. Bags stay as they are, and the hands keep vanilla rules (a two-hander fills
both, dual wield by Calling).

**Amended 2026-10-18 (#157): the Ammo slot is `PLAYER_AMMO_ID`.** As in vanilla, ammo is never
worn: the arrows or bullets stay in the bags, and the Ammo Gear Slot is the field naming the
entry the ranged weapon shoots, set by `CMSG_SET_AMMO` (the sheet's ammo slot) and saved with the
character. Each ranged attack spends one from the bags, and the ammo's DPS times the weapon's
speed adds to the shot. A bow or crossbow takes arrows, a gun bullets, a thrown weapon none.

The look of the whole body comes from the Chest. Each Chest carries a Wardrobe, a display per
hidden body part, which the realm writes into the 1.12 visible-item fields as if a full set were
worn. Those pieces are never items: they are not in the inventory fields, and the client's
character pane shows only the ten slots. No Chest worn means underclothes. The Chest is the
strongest defensive piece, but carries less than a full vanilla set would. It keeps its armour
type, gated by Calling.

Every piece of Gear is authored by hand in `data/twilights-eve/gear.csv`, in words rather than
1.12 ids (slot, type, hand, one column per stat), with the Wardrobe as columns on Chest rows and
Equip/Use/Chance-on-hit effects as references to spells in `master.csv`. A tool generates what the
realm loads. vmangos stops being an input for Gear: the sheet was seeded once from it, keeping
vanilla entry ids, and new items take entries from 900000 up. Who drops what stays in the loot
tables, not the sheet. Junk, food, quest items and bags keep their own tables.

We cut the slots so that every drop matters and there are few enough items to author each one by
hand. Tying the look to the Chest means a few hundred hand-picked Wardrobes instead of thousands
of mismatched pieces, at the cost of mixing and matching looks.

## Considered Options

- Only the chest model drawn, other parts bare: rejected; characters look half-dressed.
- Wardrobe derived from vanilla item sets or model names: rejected; many chests have no set, and
  curated looks are the point. Vanilla sets only pre-fill the seed.
- Chest carries a full set's armour and stats: rejected in favour of strongest-piece tuning.
- A separate `wardrobe.csv`: rejected; one row per Chest is easier to edit.
- vmangos as base with an override sheet: rejected; the goal is custom items throughout.

## Consequences

The realm's armour, health and mob tuning drift from vanilla until the Chest budget is tuned.
Item queries are answered from the sheet, so a custom item needs no DBC patch, only a
`display_id` the client's ItemDisplayInfo has.
