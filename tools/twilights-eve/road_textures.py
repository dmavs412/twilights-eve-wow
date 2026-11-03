#!/usr/bin/env python3
"""List the terrain textures of Elwynn, Stormwind, Duskwood and Redridge, to pick roads.toml's.

Walks every map 0 ADT tile with a chunk whose MCNK area rolls up (AreaTable's parent column) to
one of ZONES, and prints each distinct MTEX name with the count of those tiles that list it, most
used first. The output is a TOML comment block: roads.toml carries it so its choice of road,
cobble, path and street textures can be checked against the list (spec #560).

    python3.15 tools/twilights-eve/road_textures.py [--data <dir>]

Reads the game data from --data or $WOW_DATA (default C:\\Users\\davis\\wow-data\\Data).
"""
import argparse
import collections
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import closed_zones  # noqa: E402
import wowmpq  # noqa: E402

DEFAULT_DATA = r"C:\Users\davis\wow-data\Data"
MAP_ID = 0
MAP_DIR = "Azeroth"
# AreaTable zone ids: Elwynn Forest, Stormwind City, Duskwood, Redridge Mountains.
ZONES = (12, 1519, 10, 44)
TILES = 64


def texture_counts(tiles):
    """(name, count) per distinct MTEX name over `tiles` (each a list of MTEX names), most used
    first, then by name. A name counts once per tile."""
    counts = collections.Counter()
    for names in tiles:
        counts.update(set(names))
    return sorted(counts.items(), key=lambda kv: (-kv[1], kv[0].lower()))


def render(zone_names, n_tiles, counts):
    """The comment block: a header line, then one `#  <tiles>  <name>` line per texture."""
    out = ["# road_textures.py: map %d, %d tiles of %s" % (MAP_ID, n_tiles, ", ".join(zone_names)),
           "# tiles  texture"]
    out += ["# %5d  %s" % (n, name) for name, n in counts]
    return "\n".join(out)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--data", default=os.environ.get("WOW_DATA", DEFAULT_DATA))
    a = ap.parse_args()

    gd = wowmpq.GameData(a.data)
    rows, strings = gd.dbc("AreaTable")
    parent = {r[0]: r[2] for r in rows}
    names = closed_zones.area_names(rows, strings)
    terrain = wowmpq.Terrain(gd, MAP_DIR)
    tiles = []
    for tx in range(TILES):
        for ty in range(TILES):
            t = terrain._tile(tx, ty)
            if t and any(closed_zones.roll_up(c[0], parent) in ZONES for c in t["chunks"].values()):
                tiles.append(t["textures"])
            # Drop the tile: the walk needs only its MTEX, not the whole map held in memory.
            terrain.tiles.pop((tx, ty), None)
    print(render([names.get(z, str(z)) for z in ZONES], len(tiles), texture_counts(tiles)))


if __name__ == "__main__":
    main()
