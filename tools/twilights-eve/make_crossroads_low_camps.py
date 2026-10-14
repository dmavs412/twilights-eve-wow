#!/usr/bin/env python3
"""Write data/twilights-eve/crossroads_low_camps.json: hand-placed level 1-6 camps near the Crossroads.

The Barrens starts at level 10, so these camps borrow real low-level creatures from
Mulgore and Durotar (entries, names, levels, display IDs and faction templates read
from the vmangos DB) and put them on open ground around the Crossroads. The camp
centres below were picked by hand from a terrain survey (flat ground, Barrens area,
no WMO, off the Gold Road and the Ratchet/Lushwater roads, 200-350 yd out so the
low camps are closest to town). Member offsets are fixed, so output is stable.

z_hint is the terrain height from the ADT heightmap at the member's x/y. The host
must still resolve ground Z at runtime (the Local Realm's terrain trait); z_hint is
for tooling and sanity checks only.

  python -I tools/twilights-eve/make_crossroads_low_camps.py --db mangos.sqlite [--data <WoW Data>]
"""

from __future__ import annotations

import argparse
import json
import math
import os
import sqlite3
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import wowmpq  # noqa: E402
import import_barrens_spawns as imp  # noqa: E402

CROSSROADS = (-441.0, -2596.0)

# (camp key, centre x, y, members as (entry, dx, dy), wander radius, note)
# +x is north, +y is west on map 1.
CAMPS = [
    ("plainstriders", -614.0, -2496.0,
     [(2955, 0, 0), (2955, 10, 7), (2955, -9, 9), (2955, 6, -11), (2955, -8, -8)], 6.0,
     "Mulgore's level 1-2 plainstriders, south-west of town. Neutral: a level-1 player picks the fight."),
    ("mottled_boars", -506.0, -2837.0,
     [(3098, 0, 0), (3098, 11, 5), (3098, -7, 10), (3098, 4, -12), (3098, -11, -5)], 6.0,
     "Durotar's level 1-2 boars, south-east of town."),
    ("scorpid_workers", -376.0, -2837.0,
     [(3124, 0, 0), (3124, 9, 8), (3124, -10, 6), (3124, 7, -9)], 5.0,
     "Durotar's level 3 scorpids, east of town toward Thorn Hill (keep Thorn Hill's Razormane, 10-13, further out)."),
    ("vile_familiars", -731.0, -2518.0,
     [(3101, 0, 0), (3101, 8, 9), (3101, -9, 7), (3101, 5, -10), (3101, -7, -9)], 4.0,
     "Durotar's level 3-4 imps, south of the plainstriders. Hostile. Their faction template has no call-for-help flag, so only the camp rule pulls them together."),
    ("bristleback_camp", -316.0, -2379.0,
     [(2952, 0, 0), (2952, 10, 6), (2953, -6, 10), (2952, -11, -3), (2953, 4, -11), (2954, 14, -8)], 2.0,
     "A Mulgore quilboar camp, north-west of town: quilboars, shamans and a battleboar (level 3-5). Hostile; faction 111 answers calls for help."),
    ("prairie_wolves", -744.0, -2771.0,
     [(2958, 0, 0), (2958, 12, 6), (2958, -8, 11), (2958, 6, -12)], 7.0,
     "Mulgore's level 5-6 wolves, the furthest low camp (south-east). Hostile; faction 38 answers calls for help."),
]

