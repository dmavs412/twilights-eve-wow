"""Write the Forager's Grid's placeholder art (#442, spec #435): one flat-coloured TGA per sprite
under crates/benilla-app/assets/ui/herbalism/, at the sizes docs/art-assets.md lists, plus
minimap_leaf.tga for the Find Herbs leaf (#444). The real art drops in later under the same names.

The 34 files of the Forager's Grid window (#481, spec #479) are in NEW_ART with the sizes
docs/art-assets.md lists; the script checks every one of them after writing.

Each file is a 32-bit RLE true-colour TGA (image type 10), top-left origin, written by hand since
there is no PIL, in the same format as placeholder_fishing_art.py.

    ~/.local/bin/python3.15 tools/twilights-eve/placeholder_herb_art.py
"""

import pathlib
import struct

from import_art import rle_row, tga_header

OUT = pathlib.Path(__file__).resolve().parents[2] / "crates/benilla-app/assets/ui/herbalism"

CLEAR = (0, 0, 0, 0)
GREEN = (60, 140, 60, 255)
WILTED = (120, 90, 50, 255)
THORN_RED = (190, 40, 40, 255)
SOIL = (90, 60, 35, 255)
SOIL_RED = (120, 55, 45, 255)
RING = (245, 205, 80, 255)
TARGET = (240, 240, 240, 255)
WEED = (150, 170, 70, 255)
THORN = (220, 70, 60, 255)
LEAF = (90, 200, 90, 255)
WILTED_GREY = (125, 110, 90, 255)
GLOW_HAZE = (240, 200, 90, 110)
SPARKLE = (255, 240, 170, 255)
MOUND = (70, 50, 35, 255)
MOUND_RED = (120, 50, 40, 255)
HEDGE = (40, 85, 40, 255)
BED_SOIL = (55, 40, 28, 255)
BOARD = (150, 110, 65, 255)
POST = (110, 80, 45, 255)
PIP_ON = (240, 195, 70, 255)
PIP_OFF = (95, 80, 45, 255)
WHITE = (255, 255, 255, 255)

BORDER = 2


