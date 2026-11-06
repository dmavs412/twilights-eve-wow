"""Write the Alchemy window's two placeholder vial textures (#545, spec #522) under
crates/benilla-app/assets/ui/alchemy/. The real art drops in later under the same names.

- vial_glass.tga (64 x 256 RGBA): the empty glass, its neck and cork, a pale rim, and a
  see-through inside from x 13 to 50 and y 51 to 242, where the window draws the liquid.
- vial_liquid.tga (64 x 256 RGBA): a pale grey liquid column with a white top edge. The window
  tints it with the property's colour and stretches it to the fill.

Each file is a 32-bit RLE true-colour TGA, top-left origin, written by hand since there is no PIL.

    ~/.local/bin/python3.15 tools/twilights-eve/placeholder_alchemy_vials.py
"""

import pathlib

from import_art import tga_bytes

OUT = pathlib.Path(__file__).resolve().parents[2] / "crates/benilla-app/assets/ui/alchemy"

W, H = 64, 256
CLEAR = (0, 0, 0, 0)
RIM = (214, 236, 246, 230)
GLASS = (190, 220, 235, 34)
CORK = (128, 86, 48, 255)
CORK_DARK = (92, 60, 32, 255)

# The body's inside, as the window's BENILLA_ALCHEMY_GLASS reads it at 40 x 160.
BODY_LEFT, BODY_RIGHT, BODY_TOP, BODY_BOTTOM = 13, 51, 51, 243
NECK_LEFT, NECK_RIGHT, NECK_TOP = 22, 42, 14


def glass_pixel(x, y):
    if y < NECK_TOP:
        if NECK_LEFT - 2 <= x < NECK_RIGHT + 2:
            return CORK_DARK if y in (0, NECK_TOP - 1) or x in (NECK_LEFT - 2, NECK_RIGHT + 1) else CORK
        return CLEAR
    if y < NECK_TOP + 3:
        return RIM if NECK_LEFT - 4 <= x < NECK_RIGHT + 4 else CLEAR
    if y < BODY_TOP - 6:
        if x in (NECK_LEFT - 2, NECK_LEFT - 1, NECK_RIGHT, NECK_RIGHT + 1):
            return RIM
        return GLASS if NECK_LEFT <= x < NECK_RIGHT else CLEAR
    if y < BODY_TOP:
        # The shoulder: the rim widens from the neck to the body over six rows.
        t = (y - (BODY_TOP - 6)) / 6
        left = round(NECK_LEFT - 2 - t * (NECK_LEFT - BODY_LEFT))
        right = round(NECK_RIGHT + 1 + t * (BODY_RIGHT - NECK_RIGHT))
        if x in (left, left + 1, right - 1, right):
            return RIM
        return GLASS if left < x < right else CLEAR
    if y < BODY_BOTTOM:
        if x in (BODY_LEFT - 2, BODY_LEFT - 1, BODY_RIGHT, BODY_RIGHT + 1):
            return RIM
        return GLASS if BODY_LEFT <= x < BODY_RIGHT else CLEAR
    if y < BODY_BOTTOM + 3:
        return RIM if BODY_LEFT - 2 <= x < BODY_RIGHT + 2 else CLEAR
    return CLEAR


def liquid_pixel(x, y):
    if y < 8:
        return (255, 255, 255, 255)
    edge = min(x, W - 1 - x)
    shade = 200 + min(edge, 8) * 3
    return (shade, shade, shade, 255)


def paint(pixel):
    rgba = bytearray()
    for y in range(H):
        for x in range(W):
            rgba += bytes(pixel(x, y))
    return tga_bytes(W, H, rgba)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "vial_glass.tga").write_bytes(paint(glass_pixel))
    (OUT / "vial_liquid.tga").write_bytes(paint(liquid_pixel))
    print(f"wrote vial_glass.tga and vial_liquid.tga to {OUT}")


if __name__ == "__main__":
    main()
