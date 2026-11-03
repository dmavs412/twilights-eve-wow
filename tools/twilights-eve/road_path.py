#!/usr/bin/env python3
"""Trace a cart route along the road texture and write its path into carts.toml (spec #560).

For the `[[route]]` whose `key` is <route-key>:
1. A road mask on a CELL_YD grid over the route's stops plus MARGIN_YD: a cell is road when its
   top terrain texture (wowmpq.Terrain.top_texture's rule) is one roads.toml lists for the map.
2. Each stop snaps to the nearest road cell within SNAP_YD; a stop farther from any road fails
   the run, and its author moves it.
3. A* from the first stop to the last, 8-connected: ROAD_COST a cell on the road, FIELD_COST off
   it, so a gap in the texture is bridged and a field is not crossed.
4. Douglas-Peucker at SIMPLIFY_YD, each bend rounded with one midpoint (round_bends), then cut
   into segments of at most STEP_YD.
5. Z for every point and both stops from Terrain.ground_z. The route's stop positions and its
   `path` are rewritten in place; every other line of carts.toml, comments included, is kept.

Prints the path length and the leg time at the cart's `speed_yd_s`. A path point over water
(a bridge: the terrain under it is the river bed) is reported, since its Z is the terrain's.

    python3.15 tools/twilights-eve/road_path.py <route-key> [--data <dir>]

Reads the game data from --data or $WOW_DATA (default C:\\Users\\davis\\wow-data\\Data).
"""
import argparse
import heapq
import math
import os
import re
import sys
import tomllib
from array import array

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import wowmpq  # noqa: E402

CONTENT = os.path.normpath(os.path.join(HERE, "..", "..", "crates", "twilight-realm", "content"))
ROADS = os.path.join(CONTENT, "roads.toml")
CARTS = os.path.join(CONTENT, "carts.toml")
DEFAULT_DATA = r"C:\Users\davis\wow-data\Data"
# Same table as ground_check.MAP_DIRS; a route stands on map 0 (carts.toml has no map key).
MAP_DIRS = {0: "Azeroth", 1: "Kalimdor"}
MAP_ID = 0
CELL_YD = 0.5
MARGIN_YD = 100.0
SNAP_YD = 15.0
ROAD_COST = 1.0
FIELD_COST = 25.0
SIMPLIFY_YD = 0.75
# A bend is cut this far back along each leg (at most half the leg) and rounded through the
# midpoint of the curve between the two cuts.
ROUND_YD = 4.0
# Path points stand no farther apart than this, so the wagon follows the ground between them.
STEP_YD = 6.0

HEADER = re.compile(r"^\s*\[\[?\s*[A-Za-z_][\w.]*\s*\]")
KEY = re.compile(r'^\s*key\s*=\s*"([^"]*)"')
PATH = re.compile(r"^\s*path\s*=")
STOP_POSITION = re.compile(
    r"(?P<head>position\s*=\s*\[\s*)[^,\]]+(?P<s1>\s*,\s*)[^,\]]+(?P<s2>\s*,\s*)[^,\]]+(?P<tail>\s*\])")
STRING = re.compile(r'"[^"]*"')


def strip_ext(name):
    """`name` without its file extension: `Tileset\\Elwynn\\ElwynnDirtBase2.blp` loses `.blp`."""
    head, dot, ext = name.rpartition(".")
    return head if dot and "\\" not in ext and "/" not in ext else name


def road_names(roads, map_id):
    """The lower-cased, extension-free road texture names roads.toml lists for `map_id`."""
    for m in roads.get("map", []):
        if m["id"] == map_id:
            return {strip_ext(n).lower() for n in m["textures"]}
    return set()


class Grid:
    """CELL_YD cells over a box of the world: cell (i, j) spans x from x0 + i * CELL_YD and y
    from y0 + j * CELL_YD, at index i * ny + j. The corner sits on a whole number of cells, so a
    stop snapped to a cell centre snaps to itself on the next run."""

    def __init__(self, xs, ys, margin, cell=CELL_YD):
        self.cell = cell
        self.x0 = math.floor((min(xs) - margin) / cell) * cell
        self.y0 = math.floor((min(ys) - margin) / cell) * cell
        self.nx = int(math.ceil((max(xs) + margin - self.x0) / cell)) + 1
        self.ny = int(math.ceil((max(ys) + margin - self.y0) / cell)) + 1

    def index(self, x, y):
        i = max(0, min(self.nx - 1, int((x - self.x0) / self.cell)))
        j = max(0, min(self.ny - 1, int((y - self.y0) / self.cell)))
        return i * self.ny + j

    def center(self, k):
        i, j = divmod(k, self.ny)
        return self.x0 + (i + 0.5) * self.cell, self.y0 + (j + 0.5) * self.cell


