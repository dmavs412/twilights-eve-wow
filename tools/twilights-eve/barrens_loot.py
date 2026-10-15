#!/usr/bin/env python3
"""Per-creature-family loot for The Barrens, from vmangos `creature_loot_template` (1.12).

An optional layer over loot_items.json's level pools: for each creature family that lives in
The Barrens (plainstriders, Razormane quilboar, Kolkar, raptors, harpies, ...), the drops its
own loot rows give it (hides, fangs, meat, cloth, family-specific gear) and the quest-only drops
(useful for collect-N quests). Generic world drops reach creatures through
`reference_loot_template`; those are left to the level pools and only counted here.

Writes data/twilights-eve/barrens_creature_loot.json. Same DB and flags as import_items.py.
"""

from __future__ import annotations

import argparse
import os
import sys
from collections import defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import import_items as ii  # noqa: E402

# The Barrens on map 1, as a box with two cut-outs; the realm's zone check should use the real
# area id (ADT MCNK area 17) instead. The level cap keeps neighbours' higher mobs out.
BOX_X = (-4700.0, 1400.0)
BOX_Y = (-3950.0, -1050.0)
MAX_MOB_LEVEL = 30
# CreatureFamily.dbc ids used by beasts here
PET_FAMILY = {1: "wolf", 2: "cat", 3: "spider", 4: "bear", 5: "boar", 6: "crocolisk",
              7: "carrion_bird", 8: "crab", 9: "gorilla", 11: "raptor", 12: "tallstrider",
              20: "scorpid", 21: "turtle", 24: "bat", 25: "hyena", 26: "owl",
              27: "wind_serpent"}
CREATURE_TYPE = {1: "beast", 2: "dragonkin", 3: "demon", 4: "elemental", 5: "giant",
                 6: "undead", 7: "humanoid", 8: "critter", 9: "mechanical", 10: "unspecified"}


def in_barrens(x, y):
    if not (BOX_X[0] <= x <= BOX_X[1] and BOX_Y[0] <= y <= BOX_Y[1]):
        return False
    if x < -2600 and y < -2750:  # Dustwallow Marsh
        return False
    return True


# name keyword -> family, checked first (Barrens-specific groupings)
KEYWORDS = [("zhevra", "zhevra"), ("kodo", "kodo"), ("giraffe", "giraffe"),
            ("thunderhead", "thunder_lizard"), ("stormsnout", "thunder_lizard"),
            ("thunderstomp", "thunder_lizard"), ("lizard", "thunder_lizard"),
            ("harpy", "harpy"), ("witchwing", "harpy"), ("windfury", "harpy"),
            ("razormane", "quilboar_razormane"), ("bristleback", "quilboar_bristleback"),
            ("kolkar", "centaur_kolkar"), ("galak", "centaur_galak"), ("venture", "venture_co"),
            ("southsea", "southsea_pirate"), ("bael'dun", "baeldun_dwarf"),
            ("theramore", "theramore"), ("burning blade", "burning_blade"),
            ("silithid", "silithid"), ("qiraji", "silithid"), ("oasis", "oasis_snapjaw")]
SMALL_FAMILY = 2
MIN_CHANCE = 1.0  # % : rarer non-gear rows (stray recipes, potions) are only counted  # families with this many spawns or fewer merge into named_and_rare


def family_of(ct):
    """Keyword first, then beasts by pet family, then the first word of the name."""
    name = ct["name"].lower()
    if ct["rank"] in (2, 4):
        return "named_and_rare"
    for kw, fam in KEYWORDS:
        if kw in name:
            return fam
    if ct["type"] == 1 and ct["pet_family"] in PET_FAMILY:
        fam = PET_FAMILY[ct["pet_family"]]
        return "plainstrider" if fam == "tallstrider" else fam
    return name.replace("'s", "").split()[0]


