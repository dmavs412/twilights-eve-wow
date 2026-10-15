#!/usr/bin/env python3
"""Import The Barrens' hostile and neutral creature spawns from a vmangos world DB.

Offline, one-off, re-runnable. Writes data/twilights-eve/barrens_spawns.json, which
is committed, so building the game never needs the database. See
docs/reference/barrens-spawns.md for the full write-up.

Inputs:
  --db    vmangos world database as SQLite (mangos.sqlite from the
          db-sqlite-<sha>.zip asset of the vmangos/core `db_latest` release)
  --data  WoW 1.12.1 Data directory with the MPQs (for ADT area IDs)

Zone membership is decided per spawn from the ADT terrain chunk it stands on
(MCNK area ID; under the surface, the enclosing WMO's WMOAreaTable area) and
AreaTable.dbc's parent zone: the same lookup the game does. vmangos' `creature`
table has no zone column.

Standard library only. Run with -I when the DB came from the internet.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import sqlite3
import sys
from collections import defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import wowmpq  # noqa: E402

MAP_KALIMDOR = 1
ZONE_BARRENS = 17
PATCH = 10  # vmangos patch index for 1.12

# Faction templates of the eight player races (ChrRaces.dbc FactionID).
HORDE_PLAYER_FT = {"orc": 2, "undead": 5, "tauren": 6, "troll": 116}
ALLIANCE_PLAYER_FT = {"human": 1, "dwarf": 3, "night_elf": 4, "gnome": 115}

# creature_template.type
CREATURE_TYPES = {0: "none", 1: "beast", 2: "dragonkin", 3: "demon", 4: "elemental", 5: "giant",
                  6: "undead", 7: "humanoid", 8: "critter", 9: "mechanical", 10: "not_specified",
                  11: "totem", 12: "non_combat_pet"}
EXCLUDED_TYPES = {8, 11, 12}  # critter, totem, non-combat pet
RANKS = {0: "normal", 1: "elite", 2: "rare_elite", 3: "boss", 4: "rare"}

SPAWN_FLAG_DISABLED = 0x02
SPAWN_FLAG_NOT_VISIBLE = 0x40
FLAG_EXTRA_INVISIBLE = 0x80
FLAG_EXTRA_GUARD = 0x400
STATIC_UNINTERACTIBLE = 0x200  # UNIT_FLAG_NOT_SELECTABLE on spawn
FT_RESPOND_TO_CALL_FOR_HELP = 0x01

# Prefilter box (map 1) before the exact ADT zone test. Generously larger than The Barrens.
BOX = (-5200.0, 2200.0, -5200.0, 400.0)  # min_x, max_x, min_y, max_y

UNDERGROUND_YD = 3.0  # a spawn this far below the terrain surface is inside a cave/WMO
CAMP_LINK_YD = 25.0  # single-linkage distance for camp clustering
CAMP_MAX_RADIUS_YD = 30.0  # wider single-linkage clusters are split by recursive 2-means


def parse_args():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--db", required=True, help="path to vmangos mangos.sqlite")
    ap.add_argument("--data", default=os.environ.get("WOW_DATA", r"C:\Users\davis\wow-data\Data"),
                    help="WoW 1.12.1 Data dir with MPQs (default: $WOW_DATA)")
    ap.add_argument("--out", default=None, help="output JSON (default: data/twilights-eve/barrens_spawns.json)")
    ap.add_argument("--source-tag", default="db_latest", help="vmangos release tag the DB came from")
    ap.add_argument("--source-asset", default="", help="release asset file name, e.g. db-sqlite-4641790.zip")
    ap.add_argument("--camp-link", type=float, default=CAMP_LINK_YD, help="camp clustering link distance (yd)")
    ap.add_argument("--camp-radius", type=float, default=CAMP_MAX_RADIUS_YD, help="max camp radius before splitting (yd)")
    ap.add_argument("--include-faction-aligned", action="store_true",
                    help="also keep mobs friendly to one player faction and hostile to the other (Theramore, Bael'dun, Horde peons); default drops them")
    return ap.parse_args()


def repo_root():
    return os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))


# ------------------------------------------------------------------ faction logic (vmangos DBCStructure.h)

class FT:
    def __init__(self, row):
        (self.id, _build, self.faction, self.flags, self.our, self.friendly, self.hostile,
         e1, e2, e3, e4, f1, f2, f3, f4) = row
        self.enemies = [e1, e2, e3, e4]
        self.friends = [f1, f2, f3, f4]

    def friendly_to(self, o: "FT") -> bool:
        if o.faction:
            if o.faction in self.enemies:
                return False
            if o.faction in self.friends:
                return True
        return bool((self.friendly & o.our) or (self.our & o.friendly))

    def hostile_to(self, o: "FT") -> bool:
        if o.faction:
            if o.faction in self.enemies:
                return True
            if o.faction in self.friends:
                return False
        return bool(self.hostile & o.our)


def base_reputation(faction_row, race_bit):
    """vmangos ReputationMgr base standing for a race, or None if the faction has no rep list."""
    if faction_row is None or faction_row["reputation_list_id"] < 0:
        return None
    for i in range(1, 5):
        if faction_row["base_rep_race_mask%d" % i] & race_bit:
            return faction_row["base_rep_value%d" % i]
    return None


RACE_BITS = {"human": 1, "orc": 2, "dwarf": 4, "night_elf": 8, "undead": 16, "tauren": 32, "gnome": 64, "troll": 128}


def reaction(ft: FT, player_ft: FT, faction_row, race):
    """'hostile' | 'neutral' | 'friendly' of the creature toward a player of `race`."""
    rep = base_reputation(faction_row, RACE_BITS[race])
    if rep is not None and rep <= -3000:  # Hostile/Hated base standing
        return "hostile"
    if ft.hostile_to(player_ft):
        return "hostile"
    if ft.friendly_to(player_ft):
        return "friendly"
    if rep is not None and rep >= 3000:
        return "friendly"
    return "neutral"


def side_reaction(reactions):
    vals = set(reactions)
    if "friendly" in vals:
        return "friendly"
    if vals == {"hostile"}:
        return "hostile"
    if "hostile" in vals:
        return "mixed"
    return "neutral"


def dump_json(obj) -> str:
    """Pretty top level, one record per line in lists and maps of records: readable diffs, small file."""
    def one(v):
        return json.dumps(v, ensure_ascii=False, separators=(",", ":"))
    nl = chr(10)
    lines = ["{"]
    items = list(obj.items())
    for n, (k, v) in enumerate(items):
        end = "," if n < len(items) - 1 else ""
        if isinstance(v, list) and v and isinstance(v[0], dict):
            body = ("," + nl).join("  " + one(e) for e in v)
            lines.append(" %s: [%s%s%s ]%s" % (one(k), nl, body, nl, end))
        elif k == "templates":
            body = ("," + nl).join("  %s: %s" % (one(kk), one(vv)) for kk, vv in v.items())
            lines.append(" %s: {%s%s%s }%s" % (one(k), nl, body, nl, end))
        else:
            lines.append(" %s: %s%s" % (one(k), json.dumps(v, ensure_ascii=False, indent=2).replace(nl, nl + " "), end))
    lines.append("}")
    return nl.join(lines) + nl


# ------------------------------------------------------------------ main

def main():
    a = parse_args()
    out_path = a.out or os.path.join(repo_root(), "data", "twilights-eve", "barrens_spawns.json")

    db = sqlite3.connect("file:%s?mode=ro" % a.db.replace("\\", "/"), uri=True)
    db.row_factory = sqlite3.Row

    latest_migration = db.execute("select * from migrations order by 1 desc limit 1").fetchone()[0]

    # creature_template: one row per entry and patch; take the newest at or below 1.12.
    templates = {}
    for r in db.execute("select * from creature_template where patch <= ? order by entry, patch", (PATCH,)):
        templates[r["entry"]] = dict(r)

    fts = {r[0]: FT(tuple(r)) for r in db.execute("select * from faction_template")}
    factions = {}
    for r in db.execute("select * from faction order by id, build"):
        factions[r["id"]] = dict(r)  # last build wins
    quest_npcs = {r[0] for r in db.execute("select id from creature_questrelation union select id from creature_involvedrelation")}
    event_guids = {r[0] for r in db.execute("select guid from game_event_creature")}
    pools = {}
    for r in db.execute("select guid, pool_entry, chance from pool_creature where patch_min <= ? and patch_max >= ?", (PATCH, PATCH)):
        pools[r["guid"]] = (r["pool_entry"], r["chance"])
    pool_tpl = {r["entry"]: dict(r) for r in db.execute("select * from pool_template")}
    waypoint_guids = {r[0] for r in db.execute("select distinct id from creature_movement")}

    player_fts = {**{k: fts[v] for k, v in HORDE_PLAYER_FT.items()}, **{k: fts[v] for k, v in ALLIANCE_PLAYER_FT.items()}}

    gd = wowmpq.GameData(a.data)
    terrain = wowmpq.Terrain(gd, "Kalimdor")

    # AreaTable.dbc from the client: 0 ID, 1 ContinentID, 2 ParentAreaID, 11 name (enUS).
    area_rows, area_strs = gd.dbc("AreaTable")
    areas = {}
    for r in area_rows:
        name = area_strs[r[11]: area_strs.index(b"\0", r[11])].decode("utf-8", "replace")
        areas[r[0]] = {"name": name, "map": r[1], "parent": r[2]}

    def zone_of(area_id):
        ar = areas.get(area_id)
        if ar is None:
            return None
        return ar["parent"] or area_id

    rows = db.execute(
        "select * from creature where map = ? and patch_min <= ? and patch_max >= ? "
        "and position_x between ? and ? and position_y between ? and ? order by guid",
        (MAP_KALIMDOR, PATCH, PATCH, BOX[0], BOX[1], BOX[2], BOX[3])).fetchall()

    dropped = defaultdict(int)
    dropped_names = defaultdict(lambda: defaultdict(int))
    spawns = []
    wmo_resolved = defaultdict(int)
    for c in rows:
        x, y, z = c["position_x"], c["position_y"], c["position_z"]
        area = terrain.area_id(x, y)
        area_src = "terrain"
        # Under the terrain surface (caves, mines) the game takes the area from the WMO
        # the unit stands in, not the terrain chunk above it. Mulgore's Venture Co. Mine
        # runs under Barrens terrain, for example.
        # A WMO whose area is on another map (the Wailing Caverns cave is shared with the
        # instance, map 43) does not move the spawn: the terrain area stands.
        gz = terrain.ground_z(x, y)
        if gz is not None and z < gz - UNDERGROUND_YD:
            w_area, w_path = terrain.wmo_area(x, y, z)
            if w_area and areas.get(w_area, {}).get("map") == MAP_KALIMDOR and w_area != area:
                if ZONE_BARRENS in (zone_of(area), zone_of(w_area)):
                    wmo_resolved["%s: %s -> %s" % (w_path.split("\\")[-1], areas.get(area, {}).get("name"),
                                                   areas[w_area]["name"])] += 1
                area, area_src = w_area, "wmo"
        if zone_of(area) != ZONE_BARRENS:
            continue
        t = templates.get(c["id"])

        def drop(why):
            dropped[why] += 1
            dropped_names[why][t["name"] if t else str(c["id"])] += 1

        if t is None:
            drop("no_template"); continue
        if c["guid"] in event_guids:
            drop("game_event_only"); continue
        if c["spawn_flags"] & (SPAWN_FLAG_DISABLED | SPAWN_FLAG_NOT_VISIBLE):
            drop("disabled_or_invisible_spawn"); continue
        if t["flags_extra"] & FLAG_EXTRA_INVISIBLE or t["static_flags1"] & STATIC_UNINTERACTIBLE:
            drop("invisible_or_unselectable"); continue
        if t["npc_flags"] != 0:
            drop("npc_flags"); continue
        if c["id"] in quest_npcs:
            drop("quest_npc"); continue
        if t["type"] in EXCLUDED_TYPES:
            drop("critter_totem_pet"); continue
        if t["flags_extra"] & FLAG_EXTRA_GUARD:
            drop("guard"); continue
        ft = fts.get(t["faction"])
        if ft is None:
            drop("unknown_faction"); continue
        frow = factions.get(ft.faction)
        rx = {race: reaction(ft, pft, frow, race) for race, pft in player_fts.items()}
        horde = side_reaction([rx[r] for r in HORDE_PLAYER_FT])
        alliance = side_reaction([rx[r] for r in ALLIANCE_PLAYER_FT])
        aligned = {horde, alliance} == {"friendly", "hostile"}
        if (horde == "friendly" or alliance == "friendly") and not (aligned and a.include_faction_aligned):
            drop("friendly_to_a_player_faction"); continue
        if horde == "hostile" or alliance == "hostile":
            react = "hostile"
        else:
            react = "neutral"

        displays = []
        for i in range(1, 5):
            d = t["display_id%d" % i]
            if d:
                displays.append({"display_id": d, "scale": round(t["display_scale%d" % i], 3),
                                 "probability": t["display_probability%d" % i]})
        main_display = max(displays, key=lambda d: d["probability"])["display_id"] if displays else 0
        alts = [c["id%d" % i] for i in range(2, 6) if c["id%d" % i]]

        ar = areas.get(area, {})
        pool = pools.get(c["guid"])
        spawns.append({
            "guid": c["guid"],
            "entry": c["id"],
            "alt_entries": alts,
            "name": t["name"],
            "x": round(x, 3), "y": round(y, 3), "z": round(c["position_z"], 3),
            "orientation": round(c["orientation"], 4),
            "display_id": main_display,
            "level_min": t["level_min"], "level_max": t["level_max"],
            "faction_template": t["faction"],
            "reaction": react,
            "reaction_horde": horde, "reaction_alliance": alliance,
            "wander_radius": round(c["wander_distance"], 2),
            "movement_type": c["movement_type"],
            "has_waypoints": c["guid"] in waypoint_guids,
            "respawn_secs_min": c["spawntimesecsmin"], "respawn_secs_max": c["spawntimesecsmax"],
            "area_id": area, "area_name": ar.get("name", ""), "area_source": area_src,
            "pool": ({"entry": pool[0], "chance": pool[1],
                      "max_limit": pool_tpl.get(pool[0], {}).get("max_limit"),
                      "description": pool_tpl.get(pool[0], {}).get("description")} if pool else None),
            "camp_id": None,
        })

    # ------------------------------------------------------------ camps
    # vmangos call-for-help only reaches creatures with the *same faction template*
    # (Creature::CanBeTargetedByCallForHelp). A camp is a single-linkage cluster of
    # spawns sharing faction template and creature type within --camp-link yards.
    def family_key(s):
        t = templates[s["entry"]]
        return (s["faction_template"], t["type"])

    by_family = defaultdict(list)
    for i, s in enumerate(spawns):
        by_family[family_key(s)].append(i)
    parent = list(range(len(spawns)))

    def find(i):
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i

    link2 = a.camp_link ** 2
    for fam, idx in by_family.items():
        for ii in range(len(idx)):
            si = spawns[idx[ii]]
            for jj in range(ii + 1, len(idx)):
                sj = spawns[idx[jj]]
                dx, dy = si["x"] - sj["x"], si["y"] - sj["y"]
                if dx * dx + dy * dy <= link2 and abs(si["z"] - sj["z"]) < 15:
                    ri, rj = find(idx[ii]), find(idx[jj])
                    if ri != rj:
                        parent[max(ri, rj)] = min(ri, rj)

    groups = defaultdict(list)
    for i in range(len(spawns)):
        groups[find(i)].append(i)

    # Single linkage chains whole mines and forts into one 30+ mob "camp". Split any
    # cluster wider than --camp-radius by recursive 2-means (seeded on its farthest pair)
    # so one pull is a few mobs, as it is in the game.
    def radius(g):
        cx = sum(spawns[i]["x"] for i in g) / len(g)
        cy = sum(spawns[i]["y"] for i in g) / len(g)
        return max(math.hypot(spawns[i]["x"] - cx, spawns[i]["y"] - cy) for i in g)

    def d2(i, p):
        return (spawns[i]["x"] - p[0]) ** 2 + (spawns[i]["y"] - p[1]) ** 2

    def split(g):
        if len(g) < 2 or radius(g) <= a.camp_radius:
            return [g]
        g = sorted(g, key=lambda i: spawns[i]["guid"])
        far = max(((i, j) for i in g for j in g if i < j), key=lambda ij: d2(ij[0], (spawns[ij[1]]["x"], spawns[ij[1]]["y"])))
        ca = (spawns[far[0]]["x"], spawns[far[0]]["y"])
        cb = (spawns[far[1]]["x"], spawns[far[1]]["y"])
        for _ in range(20):
            ga = [i for i in g if d2(i, ca) <= d2(i, cb)]
            gb = [i for i in g if d2(i, ca) > d2(i, cb)]
            if not ga or not gb:
                break
            na = (sum(spawns[i]["x"] for i in ga) / len(ga), sum(spawns[i]["y"] for i in ga) / len(ga))
            nb = (sum(spawns[i]["x"] for i in gb) / len(gb), sum(spawns[i]["y"] for i in gb) / len(gb))
            if na == ca and nb == cb:
                break
            ca, cb = na, nb
        if not ga or not gb:
            return [g]
        return split(ga) + split(gb)

    final = []
    for g in groups.values():
        final.extend(split(g))
    # Stable camp ids: order camps by their lowest guid.
    ordered = sorted(final, key=lambda g: min(spawns[i]["guid"] for i in g))
    camps = []
    for n, g in enumerate(ordered, start=1):
        members = sorted(g, key=lambda i: spawns[i]["guid"])
        cx = sum(spawns[i]["x"] for i in members) / len(members)
        cy = sum(spawns[i]["y"] for i in members) / len(members)
        cz = sum(spawns[i]["z"] for i in members) / len(members)
        rad = max(math.hypot(spawns[i]["x"] - cx, spawns[i]["y"] - cy) for i in members)
        for i in members:
            spawns[i]["camp_id"] = n
        ft_id, ctype = family_key(spawns[members[0]])
        names = sorted({spawns[i]["name"] for i in members})
        camps.append({
            "camp_id": n,
            "faction_template": ft_id,
            "creature_type": CREATURE_TYPES.get(ctype, str(ctype)),
            "assists": bool(fts[ft_id].flags & FT_RESPOND_TO_CALL_FOR_HELP),
            "names": names,
            "size": len(members),
            "level_min": min(spawns[i]["level_min"] for i in members),
            "level_max": max(spawns[i]["level_max"] for i in members),
            "center": [round(cx, 1), round(cy, 1), round(cz, 1)],
            "radius": round(rad, 1),
            "area_name": spawns[members[0]]["area_name"],
            "guids": [spawns[i]["guid"] for i in members],
        })

    # ------------------------------------------------------------ templates (extra per-entry data)
    used = sorted({s["entry"] for s in spawns} | {e for s in spawns for e in s["alt_entries"]})
    tpl_out = {}
    for e in used:
        t = templates.get(e)
        if not t:
            continue
        tpl_out[str(e)] = {
            "name": t["name"], "subname": t["subname"] or "",
            "level_min": t["level_min"], "level_max": t["level_max"],
            "faction_template": t["faction"],
            "faction_assists": bool(fts[t["faction"]].flags & FT_RESPOND_TO_CALL_FOR_HELP) if t["faction"] in fts else False,
            "type": CREATURE_TYPES.get(t["type"], str(t["type"])),
            "rank": RANKS.get(t["rank"], str(t["rank"])),
            "displays": [{"display_id": t["display_id%d" % i], "scale": round(t["display_scale%d" % i], 3),
                          "probability": t["display_probability%d" % i]} for i in range(1, 5) if t["display_id%d" % i]],
            "speed_walk": round(t["speed_walk"], 3), "speed_run": round(t["speed_run"], 3),
            "detection_range": t["detection_range"], "call_for_help_range": t["call_for_help_range"],
            "leash_range": t["leash_range"],
            "unit_class": t["unit_class"],
            "health_multiplier": round(t["health_multiplier"], 3),
            "damage_multiplier": round(t["damage_multiplier"], 3),
            "armor_multiplier": round(t["armor_multiplier"], 3),
            "base_attack_time": t["base_attack_time"],
            "civilian": bool(t["civilian"]),
            "no_aggro": bool(t["flags_extra"] & 0x2),
            "no_assist": bool(t["flags_extra"] & 0x10000),
            "loot_id": t["loot_id"], "gold_min": t["gold_min"], "gold_max": t["gold_max"],
        }

    counts = {
        "spawns": len(spawns),
        "hostile": sum(1 for s in spawns if s["reaction"] == "hostile"),
        "neutral": sum(1 for s in spawns if s["reaction"] == "neutral"),
        "entries": len({s["entry"] for s in spawns}),
        "camps": len(camps),
        "camps_multi": sum(1 for c in camps if c["size"] > 1),
        "dropped": dict(sorted(dropped.items())),
    }
    counts["wmo_area_lookups"] = dict(sorted(wmo_resolved.items()))
    out = {
        "schema": 1,
        "source": {
            "project": "vmangos/core",
            "release_tag": a.source_tag,
            "asset": a.source_asset,
            "latest_migration": latest_migration,
            "patch": "1.12 (vmangos patch index %d)" % PATCH,
        },
        "method": {
            "map": MAP_KALIMDOR,
            "zone": ZONE_BARRENS,
            "zone_membership": "area ID of the ADT MCNK terrain chunk under each spawn (WoW 1.12.1 MPQs); for spawns more than underground_yd below the terrain, the area of the enclosing WMO placement (WMOAreaTable root row) when there is one; that area's AreaTable.dbc parent (zone) == 17. WMO areas on another map are ignored",
            "underground_yd": UNDERGROUND_YD,
            "camp_link_yd": a.camp_link,
            "camp_max_radius_yd": a.camp_radius,
            "camp_rule": "single-linkage cluster of spawns sharing faction_template and creature type, within camp_link_yd in 2D and 15 yd in Z; clusters wider than camp_max_radius_yd split by recursive 2-means",
            "reaction": "vmangos FactionTemplateEntry::IsHostileTo/IsFriendlyTo against the 8 player race faction templates, plus base reputation",
        },
        "counts": counts,
        "dropped_by_name": {k: dict(sorted(v.items())) for k, v in sorted(dropped_names.items())},
        "templates": tpl_out,
        "camps": camps,
        "spawns": spawns,
    }
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, "w", encoding="utf-8", newline="\n") as f:
        f.write(dump_json(out))
    print("wrote %s" % out_path)
    print(json.dumps(counts, indent=1))


if __name__ == "__main__":
    main()
