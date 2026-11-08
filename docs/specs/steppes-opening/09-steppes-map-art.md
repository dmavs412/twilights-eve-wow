Title: Paint the Burning Steppes onto the world map, or decide to leave them off (#PARENT)
Labels: ready-for-human

Part of #PARENT. For Davis. No blocker.

## Why

The painted world map (#421, #424) is one canvas over Elwynn, Stormwind City, Duskwood and
Redridge. `canvas_rect` (`crates/benilla-world/src/map_proj.rs` line 45) builds it from those
four zones' rectangles, so the Burning Steppes, north-east of Redridge, are off the canvas. When
the Steppes open, the M-key map shows nothing of them and puts no marks there, Morgan's Vigil's
included. The Blackrock Pass camp stands on the Redridge side, so it is already on the canvas.

Adding a fifth zone to the canvas changes its rectangle, and the 89.5 MB of painted art made for
the current rectangle would no longer line up. So this is Davis's call before any code.

## Choose one

1. **Leave the Steppes off the painted map for now.** Nothing to do. Close this ticket.
2. **A larger canvas with the Steppes on it.** Davis repaints (or extends) the art for the new
   rectangle. Then a code ticket adds zone 46 to the canvas zones, with its fog and names layers.
   Use the same ChatGPT prompt set as the 2026-10-28 survey rebuild (`docs/art-assets.md`, the
   painted map rows) with the Steppes added.
3. **A second painted map for the Steppes**, opened when the player stands there. This needs a
   short grill: it is a new map mode.

## When done

- Record the choice in a comment here.
- For 2 or 3: add the painted files to `docs/art-assets.md` under "The Burning Steppes opening
  (#PARENT)" with their exact file names and sizes, and file the code ticket (2) or the grill
  (3).
