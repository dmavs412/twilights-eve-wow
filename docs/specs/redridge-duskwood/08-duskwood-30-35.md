Title: Duskwood 30-35: Darkshire waits, the Night Watch, Tranquil Gardens and the silent house (#PARENT)
Labels: ready-for-agent

Part of #PARENT. Blocked by 03-quest-files, 05-gear-30-35 (the band's Quest Line rows) and 06-bands-30-35.

Eleven quests, 98601 to 98614 (seven story quests and four bounties), in
`crates/twilight-realm/content/quests_duskwood.toml` under `# ── Duskwood 30-35 (#PARENT) ──`,
their four quest items in `content/quest_items_duskwood.toml`, and one item that ticket 26's
Mistmantle quest collects later (980481).

## How to write the rows

As ticket 07-redridge-30-35 says, with `zone_or_sort = 10` (Duskwood). In short: the schema in
`content/quests.toml`'s header; talk rows as `quests_elwynn.toml` line 202, collect rows as line
113, explore as line 1170, bounties as 94105, `gives` as 97301. Quoted text is the contract;
beats are written in the voice of the Stockade's quests, 60 to 110 words of details, one line of
progress, 25 to 60 words of completion, no em dashes, straight quotes. Never name Sareth, Cedric,
Prestor, Deathwing or the Lich King. **No "Anchor" in Duskwood at these levels**: the manor's
secret is "the stone" at most. Money: 35 x level with an objective, 0 for a delivery or talk, 30
x level for a bounty. Quest items: `class = 12`, `quality = 1`, `item_level` the quest's level,
`max_count = 1` or `stackable = 20`, a `description`, a vanilla `display_id`, an icon
`icon_check.py` accepts.

## Story quests

**98601 Darkshire Waits.** Level 30. Giver and ender Lord Ello Ebonlocke (263). No prerequisite.
`talk` to Commander Althea Ebonlocke (264), Clerk Daltry (267) and Madame Eva (265). Money 0.
Objectives: "Speak with Commander Althea Ebonlocke, Clerk Daltry and Madame Eva in Darkshire, then return to Lord Ello Ebonlocke."
Details: "I write to Stormwind on the first of every month. Twelve letters this year. Not one answer. Not a refusal, not a rebuke, nothing. Either the roads eat my riders or the city has decided Darkshire is already lost. You came, at least. Before you go off killing things, learn the town. My daughter runs the Night Watch. Daltry keeps our records. Madame Eva keeps everything else, whether we like it or not."
Althea, option "What does the Night Watch need?": "Three more men, and the north road clear. The wolves come right to the gate now, and the spiders take whatever the wolves leave. I lose a man a month out there."
Daltry, option "What do the records say?": "That the dark came twelve years ago, in one night, and never lifted. Before that this was Brightwood, with farms and sun like anywhere. I have the Watch's log for that night. Nobody has ever asked to read it."
Eva, option "What do you see, Madame Eva?": "A house on the hill north of town where the candles have burned for twelve years and never once gone out. Nobody lights them. Nobody lives there. Ask yourself what kind of air lets a flame stand that still."
Progress: "Well?"
Completion: "Wolves at the gate, a night that never ended and a house with its candles lit. You've met Darkshire. Help my daughter first. She'll ask you anyway."

**98602 The Night Watch.** Level 31. Giver and ender Althea (264). No prerequisite. Kill 10
Starving Dire Wolves (213) and 10 Pygmy Venom Web Spiders (539) along the north road. Money 1085.
Beat: the Night Watch is three men short and the wolves come to the gate. No thread.

**98603 The Gardens Rose.** Level 33. Giver and ender Althea, `after = 98602`. Kill 10 Skeletal
Warriors (48) and 6 Skeletal Mages (203) in Tranquil Gardens Cemetery. Money 1155.
Beat (details): the cemetery south of town is the Watch's other front; the dead walk its paths
every night.
Completion: "The gardens' dead got up the same night the dark came. Twelve years, and nobody has ever told me why the two go together. I only know that they do."

**98604 A Clerk's Records.** Level 32. Giver Daltry (267), ender Watcher Backus (840, on the
road north of town), `after = 98601`. `gives` the Night Watch Log (980401); collect it. Money 0.
Beat: Backus stood on the north gate the night the dark came. Daltry wants him to read the log
and say what isn't written in it. Backus takes it like something that bites.

**98605 Candles That Don't Go Out.** Level 33. Giver and ender Backus, `after = 98604`. Kill 8
Fetid Corpses (1270) in the yard of the house on the hill. Money 1155.
Beat (details): the yard's dead walk at the foot of the house; Backus wants them put down so
someone can get near it.
Completion: "Farmhands. I knew two of them. They worked the house's fields the day before the dark, and they were lying in that yard the morning after, and then one night they weren't lying down any more. The candles in the windows were lit that morning too. Nobody has ever gone in to blow them out."

