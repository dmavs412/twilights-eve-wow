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

**Hub**:
The town at the centre of the world where the trainers and quest givers stand.
_Avoid_: base, city

**Hunting Ground**:
An area around the Hub where a character levels by killing mobs and doing repeatable quests.
_Avoid_: zone (a WoW map area, which a Hunting Ground may only partly cover)

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

**Repeatable Quest**:
A kill or collect quest tied to a Hunting Ground that can be taken again as soon as it is handed in. At its own level it is worth about one level of XP; the reward shrinks as the character outlevels it.
_Avoid_: daily

**Token**:
A currency that Repeatable Quests can reward, traded at a Hub vendor for green (uncommon) gear.
_Avoid_: mark, badge

**Dungeon**:
A solo run through a real 1.12 instance map: about ten mobs, then a boss whose kill always drops one blue (rare) item. It can be run again without limit.
_Avoid_: instance (the map a Dungeon takes place on), raid

**Tier Cap**:
The highest level a character can reach in a Tier, which is also the level at which they may advance: 10 for Tier 0, 30 for Tier 1, 50 for Tier 2, 60 for Tier 3, the last Tier, where 60 is the end of the game.
_Avoid_: max level

**Advancement Quest**:
A quest from a Calling's trainer, offered at the Tier Cap. Completing it advances the character into that Calling, so choosing which one to finish is how a player chooses their branch.

**Tier Bonus**:
A flat boost to base stats, set by each Calling and growing with each Tier; its mix suits the Calling's Line. It is what makes a character stronger after Advancement resets their level. Gear, gold and completed quests carry over; skills and talents are replaced by the new Calling's.

**Talent Point**:
Earned one per level. Talents are the only abilities a character must earn; every skill of the character's current Calling is available from level 1.

**Talent Tree**:
The talents one Calling offers. Every Calling has its own.
