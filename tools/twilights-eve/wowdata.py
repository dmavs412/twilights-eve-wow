"""Minimal read-only access to WoW 1.12.1 game data: MPQ v1 archives, the patch chain, and WDBC.

Standard library only (zlib, bz2), so it runs on any Python 3.9+ with no installs. It is a reader
for offline data preparation, not a general MPQ library: it handles what the 1.12.1 client's
DBFilesClient tables need (multi-sector zlib/bzip2, single-unit, uncompressed) and refuses the
rest loudly (PKWARE implode, encrypted files).

Mount order follows crates/benilla-formats/src/chain.rs: the base archives, then patch.MPQ, then
patch-?.MPQ by name; a later archive wins.
"""

from __future__ import annotations

import bz2
import os
import struct
import zlib
from typing import Dict, List, Optional

# The base archives in ascending priority, as benilla-formats' VANILLA_BASE_ORDER.
BASE_ORDER = [
    "base.MPQ", "dbc.MPQ", "fonts.MPQ", "interface.MPQ", "misc.MPQ", "model.MPQ",
    "sound.MPQ", "speech.MPQ", "terrain.MPQ", "texture.MPQ", "wmo.MPQ",
]


def _crypt_table() -> List[int]:
    table = [0] * 0x500
    seed = 0x00100001
    for i in range(0x100):
        idx = i
        for _ in range(5):
            seed = (seed * 125 + 3) % 0x2AAAAB
            hi = (seed & 0xFFFF) << 16
            seed = (seed * 125 + 3) % 0x2AAAAB
            lo = seed & 0xFFFF
            table[idx] = hi | lo
            idx += 0x100
    return table


_CRYPT = _crypt_table()


def _hash(name: str, kind: int) -> int:
    s1, s2 = 0x7FED7FED, 0xEEEEEEEE
    for ch in name.upper().replace("/", "\\").encode("ascii"):
        s1 = (_CRYPT[(kind << 8) + ch] ^ (s1 + s2)) & 0xFFFFFFFF
        s2 = (ch + s1 + s2 + (s2 << 5) + 3) & 0xFFFFFFFF
    return s1


def _decrypt(data: bytes, key: int) -> bytes:
    n = len(data) // 4
    vals = struct.unpack(f"<{n}I", data[: n * 4])
    out = []
    seed = 0xEEEEEEEE
    for v in vals:
        seed = (seed + _CRYPT[0x400 + (key & 0xFF)]) & 0xFFFFFFFF
        d = (v ^ (key + seed)) & 0xFFFFFFFF
        out.append(d)
        key = (((~key << 0x15) + 0x11111111) | (key >> 0x0B)) & 0xFFFFFFFF
        seed = (d + seed + (seed << 5) + 3) & 0xFFFFFFFF
    return struct.pack(f"<{n}I", *out) + data[n * 4:]


_EXISTS = 0x80000000
_COMPRESS = 0x00000200
_IMPLODE = 0x00000100
_ENCRYPTED = 0x00010000
_SINGLE_UNIT = 0x01000000


