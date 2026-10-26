# The painted world map

How the world map's painted canvas is made, and how to remake it when a zone opens or the art
changes. Spec #421; the client draws it in #424.

## Where things live

- `data/twilights-eve/map/` is the source of truth: `canvas.png` (5015 by 3343), one
  `fog_<file>.png` per misted zone, `covers/` (one per stock exploration overlay piece) and
  `manifest.json` (each file's pixel size and its rect in canvas UV). No tool writes here; a
  remake is copied in on purpose.
- `data/twilights-eve/map/fill_painting.png` is Davis's ChatGPT painting, and
  `fill_landmarks.json` pins it to the canvas.
- `data/twilights-eve/map_canvas.json` lists the open zones (`zones`, in draw order) and the
  zones the canvas holds in full while they stay misted (`held`). The canvas rect is the union
  of both, padded to 3:2, in the client (`canvas_view` in `crates/benilla-app/src/ui_world_map.rs`)
  and in the tool alike.
- `crates/benilla-formats/examples/map_draft.rs` is the tool.

## Remaking the canvas

    cargo run -q --release -p benilla-formats --example map_draft -- --data C:\Users\davis\wow-data\Data --out target/map_draft --fill data/twilights-eve/map/fill_landmarks.json > target/map_draft_run.txt

Redirect the output to a file rather than piping it into `head`, which kills the run before it
writes. It writes under `target/map_draft/`:

- `filled.png`: the finished canvas. The stock zone maps sit at their true places, and the
  painting is warped into every gap between them.
- `filled_preview.png`: the same with every fog on it, as a player sees it today.
- `landmarks.png`: the warped painting alone, with a red cross on each landmark's canvas point.
- `canvas.png` (the stock-only draft), the fogs, the covers and `manifest.json`.

Then copy `filled.png` over `data/twilights-eve/map/canvas.png`, and the fogs, covers and
manifest over theirs, unedited. Check every spot you changed at full size with
`scripts/img.ps1 crop`, since the Read tool shows a 5015 px image at 40%.

## The fill file

`fill_landmarks.json` has three lists, all in UV (0 to 1 from the top left):

- `pairs`: the same place on the canvas and on the painting, usually a label's centre. Measure
  both with `scripts/img.ps1 crop -Scale 4 -Ticks` rather than on a scaled-down view. The run
  prints each pair's leave-one-out error, worst first, and flags any over twice the median:
  re-measure a flagged pair, or accept it where the painting itself drifts (Moonbrook, Eastvale
  Logging Camp, Stonecairn Lake and the Temple of Atal'Hakkar flag today for that reason).
- `erase`: a box of the painting and an offset to copy land from, for a painted label or icon
  that shows past the stock art. Painted lettering is larger than stock, so a painted label near
  a zone's edge pokes out beside its stock twin.
- `keep`: a box of the canvas where the stock art stays solid, for a stock label its own zone's
  silhouette clips (the F of Forest's Edge).

## When a zone opens

Move its id from the misted set into `zones` in `map_canvas.json`, remake, and check its edges:
painted labels it kept under fog now show. Add `erase` and `keep` boxes until none do. A zone
that is neither in `held` nor inside the canvas rect grows the rect, and then the painting needs
to cover the new area and every landmark moves: plan that as its own ticket.

## A new painting

ChatGPT redraws the whole image on every edit and cannot hold positions; asking it to trace the
draft moved places by 100 to 600 yards. Ask it for what the warp can use instead: show more land
past every edge (zoom out), or remove something (Stormwind's own city, so the stock city lies on
land). An edit that only removes usually keeps the rest pixel for pixel; check with
`scripts/img.ps1 compare` before reusing the landmarks, and re-measure them if anything moved.

## Stormwind

Stormwind's Highlight is black, so the city is cut out by a flood from its card's frame: the
flood takes the plain backdrop and stops at the ink outlines round the walls, districts and
crest (`FLOOD_STEP`, `FLOOD_REACH`). The city also clears Elwynn's art under it, so Elwynn's own
drawing of the city does not show between the districts. The painting must have no city there.
