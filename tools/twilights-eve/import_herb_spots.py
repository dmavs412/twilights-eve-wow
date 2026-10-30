#!/usr/bin/env python3
"""Fill each `[[zone]]` row's `points` in content/herbs.toml with the zone's Herb Spots: the vanilla
herb spawn points of the vmangos world DB (spec #435, ADR 0024).

Offline, one-off, re-runnable: the same inputs give the same file. Modelled on
import_barrens_spawns.py. A re-run brings back the spots ground_check.py --fix moved or #437
deleted, so run ground_check.py after it.

Inputs:
  --db    vmangos world database as SQLite (mangos.sqlite)
  --data  WoW 1.12.1 Data directory with the MPQs (ADT area IDs, Lock.dbc, AreaTable.dbc)

A spawn is a `gameobject` row on map 0 whose template has type 3 (chest) and a `data0` lock that
Lock.dbc gives a Herbalism slot (Type 2, Index 2). Its zone is the ADT chunk's area, then
AreaTable's parent zone, as import_barrens_spawns.py decides it. A spawn within 5 yards of one
already kept in the same zone is dropped. A spot's Z is the ADT terrain's at its rounded x and y,
the height ground_check.py checks it against, not vmangos's `position_z` (#477); a spawn with no
terrain under it is dropped. Only the `points = [...]` line of each `[[zone]]` row is rewritten.

Standard library only. Run with -I.
"""

from __future__ import annotations

import argparse
import math
import os
import re
import sqlite3
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

MAP_AZEROTH = 0
PATCH = 10  # vmangos patch index for 1.12
SPACING_YD = 5.0
LOCK_TYPE_SKILL = 2  # Lock.dbc Type: the slot names a LockType
LOCK_HERBALISM = 2  # LockType.dbc: Herbalism
GO_TYPE_CHEST = 3
SPAWN_FLAG_DISABLED = 0x02
DEFAULT_DB = r"C:\Users\davis\AppData\Local\Temp\vmangos\sqlite-dump\mangos.sqlite"
DEFAULT_DATA = r"C:\Users\davis\wow-data\Data"

ZONE_HEADER = re.compile(r"^\[\[zone\]\]\s*$")
ZONE_ID = re.compile(r"^zone\s*=\s*(\d+)\s*$")
POINTS_LINE = re.compile(r"^points\s*=\s*\[.*\]\s*$")


def repo_root():
    return os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))


def dedupe(points, spacing=SPACING_YD):
    """The points in order, minus any within `spacing` yards of one already kept."""
    kept = []
    for x, y in points:
        if all(math.hypot(x - kx, y - ky) >= spacing for kx, ky in kept):
            kept.append((x, y))
    return kept


def with_z(points, ground_z):
    """Each (x, y) rounded to one decimal place, as (x, y, z) with `ground_z(x, y)` there; a point
    `ground_z` gives None is dropped."""
    out = []
    for x, y in points:
        rx, ry = round(x, 1) + 0.0, round(y, 1) + 0.0
        z = ground_z(rx, ry)
        if z is not None:
            out.append((rx, ry, z))
    return out


def points_line(points):
    """`points = [{ x = .., y = .., z = .. }, ...]`, x and y rounded to one decimal place and z to
    two (as ground_check.py writes it), sorted by x then y."""
    rounded = sorted({(round(x, 1) + 0.0, round(y, 1) + 0.0, round(z, 2) + 0.0) for x, y, z in points})
    body = ", ".join("{ x = %.1f, y = %.1f, z = %.2f }" % p for p in rounded)
    return "points = [%s]" % body if rounded else "points = []"


