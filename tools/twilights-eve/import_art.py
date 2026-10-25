"""Turn painted PNGs into the game's art files (#403, spec #396): decode, optionally shrink, and
write 32-bit RLE TGAs (what the UI layer loads) or 8-bit RGBA PNGs. Pure Python, since there is
no Pillow.

    ~/.local/bin/python3.15 tools/twilights-eve/import_art.py SRC_DIR OUT_DIR \
        [--size WxH] [--format tga|png] [--only name,...]

The decoder reads 8-bit RGB and RGBA, non-interlaced, and refuses anything else. --size shrinks
or grows by area averaging in gamma space: each output pixel is the mean of the source pixels it
covers, weighted by overlap, so ratios like 1254 to 128 work.
"""

import argparse
import pathlib
import struct
import sys
import zlib

PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def decode_png(data):
    """(width, height, RGBA bytes) of an 8-bit RGB or RGBA non-interlaced PNG."""
    if data[:8] != PNG_SIGNATURE:
        raise ValueError("not a PNG file")
    pos = 8
    ihdr = None
    idat = bytearray()
    while pos + 8 <= len(data):
        length, kind = struct.unpack_from(">I4s", data, pos)
        body = data[pos + 8 : pos + 8 + length]
        pos += 12 + length
        if kind == b"IHDR":
            ihdr = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat += body
        elif kind == b"IEND":
            break
    if ihdr is None:
        raise ValueError("PNG has no IHDR chunk")
    width, height, depth, color_type, _compression, _filter, interlace = ihdr
    if depth != 8 or color_type not in (2, 6) or interlace != 0:
        raise ValueError(
            f"unsupported PNG (bit depth {depth}, colour type {color_type}, interlace "
            f"{interlace}); only 8-bit RGB or RGBA, non-interlaced"
        )
    bpp = 3 if color_type == 2 else 4
    stride = width * bpp
    raw = zlib.decompress(bytes(idat))
    if len(raw) != (stride + 1) * height:
        raise ValueError("PNG pixel data has the wrong length")
    out = bytearray()
    prev = bytearray(stride)
    for y in range(height):
        kind = raw[y * (stride + 1)]
        row = bytearray(raw[y * (stride + 1) + 1 : (y + 1) * (stride + 1)])
        if kind == 0:
            pass
        elif kind == 1:
            for i in range(bpp, stride):
                row[i] = (row[i] + row[i - bpp]) & 0xFF
        elif kind == 2:
            for i in range(stride):
                row[i] = (row[i] + prev[i]) & 0xFF
        elif kind == 3:
            for i in range(stride):
                left = row[i - bpp] if i >= bpp else 0
                row[i] = (row[i] + ((left + prev[i]) >> 1)) & 0xFF
        elif kind == 4:
            for i in range(stride):
                a = row[i - bpp] if i >= bpp else 0
                b = prev[i]
                c = prev[i - bpp] if i >= bpp else 0
                pa, pb, pc = abs(b - c), abs(a - c), abs(a + b - 2 * c)
                p = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                row[i] = (row[i] + p) & 0xFF
        else:
            raise ValueError(f"PNG row {y} has unknown filter type {kind}")
        if bpp == 4:
            out += row
        else:
            for i in range(0, stride, 3):
                out += row[i : i + 3]
                out.append(255)
        prev = row
    return width, height, bytes(out)


def axis_weights(src, dst):
    """For each of dst output cells, [(source index, weight)] by overlap; weights sum to 1."""
    scale = src / dst
    table = []
    for o in range(dst):
        lo, hi = o * scale, (o + 1) * scale
        cells = []
        for s in range(int(lo), min(src, int(hi) + 1)):
            overlap = min(hi, s + 1) - max(lo, s)
            if overlap > 1e-9:
                cells.append((s, overlap / scale))
        table.append(cells)
    return table


