Title: Tier 2 Callings, hidden trainers and the level 30 Calling Choice (spec)
Labels:

Replaces #556, which closes into this spec when it is filed. Grilled by Davis on 2026-11-08
from `docs/specs/prep/tier2-callings.md`; its "Grill answers" section is the source of every
decision below. Every "Assumed:" line is Claude's reading of an answer, and Davis can amend it
before the tickets run.

## What this spec delivers

A Tier 1 character at level 30 gets the Calling Choice popup again, listing the two Tier 2
Callings of its Line. The pick puts that Calling's Advancement Quest in the log. The quest is a
search: its text is a riddle and names someone who knows more. The ten Tier 2 trainers are hidden
through Elwynn, Stormwind, Redridge, Duskwood and one dungeon, some easy to find and some hard.
Each trainer sets a trial questline before advancing the character. An easy trainer sets a hard
trial and a well hidden one an easy trial. After Advancement the trainer sends the character on
to Redridge and Duskwood.

Ten Tier 2 Callings, two per Line, each with an eight-skill kit, ten Talent Rows of three
choices, its armour and its trainer. The balance model grows to Power Level 50 and fits the
Tier Bonus and the Tier Step together.

## Decisions (the grill, 2026-11-08)

1. Two Tier 2 Callings per Line, all ten built now.
2. Same role, new tricks. Both branches keep their Tier 1 Calling's role and differ by
   mechanics.
3. A Tier 2 kit carries about a quarter of its Tier 1 kit, about two skills. Its new skills dig
   deeper into the Tier 1 themes.
4. Every skill, carried or new, gets three new Talent Row choices. No Tier 1 choice is copied.
5. Ten Talent Rows, one every 5 levels into the Tier: 5, 10, 15 and so on to 50. A carried skill
   starts stronger than the Tier 1 Calling's version of it.
6. Vanguard Line (tank): Ironclad (blocks build Conviction, Shield Slam) and Zealot rebuilt as a
   holy tank (Consecration, Seal of Iron). The built two-handed Zealot kit goes, except Judgement
   and Hammer of Justice.
7. Cutthroat Line (melee damage): Bloodletter (blood on the ground) and Nightblade (Stealth in
   the middle of a fight).
8. Initiate Line (ranged spell damage): Pyromancer (Embers that spread) and Leyweaver (arcane,
   hits hardest on a full mana bar, spends mana for burst). No Frostbinder.
9. Ranger Line (ranged weapon damage): Sharpshooter (rewarded for standing still, with tools to
   hold its ground) and Trapper (the mobile one: traps and ground areas laid on the move). No
   Outrider.
10. Acolyte Line: Blightcaller and Bloodmender, both the game's main healers. Blightcaller heals
    through the Blight it spreads; Bloodmender by spending and drinking its own health.
11. Level 30 flow: the popup pick puts the branch's Advancement Quest straight in the log, as at
    level 10. No Calling Rumour.
12. The Advancement Quest is "find the trainer". Trainers are hidden in the open zones and even
    dungeons. Hints come from the quest text (a riddle, plainer for easy ones) and from NPCs who
    know (the quest names one; harder trainers take a chain). The trainer sets a trial
    questline before advancing; trial difficulty runs opposite to search difficulty. After
    advancing, the trainer sends the character to Redridge and Duskwood.
13. Tier Bonus: fit by the balance model at Power Levels 30, 40 and 50, together with the Tier
    Step.
14. Armour: Ironclad and Zealot up to plate and a shield; Bloodletter and Nightblade leather;
    Pyromancer and Leyweaver cloth; Sharpshooter mail; Trapper leather; Blightcaller and
    Bloodmender cloth.

## Shared rules for every Tier 2 Calling

### The kit

- Eight skills: two carried from the Tier 1 Calling, the Line's Kick, four new skills and one
  new utility. The Ranger Line keeps Auto Shot on top, as Tier 1 does (the bow itself, never a
  row).
