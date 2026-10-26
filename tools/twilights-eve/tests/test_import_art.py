"""import_art.py's PNG decoder, area-average resize and RLE TGA writer, on tiny PNGs built here.

    ~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests
"""

from __future__ import annotations

import os
import struct
import sys
import unittest
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, ".."))
import import_art  # noqa: E402


def chunk(kind, body):
    crc = zlib.crc32(kind + body) & 0xFFFFFFFF
    return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", crc)


def png(width, height, color_type, raw_rows, depth=8, interlace=0):
    """A PNG whose filtered scanlines (filter byte included) are raw_rows."""
    ihdr = struct.pack(">IIBBBBB", width, height, depth, color_type, 0, 0, interlace)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(b"".join(raw_rows)))
        + chunk(b"IEND", b"")
    )


def filtered(kind, rows, bpp):
    """Filter unfiltered rows (bytes) with one filter type for every row."""
    out = []
    prev = bytes(len(rows[0]))
    for row in rows:
        line = bytearray([kind])
        for i, x in enumerate(row):
            a = row[i - bpp] if i >= bpp else 0
            b = prev[i]
            c = prev[i - bpp] if i >= bpp else 0
            if kind == 0:
                p = 0
            elif kind == 1:
                p = a
            elif kind == 2:
                p = b
            elif kind == 3:
                p = (a + b) // 2
            else:
                pa, pb, pc = abs(b - c), abs(a - c), abs(a + b - 2 * c)
                p = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
            line.append((x - p) & 0xFF)
        out.append(bytes(line))
        prev = row
    return out


ROWS = [
    bytes([10, 20, 30, 255, 40, 50, 60, 128]),
    bytes([70, 80, 90, 0, 200, 210, 220, 7]),
]
PIXELS = bytes(b for row in ROWS for b in row)


def read_tga(data):
    """Decode a type 10 TGA: (width, height, bits, descriptor, RGBA bytes top-down)."""
    id_len, cmap, kind = data[0], data[1], data[2]
    assert (id_len, cmap) == (0, 0)
    width, height, bits, desc = struct.unpack_from("<HHBB", data, 12)
    pos = 18
    px = []
    while len(px) < width * height:
        head = data[pos]
        pos += 1
        count = (head & 0x7F) + 1
        if head & 0x80:
            px += [data[pos : pos + 4]] * count
            pos += 4
        else:
            for _ in range(count):
                px.append(data[pos : pos + 4])
                pos += 4
    out = bytearray()
    for b, g, r, a in px:
        out += bytes((r, g, b, a))
    return kind, width, height, bits, desc, bytes(out)


class DecodeTests(unittest.TestCase):
    def test_every_filter_type_decodes_to_the_same_pixels(self):
        for kind in range(5):
            data = png(2, 2, 6, filtered(kind, ROWS, 4))
            w, h, px = import_art.decode_png(data)
            self.assertEqual((w, h), (2, 2), f"filter {kind}")
            self.assertEqual(bytes(px), PIXELS, f"filter {kind}")

    def test_rgb_gets_alpha_255(self):
        rows = [bytes([1, 2, 3, 4, 5, 6]), bytes([7, 8, 9, 10, 11, 12])]
        data = png(2, 2, 2, filtered(1, rows, 3))
        w, h, px = import_art.decode_png(data)
        self.assertEqual(
            bytes(px), bytes([1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255, 10, 11, 12, 255])
        )

    def test_refuses_other_pngs(self):
        for kwargs in ({"depth": 16}, {"interlace": 1}, {"color_type": 0}):
            args = {"color_type": 6, "depth": 8, "interlace": 0}
            args.update(kwargs)
            data = png(2, 2, args["color_type"], filtered(0, ROWS, 4), args["depth"], args["interlace"])
            with self.assertRaises(ValueError):
                import_art.decode_png(data)


class ResizeTests(unittest.TestCase):
    def test_four_to_two_averages_each_quarter(self):
        # Red channel is the x + 4y index; the other channels are constant.
        src = bytearray()
        for y in range(4):
            for x in range(4):
                src += bytes((x + 4 * y, 100, 0, 255))
        out = import_art.resize(src, 4, 4, 2, 2)
        reds = [out[i * 4] for i in range(4)]
        # Quarter means: (0+1+4+5)/4=2.5, (2+3+6+7)/4=4.5, 10.5, 12.5, rounded half up.
        self.assertEqual(reds, [3, 5, 11, 13])
        self.assertEqual([out[i * 4 + 1] for i in range(4)], [100] * 4)

    def test_three_to_two_weights_by_overlap(self):
        # One row of 3 pixels with red 0, 90, 0 and height 1 to 1.
        src = bytes((0, 0, 0, 255, 90, 0, 0, 255, 0, 0, 0, 255))
        out = import_art.resize(src, 3, 1, 2, 1)
        # Output 0 covers src 0 (weight 2/3) and half of src 1 (weight 1/3): 30.
        self.assertEqual([out[0], out[4]], [30, 30])

    def test_same_size_is_identity(self):
        self.assertEqual(bytes(import_art.resize(PIXELS, 2, 2, 2, 2)), PIXELS)


class TgaTests(unittest.TestCase):
    def test_written_tga_is_rle_32_bit_top_left_and_round_trips(self):
        w, h = 5, 3
        px = bytearray()
        for i in range(w * h):
            px += bytes((i * 10 % 256, 5, 200 if i < 8 else 9, 255 if i % 2 else 77))
        data = import_art.tga_bytes(w, h, px)
        kind, rw, rh, bits, desc, back = read_tga(data)
        self.assertEqual((kind, rw, rh, bits), (10, w, h, 32))
        self.assertTrue(desc & 0x20, "top-left origin flag")
        self.assertEqual(back, bytes(px))

    def test_long_runs_round_trip(self):
        px = bytes((9, 8, 7, 255)) * 300
        _, w, h, _, _, back = read_tga(import_art.tga_bytes(300, 1, px))
        self.assertEqual(back, px)


class TileTests(unittest.TestCase):
    def test_pieces_run_row_major_with_the_last_column_and_row_cut_short(self):
        # 5x3 pixels, red channel the x + 10y index, cut into 2 by 2 pieces.
        w, h = 5, 3
        px = bytearray()
        for y in range(h):
            for x in range(w):
                px += bytes((x + 10 * y, 0, 0, 255))
        pieces = import_art.tiles(w, h, px, 2)
        self.assertEqual(
            [(n, pw, ph) for n, pw, ph, _ in pieces],
            [(1, 2, 2), (2, 2, 2), (3, 1, 2), (4, 2, 1), (5, 2, 1), (6, 1, 1)],
        )
        self.assertEqual([pieces[1][3][i * 4] for i in range(4)], [2, 3, 12, 13])
        self.assertEqual(pieces[5][3], bytes((24, 0, 0, 255)))

    def test_an_exact_fit_has_no_short_pieces(self):
        pieces = import_art.tiles(4, 4, bytes(64), 2)
        self.assertEqual([(pw, ph) for _, pw, ph, _ in pieces], [(2, 2)] * 4)


class PngWriteTests(unittest.TestCase):
    def test_png_round_trips(self):
        data = import_art.png_bytes(2, 2, PIXELS)
        w, h, px = import_art.decode_png(data)
        self.assertEqual((w, h, bytes(px)), (2, 2, PIXELS))


if __name__ == "__main__":
    unittest.main()
