#!/usr/bin/env python3
"""The doodads (placed M2s: rocks, trunks, fences, carts) whose collision a standing body is in.

Each ADT's MDDF places M2 models (wowmpq.Terrain). A model's collision is its bounding hull, the
u16 triangles and float vertices its header names at 0xEC and 0xF4 (crates/benilla-m2, read by
crates/benilla-formats/src/models/collision.rs); a model without one, like grass, has no
collision. A placement turns the hull by scale, then the client's placement rotation
(crates/benilla-assets/src/coords.rs, placement_rotation), then its position.

A body hits a doodad when a hull triangle crosses its capsule (wmo_floor.crosses_body) or the
body's middle lies inside the hull.

    python3.15 tools/twilights-eve/doodad_check.py <x> <y> <z> [--map 0]

prints the doodads a body standing at (x, y, z) is in.

Reads the game data from $WOW_DATA (default C:\\Users\\davis\\wow-data\\Data).
"""
from __future__ import annotations

import argparse
import math
import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import wmo_floor  # noqa: E402
import wowmpq  # noqa: E402

DEFAULT_DATA = r"C:\Users\davis\wow-data\Data"
MAP_DIRS = {0: "Azeroth", 1: "Kalimdor"}
# M2 header (version 256) offsets of the bounding triangles and vertices, (count, offset) each.
HULL_TRIS = 0xEC
HULL_VERTS = 0xF4
# Placements are indexed in square cells of this many yards.
CELL = 16.0


def model_path(path):
    """The MPQ file of an ADT model name: the ADT says .mdx, the archive holds .m2."""
    stem, ext = os.path.splitext(path)
    return stem + ".m2" if ext.lower() in (".mdx", ".mdl") else path


def display_name(path):
    return model_path(path).lower().replace("\\", "/")


def load_hull(gd, path):
    """(vertices, triangles) of the model's collision hull, or None when it has none."""
    d = gd.read(model_path(path))
    if d is None or d[:4] != b"MD20" or len(d) < HULL_VERTS + 8:
        return None
    nt, ot = struct.unpack_from("<II", d, HULL_TRIS)
    nv, ov = struct.unpack_from("<II", d, HULL_VERTS)
    if not nt or nt % 3 or ot + nt * 2 > len(d) or ov + nv * 12 > len(d):
        return None
    idx = struct.unpack_from("<%dH" % nt, d, ot)
    if any(i >= nv for i in idx):
        return None
    verts = [struct.unpack_from("<3f", d, ov + 12 * k) for k in range(nv)]
    return verts, [idx[k:k + 3] for k in range(0, nt, 3)]


def _rx(a):
    c, s = math.cos(a), math.sin(a)
    return ((1, 0, 0), (0, c, -s), (0, s, c))


def _ry(a):
    c, s = math.cos(a), math.sin(a)
    return ((c, 0, s), (0, 1, 0), (-s, 0, c))


def _rz(a):
    c, s = math.cos(a), math.sin(a)
    return ((c, -s, 0), (s, c, 0), (0, 0, 1))


def _mul(m, n):
    return tuple(tuple(sum(m[i][k] * n[k][j] for k in range(3)) for j in range(3)) for i in range(3))


def placement_matrix(rot_deg):
    """The MDDF rotation in the WoW frame, Rx(90) Ry(ry - 180) Rz(-rx) Rx(rz - 90), as in
    placement_rotation. A heading-only placement turns by ry - 180 about +Z."""
    rx, ry, rz = (math.radians(v) for v in rot_deg)
    m = _rx(math.pi / 2)
    for n in (_ry(ry - math.pi), _rz(-rx), _rx(rz - math.pi / 2)):
        m = _mul(m, n)
    return m


class Placed:
    """One placement's hull in world space, with its bounds."""

    def __init__(self, name, uid, tris):
        self.name = name
        self.uid = uid
        self.tris = tris
        pts = [p for t in tris for p in t]
        self.box = (min(p[0] for p in pts), max(p[0] for p in pts), min(p[1] for p in pts),
                    max(p[1] for p in pts), min(p[2] for p in pts), max(p[2] for p in pts))


def place(name, uid, hull, origin, rot, scale):
    """The Placed of `hull` scaled, turned by placement_matrix(rot) and moved to `origin`."""
    verts, tris = hull
    m = placement_matrix(rot)
    world = [tuple(origin[i] + scale * (m[i][0] * v[0] + m[i][1] * v[1] + m[i][2] * v[2])
                   for i in range(3)) for v in verts]
    return Placed(name, uid, [tuple(world[i] for i in t) for t in tris])


