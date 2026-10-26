# One coin: Tokens go, quests pay copper, whites only drop

Grilled 2026-10-26. The Twilight Token was a second currency with little behind it: a bounty paid
one, a white cost one to three, and three vendors (the Crossroads, Northshire, Goldshire) sold
the same whites a kill already dropped at 8%. Nothing else took it. Davis's reading: the Token
had minimal value and gold had none, because gold bought nothing but bait. This ADR settles what
outlives the Goldshire spec (#459) that removed it.

- **The Twilight Token is not a currency.** No quest pays it and no vendor takes it. Item 90001
  stays defined as junk (Common, sell price 100 copper) so a save that holds Tokens still loads
  and the Tokens sell. The `tokens` key leaves the quest schema; a content test refuses a quest
  that carries it.
- **A quest that paid Tokens pays copper instead.** Each Token adds
  `round(100 x 1.05^(L - 10))` copper to the quest's `money`, L the quest's level: 71 at level
  3, 100 at 10, 148 at 18, 218 at 26. The 5% a level is ADR 0026's budget curve, so quest pay
  and gear grow together. Quest texts that promised "a Token" promise coin.
- **The white Loot Source is drops only.** ADR 0026's "Token" Source is called **White**: the
  same hand-authored rows, the same 8% roll by Loot Band, no vendor. No vendor anywhere sells
  Gear. The Token vendors leave: Sarna Duskwhisper becomes a Crossroads townsperson, Quartermaster
  Hollis and Wynne Tarrow go.
- **Blood Coin stays.** It is a trophy from one rare source, not a quest pay, and the only
  reason to fight a Bounty Hunter. The item-currency vendor code remains for it alone.
- **Gold is the one currency, and it has sinks.** Now: food, drink, bait, repairs and the talent
  respec, from townsfolk who sell them. Later, decided elsewhere: a mount, and realm services
  bought for a whole zone. Quest money is not retuned beyond the Token conversion until those
  land.

Consequences: ADR 0026's Token vendor sentence is amended by this ADR; #450 becomes the 105
whites of five Loot Bands with no vendor bands; `docs/reference/items-and-vendors.md`'s Token
vendor section is history; GLOSSARY drops **Token** and renames the Source (#461).
