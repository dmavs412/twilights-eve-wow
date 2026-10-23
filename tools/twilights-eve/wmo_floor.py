#!/usr/bin/env python3
"""The floors of the WMO buildings over a world point, read from their group meshes.

A point inside a building's bounding box can still stand outdoors: the box holds the ground
around the walls too. This drops a vertical ray through every placed WMO whose box holds the
point and lists each surface it crosses, so a caller can tell a floor from bare ground. It also
reads each group's MLIQ water (`Floors.water`) and finds the walls that cross a standing body
(`Floors.walls_near`), for ground_check.py.

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
# A standing unit's body for the wall test: a vertical capsule this wide, from this far over its
# feet to this far (ADR 0021).
BODY_RADIUS = 0.4
BODY_LOW = 0.3
BODY_HIGH = 2.0
# MLIQ grid step, 4.1666665 yd (benilla-formats MLIQ_CELL_STEP).
MLIQ_STEP = 25.0 / 6.0
# WMO liquid type nibbles that are water (LiquidKind::from_nibble): still, ocean, the still
# variant and rapids. 2, 6 magma and 3, 7 slime are not.
WATER_NIBBLES = {0, 1, 4, 8}


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


class Liquid:
    """A group's MLIQ grid in the model frame (crates/benilla-wmo/src/lib.rs, parse_mliq):
    vertex (i, j) at (base x + i step, base y + j step, heights[j xverts + i]), and per tile a flag
    whose low nibble is the liquid type, 0xF a hole. `group_liquid`, MOGP's groupLiquid, other than
    0xF overrides every tile's type."""

    def __init__(self, xverts, yverts, base, heights, tiles, group_liquid=0xF):
        self.xverts, self.yverts = xverts, yverts
        self.base = base
        self.heights = heights
        self.tiles = tiles
        self.group_liquid = group_liquid

    def surface(self, mx, my):
        """The water surface over model point (mx, my) in model z, or None."""
        i = math.floor((mx - self.base[0]) / MLIQ_STEP)
        j = math.floor((my - self.base[1]) / MLIQ_STEP)
        xt = self.xverts - 1
        if not (0 <= i < xt and 0 <= j < self.yverts - 1):
            return None
        flag = self.tiles[j * xt + i] & 0x0F
        if flag == 0x0F:
            return None
        kind = self.group_liquid & 0x0F if self.group_liquid != 0x0F else flag
        if kind not in WATER_NIBBLES:
            return None
        xv = self.xverts
        return sum(self.heights[k] for k in (j * xv + i, j * xv + i + 1,
                                             (j + 1) * xv + i, (j + 1) * xv + i + 1)) / 4.0


