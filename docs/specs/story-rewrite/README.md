# The story rewrite

Davis's rewrite of every quest and rumour to fit `docs/lore-bible.md`. Grilled 2026-10-23 in the
Pipeline lane. Most quests and rumours show the lives of ordinary people in a world that is
shrinking; the plot shows up in a detail.

## Decisions (grill 2026-10-23)

1. **The Barrens is retired in this spec.** Kalimdor is behind the mist. The Crossroads hub, its
   Hunting Grounds, about 105 Barrens quests, the Wailing Caverns and its three quests
   (92001-92003) are removed. Elwynn becomes the only start: the character screen loses the
   Barrens choice.
2. **The quest kinds stay in code.** "hunting" and "repeatable" keep their rules in
   `rules::quest` even with no quest left using them, so the removal is a content deletion.
   (Claude's call; Davis can overrule.)
3. **Two Token vendors.** One Token currency. A Token vendor in Northshire sells today's banded
   stock unchanged. A second in Goldshire sells its own flat list in `token_vendor.toml`, seeded
   with the 11-30 greens, which Davis swaps in `gear.csv` later.
4. **The Blood Coin Broker moves to Goldshire.** Rusk Ninefingers's job, with an Elwynn name and
   lines that fit the bible.
5. **Tier 2 and 3 advancement moves to Stormwind.** 64002 (Ironclad), 64003 (Zealot) and 64004
   (Templar) get a new trainer in Stormwind City as their giver, at the same levels.
6. **Text first, structure where needed.** Every quest gets new title, objectives, details,
   progress and completion text. Mobs, counts, items and chains stay unless a quest can't carry
   its new story without a change; those become separate tickets.
7. **No fixed ratio.** Most quests and rumours give insight into people's lives. Plot quests stay
   where the chains need them.
8. **Claude drafts, Davis reviews.** Claude drafted eleven existing quests and one rumour, and Davis
   kept all of them as written after a grammar and prose pass (2026-10-23). Claude wrote the rest
   in that voice. Tickets only paste reviewed text.

## The voice Davis approved

- Each quest starts with one person's own trouble. The shrinking world shows in a detail: a road
  that ends in grey, a ship that never came back, a family on the inn floor.
- At most one plot thread per quest or rumour, and only at its band's rung on the reveal ladder
  (`docs/lore-bible.md`). Many quests carry none.
- Full, grammatical sentences, short and plain-spoken. Contractions in speech. Kobolds, Theocritus
  and Brother Neals speak a little more formally.
- Nobody in Elwynn names the Twilight's Hammer, Twilight coin or Anchors. The coin is "strange
  coin" or "that coin."
- The loss stays specific, and the sources vary: Westfall, the Ironforge road and tram, the ships
  and the sea, Lordaeron's refugees, the king's ship, Kalimdor.

## The draft, for review

Every Elwynn quest, rumour note and Calling quest, by band. Approved samples are marked and
unchanged.

1. `1-northshire.md`: 94001-94009, Rumours 1-3 (94901-94906 and their notes), Tier 1 Calling
   quests 64009-64013.
2. `2-goldshire-farmlands.md`: 94101-94108, 94201-94208.
3. `3-crystal-lake-jasperlode.md`: 94301-94308, 96101-96103, 94401-94404.
4. `4-stone-cairn-azora.md`: 94501-94508, 94601-94606.
5. `5-eastvale-jerods-landing.md`: 94701-94709, 94801-94808.
6. `6-stormwind-and-fish-notes.md`: 64002-64004 at the new Stormwind trainer (named Commander
   Aldous Wray in this draft), and the legend-fish notes.

Not covered: the Barrens quests (91001-91105, 93001, 64001, 64005-64008) and the Wailing Caverns
quests (92001-92003), which are removed rather than rewritten. Spec 5's Westbrook and Hogger
quests aren't built yet; when they are, "What the Tribute Was For" follows the bible's rewrite (the
cult employs Morganth, and the tribute feeds the Anchor at Ilgalar).
