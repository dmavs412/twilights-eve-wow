Title: Each Tier 2 trainer sends the character on to Lakeshire and Darkshire, in place of Commander Wray (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 12 ("after advancing, the trainer sends the character on to Redridge
and Duskwood"). Blocked by RD:09-stormwind-30-35 (the Redridge and Duskwood arc's ticket
`docs/specs/redridge-duskwood/09-stormwind-30-35.md`, which writes 97401 and 97402) and
19-bloodmender (the last of the ten trainers).

RD:09 has Commander Aldous Wray give two deliveries to a fresh Tier 2 character: 97401 "Orders
for Lakeshire" (to Magistrate Solomon, 344) and 97402 "Orders for Darkshire" (to Lord Ello
Ebonlocke, 263). With hidden trainers, the trainer who advanced the character gives them instead.
A quest has one giver, so each trainer gets its own copy of the pair, offered only to a character
it advanced.

## The change

In `crates/twilight-realm/content/quests_stormwind.toml`, under RD:09's
`# ── Redridge and Duskwood: the way in ──` header:

- Delete 97401 and 97402. Keep 97403 to 97405 (the poster and Solomon's request) as they are.
- Add the twenty quests below. Each Lakeshire copy is 97401 with: the giver below, `after = <the
  trainer's Advancement Quest>`, `zone_or_sort = 44`, the details below, and RD:09's `gives`,
  `collect` and `choices` for 97401 (`[903204, 903206, 903208]`). Each Darkshire copy is 97402 the
  same way, with `zone_or_sort = 10` and 97402's `choices` (`[903214, 903216, 903218]`). Both stay
  level 30, so only a Tier 2 character is offered them (the `quests.toml` header's Tier rule).
- Shared text. Lakeshire: Objectives "Carry the sealed orders to Magistrate Solomon in Lakeshire,
  in Redridge." Progress "Sent to me?" Completion "Another fighter instead of a company. I'll take
  it. Sit down, and I'll tell you what the company would have done." Darkshire: Objectives "Carry
  the sealed letter to Lord Ello Ebonlocke in Darkshire, in Duskwood." Progress "From someone who
  cares? Truly?" Completion "Someone sent you who isn't allowed to help, I expect. It's more than
  the Keep has sent me in a year. Welcome to Darkshire."
- `content/quest_items.toml`: rename 980151 to "Sealed Orders for Lakeshire" ("Sealed. For
  Magistrate Solomon, Lakeshire.") and 980152 to "Sealed Letter for Darkshire" ("Sealed. For Lord
  Ebonlocke, Darkshire."). Wray no longer seals them.

| Ids | Giver | After | Lakeshire details | Darkshire details |
|---|---|---|---|---|
| 97421, 97422 | Sergeant Maera Holt (190110) | 64002 | "You hold a line now. Go and hold Lakeshire's. Magistrate Solomon has begged Stormwind for a company three times. Take him these orders, and take him yourself." | "Darkshire has held its own line for years with nobody's help. Take Lord Ebonlocke this letter, south over the river, and tell him an Ironclad sent you." |
| 97423, 97424 | Brother-Knight Aldren Vale (190111) | 64003 | "The dead are not the only thing the Light has against it. Lakeshire has orcs. Take Magistrate Solomon these orders, east past Three Corners." | "Lord Ebonlocke keeps a town lit against the dark, and I keep his graves. Take him this. He'll know my hand." |
| 97425, 97426 | Sabine Redmark (190112) | 64401 | "Solomon in Lakeshire wants soldiers. He'll get you. Take him this; he won't ask what's on your hands." | "Darkshire needs anyone who can make a thing bleed. Take this south to Lord Ebonlocke." |
| 97427, 97428 | Silas Dunmere (97290) | 64402 | "You know the way out. Keep going, east to Lakeshire. Solomon gets this. Nobody else sees it." | "And this one goes south, to Ebonlocke in Darkshire. Be gone before anyone asks who sent it." |
| 97429, 97430 | Magus Ilyra Vance (190114) | 64403 | "Lakeshire is burning slowly, and nobody in this city will put it out. Take Magistrate Solomon these orders, east through Elwynn." | "Darkshire has had no light but its own for a long time. Take Lord Ebonlocke this letter, and some fire." |
| 97431, 97432 | Teodric Lumen (190115) | 64404 | "The ley runs east too, under a lake full of trouble. Take this to the magistrate at Lakeshire." | "And this to Lord Ebonlocke in Darkshire, just south of this bank. Tell him the river is listening." |
| 97433, 97434 | Wren Ashcombe (190116) | 64405 | "Lakeshire is down that road. I've kept it clear this far. Take Solomon these orders and keep it clear the rest." | "Darkshire's south and west, past the river. Lord Ebonlocke needs eyes that don't blink. Take him this." |
| 97435, 97436 | Hale Brannock (190117) | 64406 | "East into Redridge, and don't stop moving. Solomon in Lakeshire gets these orders." | "South into Duskwood next. Lord Ebonlocke in Darkshire. Lay a trap or two on the way." |
| 97437, 97438 | Mother Ysolde Grey (190118) | 64407 | "The magistrate in Lakeshire has more wounded than healers. Take him this, and take your rot." | "Lord Ebonlocke is just down the hill. Take him this letter. Tell him the garden says hello." |
| 97439, 97440 | Sister Elowen Marr (190119) | 64408 | "Lakeshire bleeds and nobody mends it. Take Magistrate Solomon these orders, and mend what you find." | "Darkshire has been bleeding for years. Take Lord Ebonlocke this, south over the river." |

## Where

- `crates/twilight-realm/content/quests_stormwind.toml`: 97401 and 97402 as RD:09 wrote them.
- `crates/twilight-realm/content/quest_items.toml`: 980151 and 980152.
- `crates/twilight-realm/tests/stormwind_30_50.rs` (RD:09's file): its "Wray offers 97401 and
  97402 to a level 30 Zealot" test goes; the gear table test drops 97401 and 97402 and checks the
  copies' `choices` instead.
- If another landed ticket reads 97401 or 97402 (search the repo), point it at the copies and
  record it.

## Failing tests first

In `crates/twilight-realm/tests/stormwind_30_50.rs`:

- A character advanced by Sergeant Holt (64002 done) is offered 97421 and 97422 by her and none of
  the other eighteen copies; Wray offers none.
- A level 30 Tier 1 Vanguard is offered neither (level 30 is Tier 2's).
- Accepting 97421 puts Sealed Orders for Lakeshire in the bag, and Solomon's hand-in takes it.
- Every copy's `choices` is its original's.
- 97401 and 97402 no longer exist.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `ground_check.py` flags nothing;
  `icon_check.py` reports no new clash.

Assumptions allowed: the details' wording, `zone_or_sort` for the copies.
