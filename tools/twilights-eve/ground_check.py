#!/usr/bin/env python3
"""Check (and with --fix, rewrite) the Z of every placed unit in the realm content TOML.

A row is any table with a `position = [x, y, z]` line. Its Z is compared with the ADT terrain
height. A row inside a WMO placement box (with a 2 yd margin, Z range included) is interior: it
must stand on one of the building's floors (wmo_floor.py) or on the terrain, within the same
limits. `--fix` moves an interior row only onto a floor within 2 yd of its Z; farther off, a
building has several floors and only its author knows which one, so the row is flagged with the
nearest floors and its author writes the Z. ground_check_off_floor.txt lists the interior rows
that were already off every floor when this check began; they pass until fixed, and a listed row
that stands on a floor fails until its line is deleted. Rows with `grounded = false` and maps
without terrain data are left alone.

Exit 1 when any row is flagged (or fixed), 0 otherwise.

    python3.15 tools/twilights-eve/ground_check.py [--fix] [--file <path>]

Reads the game data from $WOW_DATA (default C:\\Users\\davis\\wow-data\\Data).
"""
import argparse
import glob
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import wmo_floor  # noqa: E402
import wowmpq  # noqa: E402

CONTENT = os.path.normpath(os.path.join(HERE, "..", "..", "crates", "twilight-realm", "content"))
DEFAULT_DATA = r"C:\Users\davis\wow-data\Data"
# Same table as import_barrens_spawns.MAP_DIRS (that module runs its import on load).
MAP_DIRS = {0: "Azeroth", 1: "Kalimdor"}
# Files whose rows stand on Kalimdor unless the row says otherwise (hub.rs and mobs.rs default
# to it); every other file defaults to its top-level `map`, else 0.
FILE_DEFAULT_MAP = {"hub.toml": 1, "mobs.toml": 1}
# Hand `[[spawn]]` rows in mobs.toml all load onto Kalimdor (mobs.rs), but this one has Elwynn
# coordinates. Its map is a content bug for its own ticket; checking it against Kalimdor's terrain
# would only bake a wrong Z in.
KNOWN_WRONG_MAP = {("mobs.toml", "guid 9000101")}
# Test dungeons whose realm tests (creature_gaze, phased_wards, test_grounds and others) stand on a
# flat plane at z 47.2 and hard-code that height; their Z is a test convention, not terrain.
TEST_FIXTURES = {"gaze_hall.toml", "ward_test.toml", "test_grounds.toml"}
MARGIN = 2.0
ABOVE_LIMIT = 1.0
BELOW_LIMIT = 0.5
# `--fix` moves an interior row onto the nearest floor only within this many yards.
FIX_INTERIOR_YD = 2.0
# The interior rows off every floor when the floor check began, one per line:
# `<file> <x> <y> <label>`. Delete a line when its row is fixed.
KNOWN_OFF_FLOOR = os.path.join(HERE, "ground_check_off_floor.txt")

HEADER = re.compile(r"^\s*\[")
NAME = re.compile(r'^\s*name\s*=\s*"([^"]*)"')
GUID = re.compile(r"^\s*guid\s*=\s*(\d+)")
MAP = re.compile(r"^\s*map\s*=\s*(\d+)")
GROUNDED = re.compile(r"^\s*grounded\s*=\s*(true|false)")
POSITION = re.compile(
    r"^(?P<head>\s*position\s*=\s*\[\s*)(?P<x>[^,\]\s]+)(?P<s1>\s*,\s*)(?P<y>[^,\]\s]+)"
    r"(?P<s2>\s*,\s*)(?P<z>[^,\]\s]+)(?P<tail>.*)$",
    re.S,
)


def rows(path):
    """Yield (line_index, label, map, grounded, x, y, z) for each 3D position line.

    A table's `map` and `grounded` may sit on either side of its position line, so a row is
    emitted when the next header or the end of the file closes the table."""
    with open(path, encoding="utf-8", newline="") as f:
        lines = f.read().split("\n")
    file_map = FILE_DEFAULT_MAP.get(os.path.basename(path), 0)
    label = None
    cur = None  # dict for the open table
    out = []

    def close():
        if cur and cur["pos"] is not None:
            out.append((cur["pos"], cur["label"], cur["map"] if cur["map"] is not None else file_map,
                        cur["grounded"], *cur["xyz"]))

    seen_header = False
    for i, raw in enumerate(lines):
        line = raw.rstrip("\r")
        if HEADER.match(line):
            close()
            seen_header = True
            label = None
            cur = {"pos": None, "label": None, "map": None, "grounded": True, "xyz": None}
            continue
        m = MAP.match(line)
        if m:
            if seen_header:
                cur["map"] = int(m.group(1))
            else:
                file_map = int(m.group(1))
            continue
        if not seen_header:
            continue
        m = NAME.match(line) or None
        if m:
            label = m.group(1)
            continue
        m = GUID.match(line)
        if m:
            label = "guid " + m.group(1)
            continue
        m = GROUNDED.match(line)
        if m:
            cur["grounded"] = m.group(1) == "true"
            continue
        m = POSITION.match(line)
        if m:
            try:
                xyz = (float(m.group("x")), float(m.group("y")), float(m.group("z")))
            except ValueError:
                continue
            cur["pos"] = i
            cur["label"] = label
            cur["xyz"] = xyz
    close()
    return lines, out


def interior(terrain, x, y, z):
    """True when (x, y, z) lies in a WMO placement box on its tile, margin included."""
    t, _, _ = terrain._tile_at(x, y)
    if not t:
        return False
    for _path, _ns, xs, ys, zs in t["wmos"]:
        if (xs[0] - MARGIN <= x <= xs[1] + MARGIN and ys[0] - MARGIN <= y <= ys[1] + MARGIN
                and zs[0] - MARGIN <= z <= zs[1] + MARGIN):
            return True
    return False


