# The game clock runs at 12x real time

Since #31 the login burst has held the sky at 21:15 with a timescale of 0, so the world stays at
dusk forever. Fishing (ADR 0018) adds legendary fish that bite only at certain hours, and those
hours have to show in the sky. Davis chose a real day and night cycle over the fixed dusk.

- **One game day is 2 real hours.** The realm's game clock is its wall clock times 12, taken
  modulo a day: `minute_of_day = floor(wall_secs * 12 / 60) mod 1440`. It depends on nothing
  else, so the clock keeps running while every character is logged out and needs no save.
- **The client is told once, and advances on its own.** The login burst sends
  `SMSG_LOGIN_SETTIMESPEED` with the current game hour and minute and a timescale of `0.2`
  game-minutes per real second (vanilla's real time is about `0.01667`). The client's
  `GameTime` already advances by the timescale, so the sky, the minimap clock and `GetGameTime()`
  follow with no client change.
- **Every realm rule that reads the hour reads this clock.** The first is a legendary fish's hour
  window. A later rule that wants a forced night (the closed #227's Hogger camp) overrides the
  sent time for its Phase; it does not stop the clock.
- **The fixed dusk is gone.** `DUSK` in `crates/twilight-realm/src/world.rs` and its comment go.

## Considered options

- **Keep the dusk sky and run a hidden fishing-only clock.** Rejected by Davis: the hour a fish
  bites should match the sky the player sees.
- **Swing the sky between dusk and night only.** Rejected: the realm would resend the time to
  bounce it, and the minimap clock would run backwards.
- **A 1-hour or 4-hour day.** A 2-hour day gives about 40 real minutes of night (21:00 to 05:00)
  and lets a 1-hour session see half a cycle.