**98606 The Tutor's Name.** Level 34. Giver Backus, ender Sirra Von'Indi (268), `after = 98605`.
`gives` Backus's Note (980402); collect it. Money 0.
Beat (details): Backus sends the character to the town's historian, who knew the families.
Completion: "The Mistmantle house. A family, six servants, and a tutor for the girl: Stalvan, a clever young man from the city. On the night the dark came the whole house stopped. No screams, no fire. In the morning the doors were barred from inside and the candles were lit, and nobody would go in. I've spent twelve years wondering why that house, of all houses."

**98607 The Silent House.** Level 35. Giver and ender Madame Eva (265), `after = 98606`. Explore
`{ place = "The door of the house on the hill", position = [-10362.6, -1257.0], radius = 10.0, map = 0 }`
(Mistmantle Manor's portal, `mistmantle_manor.toml`). Money 1225.
Objectives: "Stand at the door of the house on the hill north of Darkshire, then return to Madame Eva."
Details: "Sirra has told you about the house. I can tell you the rest. The door isn't barred any more. It opened a week ago, on its own, and something inside has been waiting for someone to notice. Go and stand at the door. Listen. When you hear the quiet, come back to me before you go in."
Progress: "Did you listen?"
Completion: "You heard it. No wind, no crickets, not even your own feet on the step. That house is holding its breath, and it has held it for twelve years. When you go in, take friends."

## Bounties (`kind = "bounty"`)

| Id | Giver | Level | Objective | Money |
|---|---|---|---|---|
| 98611 | Chef Grual (272) | 31 | Collect 8 Gooey Spider Legs (980403) from Venom Web Spiders (217) and Pygmy Venom Web Spiders (539) at 50% | 930 |
| 98612 | Watcher Callahan (2142) | 31 | Kill 10 Rabid Dire Wolves (565) | 930 |
| 98613 | Councilman Millstipe (270) | 32 | Kill 10 Venom Web Spiders (217) | 960 |
| 98614 | Blind Mary (302) | 33 | Collect 8 Bone Dust (980404) from Skeletal Warriors (48) and Skeletal Mages (203) at 50% | 990 |

Each bounty's text: a small local need, no thread.

## Quest items (`quest_items_duskwood.toml`)

| Entry | Name | Form | Description or pages |
|---|---|---|---|
| 980401 | The Night Watch Log | single, given by 98604 | "The Watch's log for the night the dark came. The last entry is in a shaking hand." |
| 980402 | Backus's Note | single, given by 98606 | "For Sirra. 'The Mistmantle house. Tell them.'" |
| 980403 | Gooey Spider Leg | stack of 20 | "Grual swears it tastes like crab." |
| 980404 | Bone Dust | stack of 20 | "Mary says it keeps the dead from the door. Mary says a lot of things." |
| 980481 | Surveyor's Site Record | single, readable, two pages; collected by 99001 (ticket 26) | page one: "Brightwood. The stone went under the cellar at the dark of the moon, untuned, as ordered. Result: night over the whole wood within the hour. The dead in two graveyards disturbed. The household lost. Recommend no stone be set again without the engineer." page two: "Next sites as surveyed: the hill of graves to the west, and the tower in Redridge. Below it, a hammer over a black sun." |

980481 is defined here so ticket 26 touches no item file. No quest collects it until then.

## Gear choices (decision 13 as amended)

Every story quest offers a choice of greens from its band's Quest Lines (ticket 05 made the
Lines; the band's gear ticket wrote these rows). Add the `choices` line the table gives to each
quest's row. The slots follow #448 decision 9's cycle in quest-id order across the whole band, all
tickets together (main hand, ring, Chest, amulet, off hand, ring, main hand, ring, and again).
Bounties pay money and offer no choice.

| Quest | Slot | `choices` | Rows from |
|---|---|---|---|
| 98601 | ring, the a Lines | `[903214, 903216, 903218]` | 05-gear-30-35 |
| 98602 | main hand: sword, dagger, mace | `[903204, 903206, 903209]` | 05-gear-30-35 |
| 98603 | ring, the b Lines | `[903215, 903217, 903219]` | 05-gear-30-35 |
| 98604 | main hand: sword, dagger, staff | `[903204, 903206, 903208]` | 05-gear-30-35 |
| 98605 | ring, the a Lines | `[903214, 903216, 903218]` | 05-gear-30-35 |
| 98606 | Chest, one per armour type | `[903200, 903201, 903202, 903203]` | 05-gear-30-35 |
| 98607 | amulet | `[903211, 903212, 903213]` | 05-gear-30-35 |

## Failing tests first

New file `crates/twilight-realm/tests/duskwood_30_50.rs` (the later Duskwood tickets extend it),
modelled on `tests/goldshire.rs`:

- All eleven quests load with their givers, enders and `zone_or_sort = 10`.
- The chains hold: 98603 after 98602; 98604 after 98601; 98605 after 98604; 98606 after 98605;
  98607 after 98606. 98601 and 98602 have no prerequisite.
- Speaking with Althea, Daltry and Eva credits 98601.
- 98604's and 98606's accepts put their item in the bag, and the hand-in takes it.
- Standing at the manor's door credits 98607.
- Item 980481 exists with two pages.
- Every story quest in the gear table offers exactly the `choices` it names.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `icon_check.py` reports no new clash;
  `ground_check.py` flags nothing.

Assumptions allowed: item display ids and icons, and wording inside a beat.
