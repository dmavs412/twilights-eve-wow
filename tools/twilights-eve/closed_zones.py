#!/usr/bin/env python3
"""Write data/twilights-eve/closed_chunks.json from closed_zones.toml and the Azeroth ADTs.

A chunk is closed when its MCNK area id rolls up through AreaTable's parent column to a zone
listed in closed_zones.toml (ADR 0015). Chunks are on the 1024 x 1024 grid:
`gx = tile_x * 16 + chunk_ix`, `gy = tile_y * 16 + chunk_iy`.

    python3.15 tools/twilights-eve/closed_zones.py [--data <dir>]

Reads the game data from --data or $WOW_DATA (default C:\\Users\\davis\\wow-data\\Data).
"""
import argparse
import json
import os
import sys
import tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

DATA_DIR = os.path.normpath(os.path.join(HERE, "..", "..", "data", "twilights-eve"))
TOML_PATH = os.path.join(DATA_DIR, "closed_zones.toml")
JSON_PATH = os.path.join(DATA_DIR, "closed_chunks.json")
DEFAULT_DATA = r"C:\Users\davis\wow-data\Data"
# Same table as ground_check.MAP_DIRS.
MAP_DIRS = {0: "Azeroth", 1: "Kalimdor"}
MAX_STEPS = 8
TILES = 64
CHUNKS_PER_TILE = 16


def roll_up(area, parent):
    """The top-level zone of `area`: follow `parent` until it is 0, at most MAX_STEPS steps."""
    steps = 0
    while parent.get(area, 0) and steps < MAX_STEPS:
        area = parent[area]
        steps += 1
    return area


def grid_index(tile_x, tile_y, chunk_ix, chunk_iy):
    return tile_x * CHUNKS_PER_TILE + chunk_ix, tile_y * CHUNKS_PER_TILE + chunk_iy


def closed_chunks(tiles, parent, closed):
    """Sorted `[gx, gy]` of every chunk whose zone is in `closed`, plus a count per area id.

    `tiles` maps (tile_x, tile_y) to {(chunk_ix, chunk_iy): area_id}.
    """
    found = {}
    per_area = {}
    for (tx, ty), chunks in tiles.items():
        for (ix, iy), area in chunks.items():
            if roll_up(area, parent) in closed:
                found[grid_index(tx, ty, ix, iy)] = area
    for area in found.values():
        per_area[area] = per_area.get(area, 0) + 1
    return [list(g) for g in sorted(found)], per_area


def render(map_id, zones, chunks):
    """The JSON text: the header on one line, one `[gx, gy]` per line."""
    head = '{"map": %d, "zones": %s, "chunks": [' % (map_id, json.dumps(sorted(zones)))
    body = ",\n".join(json.dumps(c) for c in chunks)
    return head + ("\n" + body + "\n" if chunks else "") + "]}\n"


def load_config(path):
    with open(path, "rb") as f:
        cfg = tomllib.load(f)
    return int(cfg["map"]), [int(z) for z in cfg["zones"]]


def area_names(rows, strings):
    names = {}
    for r in rows:
        off = r[11]
        names[r[0]] = strings[off : strings.index(b"\0", off)].decode("utf-8", "replace")
    return names


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--data", default=os.environ.get("WOW_DATA", DEFAULT_DATA))
    args = ap.parse_args()

    import wowmpq

    map_id, zones = load_config(TOML_PATH)
    if map_id not in MAP_DIRS:
        sys.exit("closed_zones.toml: unknown map %d" % map_id)
    gd = wowmpq.GameData(args.data)
    rows, strings = gd.dbc("AreaTable")
    parent = {r[0]: r[2] for r in rows}
    names = area_names(rows, strings)

    terrain = wowmpq.Terrain(gd, MAP_DIRS[map_id])
    tiles = {}
    for tx in range(TILES):
        for ty in range(TILES):
            t = terrain._tile(tx, ty)
            if t:
                tiles[(tx, ty)] = {k: v[0] for k, v in t["chunks"].items()}
    chunks, per_area = closed_chunks(tiles, parent, set(zones))

    with open(JSON_PATH, "w", newline="\n", encoding="utf-8") as f:
        f.write(render(map_id, zones, chunks))

    per_zone = {}
    for area, n in per_area.items():
        z = roll_up(area, parent)
        per_zone[z] = per_zone.get(z, 0) + n
    for z in sorted(per_zone):
        print("zone %d %s: %d chunks" % (z, names.get(z, "?"), per_zone[z]))
    for area, n in sorted(per_area.items(), key=lambda kv: (-kv[1], kv[0])):
        print("  area %d %s: %d" % (area, names.get(area, "?"), n))
    print("%d closed chunks -> %s" % (len(chunks), JSON_PATH))


if __name__ == "__main__":
    main()