def _inside(tris, x, y, z):
    """True when (x, y, z) lies inside the closed hull: a ray straight up crosses it an odd
    number of times."""
    n = 0
    for a, b, c in tris:
        if x < min(a[0], b[0], c[0]) or x > max(a[0], b[0], c[0]):
            continue
        if y < min(a[1], b[1], c[1]) or y > max(a[1], b[1], c[1]):
            continue
        d = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1])
        if abs(d) < 1e-9:
            continue
        l1 = ((b[1] - c[1]) * (x - c[0]) + (c[0] - b[0]) * (y - c[1])) / d
        l2 = ((c[1] - a[1]) * (x - c[0]) + (a[0] - c[0]) * (y - c[1])) / d
        l3 = 1 - l1 - l2
        if min(l1, l2, l3) < 0:
            continue
        if l1 * a[2] + l2 * b[2] + l3 * c[2] > z:
            n += 1
    return n % 2 == 1


def body_hits(placed, x, y, z, radius=wmo_floor.BODY_RADIUS, height=wmo_floor.BODY_HIGH):
    """True when a body standing at (x, y, z) is in the placed doodad: a hull triangle crosses
    its capsule from z + BODY_LOW to z + `height`, or its middle is inside the hull."""
    lo, hi = z + wmo_floor.BODY_LOW, z + height
    x0, x1, y0, y1, z0, z1 = placed.box
    if x < x0 - radius or x > x1 + radius or y < y0 - radius or y > y1 + radius or z1 < lo or z0 > hi:
        return False
    if any(wmo_floor.crosses_body(a, b, c, x, y, lo, hi, radius) for a, b, c in placed.tris):
        return True
    return _inside(placed.tris, x, y, (lo + hi) / 2)


class Doodads:
    """The placed doodads on one map, each tile indexed in CELL-yard cells on first use."""

    def __init__(self, gd, terrain):
        self.gd = gd
        self.terrain = terrain
        self.hulls = {}

    def _hull(self, path):
        key = path.lower()
        if key not in self.hulls:
            self.hulls[key] = load_hull(self.gd, path)
        return self.hulls[key]

    def _grid(self, tile):
        if "doodad_grid" not in tile:
            grid = {}
            for p in tile["doodads"]:
                hull = self._hull(p["path"])
                if hull is None:
                    continue
                placed = place(display_name(p["path"]), p["uid"], hull, p["origin"], p["rot"], p["scale"])
                x0, x1, y0, y1, _, _ = placed.box
                for i in range(math.floor(x0 / CELL), math.floor(x1 / CELL) + 1):
                    for j in range(math.floor(y0 / CELL), math.floor(y1 / CELL) + 1):
                        grid.setdefault((i, j), []).append(placed)
            tile["doodad_grid"] = grid
        return tile["doodad_grid"]

    def near(self, x, y, radius):
        """The placements whose cells hold (x, y), widened by `radius`, once each."""
        tile, _, _ = self.terrain._tile_at(x, y)
        if not tile:
            return []
        grid = self._grid(tile)
        seen = {}
        for i in range(math.floor((x - radius) / CELL), math.floor((x + radius) / CELL) + 1):
            for j in range(math.floor((y - radius) / CELL), math.floor((y + radius) / CELL) + 1):
                for placed in grid.get((i, j), ()):
                    seen.setdefault(placed.uid, placed)
        return list(seen.values())

    def hits(self, x, y, z, radius=wmo_floor.BODY_RADIUS, height=wmo_floor.BODY_HIGH):
        """The model names of the doodads a body standing at (x, y, z) is in."""
        return sorted({p.name for p in self.near(x, y, radius) if body_hits(p, x, y, z, radius, height)})


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("x", type=float)
    ap.add_argument("y", type=float)
    ap.add_argument("z", type=float)
    ap.add_argument("--map", type=int, default=0, choices=sorted(MAP_DIRS))
    a = ap.parse_args()
    gd = wowmpq.GameData(os.environ.get("WOW_DATA", DEFAULT_DATA))
    terrain = wowmpq.Terrain(gd, MAP_DIRS[a.map])
    for name in Doodads(gd, terrain).hits(a.x, a.y, a.z):
        print(name)


if __name__ == "__main__":
    main()