def road_mask(terrain, names, grid):
    """One byte per grid cell, 1 where the top texture at the cell's centre is in `names`."""
    chunks = {}  # (global chunk x, y) -> 4096 bytes, 1 on a road texel; None without terrain

    def chunk_roads(cx, cy):
        cells = terrain.texture_cells(cx >> 4, cy >> 4, cx & 15, cy & 15)
        if cells is None:
            return None
        layer_names, top = cells
        is_road = bytes(1 if strip_ext(n).lower() in names else 0 for n in layer_names)
        return top.translate(is_road + b"\0" * (256 - len(is_road)))

    mask = bytearray(grid.nx * grid.ny)
    for i in range(grid.nx):
        x = grid.x0 + (i + 0.5) * grid.cell
        gy = int(math.floor((32.0 - x / wowmpq.TILE) * 1024))
        row = (gy & 63) * 64
        base = i * grid.ny
        for j in range(grid.ny):
            y = grid.y0 + (j + 0.5) * grid.cell
            gx = int(math.floor((32.0 - y / wowmpq.TILE) * 1024))
            key = (gx >> 6, gy >> 6)
            if key not in chunks:
                chunks[key] = chunk_roads(*key)
            texels = chunks[key]
            if texels is not None and texels[row + (gx & 63)]:
                mask[base + j] = 1
    return mask


def snap(mask, grid, x, y, radius=SNAP_YD):
    """The centre of the road cell nearest (x, y) within `radius` yards, None when there is none."""
    r = int(math.ceil(radius / grid.cell))
    k0 = grid.index(x, y)
    i0, j0 = divmod(k0, grid.ny)
    best = None
    for i in range(max(0, i0 - r), min(grid.nx, i0 + r + 1)):
        for j in range(max(0, j0 - r), min(grid.ny, j0 + r + 1)):
            if mask[i * grid.ny + j]:
                cx, cy = grid.center(i * grid.ny + j)
                d = math.hypot(cx - x, cy - y)
                if d <= radius and (best is None or d < best[0]):
                    best = (d, cx, cy)
    return (best[1], best[2]) if best else None


def astar(mask, nx, ny, start, goal, road_cost=ROAD_COST, field_cost=FIELD_COST):
    """The cheapest 8-connected cell path from index `start` to `goal`, as indices: entering a
    cell costs `road_cost` on the road (mask 1) and `field_cost` off it, times sqrt(2) on a
    diagonal. The heuristic is the straight line at `road_cost`."""
    n = nx * ny
    g = array("d", [math.inf]) * n
    parent = array("i", [-1]) * n
    done = bytearray(n)
    gi, gj = divmod(goal, ny)
    diag = math.sqrt(2.0)
    steps = [(di, dj, diag if di and dj else 1.0) for di in (-1, 0, 1) for dj in (-1, 0, 1) if di or dj]
    g[start] = 0.0
    heap = [(0.0, start)]
    while heap:
        _, k = heapq.heappop(heap)
        if done[k]:
            continue
        if k == goal:
            break
        done[k] = 1
        i, j = divmod(k, ny)
        gk = g[k]
        for di, dj, step in steps:
            ni, nj = i + di, j + dj
            if not (0 <= ni < nx and 0 <= nj < ny):
                continue
            nk = ni * ny + nj
            if done[nk]:
                continue
            cost = gk + step * (road_cost if mask[nk] else field_cost)
            if cost < g[nk]:
                g[nk] = cost
                parent[nk] = k
                heapq.heappush(heap, (cost + road_cost * math.hypot(gi - ni, gj - nj), nk))
    out = [goal]
    while out[-1] != start:
        out.append(parent[out[-1]])
    return out[::-1]


