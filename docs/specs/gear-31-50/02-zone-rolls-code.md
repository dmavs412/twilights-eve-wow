Title: A rolls table by entry in zones.toml (#PARENT)
Labels: ready-for-agent

Part of #PARENT, decision 10. Blocked by RD:02-level-bands-code (it edits the same loader in
`src/mobs.rs`, `content()`, and the header of `content/zones.toml`).

Elwynn's named elites roll their band's Elite pool and greens through `rolls` on their
`mobs.toml` template (Princess, line 945 of `content/mobs.toml`). Redridge's and Duskwood's
templates live in the generated `content/redridge.toml` and `duskwood.toml`, which are never
edited (their header says so, and `tests/content_zones.rs` fails when they and the dump disagree).
Copying a 20-line template into `mobs.toml` per elite would shadow the generated row. Instead,
`zones.toml` gains a table that sets `rolls` on a template by entry. Ticket 08 fills it.

## The table

Under any `[[zone]]` of `content/zones.toml`:

```toml
[[zone.rolls]]
entry = 486            # Tharil'zun
rolls = [{ chance = 20, one_of = [...] }, { chance = 50, one_of = [...] }]
```

- `rolls` has the meaning it has on a template (`Template::rolls`, `src/mobs.rs` line 128): the
  first roll that succeeds adds one of its items.
- It sets the `rolls` of the template with that entry, wherever the template comes from: a
  generated zone file or `mobs.toml`. The arc's new named elites are `mobs.toml` templates with no
  rolls (RD:21 says "the gear 31-60 spec adds elite pools"), and this table covers them too, so
  every 31-50 elite roll sits in one place.

## Code

In `src/mobs.rs`:

- `ZoneOverrides` (line 587) gains `#[serde(default)] rolls: Vec<EntryRolls>`, with a new
  `#[derive(Deserialize)] struct EntryRolls { entry: u32, rolls: Vec<crate::rules::loot::Roll> }`.
- A function `apply_zone_rolls(template: &mut [Template], rows: &[EntryRolls])` sets each row's
  rolls on its template. It panics, naming the entry, when no template has the entry, when the
  template already carries `rolls` (a template's rolls are written in one place), or when two
  rows name one entry.
- `content()` calls it with every zone's rows once all templates have joined the list: after the
  loop that adds the other zones' templates (line 781) and before `lay_kits` (line 914), so the
  `roll.check` loop that follows (line 916) checks these rolls like any other.

In `content/zones.toml`'s header, say what `[[zone.rolls]]` does, in the style of the
`[[zone.added]]` line, and that ticket 08 of this spec fills it. Add no rows here.

## Failing tests first

In `src/mobs.rs`'s test module (line 2112), on templates built with `Template::hunter` (line 293)
or a small toml string:

- a row sets the rolls of its entry's template and leaves every other template's alone;
- a row naming no template panics with the entry in the message;
- a row naming a template that already has rolls panics;
- two rows naming one entry panic;
- `content()` still loads (`mobs::content()` in any existing test), and with no rows in
  `zones.toml` no template's rolls change: Princess (330) keeps hers.

## Acceptance

- The tests pass; `cargo test -q -p twilight-realm` passes; `python3.15 -I
  tools/twilights-eve/ground_check.py` flags nothing (the header comment touches `content/`).

## Assumptions allowed

- Applying the rows per zone or all together, as long as the panics above hold.
