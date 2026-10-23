# Azeroth Survivors

Game modes played offline on the benilla WoW 1.12.1 client engine.

## Modes

**Mode**:
A distinct game played on the engine, chosen before play starts.
_Avoid_: game type, variant

**Survivors**:
The Mode where an auto-casting hero survives waves of enemies on a battleground for a fixed time.

**Twilight's Eve**:
The Mode adapted from the Warcraft III custom game of the same name: a character starts as a Warrior and slowly advances through classes and gear by questing and running dungeons, playing like normal WoW.
_Avoid_: Twilight (alone)

## Twilight's Eve

**Gear Slot**:
One of the ten places a character wears an item: Weapon, Offhand, Ranged, Ammo, two Rings, two Trinkets, Chest and Amulet. Items for any other body part do not exist.
_Avoid_: equipment slot, inventory slot (bags and the backpack are not Gear Slots)

**Gear**:
An item worn in a Gear Slot. Junk, food, quest items and bags are items but not Gear.
_Avoid_: equipment, loot (loot is what a mob drops, Gear or not)

**Wardrobe**:
The whole-body look a Chest carries: what the character's head, shoulders, back, wrists, hands, waist, legs and feet show while that Chest is worn. With no Chest, the character shows underclothes.
_Avoid_: transmog, set (an item set with bonuses)

**Loot Source**:
Where a piece of Gear comes from: Token, World, Quest or Dungeon. Each has its own stat Budget per slot. The word is never Tier. Spec #272.
_Avoid_: tier, pool

**Flavour**:
The stat mix a piece of Gear carries: Might, Swift or Mind. Spec #272.
_Avoid_: archetype, role

**Budget**:
The stat points a piece of Gear may spend, set by its Loot Source and slot: 1 point is +1 Str, Agi, Sta, Int or Spi, and Spell Power costs half a point. Weapon damage and armour are free. The gear sheet's `budget` column holds it and the tool refuses a row over it. Spec #272.
_Avoid_: item level, stat weight

**Hub**:
The town at the centre of the world where the trainers, the innkeeper and most quest givers stand: the Crossroads.
_Avoid_: base, city

**Outpost**:
A smaller camp further out from the Hub, with quest givers for the Hunting Grounds around it but no trainers: Camp Taurajo, for The Barrens' southern reaches.
_Avoid_: camp (a group of mobs), town

**Hunting Ground**:
An area around the Hub where a character levels by killing mobs and doing Hunting Quests.
_Avoid_: zone (a WoW map area, which a Hunting Ground may only partly cover)

**Level Band**:
The range of Power Levels a Hunting Ground is built for, such as 1–10 for Northshire Valley. Its mobs' levels are set to fit the band, not taken from vanilla.
_Avoid_: zone level, level range (of a single mob)

**Camp**:
A group of mobs that stand together and fight together: pulling one pulls the rest. The Barrens' camps come from its vanilla spawns; the low camps by the Hub are placed by hand.
_Avoid_: pack, group (a player party)

**Local Realm**:
The simulated server that Twilight's Eve plays against, running inside the client with no network. It is the authority on all game state, as a real server would be.
_Avoid_: fake server, mock server, sim

**Class**:
The original WoW 1.12 class the client is told a character is, which decides its power bar. Every Calling on one Line shares a Class; a Calling's skills and talents may be borrowed from any Class.
_Avoid_: job

**Tier**:
How far a character has advanced: 0, 1, 2, and so on. Each Advancement puts the character one Tier higher.

**Basic Warrior**:
The Tier 0 Calling every player starts as. At level 10 it advances into one of the Tier 1 Callings, each the start of a Line.

**Calling**:
A named node in the advancement tree, at one Tier on one Line: Basic Warrior, Vanguard, Zealot, Templar. Each Calling can branch into several Callings at the next Tier, and has at most 8 skills, its own Talent Tree, its own Tier Bonus and the armour it may wear.
_Avoid_: specialization, spec (that means a talent build in WoW), job, rank, class

**Line**:
A Tier 1 Calling and every Calling that branches from it, such as the Vanguard line (Vanguard, Ironclad, Zealot, Bastion, Templar). Basic Warrior belongs to no Line.
_Avoid_: path, tree (a Talent Tree)
_Avoid_: specialization, spec (that means a talent build in WoW), job, rank

**Advancement**:
Moving from a Calling to one of the Callings it branches into at the next Tier. It resets the character's level, and the character comes out stronger than before.
_Avoid_: upgrade, promotion, prestige

**Hunting Quest**:
A quest from a Hunting Ground's giver, about the mobs or places of that Hunting Ground. It can be done once per character, from the Tier its level falls in or any later Tier. Advancement leaves completed quests and the quest log alone.
_Avoid_: repeatable quest, daily

**Repeatable Quest**:
A quest tied to a Dungeon that can be taken again as soon as it is handed in. It is worth about one level of the character's own XP, at any level.
_Avoid_: daily