def resize(pixels, src_w, src_h, dst_w, dst_h):
    """RGBA bytes resized by area averaging, channel values averaged as stored (gamma space)."""
    if (src_w, src_h) == (dst_w, dst_h):
        return bytes(pixels)
    xs = axis_weights(src_w, dst_w)
    ys = axis_weights(src_h, dst_h)
    # Horizontal pass keeps floats: dst_w * 4 per source row.
    mid = []
    for y in range(src_h):
        base = y * src_w * 4
        row = []
        for cells in xs:
            r = g = b = a = 0.0
            for s, w in cells:
                i = base + s * 4
                r += pixels[i] * w
                g += pixels[i + 1] * w
                b += pixels[i + 2] * w
                a += pixels[i + 3] * w
            row += (r, g, b, a)
        mid.append(row)
    out = bytearray()
    for cells in ys:
        for x in range(dst_w * 4):
            v = 0.0
            for s, w in cells:
                v += mid[s][x] * w
            out.append(min(255, int(v + 0.5)))
    return bytes(out)


def rle_row(row):
    """One row as RLE packets: a run of up to 128 equal pixels, or up to 128 raw ones."""
    out = bytearray()
    i = 0
    while i < len(row):
        run = 1
        while i + run < len(row) and run < 128 and row[i + run] == row[i]:
            run += 1
        if run > 1:
            out.append(0x80 | (run - 1))
            out += row[i]
            i += run
            continue
        start = i
        while i < len(row) and i - start < 128 and (i + 1 >= len(row) or row[i + 1] != row[i]):
            i += 1
        out.append(i - start - 1)
        for p in row[start:i]:
            out += p
    return bytes(out)


def tga_header(width, height):
    # Type 10, 32 bpp, descriptor 0x28: 8 alpha bits, rows top-down.
    return struct.pack("<BBBHHBHHHHBB", 0, 0, 10, 0, 0, 0, 0, 0, width, height, 32, 0x28)


def tga_bytes(width, height, rgba):
    """A 32-bit RLE TGA, top-left origin, from RGBA bytes."""
    data = bytearray(tga_header(width, height))
    for y in range(height):
        row = []
        for x in range(width):
            i = (y * width + x) * 4
            row.append(bytes((rgba[i + 2], rgba[i + 1], rgba[i], rgba[i + 3])))
        data += rle_row(row)
    return bytes(data)


def png_bytes(width, height, rgba):
    """An 8-bit RGBA PNG with filter 0 on every row."""

    def chunk(kind, body):
        return (
            struct.pack(">I", len(body))
            + kind
            + body
            + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)
        )

    stride = width * 4
    raw = b"".join(b"\x00" + bytes(rgba[y * stride : (y + 1) * stride]) for y in range(height))
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return (
        PNG_SIGNATURE
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def parse_size(text):
    try:
        w, h = text.lower().split("x")
        size = int(w), int(h)
    except ValueError:
        raise argparse.ArgumentTypeError(f"--size wants WxH, got {text!r}")
    if size[0] < 1 or size[1] < 1:
        raise argparse.ArgumentTypeError("--size must be positive")
    return size


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("src", type=pathlib.Path)
    ap.add_argument("out", type=pathlib.Path)
    ap.add_argument("--size", type=parse_size, help="WxH to resize every file to")
    ap.add_argument("--format", choices=("tga", "png"), default="tga")
    ap.add_argument("--only", help="comma-separated file names (no extension) to convert")
    args = ap.parse_args()

    only = set(args.only.split(",")) if args.only else None
    files = sorted(p for p in args.src.glob("*.png") if only is None or p.stem in only)
    if only is not None and {p.stem for p in files} != only:
        missing = sorted(only - {p.stem for p in files})
        sys.exit(f"no PNG for: {', '.join(missing)}")
    args.out.mkdir(parents=True, exist_ok=True)
    for path in files:
        try:
            w, h, px = decode_png(path.read_bytes())
        except ValueError as e:
            sys.exit(f"{path.name}: {e}")
        if args.size:
            px = resize(px, w, h, *args.size)
            w, h = args.size
        data = tga_bytes(w, h, px) if args.format == "tga" else png_bytes(w, h, px)
        (args.out / f"{path.stem}.{args.format}").write_bytes(data)
        print(f"{path.stem}: {w}x{h}")
    print(f"wrote {len(files)} files to {args.out}")


if __name__ == "__main__":
    main()
