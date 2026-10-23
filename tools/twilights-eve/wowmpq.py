"""Minimal read-only MPQ (format v1, WoW 1.12.1) and ADT area/height reader.

Standard library only. Supports the compressions the 1.12.1 client's MPQs use
for terrain and DBC files (zlib, bzip2, PKWARE implode); encrypted files are
handled. Not a general MPQ tool: no writing, no format v2+, no listfiles.

Used by import_barrens_spawns.py to resolve which area/zone a world position
lies in (from the ADT MCNK chunk's area ID, which is exactly what the game and
server use) and to give an approximate ground height.
"""

from __future__ import annotations

import bz2
import math
import os
import struct
import zlib

# ---------------------------------------------------------------- crypto

def _build_crypt_table():
    table = [0] * 0x500
    seed = 0x00100001
    for i in range(0x100):
        idx = i
        for _ in range(5):
            seed = (seed * 125 + 3) % 0x2AAAAB
            t1 = (seed & 0xFFFF) << 16
            seed = (seed * 125 + 3) % 0x2AAAAB
            t2 = seed & 0xFFFF
            table[idx] = t1 | t2
            idx += 0x100
    return table


_CRYPT = _build_crypt_table()


def _hash(name: str, kind: int) -> int:
    s1, s2 = 0x7FED7FED, 0xEEEEEEEE
    for ch in name.upper().replace("/", "\\"):
        c = ord(ch)
        s1 = (_CRYPT[(kind << 8) + c] ^ (s1 + s2)) & 0xFFFFFFFF
        s2 = (c + s1 + s2 + (s2 << 5) + 3) & 0xFFFFFFFF
    return s1


def _decrypt(data: bytes, key: int) -> bytes:
    n = len(data) // 4
    words = struct.unpack("<%dI" % n, data[: n * 4])
    out = []
    seed = 0xEEEEEEEE
    for w in words:
        seed = (seed + _CRYPT[0x400 + (key & 0xFF)]) & 0xFFFFFFFF
        v = (w ^ (key + seed)) & 0xFFFFFFFF
        out.append(v)
        key = (((~key << 0x15) + 0x11111111) | (key >> 0x0B)) & 0xFFFFFFFF
        seed = (v + seed + (seed << 5) + 3) & 0xFFFFFFFF
    return struct.pack("<%dI" % n, *out) + data[n * 4 :]


# ---------------------------------------------------------------- PKWARE DCL explode

_LEN_BITS = [3, 2, 3, 3, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6, 7, 7]
_LEN_CODE = [5, 3, 1, 6, 10, 2, 12, 20, 4, 24, 8, 48, 16, 32, 64, 0]
_LEN_BASE = [0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 14, 22, 38, 70, 134, 262]
_EX_LEN_BITS = [0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8]
_DIST_BITS = [2, 4, 4, 5, 5, 5, 5, 6, 6, 6, 6, 6, 6, 6, 6, 6,
              6, 6, 6, 6, 6, 6, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
              7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
              8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8]
_DIST_CODE = [0x03, 0x0D, 0x05, 0x19, 0x09, 0x11, 0x01, 0x3E, 0x1E, 0x2E, 0x0E, 0x36, 0x16, 0x26, 0x06, 0x3A,
              0x1A, 0x2A, 0x0A, 0x32, 0x12, 0x22, 0x42, 0x02, 0x7C, 0x3C, 0x5C, 0x1C, 0x6C, 0x2C, 0x4C, 0x0C,
              0x74, 0x34, 0x54, 0x14, 0x64, 0x24, 0x44, 0x04, 0x78, 0x38, 0x58, 0x18, 0x68, 0x28, 0x48, 0x08,
              0xF0, 0x70, 0xB0, 0x30, 0xD0, 0x50, 0x90, 0x10, 0xE0, 0x60, 0xA0, 0x20, 0xC0, 0x40, 0x80, 0x00]


