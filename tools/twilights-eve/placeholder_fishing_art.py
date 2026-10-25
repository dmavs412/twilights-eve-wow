"""Write the fishing UI's placeholder art (#328, spec #323): one flat-coloured TGA per sprite
name under crates/benilla-app/assets/ui/fishing/, at the size the Reeling Bar's frame lays it out.
The real art drops in later under the same names.

Each file is a 32-bit RLE true-colour TGA (image type 10), top-left origin, written by hand since
there is no PIL. A frame-like sprite gets a darker two-pixel border so its edge reads on screen.

    ~/.local/bin/python3.15 tools/twilights-eve/placeholder_fishing_art.py
"""

import pathlib

from import_art import rle_row, tga_header

OUT = pathlib.Path(__file__).resolve().parents[2] / "crates/benilla-app/assets/ui/fishing"

GOLD = (200, 160, 60, 255)
BLUE = (40, 90, 160, 220)
GREEN = (80, 200, 90, 200)
LIME = (150, 240, 110, 230)
ORANGE = (240, 140, 40, 255)
PURPLE = (190, 90, 230, 255)
WHITE = (255, 255, 255, 160)
BROWN = (130, 80, 40, 255)
YELLOW = (250, 220, 90, 255)
DARK = (30, 30, 35, 220)
RED = (200, 50, 40, 255)
GREY = (110, 110, 110, 255)
PARCHMENT = (210, 190, 150, 240)

# name: (width, height, colour, bordered)
SPRITES = {
    "bar_frame": (64, 256, GOLD, True),
    "bar_water": (32, 256, BLUE, False),
    "catch_zone": (32, 64, GREEN, True),
    "catch_zone_active": (32, 64, LIME, True),
    "fish": (32, 32, ORANGE, False),
    "fish_glow": (64, 64, WHITE, False),
    "fish_legend": (32, 32, PURPLE, False),
    "chest": (32, 32, BROWN, True),
    "chest_glow": (64, 64, YELLOW, False),
    "chest_open": (32, 32, YELLOW, True),
    "mini_bar_frame": (32, 8, GOLD, True),
    "progress_bg": (16, 256, DARK, True),
    "progress_fill": (16, 256, WHITE, False),
    "reel": (32, 32, GREY, True),
    "alert": (32, 32, YELLOW, True),
    "bobber": (16, 16, RED, False),
    "banner_success": (256, 64, GREEN, True),
    "banner_fail": (256, 64, RED, True),
    "level_badge": (32, 32, GOLD, True),
    "xp_bar_frame": (128, 16, DARK, True),
    "xp_bar_fill": (128, 16, PURPLE, False),
    "journal_bg": (512, 512, PARCHMENT, True),
    "slot": (64, 64, GREY, True),
    "slot_bg": (64, 64, DARK, False),
    "pip_on": (16, 16, YELLOW, False),
    "pip_off": (16, 16, GREY, False),
    "button_normal": (128, 32, BROWN, True),
    "button_hover": (128, 32, ORANGE, True),
    "button_pushed": (128, 32, DARK, True),
    "button_disabled": (128, 32, GREY, True),
    "divider": (256, 8, GOLD, False),
    "sparkle": (32, 32, WHITE, False),
}

BORDER = 2


def darker(colour):
    r, g, b, a = colour
    return (r // 2, g // 2, b // 2, max(a, 200))


def pixel(colour):
    r, g, b, a = colour
    return bytes((b, g, r, a))


def tga(width, height, colour, bordered):
    fill, edge = pixel(colour), pixel(darker(colour))
    data = bytearray(tga_header(width, height))
    for y in range(height):
        row = []
        for x in range(width):
            on_edge = bordered and (
                x < BORDER or y < BORDER or x >= width - BORDER or y >= height - BORDER
            )
            row.append(edge if on_edge else fill)
        data += rle_row(row)
    return bytes(data)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for name, (w, h, colour, bordered) in SPRITES.items():
        (OUT / f"{name}.tga").write_bytes(tga(w, h, colour, bordered))
    print(f"wrote {len(SPRITES)} sprites to {OUT}")


if __name__ == "__main__":
    main()
