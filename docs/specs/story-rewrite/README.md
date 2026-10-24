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
8. **Claude drafts, Davis reviews.** Claude drafts about ten existing quests (`samples.md`), Davis
   marks the ones that work and the ones that don't, and Claude writes the rest in that voice. Tickets then
   only paste reviewed text.
