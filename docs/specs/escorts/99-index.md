# Escort quests: index of the spec files

The parent (00), which replaces the body of #596, and 4 tickets (01 to 04) labelled
`ready-for-agent`. Paths are under `crates/twilight-realm/` unless they start with `docs/` or
`tools/`. "Blocked by" names a file here.

| File | Title | Blocked by | Edits | Runs alongside |
|---|---|---|---|---|
| 00-parent | Escort quests (spec) | | | |
| 01-per-viewer-sight | Per-viewer sight: creatures and Hub NPCs that only some characters see | | `src/shard.rs`, `src/hub.rs`, `src/mobs.rs`, `src/lib.rs`, `src/world/phase.rs`, `src/quests/dialogs.rs`, `src/vendor.rs`, `src/inn.rs`, `src/map_marks.rs`, `content/hub.toml` (header, one test row), `content/zones.toml` (header, one test row), new `tests/sight.rs` | 02 |
| 02-creatures-fight-creatures | Defenders: a friendly creature that hostile creatures fight, that fights back and that players heal | | `src/world.rs`, `src/world/cast.rs`, `src/combat.rs`, `src/mobs.rs`, `src/shard.rs`, `src/lib.rs`, new `tests/defenders.rs` | 01 |
| 03-escort-walker | The escort walker: a private escortee walks a path for his group | 01, 02 | new `src/escorts.rs`, new `content/escorts.toml`, `src/lib.rs`, `src/build_id.rs`, `src/quests/mod.rs`, `src/quests/content.rs`, `src/quests/dialogs.rs`, `src/mobs.rs`, `src/shard.rs`, `content/quests.toml` (header), `content/zones.toml` (one test row), `GLOSSARY.md`, new `tests/escorts.rs` | nothing here |
| 04-escort-ambushes | Escort ambushes and the escortee's assist | 03 | `src/escorts.rs`, `content/escorts.toml`, `tests/escorts.rs`, maybe `src/encounters.rs` | nothing here |

Not filed:
- `620-comment.md`: the comment to post on #620 (Keeshan's data ticket). It lists what the grill
  changed in #620's data: the start line, the failure that keeps the quest, two ambushes, the sight
  rules for the two Keeshans, and the tests.
- `adr-draft.md`: ADR 0035. The first escort ticket to land copies it to
  `docs/adr/0035-escorts.md` (each of 01, 02 and 03 is told to, if none has yet).

## Notes for the filer

- `00-parent.md` replaces the body and title of #596 (already in `.filed.json`).
- Add the blocker by hand: #620 is Blocked by 04-escort-ambushes. #620 already names #596, which
  closes when 01 to 04 land, so the new link only makes the order plain.
- Post `620-comment.md` on #620 after filing, with the NN-names in it turned into numbers.
- 01 and 02 both touch `src/mobs.rs`, `src/shard.rs` and `src/lib.rs`, in different functions. Two
  builders can take them the same night; a rebase conflict requeues one, as the runner does.
- Test ids used: quest 99811 (01), quest 99821 and escort `test_escort` (03 and 04), zone rows
  960811 (01) and 960821 (03), Hub entry 199811 (01). Check none is taken before filing.
