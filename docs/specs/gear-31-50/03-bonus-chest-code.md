Title: A bonus chest for a Phased Dungeon challenge (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 7. Blocked by DUNGEONS:03-partner-death-link (the 30-50 dungeons
spec's tickets 01 to 03 rework `src/phased.rs`, `src/shard.rs`, `content/ward_test.toml` and
`tests/phased_wards.rs`, which this ticket also edits).

Each of the four 30-50 dungeons gets one bonus chest, earned by an optional challenge, holding one
blue of the dungeon's set and coin (ticket 09 writes them). The Deeprun Tram's gold chest is the
model, but it is tram code (`src/tram.rs`, `Line::take_chests`, line 682), so this ticket adds a
key any Phased Dungeon file can carry. The tram's gold chest stays as it is.

## The key

One optional table per dungeon file, beside `[tram]` and `[key]`:

```toml
[bonus_chest]
boss = 974114          # the creature whose death places the chest
entry = 980722         # a run-time GameObject entry with no template, as the tram's chests
display = 259          # vanilla's small wooden chest, as the tram's chest_display
east_yd = 2.0          # it stands this far east (+x) of the boss's corpse, at the corpse's Z
money = 3800           # copper
one_of = [905160, 905161]   # it holds one of these, drawn evenly
found_text = "..."     # optional: shown to every character in the Phase when it appears
challenge = { kind = "unused", levers = [983041, 983042, 983043] }
```

Three challenge kinds, judged once, at the moment the chest would appear:

- `{ kind = "within_s", seconds = N }`: `boss` dies with the Run's `elapsed` (`Run`, line 1247 of
  `src/phased.rs`) at most N.
- `{ kind = "unused", levers = [...] }`: `boss` dies with none of the levers pulled in the Run
  (`Run::used`).
- `{ kind = "together_s", guids = [...], seconds = N }`: judged when the last creature of `guids`
  dies (`boss` must be one of them); it holds when the first and the last of those deaths are at
  most N seconds apart on the Run's clock.

When the challenge fails, no chest comes and nothing is said. A Run judges its chest once; a Run
that resets starts fresh.

## Code

- `src/phased.rs`: `Dungeon` (line 621) gains `#[serde(default)] pub(crate) bonus_chest:
  Option<BonusChest>`, with `BonusChest` and a `Challenge` enum (`#[serde(tag = "kind",
  rename_all = "snake_case")]`). A pure function `Challenge::holds(&self, deaths: &[(u32, f32)],
  used: &[u32], boss_died_at: f32) -> bool` (or close to it) carries the rule, so the unit tests
  need no Run. A content check beside `check_levers` (line 469, called at line 1166) panics, naming
  the dungeon, when `boss` or a `together_s` guid is no creature of the dungeon, when an `unused`
  lever is no lever of it, when `boss` is missing from `guids`, when `one_of` is empty, or when
  an item of `one_of` is no item (`Roll::check`'s test, `src/rules/loot.rs` line 60).
- `Run` (line 1245) records the death time (`elapsed`) of each creature the bonus chest names,
  and whether the chest was judged. `Run.dead` today records only creatures with an `appears`
  condition (`src/shard.rs`, `run_changes`, line 237); add the chest's creatures there or in a
  list of their own.
- `src/lib.rs`: on the tick after the judging creature dies with the challenge met, make the chest
  for every character in the Phase, as `tick_trams` makes the tram's chests (line 1746:
  `ObjectSpec` with `ObjectKind::Chest { items, money }`, `Audience::of` the Phase's characters,
  `self.objects.spawn_in`). Send `found_text`, when set, to each of them as a
  `SessionEvent::Notification`, as a lever's `used_text` is sent (line 2711). Keep the chest's
  guid with the shard, and remove it when the shard resets its Run, beside the tram's chests
  (line 1405).
- `GLOSSARY.md`: a **Bonus Chest** entry after **Run**: "A chest a Phased Dungeon leaves when its
  boss dies and the Run met the dungeon's optional challenge; it holds one blue of the dungeon's
  set and coin. Spec #PARENT." with `_Avoid_: gold chest (the Deeprun Tram's own)`.

## The Ward Test's chest

`content/ward_test.toml` gains a `[bonus_chest]` so the mechanic has a test where the shipped
dungeons have none yet: `boss = 800004` (the Brother whose timed call stops on lever 810005),
`entry = 980720`, `display = 259`, `east_yd = 2.0`, `money = 100`, `one_of = [905040, 905041]`
(two Hogger's camp blues), `found_text = "A strongbox lies where the Brother fell."`,
`challenge = { kind = "unused", levers = [810005] }`. Add a comment above it naming this ticket.

## Failing tests first

- `src/phased.rs` test module: `within_s` holds at N and fails at N + 1; `unused` fails when one
  listed lever is used and holds when only an unlisted one is; `together_s` holds for deaths N
  apart and fails for N + 1, and is not judged while a guid of it lives. The content check panics
  on each bad file above (the `with_lever` helper, line 1431, builds them).
- `tests/phased_wards.rs`, with its own harness (`kill`, `free_the_lever`, line 388):
  - a Run where the caller (800004) dies with the lever never pulled makes one chest of entry
    980720 about 2 yd east of its corpse, shown to the characters in the Phase, holding 100 copper
    and one of 905040 and 905041 (read it the way `tests/deeprun_tram.rs` reads the gold chest,
    `chests_made` at line 1673 and the loot response after it), and each character sees the found
    text once;
  - a Run where the lever is pulled first, then the caller dies, makes no chest;
  - a Run that resets removes the chest, and the next Run can earn it again.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `python3.15 -I
  tools/twilights-eve/ground_check.py` flags nothing; `python3.15 -I
  tools/twilights-eve/icon_check.py` is unchanged.

## Assumptions allowed

- Where the death times are kept, and whether the chest appears on the death's tick or the next.
- If a character joins the Phase after the chest appears, whether they see it: follow what the
  tram's chests do and record it.