class Archive:
    def __init__(self, path: str):
        self.path = path
        self.f = open(path, "rb")
        off = 0
        while True:
            self.f.seek(off)
            magic = self.f.read(4)
            if magic == b"MPQ\x1a":
                break
            if not magic:
                raise ValueError(f"{path}: no MPQ header")
            off += 512
        self.base = off
        hdr = self.f.read(28)
        (_hsize, _asize, _ver, shift, ht_pos, bt_pos, ht_n, bt_n) = struct.unpack("<IIHHIIII", hdr)
        self.sector_size = 512 << shift
        self.f.seek(off + ht_pos)
        ht = _decrypt(self.f.read(ht_n * 16), _hash("(hash table)", 3))
        self.f.seek(off + bt_pos)
        bt = _decrypt(self.f.read(bt_n * 16), _hash("(block table)", 3))
        self.hashes = [struct.unpack_from("<IIHHI", ht, i * 16) for i in range(ht_n)]
        self.blocks = [struct.unpack_from("<IIII", bt, i * 16) for i in range(bt_n)]

    def _find(self, name: str) -> Optional[int]:
        n = len(self.hashes)
        a, b = _hash(name, 1), _hash(name, 2)
        start = _hash(name, 0) % n
        i = start
        while True:
            ha, hb, _loc, _plat, block = self.hashes[i]
            if block == 0xFFFFFFFF:
                return None
            if ha == a and hb == b and block != 0xFFFFFFFE:
                return block
            i = (i + 1) % n
            if i == start:
                return None

    def has(self, name: str) -> bool:
        b = self._find(name)
        return b is not None and bool(self.blocks[b][3] & _EXISTS)

    def read(self, name: str) -> bytes:
        bi = self._find(name)
        if bi is None:
            raise KeyError(name)
        off, csize, fsize, flags = self.blocks[bi]
        if not flags & _EXISTS:
            raise KeyError(name)
        if flags & (_ENCRYPTED | _IMPLODE):
            raise NotImplementedError(f"{name} in {self.path}: encrypted/imploded (flags {flags:#x})")
        self.f.seek(self.base + off)
        raw = self.f.read(csize)
        if not flags & _COMPRESS:
            return raw[:fsize]
        if flags & _SINGLE_UNIT:
            return raw if csize >= fsize else self._decomp(raw, fsize)
        nsec = (fsize + self.sector_size - 1) // self.sector_size
        offs = struct.unpack_from(f"<{nsec + 1}I", raw, 0)
        out = bytearray()
        for s in range(nsec):
            chunk = raw[offs[s]:offs[s + 1]]
            want = min(self.sector_size, fsize - s * self.sector_size)
            out += chunk if len(chunk) >= want else self._decomp(chunk, want)
        return bytes(out)

    @staticmethod
    def _decomp(chunk: bytes, want: int) -> bytes:
        mask, data = chunk[0], chunk[1:]
        if mask == 0x02:
            return zlib.decompress(data)
        if mask == 0x10:
            return bz2.decompress(data)
        raise NotImplementedError(f"compression mask {mask:#x}")


class Chain:
    """The patch chain: a read resolves to the highest-priority archive holding the name."""

    def __init__(self, data_dir: str):
        names = {n.lower(): n for n in os.listdir(data_dir)}
        order = [names[b.lower()] for b in BASE_ORDER if b.lower() in names]
        if "patch.mpq" in names:
            order.append(names["patch.mpq"])
        order += sorted(
            (n for k, n in names.items() if k.startswith("patch-") and k.endswith(".mpq") and len(k) == 11),
            key=str.lower,
        )
        if not order:
            raise FileNotFoundError(f"no MPQs in {data_dir}")
        self.archives = [Archive(os.path.join(data_dir, n)) for n in order]

    def read(self, name: str) -> bytes:
        for a in reversed(self.archives):
            if a.has(name):
                return a.read(name)
        raise KeyError(name)

    def source(self, name: str) -> str:
        for a in reversed(self.archives):
            if a.has(name):
                return os.path.basename(a.path)
        raise KeyError(name)


class Dbc:
    """A WDBC table: rows of 4-byte cells, plus a string block."""

    def __init__(self, blob: bytes, name: str = "?"):
        magic, self.nrec, self.nfield, self.rsize, ssize = struct.unpack_from("<4sIIII", blob, 0)
        if magic != b"WDBC":
            raise ValueError(f"{name}: not WDBC")
        self.name = name
        self.recs = blob[20: 20 + self.nrec * self.rsize]
        self.strings = blob[20 + self.nrec * self.rsize:]
        self._index: Dict[int, int] = {}
        for i in range(self.nrec):
            self._index[self.u32(i, 0)] = i

    def u32(self, row: int, col: int) -> int:
        return struct.unpack_from("<I", self.recs, row * self.rsize + col * 4)[0]

    def i32(self, row: int, col: int) -> int:
        return struct.unpack_from("<i", self.recs, row * self.rsize + col * 4)[0]

    def f32(self, row: int, col: int) -> float:
        return struct.unpack_from("<f", self.recs, row * self.rsize + col * 4)[0]

    def str(self, row: int, col: int) -> str:
        off = self.u32(row, col)
        end = self.strings.index(b"\0", off)
        return self.strings[off:end].decode("utf-8", "replace")

    def row(self, rid: int) -> Optional[int]:
        return self._index.get(rid)

    def ids(self) -> List[int]:
        return [self.u32(i, 0) for i in range(self.nrec)]


def load_dbc(chain: Chain, short: str) -> Dbc:
    path = f"DBFilesClient\\{short}"
    return Dbc(chain.read(path), short)