def rewrite_points(text, by_zone):
    """`text` with the `points = ...` line of each `[[zone]]` row replaced by that zone id's
    points in `by_zone`. Every other line stays byte for byte; a zone not in `by_zone` is kept."""
    out = []
    zone = None
    in_zone = False
    for line in text.split("\n"):
        stripped = line.rstrip("\r")
        if stripped.startswith("["):
            in_zone = bool(ZONE_HEADER.match(stripped))
            zone = None
        elif in_zone:
            m = ZONE_ID.match(stripped)
            if m:
                zone = int(m.group(1))
            elif POINTS_LINE.match(stripped) and zone in by_zone:
                eol = line[len(stripped):]
                line = points_line(by_zone[zone]) + eol
        out.append(line)
    return "\n".join(out)


def zone_ids(text):
    """The AreaTable ids of the `[[zone]]` rows, in file order."""
    ids = []
    in_zone = False
    for line in text.split("\n"):
        stripped = line.rstrip("\r")
        if stripped.startswith("["):
            in_zone = bool(ZONE_HEADER.match(stripped))
        elif in_zone:
            m = ZONE_ID.match(stripped)
            if m:
                ids.append(int(m.group(1)))
    return ids


def herbalism_locks(lock_rows):
    """Lock ids with a slot of Type 2 (a LockType) whose Index is Herbalism.
    Lock.dbc: 0 id, 1-8 Type, 9-16 Index, 17-24 Skill, 25-32 Action."""
    return {r[0] for r in lock_rows
            if any(r[1 + i] == LOCK_TYPE_SKILL and r[9 + i] == LOCK_HERBALISM for i in range(8))}


def parse_args():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--db", default=DEFAULT_DB, help="path to vmangos mangos.sqlite")
    ap.add_argument("--data", default=os.environ.get("WOW_DATA", DEFAULT_DATA),
                    help="WoW 1.12.1 Data dir with MPQs (default: $WOW_DATA)")
    ap.add_argument("--file", default=os.path.join(repo_root(), "crates", "twilight-realm", "content", "herbs.toml"),
                    help="herbs.toml to rewrite")
    return ap.parse_args()


def main():
    import wowmpq

    a = parse_args()
    with open(a.file, "rb") as f:
        text = f.read().decode("utf-8")
    wanted = set(zone_ids(text))

    db = sqlite3.connect("file:%s?mode=ro" % a.db.replace("\\", "/"), uri=True)
    db.row_factory = sqlite3.Row

    gd = wowmpq.GameData(a.data)
    terrain = wowmpq.Terrain(gd, "Azeroth")
    locks = herbalism_locks(gd.dbc("Lock")[0])
    area_rows, _ = gd.dbc("AreaTable")
    parent = {r[0]: r[2] for r in area_rows}

    templates = {}
    for r in db.execute("select entry, data0 from gameobject_template where type = ? and patch <= ? "
                        "order by entry, patch", (GO_TYPE_CHEST, PATCH)):
        templates[r["entry"]] = r["data0"]
    herb_entries = {e for e, lock in templates.items() if lock in locks}

    rows = db.execute("select * from gameobject where map = ? and patch_min <= ? and patch_max >= ? "
                      "order by guid", (MAP_AZEROTH, PATCH, PATCH)).fetchall()
    found = {z: [] for z in wanted}
    for g in rows:
        if g["id"] not in herb_entries or g["spawn_flags"] & SPAWN_FLAG_DISABLED:
            continue
        x, y = g["position_x"], g["position_y"]
        area = terrain.area_id(x, y)
        if area is None:
            continue
        zone = parent.get(area) or area
        if zone in found:
            found[zone].append((x, y))

    kept = {}
    for zone, pts in found.items():
        # Sort first so the spacing rule keeps the same points on every run.
        spaced = dedupe(sorted((round(x, 3), round(y, 3)) for x, y in pts))
        kept[zone] = with_z(spaced, terrain.ground_z)
        print("zone %d: %d spawns, %d kept, %d without terrain"
              % (zone, len(pts), len(kept[zone]), len(spaced) - len(kept[zone])))

    with open(a.file, "wb") as f:
        f.write(rewrite_points(text, kept).encode("utf-8"))


if __name__ == "__main__":
    main()
