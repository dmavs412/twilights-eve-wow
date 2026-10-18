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
The range of character levels a Hunting Ground is built for, such as 1–10 for Northshire Valley. Its mobs' levels are set to fit the band, not taken from vanilla.
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
A quest from a Hunting Ground's giver, about the mobs or places of that Hunting Ground. It can be done once per Tier: Advancement forgets every completed Hunting Quest and drops any still in the log, so each Tier does them again from level 1.
_Avoid_: repeatable quest, daily

**Repeatable Quest**:
A quest tied to a Dungeon that can be taken again as soon as it is handed in. At its own level it is worth about one level of XP; the reward shrinks as the character outlevels it.
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
A flat boost to base stats, set by each Calling and growing with each Tier; its mix suits the Calling's Line. It is what makes a character stronger after Advancement resets their level. Gear and gold carry over, completed Hunting Quests do not; skills and talents are replaced by the new Calling's.

**Tier XP Rate**:
How much more XP a character earns from everything, set by their Tier: ×1 at Tier 0, ×3 at Tier 1, ×5 at Tier 2 and ×7 at Tier 3. It makes replaying the low levels after an Advancement quick.
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
