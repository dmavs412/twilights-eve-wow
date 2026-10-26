"""Write the Forager's Grid's placeholder art (#442, spec #435): one flat-coloured TGA per sprite
under crates/benilla-app/assets/ui/herbalism/, at the sizes docs/art-assets.md lists, plus
minimap_leaf.tga for the Find Herbs leaf (#444). The real art drops in later under the same names.

Each file is a 32-bit RLE true-colour TGA (image type 10), top-left origin, written by hand since
there is no PIL, in the same format as placeholder_fishing_art.py.

    ~/.local/bin/python3.15 tools/twilights-eve/placeholder_herb_art.py
"""

import pathlib

from import_art import rle_row, tga_header

OUT = pathlib.Path(__file__).resolve().parents[2] / "crates/benilla-app/assets/ui/herbalism"

CLEAR = (0, 0, 0, 0)
GREEN = (60, 140, 60, 255)
GOLD_GLOW = (230, 190, 70, 255)
WILTED = (120, 90, 50, 255)
THORN_RED = (190, 40, 40, 255)
SOIL = (90, 60, 35, 255)
SOIL_RED = (120, 55, 45, 255)
STAKE = (225, 215, 180, 255)
RING = (245, 205, 80, 255)
TARGET = (240, 240, 240, 255)
WEED = (150, 170, 70, 255)
THORN = (220, 70, 60, 255)
LEAF = (90, 200, 90, 255)

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


def bush(colour, glow=None):
    """A square tile: a darker border, and an optional lighter centre for a glow."""

    def at(x, y, w, h):
        if on_edge(x, y, w, h):
            return darker(colour)
        if glow and not on_edge(x, y, w, h, w // 4):
            return glow
        return colour

    return at


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
    """A diamond on a transparent ground, for the weed and thorn parts."""

    def at(x, y, w, h):
        dx, dy = abs(x - w / 2 + 0.5), abs(y - h / 2 + 0.5)
        return colour if dx + dy < w / 2 - 2 else CLEAR

    return at


# name: (width, height, painter)
SPRITES = {
    "bush_hidden": (64, 64, bush(GREEN)),
    "bush_glow": (64, 64, bush(GREEN, GOLD_GLOW)),
    "bush_wilted": (64, 64, bush(WILTED)),
    "bush_thorn": (64, 64, bush(THORN_RED)),
    "soil": (64, 64, solid(SOIL)),
    "soil_thorn": (64, 64, solid(SOIL_RED)),
    "stake": (32, 40, solid(STAKE)),
    "tile_highlight": (64, 64, ring(RING)),
    "tile_target": (64, 64, dashed(TARGET)),
    "part_weed": (32, 32, glyph(WEED)),
    "part_thorn": (32, 32, glyph(THORN)),
    "minimap_leaf": (16, 16, glyph(LEAF)),
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


if __name__ == "__main__":
    main()
