# Server Unlocks: features a server buys, on a date

Davis means to sell some features of a live server to cover its upkeep: first a run speed bonus
on roads and wagons that run between towns (spec #560). While the game is tested nothing is for
sale and both are on. Grilled 2026-11-02.

## Decision

- **One player buys, the whole server gets it.** An unlock is a server-wide state, not a
  character's. Solo play is a one-player server and buys the same way, from the same file.
- **A catalogue in content.** `crates/twilight-realm/content/unlocks.toml` lists each unlock:
  `key`, `name`, `blurb` and `free`. `free = true` is the test build's state: on with nothing
  bought. Flipping a feature to paid is a content edit, not a code change.
- **A state file beside the saves.** `unlocks.toml` in the realm's data folder (the folder that
  holds the character saves and, on a server, `accounts.toml`) has one table per bought unlock
  with an `until` date-time in UTC and, for the record, `bought`. No `until` means on for good. A
  missing file means nothing bought. An unknown key or a bad date is a logged warning.
- **A date, not a flag.** An unlock is on when the catalogue says free, or the file has it and
  `until` is absent or after the realm's wall clock. The realm checks at login and once a day,
  when the wall clock crosses midnight UTC, and logs each change. A server that stops paying
  loses the feature at `until`, with no other mechanism.
- **One resource, read at the moment of use.** Features ask `unlock_on(key)` when they act: the
  road bonus on each movement report, the cart on each seat request. Nothing caches a per-feature
  flag, so a lapse or a purchase takes effect without a restart.
- **Locked stays visible.** A locked feature shows itself and says so: the cart stands parked
  with its driver, who tells you carts are not running on this server; the map's cart stops go
  grey. A player sees what a purchase would add. The road bonus has nothing to show, so it
  simply does not apply.
- **The purchase is out of scope.** Whatever sells an unlock (a store page, a command) writes
  the file and calls `Realm::set_unlocks`. No price, store or account lives in the realm.

## Consequences

- Every purchasable feature is gated in one place and tested with one clock. A test sets the
  wall clock past `until` and sees the feature go.
- Solo and server read the same file from different folders, so a solo player can be sold the
  same unlocks later with no new code.
- Showing a locked feature costs each one a locked look: a line, a grey mark. A feature with no
  natural locked look, like the road bonus, shows nothing, and the catalogue blurb is where a
  player reads what it would add.