RESPAWN_SECS = 60  # spec: ~60 s after the player leaves the area
# Imported Barrens spawns (level 10+) whose home is this close to a low camp's centre are
# listed in suppress_guids: a level-1 player should not meet a Savannah Huntress at the boars.
SUPPRESS_YD = 70.0


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--db", required=True)
    ap.add_argument("--data", default=os.environ.get("WOW_DATA", r"C:\Users\davis\wow-data\Data"))
    ap.add_argument("--out", default=os.path.join(imp.repo_root(), "data", "twilights-eve", "crossroads_low_camps.json"))
    a = ap.parse_args()

    db = sqlite3.connect("file:%s?mode=ro" % a.db.replace("\\", "/"), uri=True)
    db.row_factory = sqlite3.Row
    templates = {}
    for r in db.execute("select * from creature_template where patch <= ? order by entry, patch", (imp.PATCH,)):
        templates[r["entry"]] = dict(r)
    fts = {r[0]: imp.FT(tuple(r)) for r in db.execute("select * from faction_template")}
    factions = {r["id"]: dict(r) for r in db.execute("select * from faction order by id, build")}
    player_fts = {**{k: fts[v] for k, v in imp.HORDE_PLAYER_FT.items()},
                  **{k: fts[v] for k, v in imp.ALLIANCE_PLAYER_FT.items()}}
    spawn_counts = {r[0]: r[1] for r in db.execute("select id, count(*) from creature where map = 1 group by id")}

    gd = wowmpq.GameData(a.data)
    terrain = wowmpq.Terrain(gd, "Kalimdor")
    area_rows, area_strs = gd.dbc("AreaTable")
    area_names = {r[0]: area_strs[r[11]: area_strs.index(b"\0", r[11])].decode() for r in area_rows}

    imported = []
    spawns_path = os.path.join(os.path.dirname(a.out), "barrens_spawns.json")
    if os.path.exists(spawns_path):
        with open(spawns_path, encoding="utf-8") as f:
            imported = json.load(f)["spawns"]
    else:
        print("warning: %s missing; suppress_guids left empty" % spawns_path)

    camps = []
    guid = 9_000_001  # outside vmangos' guid range for map 1; the realm may renumber
    for n, (key, cx, cy, members, wander, note) in enumerate(CAMPS, start=1):
        out_members = []
        for entry, dx, dy in members:
            t = templates[entry]
            assert t["npc_flags"] == 0, entry
            assert spawn_counts.get(entry, 0) > 0, "entry %d has no spawns on map 1" % entry
            ft = fts[t["faction"]]
            frow = factions.get(ft.faction)
            rx = {race: imp.reaction(ft, pft, frow, race) for race, pft in player_fts.items()}
            horde = imp.side_reaction([rx[r] for r in imp.HORDE_PLAYER_FT])
            alliance = imp.side_reaction([rx[r] for r in imp.ALLIANCE_PLAYER_FT])
            assert "friendly" not in (horde, alliance), entry
            x, y = cx + dx, cy + dy
            zh = terrain.ground_z(x, y)
            area = terrain.area_id(x, y)
            assert imp.ZONE_BARRENS in (area, next((r[2] for r in area_rows if r[0] == area), None)), (key, area)
            displays = [(t["display_id%d" % i], t["display_probability%d" % i]) for i in range(1, 5) if t["display_id%d" % i]]
            out_members.append({
                "guid": guid, "entry": entry, "name": t["name"],
                "x": round(x, 2), "y": round(y, 2), "z_hint": round(zh, 2) if zh is not None else None,
                "orientation": round(math.atan2(cy - y, cx - x) % (2 * math.pi), 3) if (dx or dy) else 0.0,
                "display_id": max(displays, key=lambda d: d[1])[0],
                "level_min": t["level_min"], "level_max": t["level_max"],
                "faction_template": t["faction"],
                "reaction": "hostile" if "hostile" in (horde, alliance) else "neutral",
                "wander_radius": wander,
                "respawn_secs": RESPAWN_SECS,
                "area_id": area, "area_name": area_names.get(area, ""),
                "vmangos_spawns_on_map1": spawn_counts.get(entry, 0),
            })
            guid += 1
        lv = [m["level_min"] for m in out_members] + [m["level_max"] for m in out_members]
        camps.append({
            "camp_id": "low-%d" % n, "key": key,
            "center": [cx, cy],
            "distance_from_crossroads_yd": round(math.hypot(cx - CROSSROADS[0], cy - CROSSROADS[1])),
            "level_min": min(lv), "level_max": max(lv),
            "faction_template": out_members[0]["faction_template"],
            "assists": all(fts[m["faction_template"]].flags & imp.FT_RESPOND_TO_CALL_FOR_HELP for m in out_members),
            "note": note,
            "suppress_radius_yd": SUPPRESS_YD,
            "suppress_guids": sorted(s["guid"] for s in imported
                                     if math.hypot(s["x"] - cx, s["y"] - cy) <= SUPPRESS_YD),
            "members": out_members,
        })

    out = {
        "schema": 1,
        "purpose": "Hand-placed level 1-6 camps near the Crossroads (Twilight's Eve ticket #7). Not vanilla spawns.",
        "crossroads": list(CROSSROADS),
        "map": 1,
        "ground_z": "z_hint is the ADT terrain height; the host resolves ground Z at runtime",
        "source": "creature entries, names, levels, display IDs and factions from the vmangos world DB (see barrens_spawns.json source)",
        "camps": camps,
    }
    with open(a.out, "w", encoding="utf-8", newline="\n") as f:
        f.write(imp.dump_json(out))
    print("wrote %s" % a.out)
    for c in camps:
        print(c["camp_id"], c["key"], c["level_min"], c["level_max"], c["distance_from_crossroads_yd"],
              [m["z_hint"] for m in c["members"]], c["members"][0]["reaction"], c["assists"])


if __name__ == "__main__":
    main()