def build(args):
    con = ii.connect(args.db)
    items, forbidden, spells, creatures, maps, icons, spawns, loot = ii.load(con)
    clt = loot["creature_loot_template"]
    counts = defaultdict(int)
    for cid, x, y in con.execute(
            "SELECT id, position_x, position_y FROM creature WHERE map = 1 "
            "AND patch_min <= ? AND patch_max >= ?", (ii.PATCH, ii.PATCH)):
        if in_barrens(x, y):
            counts[cid] += 1

    fams = defaultdict(lambda: {"creatures": [], "drops": {}, "quest_drops": {},
                                "world_drop_refs": defaultdict(int), "gear_rows": 0})
    for cid, n in sorted(counts.items()):
        ct = creatures.get(cid)
        if not ct or not ct["loot_id"] or ct["npc_flags"] or ct["type"] == 8:
            continue
        if ct["level_max"] > MAX_MOB_LEVEL or ct["rank"] == 3:
            continue
        f = fams[family_of(ct)]
        f["creatures"].append({"entry": cid, "name": ct["name"], "level_min": ct["level_min"],
                               "level_max": ct["level_max"], "rank": ct["rank"],
                               "type": CREATURE_TYPE.get(ct["type"], ct["type"]),
                               "spawns": n})
        for _e, item, chance, group, mref, maxc in clt.get(ct["loot_id"], ()):
            if mref < 0:
                f["world_drop_refs"][-mref] += 1
                continue
            it = items.get(item)
            if not it or item in forbidden or it["extra_flags"] & 0x04 or it["quality"] > 3:
                continue
            if chance >= 0 and it["class"] in (2, 4):
                f["gear_rows"] += 1  # direct gear rows: the level pools cover these
                continue
            bucket = f["quest_drops"] if chance < 0 else f["drops"]
            row = bucket.setdefault(item, {
                "entry": item, "name": it["name"], "quality": it["quality"],
                "class_name": ii.ITEM_CLASS.get(it["class"], str(it["class"])),
                "item_level": it["item_level"], "sell_price": it["sell_price"],
                "display_id": it["display_id"], "icon": icons.get(it["display_id"]),
                "chance_pct_max": 0.0, "chance_pct_min": 100.0, "dropped_by": 0,
                "max_count": max(1, mref)})
            row["chance_pct_max"] = round(max(row["chance_pct_max"], abs(chance)), 3)
            row["chance_pct_min"] = round(min(row["chance_pct_min"], abs(chance)), 3)
            row["dropped_by"] += 1

    small = [n for n, f in fams.items() if n != "named_and_rare"
             and sum(c["spawns"] for c in f["creatures"]) <= SMALL_FAMILY]
    for n in small:
        f, g = fams.pop(n), fams["named_and_rare"]
        g["creatures"] += f["creatures"]
        g["gear_rows"] += f["gear_rows"]
        for k in ("drops", "quest_drops"):
            for item, row in f[k].items():
                if item in g[k]:
                    r = g[k][item]
                    r["chance_pct_max"] = max(r["chance_pct_max"], row["chance_pct_max"])
                    r["chance_pct_min"] = min(r["chance_pct_min"], row["chance_pct_min"])
                    r["dropped_by"] += row["dropped_by"]
                else:
                    g[k][item] = row
        for ref, c in f["world_drop_refs"].items():
            g["world_drop_refs"][ref] += c
    out = {}
    for name, f in sorted(fams.items()):
        if not f["creatures"]:
            continue
        cs = f["creatures"]
        drops = sorted(f["drops"].values(), key=lambda r: (-r["chance_pct_max"], r["entry"]))
        rare = [r for r in drops if r["chance_pct_max"] < MIN_CHANCE]
        drops = [r for r in drops if r["chance_pct_max"] >= MIN_CHANCE]
        out[name] = {
            "level_min": min(c["level_min"] for c in cs),
            "level_max": max(c["level_max"] for c in cs),
            "spawns": sum(c["spawns"] for c in cs),
            "creatures": sorted(cs, key=lambda c: (c["level_min"], c["entry"])),
            # common: what most of the family drops at a real rate; the rest are one-offs
            "drops": drops,
            "quest_drops": sorted(f["quest_drops"].values(), key=lambda r: r["entry"]),
            "world_drop_refs": dict(sorted(f["world_drop_refs"].items())),
            "direct_gear_rows": f["gear_rows"],
            "rare_non_gear_rows": len(rare),
        }
    return {
        "meta": ii.meta(args, "Barrens creature families and their own loot rows (1.12). An "
                              "optional layer over loot_items.json level pools."),
        "zone_rule": f"map 1, x in {BOX_X}, y in {BOX_Y}, minus Dustwallow (x < -2600 and "
                     f"y < -2750); creature level_max <= {MAX_MOB_LEVEL}; no bosses, critters "
                     "or NPCs with gossip/vendor flags",
        "families_note": "drops are the family's own non-gear rows at >= 1%  (hides, fangs, meat, cloth, "
                         "junk, food). Gear the DB lists per creature is counted in "
                         "direct_gear_rows and left to the level pools. Rares (rank 2/4) and "
                         "families with <= 2 spawns are merged into named_and_rare.",
        "how_to_use": "On a kill: roll each `drops` row at chance_pct_max (or the creature's own "
                      "row via creature_loot_template), then roll the level pool for gear. "
                      "`quest_drops` only drop while a quest wants them (negative chance in "
                      "vmangos). `world_drop_refs` are the generic reference tables the family "
                      "uses; loot_items.json level pools stand in for them.",
        "families": out,
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--db", default=os.path.join(ii.DEFAULT_CACHE, "sqlite-dump",
                                                  "mangos.sqlite"))
    ap.add_argument("--out", default=os.path.join(ii.REPO, "data", "twilights-eve"))
    args = ap.parse_args()
    data = build(args)
    path = os.path.join(args.out, "barrens_creature_loot.json")
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(ii.dumps(data) + "\n")
    print(f"wrote {path}")
    for name, fam in data["families"].items():
        print(f"{name:22} {fam['level_min']:>2}-{fam['level_max']:<2} creatures "
              f"{len(fam['creatures']):>2} spawns {fam['spawns']:>4} drops {len(fam['drops']):>3} "
              f"quest {len(fam['quest_drops'])}")


if __name__ == "__main__":
    main()