- Assumed: the Kick is carried outside the quarter (its bonus already ties to the Line's theme),
  and the Tier 1 utility is not carried; each Tier 2 Calling gets a new utility on its theme.
- Learn levels. Assumed: the two carried skills, the Kick and the first two new skills at level
  1; the third new skill at 10; the fourth at 20; the utility at 30.
- A carried skill is a new row under the Tier 2 Calling (`<calling>/<key>`, its own `spell_id`),
  copied from the Tier 1 row. Assumed rule for "starts stronger": every amount block's `base`
  becomes `round(1.1 * (base + 30 * per_level))`, keeping `per_level` and `per_ap`; costs,
  cooldowns and auras stay. So a Tier 2 copy at its level 1 beats the Tier 1 skill at level 30 by
  about 10%. A carried skill with no amount block (Rapid Fire, Stealth, Seal of Iron, a Kick)
  takes 10% more on one named number instead, and each Kick's lockout grows by half a second.
  The aura rows a carried skill names are copied too. A carried effect that names a skill or
  aura the Tier 2 Calling lacks is replaced (Silencing Shot's Vault reset on the Sharpshooter,
  Rebuke's Conviction on the Zealot, Hush's Blight on the Bloodmender, Garrote's Stealth gate on
  the Bloodletter, Catalyst's Biting Cold half on the Pyromancer); each ticket says how.
- New skills' numbers are placeholders sized against a Tier 1 skill of the same job at L = 30.
  The balance tickets (21 to 24) fit them.

### The ten Talent Rows

Row Levels `5|10|15|20|25|30|35|40|45|50` on every Tier 2 `calling` row. Assumed reading of
answer 4 ("every skill"): all eight skills get a Skill Choice row, the Kick and the utility
included, and the two left over are Talent rows. That amends ADR 0028's "the Kick and the
utility never get a row" for Tier 2; ticket 01 writes the amendment. Every Calling uses this
template:

| Row Level | Row |
|---|---|
| 5 | first carried skill |
| 10 | the new filler or builder |
| 15 | second carried skill |
| 20 | second new skill |
| 25 | Talent row 1 |
| 30 | third new skill (learned at 10) |
| 35 | the Kick |
| 40 | fourth new skill (learned at 20) |
| 45 | the utility (learned at 30) |
| 50 | Talent row 2 |

### The Calling Choice at 30 and the search

- The popup opens at the Tier 1 Cap for a Calling whose Advancement Quests carry
  `calling_choice = true` (ticket 01). Its system line no longer names the trainer.
- Each trainer is `hidden`: no Living Map mark until the character has met them (ticket 02).
- Hint NPCs answer a gossip line only while the character holds the Advancement Quest (ticket
  02, `hint` rows on the quest).
- The Advancement Quest's hand-in waits on the trial (ticket 02, `trial = <last trial quest>`).
  The first trial quest has `after_taken = <the Advancement Quest>`; each later one `after` the
  one before. Trial quests are level 30 with `min_tier = 1`.

## Roster

| Calling | Line, role | Armour | Carries | New skills (utility last) | New realm kinds |
|---|---|---|---|---|---|
| Ironclad | Vanguard, tank | cloth to plate, shield | Righteous Smash, Shield of the Penitent, Rebuke | Shield Slam, Retort, Iron Wall, Steel Resolve, Hold the Line | per-target stacks (03) |
| Zealot | Vanguard, tank | cloth to plate, shield | Bulwark Charge, Seal of Iron, Rebuke | Judgement, Hallowed Ground, Righteous Fury, Holy Wrath, Hammer of Justice | judging Seal of Iron, threat aura, creature type (03) |
| Bloodletter | Cutthroat, melee damage | cloth, leather | Rupture, Garrote, Kick | Hemorrhage, Open Veins, Blood Fan, Red Frenzy, Blood Pool | bleed taken, tick now, spend own stacks, spill droplets (04) |
| Nightblade | Cutthroat, melee damage | cloth, leather | Eviscerate, Stealth, Kick | Backstab, Ambush, Vanish, Cold Blood, Kidney Shot | drop threat and Stealth in combat (04) |
| Pyromancer | Initiate, ranged spell damage | cloth | Searing Ember, Catalyst, Counterspell | Fireball, Pyroblast, Kindle, Flamestrike, Blast Wave | copy an aura (05) |
| Leyweaver | Initiate, ranged spell damage | cloth | Ley-Spark, Ley Ward, Counterspell | Arcane Missiles, Arcane Surge, Ley Tap, Attunement, Ley Bind | damage by mana fill and mana spent (06) |
| Sharpshooter | Ranger, ranged weapon damage | leather, mail | Puncturing Shaft, Rapid Fire, Silencing Shot (and Auto Shot) | Steady Shot, Aimed Shot, Sunder Arrow, Hold Your Ground, Pinning Shot | none |
| Trapper | Ranger, ranged weapon damage | leather | Snapshot, Vault, Silencing Shot (and Auto Shot) | Immolation Trap, Frost Trap, Caltrops, Spring the Traps, Tripwire | traps that spring on a group, set off traps (07) |
| Blightcaller | Acolyte, main healer | cloth | Creeping Blight, Miasma, Hush | Plague Tithe, Harvest, Spread Blight, Wasting Communion, Rise Again | copy an aura, death spread (05), resurrect (08) |
| Bloodmender | Acolyte, main healer | cloth | Blood Lance, Dark Mending, Hush | Blood Feast, Transfusion, Blood Ward, Circle of Blood, Last Rites | resurrect, health-cost heal over time (08) |

Each Calling's data ticket (10 to 19) holds its full kit, its thirty choices, its trainer, its
riddle, its hint chain and its trial quests.

Assumed for the Sharpshooter: "mail" means it keeps the Ranger's `leather|mail`, since `armor`
lists every kind worn. For the Trapper, "leather" means `leather` alone. For the Cutthroat Line,
"leather" keeps Tier 1's `cloth|leather`.

## The ten trainers

| Calling | Trainer | Where | Find | Hint chain (first is named in the quest) | Trial |
|---|---|---|---|---|---|
| Ironclad | Sergeant Maera Holt | Lakeshire bridge, west end, Redridge (band 32-35) | easy | Commander Aldous Wray | 4 quests, hard |
| Pyromancer | Magus Ilyra Vance | Mage Quarter courtyard, Stormwind | easy | Elsa Brightwood | 4 quests, hard |
| Trapper | Hale Brannock | tree line south of the Eastvale Logging Camp, Elwynn (20-24) | easy | Thalyssa Windbough | 4 quests, hard |
| Zealot | Brother-Knight Aldren Vale | Tranquil Gardens Cemetery, Duskwood (33-36) | medium | Wray, then Commander Althea Ebonlocke | 3 quests |
| Bloodletter | Sabine Redmark | poachers' camp on Lakeridge Highway, Redridge (33-36) | medium | Jorik Vane, then Wiley the Black | 3 quests |
| Sharpshooter | Wren Ashcombe | hilltop east of Three Corners, Redridge (30-32) | medium | Thalyssa Windbough, then Guard Parker | 3 quests |
| Bloodmender | Sister Elowen Marr | Cathedral Square, Stormwind | medium | Brother Corwin Ashgrave, then Brother Kristoff | 3 quests |
| Nightblade | Silas Dunmere | the Riot's round room, the Stockade (dungeon, 30-34) | hard | Jorik Vane, Innkeeper Farley, Warden Thelwater | 2 quests, easy |
| Leyweaver | Teodric Lumen | a ley spring on the Darkened Bank, Duskwood (30-32) | hard | Elsa Brightwood, Zaldimar Wefhellt, Watcher Callahan | 2 quests, easy |
| Blightcaller | Mother Ysolde Grey | behind the ruined house at Beggar's Haunt, above Darkshire (31-33) | hard | Brother Corwin Ashgrave, Madame Eva, Blind Mary | 2 quests, easy |

Every Line has a mix: Vanguard easy and medium, Cutthroat medium and hard, Initiate easy and
hard, Ranger easy and medium, Acolyte medium and hard. The bands are the Redridge and Duskwood
arc's (its tickets 06 and 11). A trainer must be reachable by a Tier 1 character at level 30, so
every trainer and every trial stands in a band of 30 to 36, in Elwynn or in Stormwind, except
the Pyromancer's hard trial, which sends the character into the Redridge Canyons (36 to 39). The
Rotting Orchard (45-48), Render's Rock (41-44) and Ridgepoint Tower (a cult site of the Elwynn
arc, #247) were first picks and were dropped for that reason.

The only dungeon trainer is in the Riot, which is built (#575), so nothing waits on the 30-50
dungeons spec. Assumed: the Nightblade's trainer is a friendly creature in `stockade_riot.toml`,
a Field Giver like Bertram Ash, so the dungeon's phase and map marks need no change.

Commander Aldous Wray stays in the barracks as the Vanguard Line's first hint. 64004 "The
Templar's Charge" (Zealot to Templar, Tier 3, not completable) stays his.

## Ids

| What | Range |
|---|---|
| Advancement Quests | 64002 Ironclad, 64003 Zealot (both reused), 64401 Bloodletter, 64402 Nightblade, 64403 Pyromancer, 64404 Leyweaver, 64405 Sharpshooter, 64406 Trapper, 64407 Blightcaller, 64408 Bloodmender |
| Trial quests | 64411-64414 Ironclad, 64416-64418 Zealot, 64421-64423 Bloodletter, 64426-64427 Nightblade, 64431-64434 Pyromancer, 64436-64437 Leyweaver, 64441-64443 Sharpshooter, 64446-64449 Trapper, 64451-64452 Blightcaller, 64456-64458 Bloodmender |
| Delivery quests (ticket 20) | 97421-97440, two per trainer |
| Trainer Hub NPCs | entries 190110-190119, guids 9300110-9300119 (190113 and 9300113 unused: the Nightblade's trainer is creature template 97290, spawn guid 973028 in the Riot) |
| `text_id` | 64180-64189 trainers, 64190 Wray |
| Quest items | 980811-980909, ten per Calling (only six are used) |
| `spell_id` | from the highest in the sheet, as `calling-script-guide.md` says; the data tickets run one after another, so no two take the same |

## Tickets

Realm first, then the client, then one data ticket per Calling (chained, since they share
`master.csv`, both generated JSON files, `callings.csv`, `quests.toml` and `hub.toml`), then the
deliveries and the balance model. `99-index.md` has the blockers, the files each edits and what
runs alongside.

- 01 The Calling Choice per Tier (realm, docs)
- 02 Hidden trainers, hint lines and trials (realm)
- 03 Holy tank kinds (realm)
- 04 Blade kinds (realm)
- 05 Aura copies (realm)
- 06 Mana kinds (realm)
- 07 Trap springs (realm)
- 08 Resurrection and blood heals (realm, one client line)
- 09 The Calling Choice window for Tier 2, and ten Talent Rows (client)
- 10 to 19 Sharpshooter, Ironclad, Zealot, Bloodletter, Nightblade, Pyromancer, Leyweaver,
  Trapper, Blightcaller, Bloodmender (data)
- 20 Each trainer sends the character to Redridge and Duskwood (moves RD:09's two deliveries)
- 21 to 24 The balance model to Power Level 50, the Tier 2 sims, and the Tier Bonus and Tier
  Step fit

## Overlap with #555 and #558

None. #555 (zone services in Stormwind) and #558 (Notoriety drops only in Stormwind) stay
separate specs. Advancement already halves Notoriety (`world/advance.rs`), and nothing here
changes that. #556's "Tier 2 trainers in Stormwind districts" is replaced by hidden trainers;
only two of the ten stand in Stormwind.

## Left for later

- Tier 3: whether each Tier 2 Calling leads to one Tier 3 (Ironclad to Bastion, Zealot to
  Templar).
- Switching branch after the pick works as at 10: abandon the quest, and the other trainer still
  offers theirs once found.
- Placeholder art: each data ticket lists its NPC looks and icons in `docs/art-assets.md`.
- The lore bible (Davis's live doc) does not name the trainers yet.

## Readings Davis may want to amend

- Kick and utility get rows at Tier 2 (answer 4 read as "all eight skills"), which leaves two
  Talent rows.
- "Starts stronger" as the 1.1 x level-30 rule above.
- Learn levels 1, 1, 1, 1, 1, 10, 20, 30.
- Every trainer's spot, rating, name and hint chain in the table above.
- Healers are judged in the model by a new healing rule (ticket 21) on top of the sturdy pair's
  solo rules.
- The Nightblade's trainer sits past Captain Rook in the Riot: a Tier 1 character reaches him by
  clearing the wing with friends or by sneaking.
- Ticket 08 adds `RealmCommand::ResurrectResponse` and maps the client's existing command to it,
  a small change across the client/realm seam, written into the ticket so the builder does not
  stop on it.
- Ticket 20 gives each trainer its own copy of the two deliveries (twenty quests) rather than
  teaching a quest to have several givers.
- Until tickets 11 and 12 land, a level 30 Vanguard's popup lists Zealot alone, with the old
  two-handed kit (ticket 01 turns the popup on with the one Tier 2 Calling that exists).
