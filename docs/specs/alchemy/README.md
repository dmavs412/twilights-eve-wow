# Alchemy, as the Alchemy prototype plays it (spec #522)

Davis's Alchemy prototype is the alchemy design (ADR 0025, amended 2026-11-01). This folder holds
what the Build lane's tickets read:

- `alchemy-prototype.html`, the prototype's source, published at
  https://claude.ai/artifact/YXnqKyyTNE1UGvmiigRbBZ. Its first `<script>`, `realm`, is the
  reference for every rule, and the tickets name its functions:
  - `sumProps`, `mainProp`, `resolve`, `pickHybrid` and `gradeFor`: what a make gives, before
    any roll. `failChance`, `payoff` and `carriedToxic`: the Toxic rule.
  - `make`, `learnGrade` and `openMake`: the roll, the materials used up, Herb Notes counted,
    grades learned, makes opened and the Brew Log line.
  - `bars` and `preview`: what the window shows while materials go in, from what the
    character's Herb Notes know.
  - `drink`, `applyOil`, `useFlask`, `swing`, `giveShield`, `takeDamage` and `expire`: what the
    products do.
  - `rumoursFor`, `rumour`, `sellSludge` and `setTable`: the alchemists.
  - `describe`, `recipePage`, `flaskPage`, `herbNotes` and `discoveries`: the Journal pages.
  - `_audit`: the budget check and the reachability report.
  The second `<script>`, `ui`, is the reference for the Alchemy window and the Journal pages.
  The prototype's character panel, test dummy and debug drawer are not part of the game.
- `alchemy.toml`, generated from the prototype and the realm's item entries. Its header names
  every field and the entry formula. The content ticket moves it to
  `crates/twilight-realm/content/alchemy.toml`.

Item entries: Murky Sludge 963000, potions 963100 to 963199, oils 963200 to 963299, hybrids
963300 to 963499, Tinctures 963500 to 964299, Essences 964300 to 966599, flasks 966600 to
971649: 8,551 rows, built by the realm at load. Synthetic display ids 995200 to 995245, one per
product kind. 36 icons are Davis's painted ones; the 10 hybrid icons are placeholders the
content ticket paints (`docs/art-assets.md`).

## Readings

Davis can overrule any of these. Each was a choice the prototype left open.

- **The alchemists** are new hub NPCs with the prototype's names (Davis: "wherever you want").
  Sister Marla, entry 190075, stands outdoors by the Abbey's herb beds at (-8885.0, -212.0),
  beside Brother Ansel. Alwen Carrow, 190076, stands in Goldshire at (-9452.0, 62.0), beside
  Hedda Thornwick. Quartermaster Ilsa Venn, 190077, stands inside Stormwind's Alchemy Needs on
  its ground floor (the floor at z 98.33) at (-8989.0, 757.0), between Lilyssia Nightbreeze and
  Tel'Athir. All on map 0.
- **A herb's level and rarity** for its budget are the lowest level and the rarest rarity of
  its herbs.toml pool rows. All 19 budgets then match the prototype's.
- **Fish for an oil** are any two `[[fish]]` items of fish.toml, never a Legendary Fish, from
  the Fishing Satchel or the bags.
- **An oil picks its hand.** The client has no weapon-target cursor, so using an oil coats the
  main hand if it wears no oil, else the off hand if the character holds an off-hand weapon
  that wears none, else it replaces the main hand's oil. An oil lasts through death.
- **Durations are real seconds**, as cooking's Well Fed is: a flask 2 hours, an oil 30 minutes,
  the potion cooldown 2 minutes.
- **Use spells.** The rows name stock spells with empty descriptions, so their tooltips show
  only the row's text: 2598 "Rejuvenation (OLD)" for potions and hybrids (the stock potion
  cooldown group, category 4, and usable in combat), 22 "Super Food (TEST)" for oils and
  flasks. A flask shows as stock aura 17626 "Flask of the Titans" until flasks get spells of
  their own.
- **Murky Sludge** lives in the Alchemy Satchel with the Tinctures and Essences. The Alchemy
  window sells it all for 5c each, as the prototype's Sell button does; a withdrawn one sells at
  any vendor.
- **Sell prices** by grade are cooking's by tier: 10c, 50c, 150c and 400c. Every row stacks to 20.
- **Numbers** are the prototype's placeholders: each grade is 2.5 times the one below. A
  balance pass tunes them later.
- **Thrown potions** need a hostile target within 30 yards. "Near the target" and "around you"
  are 8 yards. Deepsleep's sleep breaks on damage. Invisibility ends on an attack or a cast.
- **The Brew Log** keeps a character's last 80 makes.

## Tickets

Spec #522: #541 content and item rows, #542 the tables in the realm, #543 using potions, oils
and flasks, #544 the hybrids' effects, #545 the Alchemy window, #546 the Alchemy Journal
bookmark. #541 first; #546 last.
