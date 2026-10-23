#!/usr/bin/env python3
"""The floors of the WMO buildings over a world point, read from their group meshes.

A point inside a building's bounding box can still stand outdoors: the box holds the ground
around the walls too. This drops a vertical ray through every placed WMO whose box holds the
point and lists each surface it crosses, so a caller can tell a floor from bare ground.

    python3.15 tools/twilights-eve/wmo_floor.py <x> <y> [--map 0]

prints, top to bottom, every surface of each building over (x, y): its world z, the model-frame
point, the group (and whether the group is indoor), and whether the surface is a walkable floor
the client collides with. The terrain height under the point closes the list.

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
import wowmpq  # noqa: E402

DEFAULT_DATA = r"C:\Users\davis\wow-data\Data"
MAP_DIRS = {0: "Azeroth", 1: "Kalimdor"}
# MOGI group flag: the group is indoors.
GROUP_INDOOR = 0x2000
# MOPY triangle flags: a collision-only triangle, a rendered one, and render detail the client
# does not collide with.
MOPY_COLLISION = 0x08
MOPY_RENDER = 0x20
MOPY_DETAIL = 0x04
# A surface whose normal leans less than 50 degrees from straight up is a floor, the slope the client
# lets a unit walk; cave floors are bumpy.
WALKABLE_NZ = 0.64


class Surface:
    """One triangle the ray crossed: world z, model-frame point, group, and what it is."""

    def __init__(self, z, model, group, indoor, walkable, collides):
        self.z = z
        self.model = model
        self.group = group
        self.indoor = indoor
        self.walkable = walkable
        self.collides = collides

    @property
    def floor(self) -> bool:
        """A surface a unit stands on: walkable and collided with."""
        return self.walkable and self.collides


def _chunks(data, start=0, end=None):
    end = len(data) if end is None else end
    i = start
    while i + 8 <= end:
        cid = data[i:i + 4][::-1]
        size = struct.unpack_from("<I", data, i + 4)[0]
        yield cid, i + 8, size
        i += 8 + size


class Group:
    """One group file's triangles, in the model frame, with their xy bounds for a fast reject."""

    def __init__(self, index, flags, tris):
        self.index = index
        self.indoor = bool(flags & GROUP_INDOOR)
        self.tris = tris
        xs = [v[0] for t in tris for v in t[:3]] or [0.0]
        ys = [v[1] for t in tris for v in t[:3]] or [0.0]
        self.box = (min(xs), max(xs), min(ys), max(ys))