def _explode(data: bytes) -> bytes:
    """PKWARE Data Compression Library 'explode' (binary mode only)."""
    ctype, dict_bits = data[0], data[1]
    if ctype != 0:
        raise ValueError("PKWARE ASCII mode not supported")
    pos = 2
    bitbuf = 0
    bitcnt = 0

    def need(n):
        nonlocal pos, bitbuf, bitcnt
        while bitcnt < n:
            if pos >= len(data):
                raise EOFError
            bitbuf |= data[pos] << bitcnt
            pos += 1
            bitcnt += 8

    def bits(n):
        nonlocal bitbuf, bitcnt
        need(n)
        v = bitbuf & ((1 << n) - 1)
        bitbuf >>= n
        bitcnt -= n
        return v

    def decode(codes, lens, nsym):
        # codes are stored bit-reversed (LSB first), read bit by bit
        acc = 0
        for length in range(1, 9):
            acc |= bits(1) << (length - 1)
            for s in range(nsym):
                if lens[s] == length and codes[s] == acc:
                    return s
        raise ValueError("bad PKWARE code")

    out = bytearray()
    try:
        while True:
            if bits(1) == 0:
                out.append(bits(8))
                continue
            sym = decode(_LEN_CODE, _LEN_BITS, 16)
            length = _LEN_BASE[sym] + (bits(_EX_LEN_BITS[sym]) if _EX_LEN_BITS[sym] else 0) + 2
            if length == 519:
                break
            dsym = decode(_DIST_CODE, _DIST_BITS, 64)
            if length == 2:
                dist = (dsym << 2) | bits(2)
            else:
                dist = (dsym << dict_bits) | bits(dict_bits)
            dist += 1
            for _ in range(length):
                out.append(out[-dist])
    except EOFError:
        pass
    return bytes(out)


# ---------------------------------------------------------------- archive

