Title: Elite rolls in Redridge and Duskwood: eighteen named creatures roll their band's Elite pool and greens (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 10. Blocked by 02-zone-rolls-code (the `[[zone.rolls]]` table), 05-band-gear-41-50
(the Elite blues and greens it names) and RD:21-bands-45-50 (the arc's last band ticket: it
settles every Redridge and Duskwood spawn's level, adds the Grey Captain and the Twilight
Courier, and is the last arc ticket to edit `content/zones.toml`).

Elwynn's named elites roll 20% one blue of their band's Elite pool, else 50% one green of the
band (Princess, `content/mobs.toml` line 945). This ticket gives Redridge's and Duskwood's named
creatures the same, through the table ticket 02 built.

## The pools

| Band | Elite pool (20%) | Greens (50%) |
|---|---|---|
| 31-35 | 904024-904029 | 902100-902119 |
| 36-40 | 904030-904035 | 902120-902139 |
| 41-45 | 904036-904041 | 902140-902159 |
| 46-50, and above 50 | 904042-904047 | 902160-902179 |

Each row is `rolls = [{ chance = 20, one_of = [<six>] }, { chance = 50, one_of = [<twenty>] }]`,
written out in full, as Princess's is.

## The creatures

Each takes the band holding its **lowest open-world spawn level** as the realm loads it, after the
arc's band files and `level_overrides.toml` have applied. Read that level from the loaded content
(the test below computes it); the "level today" column is the Level Stretch's value before the
arc's band tickets and is only a guide. Rows go under the creature's zone's `[[zone]]` in
`content/zones.toml`, each with its name in a comment.

| Entry | Name | Zone | Rank | Level today |
|---|---|---|---|---|
| 486 | Tharil'zun | Redridge | elite | 45 |
| 584 | Kazon | Redridge | rare | 50 |
| 616 | Chatter | Redridge | rare | 44 |
| 947 | Rohh the Silent | Redridge | rare | 48 |
| 14269 | Seeker Aqualon | Redridge | rare | 41 |
| 14270 | Squiddic | Redridge | rare | 38 |
| 14271 | Ribchaser | Redridge | rare | 35 |
| 14272 | Snarlflare | Redridge | rare | 36 |
| 14273 | Boulderheart | Redridge | rare | 47 |
| 14357 | Lake Thresher | Redridge | elite | 47 (the arc's 06 puts it in 32-35) |
| 96054 | Twilight Courier | Redridge (`mobs.toml`, placed by the arc's 21) | elite | 48 |
| 507 | Fenros | Duskwood | rare | 46 |
| 521 | Lupos | Duskwood | rare | 36 |
| 522 | Mor'Ladim | Duskwood | elite | 50 |
| 534 | Nefaru | Duskwood | rare | 49 |
| 574 | Naraxis | Duskwood | rare | 41 (the arc's 06 puts it in 31-33) |
| 1200 | Morbent Fel | Duskwood | elite | 46 |
| 96053 | The Grey Captain | Duskwood (`mobs.toml`, placed by the arc's 21) | elite | 47 |

- These are every named elite (rank 1) and rare (rank 4) template with an open-world spawn in
  the two zones, and the arc's two new named elites. Gath'Ilzogg, Singe, Morganth, Lord
  Malathrom and Commander Felstrom have no open-world spawn (their spawns sit in Phased Dungeon
  footprints and `zones.toml` removes them).
- The Blackrock Scouts, Sentries, Hunters and Shadowcasters are elite packs, not named creatures.
  They roll the Loot Band only, as Elwynn's packs do.
- Stormwind's Sewer Beasts (3581) stay out: Stormwind has no band.
- None of these templates carries `rolls` today; ticket 02's loader panics if one does.

## Failing tests first

In `src/mobs.rs`'s test module (it can read `content()` and the spawn list):

- for each of the 18 entries, take the lowest `level_min` among its open-world spawns, find the
  band with `crate::rules::loot::band_of`, and assert the template's rolls are exactly
  `[{ 20, that band's six Elite blues }, { 50, that band's twenty greens }]` (the Elite ranges by
  band are the table's; the greens are `band_of(level).green_rows()`);
- no other Redridge, Duskwood or Stormwind template has rolls from 902100-902179 or
  904024-904047.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `python3.15 -I
  tools/twilights-eve/ground_check.py` flags nothing; `python3.15 -I
  tools/twilights-eve/icon_check.py` is unchanged.

## Assumptions allowed

- An entry with no open-world spawn when this runs (the arc moved or removed it) is left out and
  recorded.
- A named creature the arc added that the table misses, if the session finds one of rank 1 or 4
  with an open-world spawn in the two zones: add it with its band and record it.
