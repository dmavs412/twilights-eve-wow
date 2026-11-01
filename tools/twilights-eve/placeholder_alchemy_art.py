"""Write the ten hybrid potion placeholder icons (#541, spec #522): one 128 x 128 PNG per
`[[product]]` of family "hybrid" in crates/twilight-realm/content/alchemy.toml, named by its `icon`
(`hybrid_<p1>_<p2>`, the two Properties in alphabetical order), in data/twilights-eve/art/. Each
is the round potion bottle split down the middle in the two Properties' colours, the first on the
left. Davis's painted icons drop in later under the same names.

The PNG is written by hand, since there is no Pillow (import_art.png_bytes).

    ~/.local/bin/python3.15 tools/twilights-eve/placeholder_alchemy_art.py
"""

import pathlib
import tomllib

from import_art import png_bytes

ROOT = pathlib.Path(__file__).resolve().parents[2]
CONTENT = ROOT / "crates/twilight-realm/content/alchemy.toml"
OUT = ROOT / "data/twilights-eve/art"

SIZE = 128

COLOURS = {
    "arcane": (150, 70, 210, 255),
    "calming": (60, 200, 220, 255),
    "fortifying": (225, 165, 40, 255),
    "restorative": (200, 35, 45, 255),
    "volatile": (240, 120, 30, 255),
}

CLEAR = (0, 0, 0, 0)
OUTLINE = (30, 22, 18, 255)
GLASS = (200, 215, 225, 150)
CORK = (130, 90, 55, 255)
SHINE = (255, 255, 255, 170)

# The bottle in pixels: a round body, a neck above it and a cork on the neck.
BODY = (64, 80, 42)
NECK = (53, 75, 22, 42)
CORK_BOX = (49, 79, 10, 23)
RIM = 3


def in_body(x, y, grow=0):
    cx, cy, r = BODY
    return (x + 0.5 - cx) ** 2 + (y + 0.5 - cy) ** 2 <= (r + grow) ** 2


def in_box(x, y, box, grow=0):
    x0, x1, y0, y1 = box
    return x0 - grow <= x < x1 + grow and y0 - grow <= y < y1 + grow


def painter(left, right):
    """The bottle with `left`'s colour in its left half and `right`'s in its right."""

    def at(x, y):
        if in_box(x, y, CORK_BOX):
            return CORK
        if in_body(x, y, -RIM):
            cx, cy, r = BODY
            if (x + 0.5 - (cx - r * 0.4)) ** 2 + (y + 0.5 - (cy - r * 0.4)) ** 2 <= (r * 0.15) ** 2:
                return SHINE
            return left if x < cx else right
        if in_box(x, y, NECK) and not in_body(x, y):
            return OUTLINE if not in_box(x, y, NECK, -RIM) else GLASS
        if in_body(x, y) or in_box(x, y, NECK) or in_box(x, y, CORK_BOX, RIM):
            return OUTLINE
        return CLEAR

    return at


def png(paint):
    rgba = bytearray()
    for y in range(SIZE):
        for x in range(SIZE):
            rgba += bytes(paint(x, y))
    return png_bytes(SIZE, SIZE, bytes(rgba))


def hybrids():
    """(icon, first property, second property) of each hybrid product, from its icon name."""
    with open(CONTENT, "rb") as f:
        products = tomllib.load(f)["product"]
    out = []
    for p in products:
        if p["family"] != "hybrid":
            continue
        _, first, second = p["icon"].split("_")
        assert sorted(p["key"].lower().split("+")) == [first, second], p["icon"]
        out.append((p["icon"], first, second))
    return out


def main():
    rows = hybrids()
    assert len(rows) == 10, f"alchemy.toml has {len(rows)} hybrids, not 10"
    for icon, first, second in rows:
        (OUT / f"{icon}.png").write_bytes(png(painter(COLOURS[first], COLOURS[second])))
    print(f"wrote {len(rows)} hybrid icons to {OUT}")


if __name__ == "__main__":
    main()
