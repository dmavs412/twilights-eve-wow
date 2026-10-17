"""What the 1.12.1 ADTs say about the ground around the Crossroads, for camp placement.

`Survey(gd, centre, radius)` reads every ADT tile within `radius` of `centre` once and keeps:

  * `road_chunks`: the 33-yard MCNK chunks that paint a road texture (`BarrensRoad*`);
  * `wmos`: the WMO placements (bounding boxes), with their paths;
  * `doodads`: the M2 placements by model path, as (x, y, z, scale).

Coordinates are world x (north) and y (west), the realm's.
"""

from __future__ import annotations

import math
import struct

import wowmpq

ROAD_WORDS = ("road",)


class Survey:
    def __init__(self, gd, centre, radius, map_dir="Kalimdor"):
        self.gd = gd
        self.terrain = wowmpq.Terrain(gd, map_dir)
        self.road_chunks = set()
        self.wmos = []
        self.doodads = []
        cx, cy = centre
        tiles = set()
        step = wowmpq.TILE / 2
        n = int(radius // step) + 2
        for i in range(-n, n + 1):
            for j in range(-n, n + 1):
                x, y = cx + i * step, cy + j * step
                tiles.add((int(math.floor(32 - y / wowmpq.TILE)), int(math.floor(32 - x / wowmpq.TILE))))
        for tx, ty in sorted(tiles):
            d = gd.read("World\\Maps\\%s\\%s_%d_%d.adt" % (map_dir, map_dir, tx, ty))
            if d:
                self._read(d, tx, ty)

    def _read(self, d, tx, ty):
        o = wowmpq.ORIGIN
        secs = {}
        chunks = []
        p = 0
        while p + 8 <= len(d):
            tag = d[p:p + 4][::-1]
            size = struct.unpack_from("<I", d, p + 4)[0]
            if tag == b"MCNK":
                chunks.append(p)
            else:
                secs[tag] = (p + 8, size)
            p += 8 + size
        mtex = []
        if b"MTEX" in secs:
            a, s = secs[b"MTEX"]
            mtex = [t.decode("ascii", "replace").lower() for t in d[a:a + s].split(b"\0")]
        roads = {i for i, t in enumerate(mtex) if any(w in t for w in ROAD_WORDS)}
        for p in chunks:
            h = p + 8
            ix, iy = struct.unpack_from("<II", d, h + 4)
            n_layers = struct.unpack_from("<I", d, h + 0x0C)[0]
            ofs_layer = struct.unpack_from("<I", d, h + 0x1C)[0]
            for k in range(n_layers):
                e = p + ofs_layer + 8 + 16 * k
                if struct.unpack_from("<I", d, e)[0] in roads:
                    self.road_chunks.add((tx * 16 + ix, ty * 16 + iy))
        if b"MMDX" in secs and b"MDDF" in secs:
            a, s = secs[b"MMDX"]
            mmdx = d[a:a + s]
            a, s = secs[b"MMID"]
            mmid = struct.unpack_from("<%dI" % (s // 4), d, a)
            a, s = secs[b"MDDF"]
            for k in range(s // 36):
                e = a + k * 36
                nid = struct.unpack_from("<I", d, e)[0]
                px, py, pz = struct.unpack_from("<3f", d, e + 8)
                sc = struct.unpack_from("<H", d, e + 32)[0] / 1024.0
                off = mmid[nid]
                name = mmdx[off:mmdx.index(b"\0", off)].decode("ascii", "replace").lower().replace("\\", "/")
                self.doodads.append((o - pz, o - px, py, sc, name))
        if b"MWMO" in secs and b"MODF" in secs:
            a, s = secs[b"MWMO"]
            mwmo = d[a:a + s]
            a, s = secs[b"MWID"]
            mwid = struct.unpack_from("<%dI" % (s // 4), d, a)
            a, s = secs[b"MODF"]
            for k in range(s // 64):
                e = a + k * 64
                nid = struct.unpack_from("<I", d, e)[0]
                lo = struct.unpack_from("<3f", d, e + 32)
                hi = struct.unpack_from("<3f", d, e + 44)
                off = mwid[nid]
                path = mwmo[off:mwmo.index(b"\0", off)].decode("ascii", "replace").lower().replace("\\", "/")
                xs = sorted((o - lo[2], o - hi[2]))
                ys = sorted((o - lo[0], o - hi[0]))
                self.wmos.append((path, xs, ys))

    def chunk_of(self, x, y):
        """The (column, row) of the MCNK chunk under (x, y), in the whole-map chunk grid."""
        return int(math.floor((32 - y / wowmpq.TILE) * 16)), int(math.floor((32 - x / wowmpq.TILE) * 16))

    def near_road(self, x, y, margin):
        """Whether a road chunk lies within `margin` yards of (x, y)."""
        c0, r0 = self.chunk_of(x - margin, y - margin)
        c1, r1 = self.chunk_of(x + margin, y + margin)
        return any((c, r) in self.road_chunks for c in range(min(c0, c1), max(c0, c1) + 1)
                   for r in range(min(r0, r1), max(r0, r1) + 1))

    def near_wmo(self, x, y, margin):
        return any(xs[0] - margin <= x <= xs[1] + margin and ys[0] - margin <= y <= ys[1] + margin
                   for _, xs, ys in self.wmos)

    def flat(self, x, y, half, tol):
        """Whether the terrain within `half` yards of (x, y) stays within `tol` yards of its height."""
        z0 = self.terrain.ground_z(x, y)
        if z0 is None:
            return False
        for dx in (-half, 0, half):
            for dy in (-half, 0, half):
                z = self.terrain.ground_z(x + dx, y + dy)
                if z is None or abs(z - z0) > tol:
                    return False
        return True