class MpqArchive:
    def __init__(self, path: str):
        self.path = path
        self.f = open(path, "rb")
        head = self.f.read(32)
        off = 0
        while head[:4] != b"MPQ\x1a":
            off += 0x200
            self.f.seek(off)
            head = self.f.read(32)
            if len(head) < 32:
                raise ValueError("not an MPQ: %s" % path)
        self.base = off
        (_, _hsz, _asz, _ver, sshift, hoff, boff, hcount, bcount) = struct.unpack("<4sIIHHIIII", head)
        self.sector_size = 512 << sshift
        self.f.seek(off + hoff)
        ht = _decrypt(self.f.read(hcount * 16), _hash("(hash table)", 3))
        self.hashes = [struct.unpack_from("<IIHHI", ht, i * 16) for i in range(hcount)]
        self.f.seek(off + boff)
        bt = _decrypt(self.f.read(bcount * 16), _hash("(block table)", 3))
        self.blocks = [struct.unpack_from("<IIII", bt, i * 16) for i in range(bcount)]

    def _find(self, name: str):
        n = len(self.hashes)
        i = _hash(name, 0) % n
        a, b = _hash(name, 1), _hash(name, 2)
        start = i
        while True:
            h1, h2, _loc, _plat, bi = self.hashes[i]
            if bi == 0xFFFFFFFF:
                return None
            if h1 == a and h2 == b and bi != 0xFFFFFFFE:
                return self.blocks[bi]
            i = (i + 1) % n
            if i == start:
                return None

    def has(self, name: str) -> bool:
        return self._find(name) is not None

    def read(self, name: str) -> bytes | None:
        blk = self._find(name)
        if blk is None:
            return None
        boff, csize, fsize, flags = blk
        if not flags & 0x80000000:
            return None
        self.f.seek(self.base + boff)
        raw = self.f.read(csize)
        key = None
        if flags & 0x10000:
            key = _hash(name.replace("/", "\\").split("\\")[-1], 3)
            if flags & 0x20000:
                key = ((key + boff) ^ fsize) & 0xFFFFFFFF
        if flags & 0x01000000:  # single unit
            data = _decrypt(raw, key) if key is not None else raw
            return self._decomp(data, fsize, flags)
        if not flags & 0x300:  # stored, sectored
            if key is None:
                return raw[:fsize]
            out = bytearray()
            for s in range(0, fsize, self.sector_size):
                out += _decrypt(raw[s : s + self.sector_size], (key + s // self.sector_size) & 0xFFFFFFFF)
            return bytes(out[:fsize])
        nsec = (fsize + self.sector_size - 1) // self.sector_size
        tbl = raw[: (nsec + 1) * 4]
        if key is not None:
            tbl = _decrypt(tbl, (key - 1) & 0xFFFFFFFF)
        offs = struct.unpack("<%dI" % (nsec + 1), tbl)
        out = bytearray()
        for s in range(nsec):
            chunk = raw[offs[s] : offs[s + 1]]
            if key is not None:
                chunk = _decrypt(chunk, (key + s) & 0xFFFFFFFF)
            want = min(self.sector_size, fsize - s * self.sector_size)
            out += self._decomp(chunk, want, flags)
        return bytes(out[:fsize])

    @staticmethod
    def _decomp(chunk: bytes, want: int, flags: int) -> bytes:
        if len(chunk) >= want:
            return chunk[:want]
        if flags & 0x100:
            return _explode(chunk)
        mask = chunk[0]
        data = chunk[1:]
        if mask & 0x10:
            data = bz2.decompress(data)
        if mask & 0x08:
            data = _explode(data)
        if mask & 0x02:
            data = zlib.decompress(data)
        if mask & ~0x1A:
            raise ValueError("unsupported MPQ compression mask 0x%02x" % mask)
        return data


# Patch archives first: the first archive that has a file wins.
ARCHIVE_ORDER = [
    "patch-2.MPQ", "patch.MPQ", "terrain.MPQ", "dbc.MPQ", "model.MPQ", "texture.MPQ",
    "wmo.MPQ", "misc.MPQ", "interface.MPQ", "base.MPQ", "backup.MPQ",
]


class GameData:
    def __init__(self, data_dir: str):
        self.archives = []
        for n in ARCHIVE_ORDER:
            p = os.path.join(data_dir, n)
            if os.path.exists(p):
                self.archives.append(MpqArchive(p))
        if not self.archives:
            raise FileNotFoundError("no MPQs in %s" % data_dir)

    def read(self, name: str) -> bytes | None:
        for a in self.archives:
            if a.has(name):
                d = a.read(name)
                if d is not None:
                    return d
        return None

    def dbc(self, name: str):
        """Rows of a DBC as tuples of int32, plus the string block."""
        d = self.read("DBFilesClient\\%s.dbc" % name)
        if d is None:
            raise FileNotFoundError(name)
        magic, n, f, rs, _ss = struct.unpack_from("<4sIIII", d, 0)
        if magic != b"WDBC":
            raise ValueError("bad DBC %s" % name)
        rows = [struct.unpack_from("<%di" % f, d, 20 + i * rs) for i in range(n)]
        return rows, d[20 + n * rs :]

    _wmo_area = None

    def wmo_area_table(self):
        """(WMOID, NameSetID) -> AreaTable ID of the WMO's root row (WMOGroupID -1)."""
        if self._wmo_area is None:
            rows, _ = self.dbc("WMOAreaTable")
            m = {}
            for r in rows:
                if r[3] == -1 and r[10]:
                    m[(r[1], r[2])] = r[10]
            for r in rows:  # groups whose WMO has no root row: first non-zero group area
                if r[10]:
                    m.setdefault((r[1], r[2]), r[10])
            self._wmo_area = m
        return self._wmo_area

    _wmo_ids = {}

    def wmo_id(self, path: str) -> int | None:
        """MOHD wmoID of a root WMO file (the key into WMOAreaTable)."""
        if path not in self._wmo_ids:
            d = self.read(path)
            wid = None
            p = 0
            while d and p + 8 <= len(d):
                tag = d[p : p + 4][::-1]
                size = struct.unpack_from("<I", d, p + 4)[0]
                if tag == b"MOHD":
                    wid = struct.unpack_from("<I", d, p + 8 + 32)[0]
                    break
                p += 8 + size
            self._wmo_ids[path] = wid
        return self._wmo_ids[path]


# ---------------------------------------------------------------- ADT

TILE = 1600.0 / 3.0          # 533.333 yards per ADT tile
CHUNK = TILE / 16.0          # 33.333 yards per MCNK
UNIT = CHUNK / 8.0           # 4.1667 yards between outer height vertices
ORIGIN = 32.0 * TILE         # 17066.666: ADT placement-space origin
# Bytes per MCLQ block: {f32 min, f32 max, 81 x 8 B verts, 64 B cell flags, u32 nFlow, 2 x 40 B flow}.
MCLQ_BLOCK = 0x324
# A dry MCLQ vertex holds a huge height; benilla-formats' liquid.rs skips cells with one.
LIQUID_SENTINEL = 1.0e9


class Terrain:
    """Per-map MCNK area IDs and outer-vertex heights, loaded lazily per ADT."""

    def __init__(self, gd: GameData, map_dir: str):
        self.gd = gd
        self.map_dir = map_dir
        self.tiles = {}

    def _tile(self, tx: int, ty: int):
        k = (tx, ty)
        if k not in self.tiles:
            self.tiles[k] = self._load(tx, ty)
        return self.tiles[k]

    def _load(self, tx, ty):
        name = "World\\Maps\\%s\\%s_%d_%d.adt" % (self.map_dir, self.map_dir, tx, ty)
        d = self.gd.read(name)
        if d is None:
            return None
        chunks = {}
        mwmo = b""
        mwid = ()
        wmos = []
        # Each MODF entry's model path, world origin and rotation (degrees, placement axes).
        placements = []
        mmdx = b""
        mmid = ()
        doodads = []
        p = 0
        while p + 8 <= len(d):
            tag = d[p : p + 4][::-1]
            size = struct.unpack_from("<I", d, p + 4)[0]
            if tag == b"MWMO":
                mwmo = d[p + 8 : p + 8 + size]
            elif tag == b"MWID":
                mwid = struct.unpack_from("<%dI" % (size // 4), d, p + 8)
            elif tag == b"MMDX":
                mmdx = d[p + 8 : p + 8 + size]
            elif tag == b"MMID":
                mmid = struct.unpack_from("<%dI" % (size // 4), d, p + 8)
            elif tag == b"MDDF":
                # Each placed M2: model path, uniqueId, world position, rotation (degrees,
                # placement axes) and scale.
                for k in range(size // 36):
                    e = p + 8 + k * 36
                    name_id, uid = struct.unpack_from("<II", d, e)
                    pos = struct.unpack_from("<3f", d, e + 8)
                    off = mmid[name_id]
                    path = mmdx[off : mmdx.index(b"\0", off)].decode("ascii", "replace")
                    doodads.append({"path": path, "uid": uid,
                                    "origin": (ORIGIN - pos[2], ORIGIN - pos[0], pos[1]),
                                    "rot": struct.unpack_from("<3f", d, e + 20),
                                    "scale": struct.unpack_from("<H", d, e + 32)[0] / 1024.0})
            elif tag == b"MODF":
                for k in range(size // 64):
                    e = p + 8 + k * 64
                    name_id = struct.unpack_from("<I", d, e)[0]
                    lo = struct.unpack_from("<3f", d, e + 32)
                    hi = struct.unpack_from("<3f", d, e + 44)
                    name_set = struct.unpack_from("<H", d, e + 60)[0]
                    off = mwid[name_id]
                    path = mwmo[off : mwmo.index(b"\0", off)].decode("ascii", "replace")
                    # placement space (x', up, z') -> world: X = O - z', Y = O - x', Z = up
                    xs = sorted((ORIGIN - lo[2], ORIGIN - hi[2]))
                    ys = sorted((ORIGIN - lo[0], ORIGIN - hi[0]))
                    zs = sorted((lo[1], hi[1]))
                    wmos.append((path, name_set, xs, ys, zs))
                    pos = struct.unpack_from("<3f", d, e + 8)
                    rot = struct.unpack_from("<3f", d, e + 20)
                    placements.append({"path": path, "origin": (ORIGIN - pos[2], ORIGIN - pos[0], pos[1]),
                                       "rot": rot, "xs": xs, "ys": ys, "zs": zs})
            elif tag == b"MCNK":
                h = p + 8
                ix, iy = struct.unpack_from("<II", d, h + 4)
                area = struct.unpack_from("<I", d, h + 0x34)[0]
                ofs_h = struct.unpack_from("<I", d, h + 0x14)[0]
                px, py, pz = struct.unpack_from("<fff", d, h + 0x68)
                heights = None
                if ofs_h:
                    hp = p + ofs_h + 8  # ofs is from MCNK chunk start; skip MCVT header
                    if d[p + ofs_h : p + ofs_h + 4][::-1] == b"MCVT":
                        heights = struct.unpack_from("<145f", d, hp)
                chunks[(ix, iy)] = (area, px, py, pz, heights, self._liquids(d, p, h))
            p += 8 + size
        return {"chunks": chunks, "wmos": wmos, "placements": placements, "doodads": doodads}

    @staticmethod
    def _liquids(d, p, h):
        """The MCNK's MCLQ blocks as (81 vertex heights, 64 cell flags), the layout in
        crates/benilla-adt/src/lib.rs: one block per set liquid header bit (0x3c), at least one."""
        flags = struct.unpack_from("<I", d, h)[0]
        ofs, size = struct.unpack_from("<II", d, h + 0x60)
        if not ofs or d[p + ofs : p + ofs + 4][::-1] != b"MCLQ":
            return []
        start, end = p + ofs + 8, min(len(d), p + ofs + 8 + size)
        out = []
        for k in range(max(1, bin(flags & 0x3C).count("1"))):
            b = start + k * MCLQ_BLOCK
            if b + 8 + 81 * 8 + 64 > end:
                break
            heights = [struct.unpack_from("<f", d, b + 8 + i * 8 + 4)[0] for i in range(81)]
            out.append((heights, d[b + 8 + 81 * 8 : b + 8 + 81 * 8 + 64]))
        return out

    def _tile_at(self, x: float, y: float):
        fx = 32.0 - y / TILE
        fy = 32.0 - x / TILE
        tx, ty = int(math.floor(fx)), int(math.floor(fy))
        return self._tile(tx, ty), fx - tx, fy - ty

    def _locate(self, x: float, y: float):
        t, rx, ry = self._tile_at(x, y)
        if not t:
            return None
        return t["chunks"].get((int(math.floor(rx * 16)), int(math.floor(ry * 16))))

    def wmo_area(self, x: float, y: float, z: float, margin: float = 2.0):
        """(area_id, wmo_path) of the smallest WMO placement whose world bounding box
        contains the point and that WMOAreaTable maps to an area; else (None, None).
        Uses the placement's axis-aligned bounds only, not the per-group geometry."""
        t, _, _ = self._tile_at(x, y)
        if not t:
            return None, None
        table = self.gd.wmo_area_table()
        best = None
        for path, name_set, xs, ys, zs in t["wmos"]:
            if not (xs[0] - margin <= x <= xs[1] + margin and ys[0] - margin <= y <= ys[1] + margin
                    and zs[0] - margin <= z <= zs[1] + margin):
                continue
            wid = self.gd.wmo_id(path)
            area = table.get((wid, name_set)) or table.get((wid, 0))
            if not area:
                continue
            vol = (xs[1] - xs[0]) * (ys[1] - ys[0]) * (zs[1] - zs[0])
            if best is None or vol < best[0]:
                best = (vol, area, path)
        return (best[1], best[2]) if best else (None, None)

    def area_id(self, x: float, y: float) -> int | None:
        c = self._locate(x, y)
        return c[0] if c else None

    def ground_z(self, x: float, y: float) -> float | None:
        """Approximate terrain height (bilinear over the 9x9 outer grid). Ignores WMOs."""
        c = self._locate(x, y)
        if not c or c[4] is None:
            return None
        _, px, py, pz, hts, _ = c
        # chunk corner (px, py) is its max-x, max-y corner; rows run toward -x, cols toward -y
        r = (px - x) / UNIT
        q = (py - y) / UNIT
        r0, q0 = max(0, min(7, int(r))), max(0, min(7, int(q)))
        fr, fq = r - r0, q - q0

        def outer(i, j):
            return hts[i * 17 + j]

        z = (outer(r0, q0) * (1 - fr) * (1 - fq) + outer(r0 + 1, q0) * fr * (1 - fq)
             + outer(r0, q0 + 1) * (1 - fr) * fq + outer(r0 + 1, q0 + 1) * fr * fq)
        return pz + z

    def water_z(self, x: float, y: float) -> float | None:
        """The water surface over (x, y) from the MCNK's MCLQ blocks: the mean of the wet cell's
        four corners, the highest block's when two cover it. A cell's low nibble is its liquid
        type: 0xF is dry, and by its class (nibble & 3) 0 is still water and 1 ocean, while 2,
        magma, and 3, slime, are not water (LiquidKind::from_adt_nibble)."""
        c = self._locate(x, y)
        if not c:
            return None
        _, px, py, _, _, liquids = c
        row = max(0, min(7, int((px - x) / UNIT)))
        col = max(0, min(7, int((py - y) / UNIT)))
        best = None
        for heights, cells in liquids:
            nib = cells[row * 8 + col] & 0x0F
            if nib == 0x0F or nib & 3 not in (0, 1):
                continue
            corners = [heights[i] for i in (row * 9 + col, row * 9 + col + 1,
                                            (row + 1) * 9 + col, (row + 1) * 9 + col + 1)]
            if not all(math.isfinite(h) and abs(h) < LIQUID_SENTINEL for h in corners):
                continue
            z = sum(corners) / 4.0
            best = z if best is None else max(best, z)
        return best