def load_model(gd, path):
    """The groups of the WMO at `path` (a root .wmo), or None when the archive lacks it."""
    root = gd.read(path)
    if root is None:
        return None
    flags = []
    for cid, off, size in _chunks(root):
        if cid == b"MOGI":
            flags = [struct.unpack_from("<I", root, off + k * 32)[0] for k in range(size // 32)]
    stem = path[:-4]
    groups = []
    for g, gflags in enumerate(flags):
        data = gd.read(f"{stem}_{g:03d}.wmo")
        if data is None:
            continue
        mopy = movi = movt = None
        for cid, off, size in _chunks(data):
            if cid != b"MOGP":
                continue
            for sid, soff, ssize in _chunks(data, off + 0x44, off + size):
                if sid == b"MOPY":
                    mopy = data[soff:soff + ssize]
                elif sid == b"MOVI":
                    movi = struct.unpack_from(f"<{ssize // 2}H", data, soff)
                elif sid == b"MOVT":
                    movt = struct.unpack_from(f"<{ssize // 4}f", data, soff)
        if not (mopy and movi and movt):
            continue
        tris = []
        for t in range(len(movi) // 3):
            a, b, c = (movt[3 * movi[3 * t + j]:3 * movi[3 * t + j] + 3] for j in range(3))
            tris.append((a, b, c, mopy[2 * t]))
        groups.append(Group(g, gflags, tris))
    return groups


def ray(groups, mx, my):
    """Every triangle over model point (mx, my): (model z, group, |normal z|, MOPY flags)."""
    out = []
    for grp in groups:
        x0, x1, y0, y1 = grp.box
        if not (x0 <= mx <= x1 and y0 <= my <= y1):
            continue
        for a, b, c, fl in grp.tris:
            if mx < min(a[0], b[0], c[0]) or mx > max(a[0], b[0], c[0]):
                continue
            if my < min(a[1], b[1], c[1]) or my > max(a[1], b[1], c[1]):
                continue
            d = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1])
            if abs(d) < 1e-9:
                continue
            l1 = ((b[1] - c[1]) * (mx - c[0]) + (c[0] - b[0]) * (my - c[1])) / d
            l2 = ((c[1] - a[1]) * (mx - c[0]) + (a[0] - c[0]) * (my - c[1])) / d
            l3 = 1 - l1 - l2
            if min(l1, l2, l3) < -1e-6:
                continue
            z = l1 * a[2] + l2 * b[2] + l3 * c[2]
            ux, uy, uz = (b[i] - a[i] for i in range(3))
            vx, vy, vz = (c[i] - a[i] for i in range(3))
            nx, ny, nz = uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx
            n = math.sqrt(nx * nx + ny * ny + nz * nz) or 1.0
            out.append((z, grp, abs(nz) / n, fl))
    return out


def collides(mopy_flags):
    """The client collides with a collision-only triangle, and with a rendered non-detail one."""
    return bool(mopy_flags & MOPY_COLLISION) or (
        bool(mopy_flags & MOPY_RENDER) and not mopy_flags & MOPY_DETAIL)


class Floors:
    """The WMO surfaces over world points on one map, loading each building once."""

    def __init__(self, gd, terrain):
        self.gd = gd
        self.terrain = terrain
        self.models = {}

    def _model(self, path):
        if path not in self.models:
            self.models[path] = load_model(self.gd, path)
        return self.models[path]

    def surfaces(self, x, y, margin=0.0):
        """Every surface of every placed WMO whose box holds (x, y), highest first."""
        tile, _, _ = self.terrain._tile_at(x, y)
        if not tile:
            return []
        out = []
        for p in tile["placements"]:
            if not (p["xs"][0] - margin <= x <= p["xs"][1] + margin
                    and p["ys"][0] - margin <= y <= p["ys"][1] + margin):
                continue
            groups = self._model(p["path"])
            if not groups:
                continue
            ox, oy, oz = p["origin"]
            # Model to world turns by MODF heading plus 180 about +Z. A tilted placement is rare
            # and not modelled: its surfaces are left out.
            if abs(p["rot"][0]) > 0.01 or abs(p["rot"][2]) > 0.01:
                continue
            yaw = math.radians(p["rot"][1] + 180.0)
            c, s = math.cos(yaw), math.sin(yaw)
            dx, dy = x - ox, y - oy
            mx, my = dx * c + dy * s, -dx * s + dy * c
            for z, grp, nz, fl in ray(groups, mx, my):
                out.append(Surface(oz + z, (mx, my, z), grp.index, grp.indoor,
                                   nz >= WALKABLE_NZ, collides(fl)))
        return sorted(out, key=lambda s: -s.z)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("x", type=float)
    ap.add_argument("y", type=float)
    ap.add_argument("--map", type=int, default=0, choices=sorted(MAP_DIRS))
    a = ap.parse_args()
    gd = wowmpq.GameData(os.environ.get("WOW_DATA", DEFAULT_DATA))
    terrain = wowmpq.Terrain(gd, MAP_DIRS[a.map])
    seen = set()
    for s in Floors(gd, terrain).surfaces(a.x, a.y):
        key = (round(s.z, 2), s.group, s.floor)
        if key in seen:
            continue
        seen.add(key)
        kind = "floor" if s.floor else ("wall/roof" if not s.walkable else "no-collide")
        print("z %8.2f  model (%6.1f, %6.1f, %6.2f)  group %2d %-7s %s"
              % (s.z, *s.model, s.group, "indoor" if s.indoor else "outdoor", kind))
    g = terrain.ground_z(a.x, a.y)
    print("terrain z %s" % ("%.2f" % g if g is not None else "none"))


if __name__ == "__main__":
    main()