def stands_on(z, height):
    """True when a unit at `z` stands on a surface at `height`, within the limits."""
    d = z - height
    return -BELOW_LIMIT <= d <= ABOVE_LIMIT


def interior_floors(floors, terrain, x, y):
    """The heights an interior row may stand on: the buildings' floors over it, and the terrain."""
    heights = [s.z for s in floors.surfaces(x, y, MARGIN) if s.floor]
    g = terrain.ground_z(x, y)
    return heights + ([g] if g is not None else [])


def off_floor_key(name, label, x, y):
    return "%s %.1f %.1f %s" % (name, x, y, label or "?")


def known_off_floor():
    if not os.path.exists(KNOWN_OFF_FLOOR):
        return set()
    with open(KNOWN_OFF_FLOOR, encoding="utf-8") as f:
        return {line.strip() for line in f if line.strip() and not line.startswith("#")}


def set_z(lines, idx, z):
    raw = lines[idx]
    cr = "\r" if raw.endswith("\r") else ""
    m = POSITION.match(raw.rstrip("\r"))
    lines[idx] = (m.group("head") + m.group("x") + m.group("s1") + m.group("y")
                  + m.group("s2") + "%.2f" % z + m.group("tail") + cr)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--fix", action="store_true")
    ap.add_argument("--file")
    a = ap.parse_args()

    data = os.environ.get("WOW_DATA", DEFAULT_DATA)
    gd = wowmpq.GameData(data)
    terrains = {}
    floors_by_map = {}
    paths = [a.file] if a.file else sorted(glob.glob(os.path.join(CONTENT, "*.toml")))
    verb = "fixed" if a.fix else "flagged"
    counts = {"checked": 0, verb: 0, "interior": 0, "off-floor": 0, "known": 0, "no-terrain": 0}
    known = known_off_floor()
    per_file = {}

    for path in paths:
        if os.path.basename(path) in TEST_FIXTURES:
            print("skipped %s (test fixture on a flat plane)" % os.path.basename(path))
            continue
        lines, found = rows(path)
        changed = False
        for idx, label, map_id, grounded, x, y, z in found:
            if not grounded:
                continue
            if (os.path.basename(path), label) in KNOWN_WRONG_MAP:
                print("skipped %s:%d %s (wrong map in content)" % (os.path.basename(path), idx + 1, label))
                continue
            if map_id not in MAP_DIRS:
                counts["interior"] += 1
                continue
            terrain = terrains.get(map_id)
            if terrain is None:
                terrain = terrains[map_id] = wowmpq.Terrain(gd, MAP_DIRS[map_id])
            if interior(terrain, x, y, z):
                counts["interior"] += 1
                if map_id not in floors_by_map:
                    floors_by_map[map_id] = wmo_floor.Floors(gd, terrain)
                name = os.path.basename(path)
                key = off_floor_key(name, label, x, y)
                heights = interior_floors(floors_by_map[map_id], terrain, x, y)
                if any(stands_on(z, h) for h in heights):
                    if key in known:
                        counts["off-floor"] += 1
                        print("on a floor now %s:%d %s: delete its line from %s"
                              % (name, idx + 1, label or "?", os.path.basename(KNOWN_OFF_FLOOR)))
                    continue
                nearest = min(heights, key=lambda h: abs(z - h), default=None)
                if a.fix and key not in known and nearest is not None and abs(z - nearest) <= FIX_INTERIOR_YD:
                    counts[verb] += 1
                    per_file[name] = per_file.get(name, 0) + 1
                    print("fixed %s:%d %s z=%.2f floor=%.2f diff=%+.2f"
                          % (name, idx + 1, label or "?", z, nearest, z - nearest))
                    set_z(lines, idx, nearest)
                    changed = True
                    continue
                if key in known:
                    counts["known"] += 1
                    continue
                counts["off-floor"] += 1
                per_file[name] = per_file.get(name, 0) + 1
                below = max((h for h in heights if h < z), default=None)
                above = min((h for h in heights if h > z), default=None)
                print("off-floor %s:%d %s z=%.2f: nearest floor below %s, above %s (wmo_floor.py %.1f %.1f)"
                      % (name, idx + 1, label or "?", z,
                         "none" if below is None else "%.2f" % below,
                         "none" if above is None else "%.2f" % above, x, y))
                continue
            g = terrain.ground_z(x, y)
            if g is None:
                counts["no-terrain"] += 1
                continue
            counts["checked"] += 1
            d = z - g
            if not (d > ABOVE_LIMIT or -d > BELOW_LIMIT):
                continue
            counts[verb] += 1
            per_file[os.path.basename(path)] = per_file.get(os.path.basename(path), 0) + 1
            print("%s %s:%d %s z=%.2f terrain=%.2f diff=%+.2f"
                  % (verb, os.path.basename(path), idx + 1, label or "?", z, g, d))
            if a.fix:
                set_z(lines, idx, g)
                changed = True
        if changed:
            with open(path, "w", encoding="utf-8", newline="") as f:
                f.write("\n".join(lines))

    for name, n in sorted(per_file.items()):
        print("  %s: %d" % (name, n))
    print("checked %d, %s %d, interior %d (off-floor %d, known off-floor %d), no-terrain %d"
          % (counts["checked"], verb, counts[verb], counts["interior"], counts["off-floor"],
             counts["known"], counts["no-terrain"]))
    return 1 if counts[verb] or counts["off-floor"] else 0


if __name__ == "__main__":
    sys.exit(main())