**Token**:
A currency that Hunting Quests and Repeatable Quests reward, traded at a Hub vendor for green (uncommon) gear.
_Avoid_: mark, badge

**Dungeon**:
A run through mobs to a boss whose kill always drops one blue (rare) item, either on a real 1.12 instance map (Wailing Caverns) or as a Phased Dungeon on the open world. It can be run again without limit.
_Avoid_: instance (the map a Dungeon takes place on), raid

**Phased Dungeon**:
A Dungeon inside a building on the open world, such as Northshire Abbey, instead of on an instance map of its own. Walking through its Portal puts a character in its Phase. Everyone who enters shares the one copy.

**Phase**:
The side of a Portal a character is on. A character sees and can target only creatures on its own side: dungeon creatures are invisible and untargetable from outside, and the open world's are from inside.
_Avoid_: instance, layer

**Run**:
One go at a Phased Dungeon, shared by everyone inside. It starts when the first character enters an empty dungeon, keeps a clock that the dungeon's timed events read, and ends when the last character leaves, resetting the dungeon fully.

**Portal**:
The doorway that moves a character between the open world and a Phased Dungeon's Phase, with no loading screen.
_Avoid_: area trigger (the client's mechanism for a real instance's entrance)

**Tier Cap**:
The highest level a character can reach in a Tier, which is also the level at which they may advance: 10 for Tier 0, 30 for Tier 1, 50 for Tier 2, 60 for Tier 3, the last Tier, where 60 is the end of the game.
_Avoid_: max level

**Advancement Quest**:
A quest from a Calling's trainer, offered at the Tier Cap. Completing it advances the character into that Calling, so choosing which one to finish is how a player chooses their branch.

**Tier Bonus**:
A flat boost to base stats, set by each Calling and growing with each Tier; its mix suits the Calling's Line. With the Power Level, it is what makes a character stronger after Advancement resets their level. Gear, gold and completed quests carry over; skills and talents are replaced by the new Calling's.

**Power Level**:
The level a character's body and combat count as: each Tier's levels mapped in a straight line onto the band that Tier's content is built for, 1–10 for Tier 0, 10–30 for Tier 1, 30–50 for Tier 2 and 50–60 for Tier 3. A Tier 1 character at level 1 has Power Level 10. Base stats and combat formulas use it; XP, Talent Points, the Tier Cap and the level the client shows use the real level. ADR 0012.
_Avoid_: effective level, item level

**Zone Factor**:
A multiplier on the health and damage of every mob in one area, set in `mobs.toml` per `AreaTable.dbc` area. A subzone without its own takes its parent zone's. It is what makes a new zone feel hostile: Northshire Valley is ×1, the rest of Elwynn Forest is harsher.
_Avoid_: zone difficulty, zone level

