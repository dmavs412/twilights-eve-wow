# Trinkets: the Loaded Die, the Hourglass and the Echo Shard (spec #458)

Status: tickets #567 to #571 ready-for-agent, written 2026-11-02. Unplayed.

Grilled 2026-11-01 and confirmed by Davis on 2026-11-02. The two Trinket Slots (ADR 0007) are
empty through level 30 until these land. Each trinket here is fun to click and gives no stats.
This note holds the rules the tickets build and every reading taken where the grill left a choice
open.

## The three trinkets

All three are blue (Rare), bind when picked up, are unique (a character holds one of each), carry
no stats and have no required level. Each has one Use and does nothing while worn.

| Trinket | Entry | Item level | From | Cooldown |
|---|---|---|---|---|
| Morgan's Loaded Die | 906000 | 14 | Morgan the Collector (96012), 25% a kill, on top of his loot | 5 min |
| Apprentice's Hourglass | 906001 | 22 | a sure reward on Where It Pointed (94604), beside its four chests | 5 min |
| Vael's Echo Shard | 906002 | 24 | Speaker Vael (96040), 25% a kill, on top of his loot | 2 min |

### Morgan's Loaded Die

Click it to roll a fair die, 1 to 6. The roll is shown in chat as a stock `/random` line ("You
roll 4 (1-6)"). The die isn't loaded, and the name is a joke.

| Face | What happens |
|---|---|
| 1 | You lose 30% of your current health. It never kills: health stops at 1. |
| 2 | You heal 20% of your maximum health. |
| 3 | A quarter of your power bar fills (rage, energy or mana). |
| 4 | Quick Hands, 15 s: your swings, shots and casts are 25% faster. |
| 5 | Blood Luck, 15 s: you heal for 20% of the damage your hits deal. |
| 6 | Sixes, 15 s: you deal double damage. |

### Apprentice's Hourglass

Morganth was Theocritus's apprentice, so the Hourglass is his. Click it, and the Falling Sand buff
runs for 6 seconds. When the buff ends, you snap back to where you stood when you clicked, with
the health and power you had then. The rewind is true, so you can end up worse off than before.
Nothing else rewinds: cooldowns stay spent, debuffs keep ticking, and enemies keep their threat
and chase you. Dying ends the buff with no snap, and so does changing maps (a dungeon door, the
Hearthstone, a worldport).

### Vael's Echo Shard

Click it, and the Echo buff waits up to 20 seconds. The next Calling skill you use goes off a second
time at once, at the same target, free and off the global cooldown. The second cast never echoes
again (the Twin Arrows rule). Auto Shot, wand shots, channelled skills and skills that move you
(charges, leaps) pass the buff by and don't use it up.

## Rules for all three

- A click is instant, triggers no global cooldown, and is refused while a cast bar or a channel
  runs (`SPELL_FAILED_SPELL_IN_PROGRESS`) or while dead.
- Putting a trinket on starts a 30 second wait before it can be clicked, unless its own cooldown
  has longer to run (vanilla's equip cooldown). Swapping between fights works; swapping mid-fight
  to fire all three doesn't.
- Cooldowns are per item, kept in the save (`Char::item_cooldowns`), and shown on the character
  sheet's slot and on an action button.
- The three spells and their buffs live in the realm's own file,
  `crates/twilight-realm/content/trinket_spells.toml`, beside `creature_spells.toml`. They don't go
  in the master CSV. `build_spells.py` writes their client rows, as it does for Creature Spells.

## Readings taken

These are the session's choices where the grill was silent. Change any of them in the ticket that
builds it.

- Entries 906000 to 906002 (no content file names a 906xxx entry). Spell ids 1,960,000 and up
  (Creature Spells hold 1,900,000 and up, and #560 plans 1,950,000 for Open Road): the die
  1960000, its buffs 1960001 to 1960003, the Hourglass 1960004, the shard 1960005, Falling Sand
  1960006 and Echo 1960007.
- Unique, `max_count = 1`. Wearing two dies would double the gamble.
- The item level is the source's level (14, 22, 24). It drives nothing but the sell price.
- Sell price: the same as an Elite blue of the source's Loot Band.
- Face 3 fills a quarter of the maximum power: 25 rage, 25 energy, or a quarter of the mana bar.
- Face 4 is three effects on one buff: melee haste, ranged haste and cast speed, 25% each.
- Face 5 heals on direct damage only: swings, shots, and skill and spell hits. Periodic ticks don't
  heal.
- Face 6 doubles every school's damage the player deals, periodic ticks included, while the buff is
  on. It reuses `ModDamageDonePct` (100) where a damage path already goes through
  `InWorld::dealt`, and the ticket routes any path that doesn't.
- A stealthed character can use a trinket, and using one doesn't break stealth. The echoed skill
  follows its own stealth rule.
- Right-clicking Falling Sand off ends it with no snap, the same as dying.
- The snap restores health and power capped at today's maximum (a buff that ran out lowers it).
- The echo goes off whether the first cast hit, missed or was dodged.
- Buff names (Quick Hands, Blood Luck, Sixes, Falling Sand, Echo) are placeholders for Davis to
  rename.

## Tickets

- #567 Trinket spells file, trinket rows, clicking a worn trinket, and cooldowns.
- #568 The Loaded Die and the extra loot roll, blocked by #567.
- #569 The Apprentice's Hourglass, blocked by #568.
- #570 Vael's Echo Shard, blocked by #568.
- #571 Client: a worn item's cooldown on the character sheet. It stands alone.
