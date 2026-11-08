# Tier 2 Callings, levels 30 to 50: grill prep

Draft for the grill of #556, written 2026-11-07. It covers which Tier 2 Callings exist, what
each one plays like, how a level 30 character advances, and where the trainers stand in
Stormwind. Every guess is marked "Assumed:". Nothing here is decided until Davis answers the
grill.

## 1. What exists today

### What a Tier 1 Calling is in data

- **`data/twilights-eve/callings.csv`**: one row per Calling with its power, `stat_profile`
  (the vanilla Class, which sets the power bar), health and mana percent, armour, weapons and
  the Calling Choice `blurb`. All five Tier 1 rows are there, plus a stub `zealot` row with no
  armour and no blurb.
- **`data/twilights-eve/master.csv`**, read with `tools/twilights-eve/sheet.py`:
  - a `calling` row: `tier`, `tier_cap`, `tier_bonus`, `shapeshift_form`, `row_levels`
    (`4|8|12|16|20|24` for every Tier 1 Calling);
  - seven or eight `skill` rows: five or six skills learned at level 1, a Kick at level 10 and
    a utility at level 20 (ADR 0020, spec #334). Ranger has eight plus Auto Shot;
  - `aura` rows for everything that lasts (Conviction, Bloodlust, Blood Droplet, Searing
    Ember, Creeping Blight);
  - 18 `talent` rows: six Talent Rows of three, each a Skill Choice (`variant_of`) or a
    Talent (`row_key`). The Kick and the utility never get a row (ADR 0028).
- **Tier Bonus**, the whole bonus for the current Calling (the realm does not add Tier 1's to
  Tier 2's, `rules/stats.rs`). About 30 to 35 points at Tier 1: Vanguard 12/6/12/0/3,
  Cutthroat 6/12/12/0/3, Initiate 0/0/5/15/10, Ranger 0/15/10/5/0, Acolyte 0/0/12/8/15
  (str/agi/sta/int/spi).
- **Power Level** (ADR 0012): a Tier 2 character's real levels 1 to 50 map onto Power Level
  30 to 50. Formulas use `L`, the level within the Tier. Mobs from level 31 take the Tier Step,
  1.5 health and 1.5 damage, "a placeholder until Tier 2 Callings exist to fit it". The XP
  table above 30 is also provisional (`content/levels.toml`). Tier XP Rate is x5 at Tier 2.
- **Generic effects the realm runs** (`crates/twilight-realm/src/callings.rs`, `Effect`):
  weapon strike, school damage, periodic damage and heal, heal, drain, interrupt, taunt,
  knockback, root, stun, incapacitate, movement and attack speed slows, absorb, ground area,
  blink, charge, cast skill, consume aura and its stacks, grow aura, add aura stacks, combo
  points, energize, reset cooldowns, refund cost, stealth, seal, judgement, execute, sweeping
  strikes, block, parry and dodge percent, block value percent, max health percent, damage
  percent, healing reduction. The sheet vocabulary is in
  `docs/agents/calling-script-guide.md`.

### The Tier 1 kits, in one line each

| Line (Class, power) | Armour | Kit | Identity |
|---|---|---|---|
| Vanguard (Warrior, rage, Battle Stance) | cloth to plate, shield | Righteous Smash, Shield of the Penitent, Zealous Cleave, Bulwark Charge (leaves Consecration), Seal of Iron; Rebuke, Bulwark Taunt | holy plate tank: blocks, builds Conviction, spends it on Cleave |
| Cutthroat (Rogue, energy) | cloth, leather | Sinister Strike, Eviscerate, Rupture, Garrote, Evasion, Stealth; Kick, Sap | bleeds spill Blood Droplets; walking over them stacks Bloodlust |
| Initiate (Mage, mana) | cloth | Ley-Spark, Searing Ember, Biting Cold, Catalyst, Blink, Ley Ward; Counterspell, Ember Blast | fire and frost on the target, Catalyst cashes them in |
| Ranger (Hunter, mana) | leather, mail | Auto Shot, Snapshot, Puncturing Shaft, Barbed Arrow, Vault, Rapid Fire, Field Dressing, Explosive Shot; Silencing Shot, Snare Trap | bow, distance, mobile filler and planted heavy shots |
| Acolyte (Priest, mana and health) | cloth | Creeping Blight, Blood Lance, Dark Mending, Miasma, Siphon Life; Hush, Grasping Shadows | growing shadow DoT, drains, heals paid in health |

### Tier 2 and Stormwind today

- **Zealot** is the only Tier 2 Calling with a kit. Its kit, Tier Bonus (+28/14/28/0/7, a
  placeholder) and armour live in `crates/twilight-realm/content/callings.toml`, not in the
  sheet. Six skills: Seal of Command, Seal of the Crusader, Judgement, Hammer of Justice
  (borrowed Paladin rows, ADR 0003), Mortal Strike and Charge. The last two copy the old
  Vanguard, which no longer has them since the redesign (#95). No Talent Rows (ADR 0028).
- **Ironclad and Templar** exist only as names in Advancement Quests that never complete:
  64002 "The Ironclad's Oath" (Vanguard to Ironclad) and 64004 "The Templar's Charge" (Zealot
  to Templar). 64003 "The Zealot's Vow" works. All three are given by **Commander Aldous Wray**
  in the Stormwind barracks (`content/hub.toml` guid 9300034, at -8687, 328, beside the stock
  warrior trainers). A comment in `content/quests.toml` still says "Tier 2 stays at the
  Crossroads"; it is stale.
- **GLOSSARY.md** names the Vanguard Line as Vanguard, Ironclad, Zealot, Bastion, Templar.
  Lyria Du Lac in Goldshire is a "Veteran of the Ironclads" and points to the barracks.
- **The Calling Choice** (#414, `crates/twilight-realm/src/calling_choice.rs`,
  `crates/benilla-app/src/calling_choice.rs`) opens only for a Tier 0 Calling at its Tier Cap.
  `calling_chosen` is one saved bool, so a second popup needs a per-Tier record.
- **Calling Rumours**: `rumours.toml` accepts `kind = "calling"` with
  `calling = { from, to }`, no timer, and a last quest handed in to the trainer of `to`
  (`rumours.rs`). No Calling Rumour row exists yet.
- **Advancement already halves Notoriety** (`world/advance.rs` calls
  `rules::notoriety::advanced`, which floors half). #556's "Advancement halves Notoriety"
  needs no realm work. #558 (Notoriety drops only in Stormwind) is a separate spec.
- **Stock Stormwind class trainers** still stand at their vanilla spots
  (`content/stormwind.toml`): warriors in the Old Town barracks, rogues at SI:7 and the Pig
  and Whistle in Old Town, mages in the Wizard's Sanctum in the Mage Quarter, hunters in the
  Dwarven District (-8413, 548), one priest at -8728, 1103 (Assumed: the Park). The Cathedral
  priests and paladins were deleted with the Cathedral of Light's dungeon footprint
  (`content/zones.toml`).
- **Tier 2 content** the kits will fight: Redridge and Duskwood stretched to 30 to 50, the
  Stockade (30 to 34, 36 to 40), Mistmantle Manor (32 to 36), Tower of Ilgalar (34 to 38), the
  Deeprun Tram (38 to 42), Raven Hill Deeps (40 to 44), Stonewatch Keep (45 to 48), the
  Cathedral (48 to 50). Duskwood is full of undead.
- **The balance model** (`tools/twilights-eve/balance/`, rules in `docs/balance/rules.md`)
  covers levels 1 to 30 only. `gear.csv` has rows up to item level 42.

## 2. Proposed roster

Two Tier 2 Callings per Line, ten in all. In each Line one branch sharpens the Tier 1 role and
one shifts it, so a group of friends can find a tank, a healer and control. Every Calling on a
Line keeps the Line's Class and power bar (GLOSSARY, Class). Skills borrowed from another
class follow ADR 0003. "Carries" lists Tier 1 skills kept with the same data; each kit stays
at 8 skills or fewer.

Assumed: each kit carries the Line's Kick and utility, learned at level 1 instead of 10 and
20, because a Tier 2 character starts at Power Level 30.

### Vanguard line (Warrior, rage)

**Ironclad.** Holds the last line in plate behind a shield, and makes the pack hit them
instead of their friends. Role: tank. Armour: cloth to plate, shield.
- Carries: Righteous Smash, Shield of the Penitent, Bulwark Charge, Rebuke, Bulwark Taunt.
- New ideas:
  - Shield Slam: a strike for weapon damage plus 200% of Block Value that stuns for 2 sec
    (weapon strike, block value percent, stun).
  - Iron Wall: 8 sec of 40% less damage taken, long cooldown (the damage-taken aura that
    Consecrated Ground already uses).
  - Hold the Line: taunts every enemy within 10 yd and gives a stack of block chance per enemy
    taunted (taunt, add aura stacks, block percent).
  - Rallying Standard: a ground area where allies have 15% more max health for 15 sec
    (ground area, max health percent on allies).

**Zealot.** Fights with holy fury on the rage bar, two-handed and in plate, and is the dead's
worst enemy. Role: melee damage. Armour: cloth to plate, shield (as `callings.toml` has it).
- Carries: Zealous Cleave and Conviction, Bulwark Charge, Rebuke; keeps its own Seal of
  Command, Seal of the Crusader, Judgement and Hammer of Justice.
- Drops: Mortal Strike and Charge, which belong to the old Vanguard.
- New ideas:
  - Exorcism: a holy hit that deals double to undead (school damage; needs a "bonus against
    creature type" kind). Duskwood makes this matter.
  - Holy Wrath: holy damage to every enemy within 8 yd, stunning undead for 2 sec (school
    damage, self area, stun gated on creature type).
  - Zealous Judgement: judging a Seal grants 2 Conviction (add aura stacks on Judgement).

### Cutthroat line (Rogue, energy)

**Bloodletter.** Keeps every wound open and lives on the blood it spills. Role: melee damage,
sustained. Armour: cloth, leather.
- Carries: Sinister Strike, Rupture, Garrote, Evasion, Stealth, Kick.
- New ideas:
  - Hemorrhage: a strike that makes the target take 20% more damage from your bleeds for
    15 sec (weapon strike, a target debuff on bleed damage).
  - Open Veins: a finisher that makes every bleed on the target tick at once, per combo point
    (needs a "tick now" kind).
  - Blood Fan: up to five enemies in front bleed for 8 sec, each tick may spill a droplet
    (periodic damage, self area cone, the droplet proc).
  - Thirst (Talent idea): walking over a droplet also heals 2% of max health.

**Nightblade.** Strikes once from the dark, takes one foe out of the fight, and is gone.
Role: melee burst and control. Armour: cloth, leather.
- Carries: Sinister Strike, Eviscerate, Stealth, Sap, Kick.
- New ideas:
  - Ambush: from Stealth and from behind, 250% weapon damage (weapon strike, stealth and
    position gates).
  - Cheap Shot: from Stealth, stun 4 sec and 2 combo points (stun, add combo points).
  - Kidney Shot: a finisher that stuns 1 sec per combo point (stun with `per_combo`).
  - Vanish: enter Stealth in combat and drop all threat (needs a "drop threat" kind and
    stealth in combat).
  - Blind: incapacitate a foe for 10 sec at 10 yd (incapacitate).

### Initiate line (Mage, mana)

**Pyromancer.** Lets the fire spread and burns down the pack. Role: ranged damage, area.
Armour: cloth.
- Carries: Ley-Spark, Searing Ember, Catalyst, Blink, Counterspell, Ember Blast.
- New ideas:
  - Pyroblast: a 4 sec cast for a large Fire hit and a 12 sec burn (school damage, periodic
    damage).
  - Flamestrike: a ground area of Fire damage that leaves a short burn (ground area).
  - Combustion: each Fire spell adds 10% crit chance to your next one, spent on a crit
    (add aura stacks, crit chance per stack).
  - Spreading Flame (Talent idea): Catalyst on an Ember copies the Ember to two nearby foes.

**Frostbinder.** Freezes the field and breaks what stands still. Role: ranged control,
light support. Armour: cloth.
- Carries: Biting Cold, Catalyst, Ley Ward, Blink, Counterspell.
- New ideas:
  - Frost Nova: roots every enemy within 10 yd for 8 sec (root, self area).
  - Cone of Cold: Frost damage and a 50% slow in a cone (school damage, movement slow).
  - Ice Barrier: absorbs damage equal to 15% of max mana, castable on an ally (absorb on
    ally).
  - Polymorph: turns a foe into a sheep for 20 sec, broken by damage (incapacitate).

### Ranger line (Hunter, mana)

**Sharpshooter.** Plants their feet and drops the target before it arrives. Role: ranged
damage, burst. Armour: leather, mail.
- Carries: Auto Shot, Snapshot, Puncturing Shaft, Rapid Fire, Silencing Shot, Field Dressing.
- New ideas:
  - Aimed Shot: a 3 sec cast for heavy damage that halves healing on the target (weapon
    strike, healing reduction).
  - Hunter's Mark: the target takes 10% more ranged damage for 2 min (a target debuff).
  - Volley: arrows rain on a ground area for 6 sec, channelled (ground area, channel).
  - Steady Aim (Talent idea): each second standing still stacks 5% shot damage, up to 5
    (needs a "standing still" trigger).

**Outrider.** Fights on the move, lays traps behind them, and keeps a blade for close work.
Role: skirmisher, ranged with melee and control. Armour: leather, mail.
- Carries: Auto Shot, Snapshot, Barbed Arrow, Vault, Snare Trap, Silencing Shot.
- New ideas:
  - Wing Clip: a melee strike that slows 50% for 10 sec (weapon strike, movement slow).
  - Mongoose Bite: a melee strike usable only after a dodge (`caster_state=after_dodge`).
  - Frost Trap: a trap that becomes a slowing ground area (ground area, movement slow).
  - Scatter Shot: incapacitates a foe for 4 sec at range (incapacitate).
  - Deterrence: 10 sec of +50% dodge and parry (dodge and parry percent).

Assumed: no pet branch. Pets have no realm support, and the Tier 1 sheet guide leaves them
out of scope.

### Acolyte line (Priest, mana and health)

**Blightcaller.** Lets the Blight jump from one foe to the whole pack. Role: ranged damage,
damage over time. Armour: cloth.
- Carries: Creeping Blight, Blood Lance, Miasma, Siphon Life, Hush, Grasping Shadows.
- New ideas:
  - Contagion: when a foe with your Creeping Blight dies, its stacks jump to the nearest foe
    (a proc on the Blighted target's death; new trigger).
  - Mind Flay: a channel of Shadow damage that slows (channel, school damage, movement slow).
  - Plague Burst: consumes Creeping Blight's stacks for Shadow damage to every enemy within
    8 yd of the target (consume aura stacks, area around the target).
  - Devouring Plague: a DoT that heals you for all it deals (periodic damage, drain).

**Bloodmender.** Spends their own blood to keep friends standing, and drinks the foe's to
pay it back. Role: healer. Armour: cloth.
- Carries: Dark Mending, Blood Lance, Siphon Life, Creeping Blight, Hush.
- New ideas:
  - Transfusion: a channel that moves your health to an ally at 150% (channel, drain on self,
    heal on ally).
  - Blood Ward: an absorb on an ally paid in your health (absorb, health cost).
  - Circle of Blood: a ground area that heals allies inside every 2 sec (ground area, periodic
    heal).
  - Last Rites: brings a dead ally back at 30% health, out of combat (needs a resurrection
    kind).

Assumed: a healer must still solo, so the Bloodmender keeps the Line's drains and its
heals reach the caster.

## 3. The Advancement at 30

A Tier 1 character at level 30 advances in Stormwind, the way a Basic Warrior advances in
Northshire (#214, #414).

1. **The popup.** At the Tier 1 Cap the character gets the Calling Choice again. It lists
   only the two Tier 2 Callings of their Line, each with its icon, power, blurb, skill icons
   and its trainer's place. Assumed: same window, same protocol event and command; the realm
   lists the Callings that the current Calling's Advancement Quests lead to.
2. **The Calling Rumour.** A Calling Rumour per Tier 2 Calling (`kind = "calling"`) points the
   character to the city. Its giver stands in Elwynn and its last step hands in to the Tier 2
   trainer in Stormwind. Assumed: the pick in the popup decides which Calling Rumour drops
   (decision 8).
3. **The trainer.** Each Line has one trainer in its own district. The trainer offers the
   Advancement Quests of both branches, as Commander Wray does today. Handing one in advances
   the character: level 1, the new kit, every Talent Row empty, gear, gold and quests kept.
4. **Notoriety halves** on Advancement. That works today.

### Trainers, one per district

| Line | Trainer | District, spot | Why there |
|---|---|---|---|
| Vanguard | Commander Aldous Wray (exists) | Old Town, the barracks | the stock warrior trainers stand there |
| Cutthroat | Assumed: a new name, a contact of Jorik Vane | the Canals, near the Stockade's jail door | the underworld, the Defias riot, Notoriety |
| Initiate | Assumed: a new name, a Tower of Azora master of Elsa Brightwood | the Mage Quarter, the Wizard's Sanctum | the stock mage trainers stand there |
| Ranger | Assumed: Thalyssa Windbough's contact, or a dwarf hunter | the Dwarven District, by the stock hunter trainers | the tram tunnels and the Deeprun Tram dungeon start here |
| Acolyte | Assumed: a monk turned out like Brother Corwin | Cathedral Square, outside the Cathedral doors | the Cathedral is a 48 to 50 dungeon; an outcast at its door fits |

Assumed: each trainer is a Hub NPC like Wray (`hub.toml`, `map = 0`, quest giver only, no
training), with z from `wmo_floor.py` or `ground_check.py`. The Trade District stays for the
Writ Board and the Park stays free.

## 4. Open decisions, most important first

Each is one question for Davis, asked one at a time.

**1. How many Tier 2 Callings does each Line get?**
- a. Two each, ten in all, built Line by Line.
- b. Two for Vanguard now (Ironclad, Zealot), the other Lines later.
- c. One each now, a second later.
- Recommend a: the popup and the trainers need every Line to have a branch at 30, and two
  gives the choice the Tier is for.

**2. What makes the two branches of a Line different?**
- a. One sharpens the Tier 1 role, one shifts it (tank, healer, control).
- b. Both keep the Tier 1 role with different mechanics.
- c. Each branch is a new role, nothing like Tier 1.
- Recommend a: a group of friends needs a tank and a healer, and Tier 1 has neither a pure
  healer nor control.

**3. How much of the Tier 1 kit carries over?**
- a. About half: three or four core skills plus the Kick and the utility, and three or four
  new skills, 8 at most.
- b. All of it, plus two new skills (which breaks the 8 cap for Ranger and Cutthroat).
- c. None: a whole new kit.
- Recommend a: the player keeps what they learned, and each Calling still feels new.

**4. What Talent Rows does a carried skill get?**
- a. Its Tier 1 three choices, copied under the new Calling; new skills get new rows.
- b. Three new choices for every skill, carried or not.
- c. Carried skills get no row; only new skills and Talent rows do.
- Recommend a: half the row design is already done and played, and the build can copy rows.

**5. What are Tier 2's Row Levels?**
- a. Eight rows at 5, 10, 15, 20, 25, 30, 35, 40.
- b. Six rows at 5, 12, 19, 26, 33, 40.
- c. Ten rows, every 5 levels to 50.
- Recommend a: six skill rows plus two Talent rows fit an 8-skill kit, and the last ten
  levels go to the Advancement grind as at Tier 1.

**6. Vanguard line: are Ironclad (tank) and Zealot (holy melee damage) its branches?**
- a. Yes, as proposed.
- b. Yes, but Zealot keeps its current kit untouched.
- c. Rename or swap one.
- Recommend a: both names are already in the game and the glossary.

**7. Zealot's current kit: keep, rebuild or blend?**
- a. Blend: keep the Seals, Judgement and Hammer of Justice; replace Mortal Strike and Charge
  with Zealous Cleave, Conviction and Bulwark Charge.
- b. Keep it as built.
- c. Rebuild from the new Vanguard with no Paladin rows.
- Recommend a: the Seals work today, and the Vanguard player keeps Conviction instead of
  meeting the old Vanguard's skills again.

**8. How does the level 30 flow work?**
- a. The popup pick makes that branch's Calling Rumour drop on the next kill in Elwynn; its
  steps lead to the Stormwind trainer.
- b. The popup pick puts the Advancement Quest in the log, as at 10; no Rumour.
- c. No popup; each branch's Calling Rumour drops by chance at the Tier Cap.
- d. Both, independent: the popup gives the quest, and the Rumours also drop.
- Recommend a: it keeps #556's Rumour and the popup's clear prompt, and the player never waits
  on luck.

**9. What does a Calling Rumour ask before the trainer?**
- a. Two steps: a Field Giver in Elwynn sends the player to someone in Stormwind, who sends
  them to the trainer.
- b. One step: the Field Giver sends the player straight to the trainer.
- c. A short chain of three or four with a fight, like an ordinary hard Rumour.
- Recommend a: it shows the player the city on the way, with no fight to gate the
  Advancement.

**10. Cutthroat line: are Bloodletter (bleeds) and Nightblade (burst and control) its
branches?**
- a. Yes, as proposed.
- b. Rename one.
- c. Replace Nightblade with a poison branch.
- Recommend a: the bleeds and the stealth openers are both in the Tier 1 kit already.

**11. Initiate line: are Pyromancer (fire area) and Frostbinder (control) its branches?**
- a. Yes, as proposed.
- b. Replace Frostbinder with an arcane support branch (wards on allies, mana for the group).
- c. Rename one.
- Recommend a: Tier 1 is fire and frost with Catalyst between, so each branch keeps half.

**12. Ranger line: are Sharpshooter (planted burst) and Outrider (traps and skirmish) its
branches?**
- a. Yes, as proposed.
- b. Replace Outrider with a pet branch, and build pets.
- c. Rename one.
- Recommend a: pets are a large realm feature; the two halves of the Tier 1 kit already split
  this way.

**13. Acolyte line: are Blightcaller (spreading rot) and Bloodmender (healer) its branches?**
- a. Yes, as proposed.
- b. Make the healer a holy one with no health costs.
- c. Rename one.
- Recommend a: health as a cost is the Acolyte's mark, and the game has no healer yet.

**14. Where does each Line's trainer stand?**
- a. As in the table in section 3: Old Town, the Canals, the Mage Quarter, the Dwarven
  District, Cathedral Square.
- b. All five in Old Town's barracks yard beside Wray.
- c. The stock trainer spots for every Line, with the Acolyte in the Park.
- Recommend a: one district each sends the player across the city, which is what #556 asks.

**15. How big is a Tier 2 Tier Bonus?**
- a. Fit by the balance model at Power Levels 30, 40 and 50, together with the Tier Step.
- b. Double the Tier 1 bonus, then tune by play.
- c. Keep Zealot's +28/14/28/0/7 size for every Line.
- Recommend a: the Tier Step is a placeholder waiting for this number, and both should be set
  together.

**16. Does any Tier 2 Calling wear heavier armour than Tier 1?**
- a. No: each keeps its Line's Tier 1 armour.
- b. Yes, one step for the role (Ironclad plate only with a shield, Outrider and
  Sharpshooter mail).
- c. Davis picks per Calling.
- Recommend a: armour is part of a Line's look, and the gear rows to 50 assume it.

Lower priority, can wait for a later grill: whether each Tier 2 Calling branches into exactly
one Tier 3 (Ironclad to Bastion, Zealot to Templar); whether switching branch after the pick
works as at 10 (abandon the quest, the other trainer still offers theirs); and whether the
balance model needs a healer rule.

## 5. Ticket sketch

The order is the Advancement first, then one kit per Line, as Tier 1 went one Calling per spec.

### Data (sheet and content)

- `callings.csv`: rows for the nine new Callings, and armour and a blurb for Zealot.
- `master.csv` through `sheet.py add-row`: a `calling` row per Tier 2 Calling with
  `tier=2`, `tier_cap=50`, `tier_bonus`, `row_levels`; skill rows (carried skills copied,
  new skills); aura rows; talent rows. `status=placeholder`. Zealot's kit moves from
  `callings.toml` into the sheet.
- `content/quests.toml`: an Advancement Quest per Tier 2 Calling, from the Line's trainer;
  64002 becomes completable; Calling Rumour step quests and their notes in
  `quest_items.toml`.
- `content/rumours.toml`: a `kind = "calling"` Rumour per Tier 2 Calling, giver points in
  Elwynn.
- `content/hub.toml`: four new Stormwind trainers; z by `ground_check.py`; their spots on the
  Living Map as service marks.
- Icons through `icon_check.py`; placeholder art listed in `docs/art-assets.md`.

### Realm

- The Calling Choice at every Tier Cap: a per-Tier record in place of `calling_chosen`, and
  the options taken from the current Calling's Advancement Quests.
- The pick arms a Calling Rumour (if decision 8 goes that way): a guaranteed drop on the next
  kill.
- New generic kinds the kits need, one ticket each with tests: bonus against a creature type,
  drop threat and stealth in combat, tick bleeds now, a proc on the death of a debuffed target,
  standing-still stacks, resurrection, channels that heal an ally.
- Content tests: shared skills stay identical across Callings of a Line; every Tier 2 Calling
  has an Advancement Quest and a Calling Rumour.

### Client

- The Calling Choice window with two entries and a Tier 2 heading. Assumed: the layout copes,
  since it lists from the event.
- The Talents panel with eight rows instead of six. Assumed: the strips are built on demand
  in `assets/ui/Talents.xml`, but the frame height needs checking.
- Spell.dbc patch rows for borrowed spells (ADR 0003); no new protocol.

### Balance model

- Tier 2 cells: Power Levels 30 to 50, real levels 1 to 50, with the Tier Step and the Tier 2
  Tier Bonus.
- Greens and blues to 50 (needs the gear prep in `docs/specs/prep/gear-31-60.md`; `gear.csv`
  stops at 42).
- `docs/balance/rules.md` extended to Tier 2: the same P-2, P-3, DPS, E-dmg, E-tank, B-2E and
  B-boss rules by role, and the named bosses of the 30 to 50 dungeons.
- Assumed: the Bloodmender and the Frostbinder are judged on solo survival, not damage.
