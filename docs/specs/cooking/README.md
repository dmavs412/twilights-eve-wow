# Cooking, as the Kitchen prototype plays it (spec #447)

Davis's Kitchen prototype is the cooking design (ADR 0025, amended 2026-10-27). This folder
holds what the Build lane's tickets read:

- `kitchen-prototype.html`, the prototype's source, published at
  https://claude.ai/artifact/ENPoEXoRt3uiqNf3pVPU8B. Its `realm` script is the reference for every
  rule: `resolve`, `distance` and `cook` for cooking, `mealInfo` for strengths, `rollOrders`,
  `orderAccepts` and `deliver` for orders, `lessons` for the checklist. Its items use short ids
  (`gperch`); `cooking.toml` uses the realm's entries.
- `cooking.toml` now lives at `crates/twilight-realm/content/cooking.toml` (#490). It was generated
  from the prototype and the realm's item entries: the eat spells, the 11 effects and their
  placeholder strengths, the two kitchens, herb categories, the 93 recipes, Hearty Mush, the 22 cut
  baits, the order tunables and the 116 item rows (94 meals, 22 cut baits). Its header names every
  field.

Item entries: meals 962001 to 962044 (tier recipes), 962101 to 962130 (fish dishes), 962201 to
962216 (legendary meals), 962301 to 962303 (plant meals) and 962400 (Hearty Mush); cut baits
960501 to 960522. Synthetic display ids 995060 to 995175. Every icon is in
`data/twilights-eve/art/` (docs/art-assets.md lists the placeholders).

## Fishing changes

Each cut bait is the favourite of a bigger fish, and each legend bites only on one cut bait.
`crates/twilight-realm/content/fish.toml` changes these fields; every other fish keeps its own.
Dreaming Glassfin is the favourite of both Loach Bits and Grayling Slices in the prototype; a
fish has one `favourite_bait`, so it keeps Loach Bits, and Grayling Slices are Moonless Muskie's bait.

| Fish | Entry | `favourite_bait` becomes |
|---|---|---|
| Canal Eel | 960408 | 960502 (Cut Perch) |
| Bristle Whisker Catfish | 6308 | 960504 (Snapper Belly) |
| Everstill Perch | 960411 | 960514 (Mackerel Strips) |
| Redridge Bluegill | 960412 | 960515 (Albacore Belly) |
| Mirror Lake Shiner | 960403 | 960501 (Smallfish Chunks) |
| Stonefield Carp | 960404 | 960503 (Trout Strips) |
| Slitherskin Mackerel | 6303 | 960505 (Herring Chunks) |
| Rainbow Fin Albacore | 6361 | 960507 (Whisker Bait) |
| Oily Blackmouth | 6358 | 960508 (Cut Lake Perch) |
| Millrace Trout | 960413 | 960513 (Carp Dough) |
| Lakeshire Sunfish | 960414 | 960516 (Blackmouth Oil Bait) |
| Ironjaw Pike | 960415 | 960511 (Chub Chunks) |
| Mithril Head Trout | 8365 | 960518 (Sunfish Chunks) |
| Duskfin | 960405 | 960506 (Eel Slices) |
| Crystal Lake Glimmerfish | 960406 | 960512 (Shiner Flash) |
| Gilded Koi | 960409 | 960509 (Bluegill Strips) |
| Dockside Gulper | 960410 | 960510 (Bleak Fry) |
| Emberscale Darter | 960416 | 960517 (Millrace Strips) |
| Everstill Whitefish | 960417 | 960519 (Pike Fillet Bait) |
| Ghostscale Carp | 960422 | 960520 (Mithril Scale Bait) |
| Dreaming Glassfin | 960423 | 960521 (Loach Bits) |

| Legend | Entry | `bait` becomes |
|---|---|---|
| Brother Bass | 960002 | 960502 (Cut Perch) |
| Moonlit Gar | 960003 | 960501 (Smallfish Chunks) |
| Gnoll Tooth Snapper | 960004 | 960504 (Snapper Belly) |
| Old Goldfin | 960001 | 960512 (Shiner Flash) |
| The Lightscale | 960006 | 960505 (Herring Chunks) |
| Bell-Toll Bream | 960007 | 960506 (Eel Slices) |
| Stockade Sucker | 960008 | 960513 (Carp Dough) |
| The Harbourmaster's Eel | 960005 | 960514 (Mackerel Strips) |
| The Millstone Carp | 960010 | 960507 (Whisker Bait) |
| Worgfang Pike | 960012 | 960509 (Bluegill Strips) |
| Blackrock Bloodfin | 960009 | 960515 (Albacore Belly) |
| Everstill Leviathan | 960011 | 960508 (Cut Lake Perch) |
| Sorrowgill | 960013 | 960510 (Bleak Fry) |
| The Twilight Koi | 960014 | 960511 (Chub Chunks) |
| Stitchjaw Eel | 960015 | 960521 (Loach Bits) |
| Moonless Muskie | 960016 | 960522 (Grayling Slices) |