def darker(colour):
    r, g, b, a = colour
    return (r // 2, g // 2, b // 2, a)


def pixel(colour):
    r, g, b, a = colour
    return bytes((b, g, r, a))


def on_edge(x, y, w, h, width=BORDER):
    return x < width or y < width or x >= w - width or y >= h - width


def solid(colour):
    """A flat fill with a darker border."""
    return lambda x, y, w, h: darker(colour) if on_edge(x, y, w, h) else colour



def ring(colour):
    """A thick outline over a transparent centre."""
    return lambda x, y, w, h: colour if on_edge(x, y, w, h, 4) else CLEAR


def dashed(colour):
    """A thin outline cut into 8 px dashes over a transparent centre."""

    def at(x, y, w, h):
        if not on_edge(x, y, w, h, 3):
            return CLEAR
        return colour if (x + y) // 8 % 2 == 0 else CLEAR

    return at


def glyph(colour):
    """A diamond on a transparent ground, for the weed and thorn parts and the pull pips."""

    def at(x, y, w, h):
        dx, dy = abs(x - w / 2 + 0.5), abs(y - h / 2 + 0.5)
        return colour if dx + dy < w / 2 - 2 else CLEAR

    return at


def flat(colour):
    """A flat fill edge to edge, so a hedge or soil piece tiles without a seam."""
    return lambda x, y, w, h: colour


def ellipse_at(x, y, cx, cy, rx, ry):
    """Whether pixel (x, y) lies inside the ellipse; all measures are fractions of the tile."""
    return ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1


# The three bush shapes as ellipses in tile fractions, each resting on the bottom centre.
BUSH_SHAPES = (
    ((0.5, 0.6, 0.36, 0.36),),
    ((0.5, 0.66, 0.44, 0.3),),
    ((0.36, 0.64, 0.24, 0.32), (0.64, 0.64, 0.24, 0.32), (0.5, 0.5, 0.22, 0.26)),
)


def blob(colour, shape):
    """A bush: one flat blob of ellipses with a darker rim, transparent around it."""
    ellipses = BUSH_SHAPES[shape]

    def inside(u, v):
        return any(ellipse_at(u, v, *e) for e in ellipses)

    def at(x, y, w, h):
        u, v = (x + 0.5) / w, (y + 0.5) / h
        if not inside(u, v):
            return CLEAR
        rim = 3 / w
        if all(inside(u + du, v + dv) for du, dv in ((rim, 0), (-rim, 0), (0, rim), (0, -rim))):
            return colour
        return darker(colour)

    return at


def glow():
    """The plant overlay: a soft gold disc and four sparkles, transparent elsewhere."""
    sparkles = ((0.22, 0.25), (0.78, 0.3), (0.3, 0.78), (0.74, 0.72))

    def at(x, y, w, h):
        u, v = (x + 0.5) / w, (y + 0.5) / h
        for sx, sy in sparkles:
            if abs(u - sx) + abs(v - sy) < 0.06:
                return SPARKLE
        if ellipse_at(u, v, 0.5, 0.56, 0.4, 0.4):
            return GLOW_HAZE
        return CLEAR

    return at


def mound(colour):
    """A low ellipse of dirt in the lower half of the tile."""

    def at(x, y, w, h):
        u, v = (x + 0.5) / w, (y + 0.5) / h
        return colour if ellipse_at(u, v, 0.5, 0.7, 0.36, 0.18) else CLEAR

    return at


def stake():
    """A board on the top half and a post down the middle of the rest."""

    def at(x, y, w, h):
        if 2 <= y < h // 2 and 1 <= x < w - 1:
            return darker(BOARD) if on_edge(x, y - 2, w - 2, h // 2 - 2, 1) else BOARD
        if y >= h // 2 and abs(x - w / 2 + 0.5) < 3:
            return POST
        return CLEAR

    return at


def disc(colour):
    """A plain disc on a transparent ground, the stand-in for a plant part's glyph."""

    def at(x, y, w, h):
        u, v = (x + 0.5) / w, (y + 0.5) / h
        return colour if ellipse_at(u, v, 0.5, 0.5, 0.4, 0.4) else CLEAR

    return at


# The Forager's Grid window's art (#481), name: (width, height), as docs/art-assets.md lists it.
NEW_ART = {
    **{f"bush_{kind}_{n}": (128, 128) for kind in ("hidden", "wilted", "thorn") for n in (1, 2, 3)},
    "bush_glow": (128, 128),
    "mound": (128, 128),
    "mound_thorn": (128, 128),
    **{f"bed_corner_{c}": (64, 64) for c in ("tl", "tr", "bl", "br")},
    **{f"bed_edge_{e}": (64, 64) for e in ("top", "bottom", "left", "right")},
    "bed_soil": (128, 128),
    "stake": (32, 64),
    "pip_on": (16, 16),
    "pip_off": (16, 16),
    **{
        f"glyph_{g}": (64, 64)
        for g in (
            "flower",
            "leaf",
            "root",
            "berry",
            "mushroom",
            "nest",
            "nut",
            "honeycomb",
            "sap",
            "purse",
        )
    },
}

BUSH_COLOURS = {"hidden": GREEN, "wilted": WILTED_GREY, "thorn": THORN_RED}


def new_painter(name):
    """The flat placeholder for one of NEW_ART's files."""
    if name == "bush_glow":
        return glow()
    if name.startswith("bush_"):
        _, kind, n = name.split("_")
        return blob(BUSH_COLOURS[kind], int(n) - 1)
    if name.startswith("mound"):
        return mound(MOUND_RED if name == "mound_thorn" else MOUND)
    if name == "bed_soil":
        return flat(BED_SOIL)
    if name.startswith("bed_"):
        return flat(HEDGE)
    if name == "stake":
        return stake()
    if name.startswith("pip_"):
        return glyph(PIP_ON if name == "pip_on" else PIP_OFF)
    return disc(WHITE)


# name: (width, height, painter)
SPRITES = {
    # Ticket 3 of #479 retires bush_hidden, bush_wilted, bush_thorn, soil and soil_thorn once
    # nothing loads them; bush_glow and stake are NEW_ART's now.
    "bush_hidden": (64, 64, solid(GREEN)),
    "bush_wilted": (64, 64, solid(WILTED)),
    "bush_thorn": (64, 64, solid(THORN_RED)),
    "soil": (64, 64, solid(SOIL)),
    "soil_thorn": (64, 64, solid(SOIL_RED)),
    "tile_highlight": (64, 64, ring(RING)),
    "tile_target": (64, 64, dashed(TARGET)),
    "part_weed": (32, 32, glyph(WEED)),
    "part_thorn": (32, 32, glyph(THORN)),
    "minimap_leaf": (16, 16, glyph(LEAF)),
    **{name: (w, h, new_painter(name)) for name, (w, h) in NEW_ART.items()},
}


def tga(width, height, paint):
    data = bytearray(tga_header(width, height))
    for y in range(height):
        data += rle_row([pixel(paint(x, y, width, height)) for x in range(width)])
    return bytes(data)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for name, (w, h, paint) in SPRITES.items():
        (OUT / f"{name}.tga").write_bytes(tga(w, h, paint))
    print(f"wrote {len(SPRITES)} sprites to {OUT}")
    check()


def check():
    """Every Forager's Grid file is on disk at the size the art list gives it."""
    assert len(NEW_ART) == 34, f"the art list has 34 files, NEW_ART has {len(NEW_ART)}"
    for name, size in NEW_ART.items():
        header = (OUT / f"{name}.tga").read_bytes()[:18]
        found = struct.unpack("<HH", header[12:16])
        assert found == size, f"{name}.tga is {found[0]} x {found[1]}, the list says {size[0]} x {size[1]}"
    print(f"checked {len(NEW_ART)} Forager's Grid files")


if __name__ == "__main__":
    main()