def douglas_peucker(points, tolerance):
    """`points` (x, y pairs) with every point closer than `tolerance` to the chord of its kept
    neighbours dropped; the first and last are kept."""
    if len(points) < 3:
        return list(points)
    keep = bytearray(len(points))
    keep[0] = keep[-1] = 1
    stack = [(0, len(points) - 1)]
    while stack:
        a, b = stack.pop()
        (ax, ay), (bx, by) = points[a], points[b]
        dx, dy = bx - ax, by - ay
        length = math.hypot(dx, dy)
        worst, far = -1.0, None
        for k in range(a + 1, b):
            px, py = points[k]
            if length == 0.0:
                d = math.hypot(px - ax, py - ay)
            else:
                d = abs(dx * (ay - py) - dy * (ax - px)) / length
            if d > worst:
                worst, far = d, k
        if far is not None and worst > tolerance:
            keep[far] = 1
            stack += [(a, far), (far, b)]
    return [p for p, k in zip(points, keep) if k]


def round_bends(points, cut=ROUND_YD):
    """Each interior point B (between A and C) becomes three: P, `cut` yards back toward A, Q as
    far on toward C (at most half of either leg), and between them the midpoint of the curve
    from P through B to Q, (P + 2B + Q) / 4. The wagon then turns through two half bends."""
    if len(points) < 3:
        return list(points)
    out = [points[0]]
    for a, b, c in zip(points, points[1:], points[2:]):
        ab, bc = math.dist(a, b), math.dist(b, c)
        d = min(cut, ab / 2.0, bc / 2.0)
        if d <= 0.0:
            out.append(b)
            continue
        p = (b[0] + (a[0] - b[0]) * d / ab, b[1] + (a[1] - b[1]) * d / ab)
        q = (b[0] + (c[0] - b[0]) * d / bc, b[1] + (c[1] - b[1]) * d / bc)
        m = ((p[0] + 2 * b[0] + q[0]) / 4.0, (p[1] + 2 * b[1] + q[1]) / 4.0)
        # Two bends half a leg apart cut that leg at one point: keep it once.
        out += [p, m, q] if math.dist(out[-1], p) > 1e-6 else [m, q]
    out.append(points[-1])
    return out


def densify(points, step=STEP_YD):
    """`points` with each segment longer than `step` cut into equal parts no longer than it, and
    a point that repeats the one before it dropped."""
    out = [points[0]]
    for a, b in zip(points, points[1:]):
        d = math.dist(a, b)
        if d <= 1e-6:
            continue
        n = max(1, int(math.ceil(d / step)))
        out += [(a[0] + (b[0] - a[0]) * k / n, a[1] + (b[1] - a[1]) * k / n) for k in range(1, n + 1)]
    return out


def path_length(points):
    return sum(math.dist(a, b) for a, b in zip(points, points[1:]))


def trace(mask, grid, a, b):
    """The (x, y) path from snapped stop `a` to snapped stop `b`: A*, simplified, rounded and cut
    into STEP_YD segments, starting and ending exactly on the stops."""
    cells = astar(mask, grid.nx, grid.ny, grid.index(*a), grid.index(*b))
    pts = [grid.center(k) for k in cells]
    pts[0], pts[-1] = a, b
    return densify(round_bends(douglas_peucker(pts, SIMPLIFY_YD)))


def fmt(v, digits):
    """A coordinate as the content writes it, never `-0.0`."""
    s = "%.*f" % (digits, v)
    return s[1:] if s.startswith("-") and float(s) == 0.0 else s


def route_span(lines, key):
    """(start, end) line indices of the `[[route]]` table whose `key` is `key`: its header line
    and the line before the next table header."""
    start = None
    for i, line in enumerate(lines):
        if HEADER.match(line):
            if start is not None:
                return start, i
            start = i if line.strip() == "[[route]]" else None
            found = False
            continue
        if start is not None and not found:
            m = KEY.match(line)
            if m:
                if m.group(1) != key:
                    start = None
                found = True
    if start is not None:
        return start, len(lines)
    raise SystemExit("carts.toml: no [[route]] with key = %r" % key)


def array_end(lines, i):
    """The index of the line that closes the array opened on line `i`."""
    depth = 0
    for k in range(i, len(lines)):
        code = STRING.sub("", lines[k]).split("#")[0]
        depth += code.count("[") - code.count("]")
        if depth <= 0:
            return k
    raise SystemExit("carts.toml: the array on line %d never closes" % (i + 1))


