"""Minimal read-only MPQ (v1) and DBC reader for the WoW 1.12.1 client data.

Mirrors crates/benilla-mpq: Storm string hash, decrypted hash/block tables, sectored files whose
sectors are zlib (codec byte 0x02) or stored verbatim. 1.12.1 data uses nothing else, so other
codecs, encryption and single-unit files are refused rather than guessed at.

Stdlib only, so it runs under an embeddable Python with `-I`.
"""

from __future__ import annotations

import os
import struct
import zlib

_SIGNATURE = 0x1A51504D  # "MPQ\x1A"
_USERDATA = 0x1B51504D  # "MPQ\x1B"
_HASH_EMPTY = 0xFFFFFFFF
_HASH_DELETED = 0xFFFFFFFE
_FLAG_IMPLODE = 0x00000100
_FLAG_COMPRESS = 0x00000200
_FLAG_ENCRYPTED = 0x00010000
_FLAG_SINGLE_UNIT = 0x01000000
_FLAG_DELETE_MARKER = 0x02000000
_FLAG_EXISTS = 0x80000000
_M32 = 0xFFFFFFFF


def _build_table() -> list[int]:
    table = [0] * 0x500
    seed = 0x00100001
    for i1 in range(0x100):
        for i2 in range(5):
            seed = (seed * 125 + 3) % 0x2AAAAB
            t1 = (seed & 0xFFFF) << 0x10
            seed = (seed * 125 + 3) % 0x2AAAAB
            t2 = seed & 0xFFFF
            table[i1 + i2 * 0x100] = t1 | t2
    return table


_TABLE = _build_table()


def hash_string(name: str, kind: int) -> int:
    seed1, seed2 = 0x7FED7FED, 0xEEEEEEEE
    for b in name.encode("ascii"):
        ch = ord("\\") if b == ord("/") else (b - 32 if 97 <= b <= 122 else b)
        seed1 = (_TABLE[kind + ch] ^ ((seed1 + seed2) & _M32)) & _M32
        seed2 = (ch + seed1 + seed2 + ((seed2 << 5) & _M32) + 3) & _M32
    return seed1


def _decrypt(words: list[int], key: int) -> list[int]:
    seed = 0xEEEEEEEE
    out = []
    for v in words:
        seed = (seed + _TABLE[0x400 + (key & 0xFF)]) & _M32
        ch = v ^ ((key + seed) & _M32)
        out.append(ch)
        key = ((((~key) << 0x15) & _M32) + 0x11111111 & _M32) | (key >> 0x0B)
        seed = (ch + seed + ((seed << 5) & _M32) + 3) & _M32
    return out


class Archive:
    def __init__(self, path: str):
        self.path = path
        with open(path, "rb") as f:
            size = os.fstat(f.fileno()).st_size
            off = 0
            while off + 4 <= size:
                f.seek(off)
                (sig,) = struct.unpack("<I", f.read(4))
                if sig == _SIGNATURE:
                    break
                if sig == _USERDATA:
                    _, hdr_off = struct.unpack("<II", f.read(8))
                    off += hdr_off
                    break
                off += 512
            else:
                raise ValueError(f"{path}: not an MPQ")
            self.base = off
            f.seek(off)
            hdr = f.read(32)
            shift = struct.unpack_from("<H", hdr, 14)[0]
            self.sector_size = 512 << shift
            ht_pos, bt_pos, ht_n, bt_n = struct.unpack_from("<IIII", hdr, 16)
            f.seek(off + ht_pos)
            words = _decrypt(list(struct.unpack(f"<{ht_n * 4}I", f.read(ht_n * 16))),
                             hash_string("(hash table)", 0x300))
            self.hash_table = [tuple(words[i * 4:i * 4 + 4]) for i in range(ht_n)]
            f.seek(off + bt_pos)
            words = _decrypt(list(struct.unpack(f"<{bt_n * 4}I", f.read(bt_n * 16))),
                             hash_string("(block table)", 0x300))
            self.block_table = [tuple(words[i * 4:i * 4 + 4]) for i in range(bt_n)]

    def _find(self, name: str):
        n = len(self.hash_table)
        if not n:
            return None
        start = hash_string(name, 0) & (n - 1)
        a, b = hash_string(name, 0x100), hash_string(name, 0x200)
        for i in range(n):
            na, nb, _loc, block = self.hash_table[(start + i) % n]
            if block == _HASH_EMPTY:
                return None
            if block == _HASH_DELETED:
                continue
            if na == a and nb == b:
                return self.block_table[block]
        return None

    def has(self, name: str) -> bool:
        blk = self._find(name)
        return bool(blk and blk[3] & _FLAG_EXISTS and not blk[3] & _FLAG_DELETE_MARKER)

    def read(self, name: str) -> bytes:
        blk = self._find(name)
        if not blk or not blk[3] & _FLAG_EXISTS or blk[3] & _FLAG_DELETE_MARKER:
            raise KeyError(name)
        pos, _csize, fsize, flags = blk
        if flags & (_FLAG_ENCRYPTED | _FLAG_SINGLE_UNIT):
            raise ValueError(f"{name}: unsupported flags {flags:#x}")
        with open(self.path, "rb") as f:
            f.seek(self.base + pos)
            if not flags & (_FLAG_COMPRESS | _FLAG_IMPLODE):
                return f.read(fsize)
            nsec = (fsize + self.sector_size - 1) // self.sector_size
            offs = struct.unpack(f"<{nsec + 1}I", f.read((nsec + 1) * 4))
            out = bytearray()
            for i in range(nsec):
                want = min(self.sector_size, fsize - i * self.sector_size)
                f.seek(self.base + pos + offs[i])
                raw = f.read(offs[i + 1] - offs[i])
                if len(raw) >= want:
                    out += raw[:want]
                elif flags & _FLAG_COMPRESS and raw[0] == 0x02:
                    out += zlib.decompress(raw[1:])
                else:
                    raise ValueError(f"{name}: sector codec {raw[0]:#x} unsupported")
            return bytes(out)


# Highest priority first, as the 1.12.1 client chains them.
_CHAIN = ["patch-2.MPQ", "patch.MPQ", "dbc.MPQ"]


def read_dbc_file(data_dir: str, name: str) -> tuple[bytes, str]:
    """The newest copy of DBFilesClient\\<name> across the patch chain, and which MPQ had it."""
    path = "DBFilesClient\\" + name
    for mpq in _CHAIN:
        full = os.path.join(data_dir, mpq)
        if not os.path.exists(full):
            continue
        arc = Archive(full)
        if arc.has(path):
            return arc.read(path), mpq
    raise FileNotFoundError(f"{path} not found in {data_dir}")


def parse_dbc(blob: bytes, fmt: str) -> list[tuple]:
    """Rows of a WDBC file. `fmt` per field: i=int32, u=uint32, f=float, s=string."""
    magic, nrec, nfield, recsize, strsize = struct.unpack_from("<4sIIII", blob, 0)
    if magic != b"WDBC":
        raise ValueError("not a WDBC file")
    if len(fmt) != nfield:
        raise ValueError(f"format has {len(fmt)} fields, file has {nfield}")
    strings = blob[20 + nrec * recsize:]
    codes = "".join({"i": "i", "u": "I", "f": "f", "s": "I"}[c] for c in fmt)
    rows = []
    for r in range(nrec):
        vals = list(struct.unpack_from("<" + codes, blob, 20 + r * recsize))
        for k, c in enumerate(fmt):
            if c == "s":
                o = vals[k]
                vals[k] = strings[o:strings.index(b"\0", o)].decode("utf-8", "replace")
        rows.append(tuple(vals))
    return rows