**Tier XP Rate**:
How much more XP a character earns from everything, set by their Tier: ×1 at Tier 0, ×3 at Tier 1, ×5 at Tier 2 and ×7 at Tier 3. With kill and quest XP paid by the mob's or quest's own level, it makes the first levels after an Advancement go by in a few hand-ins.
_Avoid_: XP bonus, rested XP (WoW's own, which is separate)

**Talent Point**:
Earned one per level. Talents are the only abilities a character must earn; every skill of the character's current Calling is available from level 1.

**Talent Tree**:
The talents one Calling offers. Every Calling has its own.

**Creature Spell**:
A spell a creature casts in a fight, such as Geomancer Vurx's Corruption. Creature Spells are written by hand in the realm's `creature_spells.toml`, not in the master CSV, and a creature lists the ones it knows. The client only shows them: the cast bar, the icon and the debuff come from a stock spell's row.
_Avoid_: mob ability, creature ability

**Field Giver**:
A quest giver or ender who is not a Hub NPC: a friendly creature the realm places out in the world (Marshal McBride on the Abbey steps) or inside a Phased Dungeon (Brother Neals in the Abbey). It shows the `!` and `?` and answers the quest dialogs exactly as a Hub NPC does.
_Avoid_: field NPC, quest NPC

**Story Quest**:
A quest that tells part of the world's story, such as the Idol chain out of Echo Ridge Mine. It can be done once per character and Advancement never forgets it. It pays XP as a Hunting Quest does, and no Token.
_Avoid_: main quest, campaign quest

**Quest Starter**:
An item that begins a quest when used, as the Smoldering Twilight Idol begins The Idol in the Deep. It drops only until its quest is taken or done, only one can be carried, and it stays in the bags until the quest is handed in.
_Avoid_: quest item (any item a quest asks for)

**Rumour**:
A quest chain that a kill in a Hunting Ground drops by chance (spec #284). The drop spawns a Field Giver at one of the Rumour's listed points for its Audience, and a note for the looter that names the place. Its steps are ordinary quests from that giver; a step may spawn a named creature or a GameObject for the Audience. A timer from the drop ends it: the giver and everything it spawned despawn and the quest fails. Its Grade (easy, hard, deadly) sets the XP and the chest at the end. An ordinary Rumour can be found and done again; a Calling Rumour has no timer, leads to a trainer and is done once per character.
_Avoid_: event, world quest, random quest

**Audience**:
The characters who can see, fight, click and loot what a Rumour spawned: everyone within the kill credit range of the corpse when the Rumour dropped, fixed then (ADR 0014). An entity with no Audience is seen by everyone in its Phase.
_Avoid_: owners, party, group

**Calling Rumour**:
A Rumour that drops only for a character at their Tier Cap whose Calling advances into the Calling it names, and leads to that Calling's trainer. It has no timer, pays no chest and never drops again for a character who finished it.
_Avoid_: trainer quest, rumour quest

**Notoriety**:
How much the underworld has noticed a character. Everything the character achieves raises it: kills, quest hand-ins, Dungeon clears and rare drops. It drains slowly while the character rests in a Hub, falls to zero on any death, and is halved by Advancement. Every character has their own.
_Avoid_: heat (the meter), infamy, wanted level

**Heat Level**:
One of a few steps of Notoriety. Each sets a multiplier on all XP, applied on top of the Tier XP Rate, and how often a Bounty Hunter comes. The character sees their Heat Level as an aura.
_Avoid_: tier (Advancement's), notoriety tier

**Bounty Hunter**:
A hard elite sent after a character once their Heat Level is above zero, one at a time, at the character's own Power Level. It tracks its target across the open world and gives up only when the target enters a Hub or a Phased Dungeon. Each comes from one of a few archetypes. It is not a bounty quest, and not WoW's Hunter class.
_Avoid_: hunter, stalker, assassin

**Bounty Rank**:
How many Bounty Hunters a character has killed since their last death. Each rank makes the next Bounty Hunter much stronger and pays more Blood Coin. Any death sets it back to zero.

**Blood Coin**:
The currency only Bounty Hunters drop, more at a higher Bounty Rank. It is never lost on death and buys consumables, cosmetics and titles from a single vendor. It is not a Token.
_Avoid_: token, mark, spoils

**Marked**:
The state of a character a Bounty Hunter has killed: their stats are 10% lower until they kill that Bounty Hunter or two hours of play pass. The Bounty Hunter stays in that zone, roaming near where it won and shown on the map, and no other Bounty Hunter comes meanwhile. Being killed by it again does not deepen the loss. Killing it pays the Blood Coin of the rank it had, but does not raise the character's Bounty Rank.
_Avoid_: debuff, death penalty, nemesis

**Grounded**:
A placed unit whose Z the realm snaps to the ground under it at create time; the default. `grounded = false` on a content row keeps the written Z, for a unit meant to hover or hang. A ticket never gives a Z: the Build lane writes `0.0` and `tools/twilights-eve/ground_check.py --fix` computes it.
_Avoid_: snapped, clamped (the client's own ground clamp is a different mechanism)

**Angler Level**:
A character's fishing level, 1 to 60, with its own XP, kept across Callings (ADR 0018). Each level makes the Reeling Bar's catch zone taller and the treasure chest likelier. A fish gives Angler XP until it turns grey against the Angler Level by the same rule mob XP uses; a fish 10 or more levels above it never bites.
_Avoid_: fishing skill, fishing rank

**Reeling Bar**:
The minigame that opens when a fish bites: holding the mouse button lifts a catch zone, the fish moves by its style, and progress fills while the fish is inside the zone. The realm picks the fish and seeds the bar; the client runs it and reports the outcome (ADR 0018).
_Avoid_: minigame, fishing bar, catch bar

**Fishing Water**:
A named place, an AreaTable subzone or a circle in `content/fish.toml`, whose own fish pool replaces its zone's. Whether a point is water at all comes from the Host's terrain, never from a Fishing Water.
_Avoid_: fishing spot, fishing hole, pool (a pool is the weighted list of fish)

**Bait**:
A consumable chosen in the Fishing Journal's bait slot; each cast uses one from the bags. A legendary may bite only with its bait chosen. Not a lure: a lure goes on the pole for a time and gives one bonus.
_Avoid_: lure

**Legendary Fish**:
One of four rare fish per fishing zone: one bites anywhere in the zone at 1%, and three need a criterion (a Fishing Water, an hour window on the game clock, or a bait). Its Legend Rumour only hints at the criterion; the fish can bite without it.
_Avoid_: rare fish, boss fish

**Legend Rumour**:
A Rumour kind with no giver, no steps and no timer: a readable note that names a Legendary Fish and its criterion. It rolls on a catch in its zone or on a kill, and an NPC's gossip can tell the same hint.
_Avoid_: fishing quest, hint quest

**Kick**:
A Calling's skill that stops the cast a mob has under way and puts a Lockout on it. Each Tier 1 Calling learns one at level 10. Effects listed after the interrupt in its row are its bonus, and they happen only when it stopped a cast. ADR 0020.
_Avoid_: interrupt (the effect kind a Kick uses), silence

**Lockout**:
The seconds after a Kick during which a mob can't start any Creature Spell of the school it was casting. It still swings and casts spells of other schools.
_Avoid_: silence, school lock
