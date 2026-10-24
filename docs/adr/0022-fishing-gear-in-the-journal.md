# Fishing gear lives in the Journal; lures wear per bite; the realm reveals what a fish teaches

Spec #323 put the pole in the main hand and made a lure a ten-minute aura, as vanilla does. A
Calling then has to swap its weapon out to fish, and the Fishing Journal records only each fish's
best catch. This ADR moves the gear into the Journal, gives lures wear, and makes catching the
same fish again teach you where and when to find it.

- **The Journal holds the pole and the lure.** The Journal has three slots: Pole, Lure and Bait.
  A pole or a lure dropped on its slot leaves the bags and lives on the character, saved with it.
  A cast needs a pole in the Pole slot. It never reads the main hand. While the cast lasts, the
  realm shows the pole in the main hand's visible-item field and puts the weapon's entry back
  when the cast ends. The weapon and its stats never move.
- **Poles differ through data and never wear.** A `[[pole]]` row in `content/fish.toml` gives a
  pole the effect keys a lure has. A stock pole's +Fishing becomes Angler levels, one per 5
  skill. A pole in the Pole slot loses no durability.
- **A lure is an item with durability, worn one per bite.** A lure's item row has `durability`
  and `stackable = 1`, so the client's own tooltip shows its wear and a half-worn lure goes back
  to the bags as it is. Each bite (each `FishingBite`) costs the slotted lure one point, whether
  the fish is landed or not. At zero the lure is destroyed and the next one of the same entry
  in the bags takes the slot. Repair skips lures. The aura and `duration_secs` are gone.
- **Fish behaviour gains three data keys, and each one changes the roll.** A `[[fish]]` row may
  name `kinds` (river, lake, coast, pond), `peak_hours` on the game clock (ADR 0019), and a
  `favourite_bait`. Every `[[pool]]` has a default `kind` for its zone's unnamed water, and a
  `[[water]]` row may override it. A fish whose `kinds` leave out the water's kind drops out of
  the roll there. A fish with no `kinds` lives in every kind. Inside its peak hours, and with
  its favourite bait on the cast, a fish's weight is multiplied by factors in `fish.toml`.
  Legendary Fish keep their own criteria and ignore kinds.
- **The Journal keeps a catch log and reveals facts by count.** Per fish the character keeps
  the total caught, the perfect catches, and a count per zone or named Fishing Water. Time of
  day is not logged. Facts from the data unlock at catch counts set in `fish.toml`: 5 for the
  zones and waters it lives in, 10 for its kinds and peak hours, 25 for its favourite bait. The
  realm sends only the facts a character has unlocked, so a locked fact never reaches the client.

## Considered options

- **The pole stays in the main hand and the Journal only shows it.** Rejected: every Calling
  would still swap weapons to fish, which is the friction this ADR removes.
- **Lure wear on a counter in the slot, with lures still stacking.** Rejected: taking a lure out
  early would throw away its remaining bites. Item durability keeps each lure's wear with it.
- **Wear per catch, or per cast.** Per catch lets a lost fish cost nothing; per cast charges for
  empty water. Per bite charges for each Reeling Bar the lure helped open.
- **Revealed facts as text only, with no behaviour behind them.** Rejected: "bites at dusk" would
  be a lie for every fish. Each revealed fact is a rule the bite roll follows.
- **The client holds the facts and hides the locked ones.** Rejected: the realm already decides
  what each character may see (ADR 0014), and a locked fact sent to the client is a fact a
  friend's edited client can read.