class Group:
    """One group file's triangles, in the model frame, with their xy bounds for a fast reject."""

    def __init__(self, index, flags, tris, liquid=None):
        self.index = index
        self.indoor = bool(flags & GROUP_INDOOR)
        self.tris = tris
        self.liquid = liquid
        xs = [v[0] for t in tris for v in t[:3]] or [0.0]
        ys = [v[1] for t in tris for v in t[:3]] or [0.0]
        self.box = (min(xs), max(xs), min(ys), max(ys))
        self._walls = None

    @property
    def walls(self):
        """The colliding triangles a unit cannot stand on, each with its model-frame bounds:
        (x0, x1, y0, y1, z0, z1, a, b, c)."""
        if self._walls is None:
            self._walls = []
            for a, b, c, fl in self.tris:
                if not collides(fl) or _normal_z(a, b, c) >= WALKABLE_NZ:
                    continue
                self._walls.append((min(a[0], b[0], c[0]), max(a[0], b[0], c[0]),
                                    min(a[1], b[1], c[1]), max(a[1], b[1], c[1]),
                                    min(a[2], b[2], c[2]), max(a[2], b[2], c[2]), a, b, c))
        return self._walls


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
        mopy = movi = movt = liquid = None
        for cid, off, size in _chunks(data):
            if cid != b"MOGP":
                continue
            group_liquid = struct.unpack_from("<I", data, off + 0x34)[0]
            for sid, soff, ssize in _chunks(data, off + 0x44, off + size):
                if sid == b"MOPY":
                    mopy = data[soff:soff + ssize]
                elif sid == b"MOVI":
                    movi = struct.unpack_from(f"<{ssize // 2}H", data, soff)
                elif sid == b"MOVT":
                    movt = struct.unpack_from(f"<{ssize // 4}f", data, soff)
                elif sid == b"MLIQ":
                    liquid = _mliq(data[soff:soff + ssize], group_liquid)
        if not (mopy and movi and movt):
            continue
        tris = []
        for t in range(len(movi) // 3):
            a, b, c = (movt[3 * movi[3 * t + j]:3 * movi[3 * t + j] + 3] for j in range(3))
            tris.append((a, b, c, mopy[2 * t]))
        groups.append(Group(g, gflags, tris, liquid))
    return groups


def _mliq(s, group_liquid):
    """An MLIQ payload: a 30-byte header (xverts, yverts, xtiles, ytiles, base[3], material),
    8-byte vertices with the height at +4, then one flag byte per tile. None when malformed."""
    if len(s) < 30:
        return None
    xverts, yverts, xtiles, ytiles = struct.unpack_from("<4I", s, 0)
    base = struct.unpack_from("<3f", s, 16)
    nverts, ntiles = xverts * yverts, xtiles * ytiles
    if (not nverts or xtiles + 1 != xverts or ytiles + 1 != yverts or nverts > 1 << 20
            or len(s) < 30 + nverts * 8 + ntiles):
        return None
    heights = [struct.unpack_from("<f", s, 30 + k * 8 + 4)[0] for k in range(nverts)]
    tiles = s[30 + nverts * 8:30 + nverts * 8 + ntiles]
    return Liquid(xverts, yverts, base, heights, tiles, group_liquid)


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
            out.append((z, grp, _normal_z(a, b, c), fl))
    return out


def _normal_z(a, b, c):
    """|normal z| of triangle abc: 1 flat, 0 vertical."""
    ux, uy, uz = (b[i] - a[i] for i in range(3))
    vx, vy, vz = (c[i] - a[i] for i in range(3))
    nx, ny, nz = uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx
    n = math.sqrt(nx * nx + ny * ny + nz * nz) or 1.0
    return abs(nz) / n


def _clip_slab(poly, lo, hi):
    """The part of polygon `poly` (3D points) with lo <= z <= hi."""
    for keep_above, h in ((True, lo), (False, hi)):
        out = []
        for k, p in enumerate(poly):
            q = poly[(k + 1) % len(poly)]
            p_in = p[2] >= h if keep_above else p[2] <= h
            q_in = q[2] >= h if keep_above else q[2] <= h
            if p_in:
                out.append(p)
            if p_in != q_in:
                t = (h - p[2]) / (q[2] - p[2])
                out.append(tuple(p[i] + t * (q[i] - p[i]) for i in range(3)))
        poly = out
        if not poly:
            return []
    return poly


def _distance_2d(px, py, poly):
    """Horizontal distance from (px, py) to polygon `poly` seen from above; 0 inside it."""
    n = len(poly)
    area = sum(poly[k][0] * poly[(k + 1) % n][1] - poly[(k + 1) % n][0] * poly[k][1] for k in range(n))
    if n >= 3 and abs(area) > 1e-9:
        sign = 1 if area > 0 else -1
        if all(sign * ((poly[(k + 1) % n][0] - poly[k][0]) * (py - poly[k][1])
                       - (poly[(k + 1) % n][1] - poly[k][1]) * (px - poly[k][0])) >= 0
               for k in range(n)):
            return 0.0
    best = math.inf
    for k in range(n):
        ax, ay = poly[k][0], poly[k][1]
        bx, by = poly[(k + 1) % n][0], poly[(k + 1) % n][1]
        dx, dy = bx - ax, by - ay
        L = dx * dx + dy * dy
        t = 0.0 if L < 1e-12 else max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / L))
        best = min(best, math.hypot(px - ax - t * dx, py - ay - t * dy))
    return best


def walls_at(groups, mx, my, mz, radius=BODY_RADIUS):
    """The wall triangles that cross a body standing at model point (mx, my, mz): a vertical
    capsule of `radius` from mz + BODY_LOW to mz + BODY_HIGH. Each hit is (group, a, b, c)."""
    lo, hi = mz + BODY_LOW, mz + BODY_HIGH
    out = []
    for grp in groups:
        x0, x1, y0, y1 = grp.box
        if not (x0 - radius <= mx <= x1 + radius and y0 - radius <= my <= y1 + radius):
            continue
        for tx0, tx1, ty0, ty1, tz0, tz1, a, b, c in grp.walls:
            if (mx < tx0 - radius or mx > tx1 + radius or my < ty0 - radius or my > ty1 + radius
                    or tz1 < lo or tz0 > hi):
                continue
            part = _clip_slab([a, b, c], lo, hi)
            if part and _distance_2d(mx, my, part) < radius:
                out.append((grp, a, b, c))
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

    def _placed(self, x, y, margin):
        """(groups, model x, model y, origin z) for each placed WMO whose box holds (x, y)."""
        tile, _, _ = self.terrain._tile_at(x, y)
        if not tile:
            return
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
            yield groups, dx * c + dy * s, -dx * s + dy * c, oz

    def surfaces(self, x, y, margin=0.0):
        """Every surface of every placed WMO whose box holds (x, y), highest first."""
        out = []
        for groups, mx, my, oz in self._placed(x, y, margin):
            for z, grp, nz, fl in ray(groups, mx, my):
                out.append(Surface(oz + z, (mx, my, z), grp.index, grp.indoor,
                                   nz >= WALKABLE_NZ, collides(fl)))
        return sorted(out, key=lambda s: -s.z)

    def water(self, x, y):
        """The highest MLIQ water surface over (x, y) in world z, or None."""
        best = None
        for groups, mx, my, oz in self._placed(x, y, 0.0):
            for grp in groups:
                z = grp.liquid.surface(mx, my) if grp.liquid else None
                if z is not None and (best is None or oz + z > best):
                    best = oz + z
        return best

    def walls_near(self, x, y, z, radius=BODY_RADIUS, margin=0.0):
        """The wall triangles that cross a body standing at world (x, y, z) (walls_at), in every
        placed WMO whose box, widened by `margin`, holds the point."""
        out = []
        for groups, mx, my, oz in self._placed(x, y, margin):
            out.extend(walls_at(groups, mx, my, z - oz, radius))
        return out


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