def write_route(text, key, stops, path, note):
    """carts.toml's text with route `key`'s stop positions (in order, one stop per line) and its
    `path` array replaced; `path` is (x, y, z) points, `note` the comment on its first line."""
    lines = text.split("\n")
    start, end = route_span(lines, key)
    k = 0
    for i in range(start, end):
        m = STOP_POSITION.search(lines[i])
        if m and k < len(stops):
            x, y, z = stops[k]
            new = "%s%s%s%s%s%s%s" % (m.group("head"), fmt(x, 1), m.group("s1"), fmt(y, 1),
                                      m.group("s2"), fmt(z, 2), m.group("tail"))
            lines[i] = lines[i][: m.start()] + new + lines[i][m.end():]
            k += 1
    if k != len(stops):
        raise SystemExit("carts.toml: route %s has %d stop positions, not %d" % (key, k, len(stops)))
    at = next((i for i in range(start, end) if PATH.match(lines[i])), None)
    if at is None:
        raise SystemExit("carts.toml: route %s has no path line" % key)
    indent = lines[at][: len(lines[at]) - len(lines[at].lstrip())]
    block = ["%spath = [  # %s" % (indent, note)]
    block += ["%s  [%s, %s, %s]," % (indent, fmt(x, 1), fmt(y, 1), fmt(z, 2)) for x, y, z in path]
    block.append(indent + "]")
    lines[at : array_end(lines, at) + 1] = block
    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("route")
    ap.add_argument("--data", default=os.environ.get("WOW_DATA", DEFAULT_DATA))
    ap.add_argument("--margin", type=float, default=MARGIN_YD,
                    help="yards of road searched around the stops' box (default %g)" % MARGIN_YD)
    a = ap.parse_args()

    with open(ROADS, "rb") as f:
        names = road_names(tomllib.load(f), MAP_ID)
    with open(CARTS, "rb") as f:
        carts = tomllib.load(f)
    route = next((r for r in carts["route"] if r["key"] == a.route), None)
    if route is None:
        sys.exit("carts.toml: no route %r" % a.route)
    if not names:
        sys.exit("roads.toml: no textures for map %d" % MAP_ID)
    stops = [s["position"][:2] for s in route["stops"]]
    terrain = wowmpq.Terrain(wowmpq.GameData(a.data), MAP_DIRS[MAP_ID])
    grid = Grid([p[0] for p in stops], [p[1] for p in stops], a.margin)
    mask = road_mask(terrain, names, grid)

    snapped = []
    for s, (x, y) in zip(route["stops"], stops):
        at = snap(mask, grid, x, y)
        if at is None:
            sys.exit("stop %s (%.1f %.1f) is farther than %g yd from any road: move it"
                     % (s["key"], x, y, SNAP_YD))
        print("stop %s: (%.1f %.1f) snapped %.1f yd to (%.1f %.1f)"
              % (s["key"], x, y, math.dist((x, y), at), at[0], at[1]))
        snapped.append(at)

    xy = [snapped[0]]
    for p, q in zip(snapped, snapped[1:]):
        xy += trace(mask, grid, p, q)[1:]
    path = [(x, y, terrain.ground_z(x, y)) for x, y in xy]
    if any(z is None for _, _, z in path):
        sys.exit("a path point has no terrain under it")
    for x, y, z in path:
        w = terrain.water_z(x, y)
        if w is not None and w > z:
            print("warning: path point (%.1f %.1f) is %.1f yd under water: a bridge?" % (x, y, w - z))
    on_road = sum(mask[grid.index(x, y)] for x, y in xy)
    length = path_length([(x, y) for x, y, _ in path])
    straight = sum(math.dist(p, q) for p, q in zip(snapped, snapped[1:]))
    leg = length / carts["cart"]["speed_yd_s"]
    run = a.route if a.margin == MARGIN_YD else "%s --margin %g" % (a.route, a.margin)
    note = "road_path.py %s: %.1f yd, a %.1f s leg at %g yd/s" % (run, length, leg, carts["cart"]["speed_yd_s"])
    stop_xyz = [(x, y, terrain.ground_z(x, y)) for x, y in snapped]
    with open(CARTS, encoding="utf-8", newline="") as f:
        text = f.read()
    with open(CARTS, "w", encoding="utf-8", newline="") as f:
        f.write(write_route(text, a.route, stop_xyz, path, note))
    print("%s: %.1f yd (%.2fx the stops' %.1f yd), %d points, %d on the road, a %.1f s leg at %g yd/s"
          % (a.route, length, length / straight, straight, len(path), on_road, leg,
             carts["cart"]["speed_yd_s"]))


if __name__ == "__main__":
    main()
