# ADR 0015 amendment draft

Ticket 05 appends the section below to `docs/adr/0015-closed-zones.md`, after its Consequences,
with `#PARENT` replaced by the spec's number. It changes one sentence of the ADR's "The client
cuts, the realm enforces" point: the client no longer leaves every closed chunk empty.

---

## Amended 2026-11-08: one row of closed land is drawn inside the mist

Spec #PARENT, the mist look. Davis played the #554 curtain and called it a flat painted wall that
follows the chunk grid in straight steps. Land that stops at a sheet shows exactly where the world
ends; the lore's mist is a grey that the roads run into.

- **The rim row draws its land.** A rim chunk is a closed chunk with an open chunk among its eight
  neighbours. The client draws its terrain and its water, and fogs both toward the colour of the
  mist's foot by their distance past the edge, so the land is pure mist 30 yards in.
- **Everything else on a rim chunk stays cut.** It grows no clutter and keeps no doodad or WMO. It
  stays impassable and keeps its fence. Its horizon cell stays cut.
- **The realm does not change.** It refuses a move onto any closed chunk, rim or not, with "You
  cannot go that way."
- **The mist stands inside the rim.** The curtain is five shells from the edge to the rim's far
  side, 33.3 yards in. The last one is solid, so the empty chunks beyond the rim stay hidden.
- **Chunks past the rim** draw nothing, as before.

### Consequences

- A terrain cell that holds a rim chunk carries a mist-depth vertex attribute, and the terrain
  and liquid shaders each have a rim variant chosen by that attribute.
- A zone that a Milestone opens (ADR 0034) loses its rim when its tiles reload; the fogged rim
  shows clear land from that frame on.
- The minimap and the world map are unchanged.
- Content rules are unchanged: the realm test still fails on anything placed on a closed chunk,
  and a rim chunk is a closed chunk.

### Considered options

- **Two rows** (66.7 yards). Davis chose one. The solid back shell already stands at the end of
  the first row, so a second row would sit behind it unseen.
- **Keep the cut and improve only the curtain.** Rejected: a better sheet is still a sheet with
  the land stopping in front of it.
