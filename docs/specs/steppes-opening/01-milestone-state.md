Title: Milestones: the catalogue, the realm-written state file and the session event (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decisions 3, 4 and 10. ADR 0034 (draft: `docs/specs/steppes-opening/adr-draft.md`).
Code only, plus a two-row catalogue: after this ticket nothing in the game reaches a Milestone
yet. Tickets 04 and 05 do.

## What

A Milestone is a fact about the whole server that the realm records once and never forgets in
play. Build its catalogue, its state file (which the realm writes, unlike `unlocks.toml`), the
`Storage` variant that carries the file to the host, both hosts' read and write, and the session
event that tells clients which Milestones are reached. Server Unlocks (ADR 0031,
`crates/twilight-realm/src/unlocks.rs`) are the model to copy for the catalogue and the parse.

## The catalogue

New `crates/twilight-realm/content/milestones.toml`, added to the `include_str!` list in
`src/build_id.rs` beside `unlocks.toml` (line 72):

```toml
# Milestones (ADR 0034): facts about the whole server that the realm records once and never
# forgets in play. The state file is `milestones.toml` in the realm's data folder, which the realm
# writes; a host resets a Milestone by deleting its table while the realm is stopped.
#
# key        the Milestone's name in code, content and the state file
# name       what the log says
# after      optional: a Milestone that must be reached before this one can be

[[milestone]]
key = "ilgalar_anchor_broken"
name = "The Ilgalar Anchor is broken"

[[milestone]]
key = "steppes_open"
name = "The Burning Steppes are open"
after = "ilgalar_anchor_broken"
```

Ticket 04 adds `goal`, `feeds`, `announce` and `yell` to the header and to `steppes_open`; parse
unknown fields as an error so a typo fails the load. A repeated key, or an `after` that names no
row, panics at load with the key in the message, as `unlocks::catalogue` does.

## The state file

`milestones.toml` in the realm's data folder. One table per Milestone that has a count or is
reached:

```toml
[ilgalar_anchor_broken]
reached = "2026-11-20T21:14:03Z"

[steppes_open]
count = 315
```

- `reached`: an RFC 3339 date-time, as `unlocks.rs`'s `unix_secs` reads `until`. Present means
  reached. The realm writes it from its wall clock (`Realm::wall`).
- `count`: a whole number, the Tally (ticket 04). Absent is 0.
- A missing file means nothing is reached. A file that does not parse reaches nothing, a table
  naming no catalogue key is skipped, and a `reached` that is no date-time leaves that Milestone
  unreached. Each is a warning in the realm's log (`drain_log`), worded like `Unlocks::parse`'s.
- A reached Milestone whose `after` is not reached stays reached, with a warning.
- The realm writes the whole file, its tables in catalogue order, with a header comment that says
  the realm writes it and edits are made while the realm is stopped.

## Realm

- New `src/milestones.rs`, declared in `src/lib.rs` beside `mod unlocks;` (line 96): the catalogue
  rows, `Milestones` (catalogue plus state: reached keys with their date, counts, warnings), its
  parse, its render, and `reached(&self, key) -> bool`.
- `Realm` holds a `Milestones` (beside `unlocks`, line 775), starting with the shipped catalogue
  and nothing reached.
- `pub fn set_milestones(&mut self, text: Option<&str>)`, beside `set_unlocks` (line 993): reads
  the state file's text, or none, and logs its warnings.
- `pub fn with_milestone_catalogue(self, toml: &str) -> Self`, beside `with_unlock_catalogue`
  (line 1003), for tests.
- `pub fn milestone_reached(&self, key: &str) -> bool`, and `pub fn reached_milestones(&self) ->
  Vec<String>` in catalogue order.
- `pub fn reach_milestone(&mut self, key: &str)`: the one way a Milestone is reached, used by tests
  now and by tickets 04 and 05. It does nothing for a key already reached or a key the catalogue
  lacks (a warning). Otherwise it records the key with the wall clock's date, logs "Milestone
  <name> reached.", marks the state dirty, and sends `SessionEvent::Milestones` (below) to every
  session with a character in the world: the active one and every parked one, as
  `tick_encounters` (line 1662) walks `self.world` and `self.parked`.
- At most once per tick, after the tick's work, a dirty state pushes one
  `Storage::Milestones { blob }` with the whole rendered file onto `self.storage`, so
  `drain_storage` (line 2731) hands it to the host. Two Milestones reached in one tick give one
  write.
- `enter_world` (line 2922) pushes `SessionEvent::Milestones` before
  `world::InWorld::enter` (line 2946), so the client knows the reached set before its world loads.

## Storage and the session event

- `Storage::Milestones { blob: String }` in `src/save.rs` (enum at line 64), documented as the
  realm's server-wide state file, which the host writes beside the saves.
- `SessionEvent::Milestones { reached: Vec<String> }` in
  `crates/benilla-protocol/src/events.rs` (enum at line 539), with a "Twilight's Eve, not 1.12"
  doc comment like `CartRoute`'s: every reached Milestone key, sent at login and on each change.
  Fix any exhaustive match the new variant breaks; the client only ignores it in this ticket
  (ticket 07 reads it).

## Hosts

- Server, `crates/twilight-server/src/lib.rs`: `load_milestones(&mut realm, data_dir)` beside
  `load_unlocks` (line 342, called at line 208) reads `<data-dir>/milestones.toml`; a missing file
  is `None`, another read error is logged and treated as `None`. `store` (line 384) writes
  `Storage::Milestones` to `<data-dir>/milestones.toml` through a `.toml.tmp` file and a rename,
  as it writes a save. Every place that matches on `Storage` (lines 257, 283, 310) keeps working.
- Solo, `crates/benilla-app/src/twilight/mod.rs`: `load_milestones` beside `load_unlocks` (line
  238, called at line 207) reads `milestones.toml` beside the saves
  (`index_path().map(|p| p.with_file_name("milestones.toml"))`). `store` (line 256) writes it with
  `crate::local_state::write_atomic`; with persistence off (a capture run) it writes nothing.

## Docs

- Copy `docs/specs/steppes-opening/adr-draft.md` to `docs/adr/0034-milestones-and-gated-zones.md`.
  Change only its first line to `# Milestones: server-wide facts the realm writes, and land a
  Milestone opens` and drop the sentence about ticket 01.
- `GLOSSARY.md`: a **Milestone** entry after **Server Unlock** (line 409), in its form: "A fact
  about the whole server that the realm records once and never forgets in play (ADR 0034): a row
  of `content/milestones.toml`, and a table in the `milestones.toml` the realm writes beside the
  saves. Solo shares one file across the roster." `_Avoid_: world state, flag, achievement`.

## Failing tests first

New `crates/twilight-realm/tests/milestones.rs`, built like `tests/unlocks.rs` (a `Realm` with
`set_wall_clock`) and, for sessions, like `tests/closed_zones.rs` (`common::party`):

- With no state file nothing is reached, and the log is empty.
- A file with `[ilgalar_anchor_broken] reached = "2026-11-20T21:14:03Z"` reaches it and not
  `steppes_open`. A table named `nope` and a `reached = "yesterday"` each log one warning and reach
  nothing.
- `reach_milestone("ilgalar_anchor_broken")` makes it reached, and `drain_storage` then holds
  exactly one `Storage::Milestones` whose blob, read back with `set_milestones` on a new realm,
  has it reached with the wall clock's date. Reaching it again pushes no second write.
- Two Milestones reached in one tick give one `Storage::Milestones`.
- Two players in the world: after `reach_milestone`, each one's events hold
  `SessionEvent::Milestones` with the key. A character entering the world afterwards sees
  `SessionEvent::Milestones` with the key before its first `Teleport` or create.
- In `src/milestones.rs`'s test module: a catalogue with a repeated key panics, and one whose
  `after` names no row panics. The shipped catalogue loads with both rows.
- A save round trip in `tests/saving.rs` still passes: `Storage::Write` is untouched.

## Acceptance

- The tests pass. `cargo test -q -p twilight-realm` passes. `cargo check -q -p twilight-realm`,
  `-p benilla-protocol` and `-p twilight-server` are clean, and `cargo test -q -p twilight-server`
  passes.
- `cargo check -q -p benilla-app` is clean (a client build: CLAUDE.md says how to run it).

Assumptions allowed: the module's inner names; whether the dirty flag lives in `Milestones` or
`Realm`; where in the tick the write is pushed, as long as it is once per tick at most. Record
them as `Assumed:` lines.
