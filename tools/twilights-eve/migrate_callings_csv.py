#!/usr/bin/env python3
"""One-time migration: each Calling's Class becomes its own row in callings.csv (ADR 0011, #206).

    python tools/twilights-eve/migrate_callings_csv.py [--csv FILE] [--out FILE] [--dry-run]

Writes data/twilights-eve/callings.csv, one row per master.csv `calling` row: its `name`, the
`power` and `stat_profile` its old `class` gave (Warrior rage, Rogue energy, every other Class
mana, on that Class's table), multipliers of 100, the `armour` cell as `armor`, every Warrior
weapon as `weapons` (and `dual_wield` for the Rogue Class, which wore a one-hander in the off
hand), its first skill's icon as `icon` and its name as the spell book tab. Then the `class` and
`armour` columns leave master.csv. Run once; a sheet without `class` is refused.
"""

from __future__ import annotations

import argparse
import csv
import io
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
DEFAULT_CSV = os.path.join(REPO, "data", "twilights-eve", "master.csv")
DEFAULT_OUT = os.path.join(REPO, "data", "twilights-eve", "callings.csv")
PATCH = os.path.join(REPO, "data", "twilights-eve", "client_dbc_patch.json")
COLUMNS = ("calling", "name", "icon", "power", "stat_profile", "health_pct", "mana_pct", "armor",
           "weapons", "spellbook")
PROFILES = {1: "warrior", 2: "paladin", 3: "hunter", 4: "rogue", 5: "priest", 7: "shaman", 8: "mage",
            9: "warlock", 11: "druid"}
WARRIOR_WEAPONS = ("one_hand_axe", "two_hand_axe", "bow", "gun", "one_hand_mace", "two_hand_mace", "polearm",
                   "one_hand_sword", "two_hand_sword", "staff", "fist", "misc", "dagger", "thrown", "crossbow",
                   "fishing_pole")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--csv", default=DEFAULT_CSV)
    parser.add_argument("--out", default=DEFAULT_OUT)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args(argv)

    with open(args.csv, newline="", encoding="utf-8") as f:
        header = next(csv.reader(f))
        f.seek(0)
        rows = [{k: v or "" for k, v in row.items() if k is not None} for row in csv.DictReader(f)]
    if "class" not in header:
        print("migrate_callings_csv: the sheet has no class column; it is migrated already", file=sys.stderr)
        return 1
    with open(PATCH, encoding="utf-8") as f:
        icons = {}
        for e in json.load(f)["spells"]:
            icons.setdefault(e["calling"], e["fields"].get("icon", ""))
    out_rows = []
    for row in rows:
        if row["kind"] != "calling":
            continue
        cls = int(row["class"])
        weapons = list(WARRIOR_WEAPONS) + (["dual_wield"] if cls == 4 else [])
        out_rows.append({
            "calling": row["calling"], "name": row["name"], "icon": icons.get(row["calling"], ""),
            "power": {1: "rage", 4: "energy"}.get(cls, "mana"), "stat_profile": PROFILES[cls],
            "health_pct": "100", "mana_pct": "100", "armor": row["armour"], "weapons": "|".join(weapons),
            "spellbook": row["name"],
        })
    for r in out_rows:
        print(f"{r['calling']}: {r['power']} on {r['stat_profile']}, armor {r['armor']}")
    if args.dry_run:
        return 0
    out = io.StringIO()
    writer = csv.DictWriter(out, COLUMNS, lineterminator="\n")
    writer.writeheader()
    writer.writerows(out_rows)
    with open(args.out, "w", encoding="utf-8", newline="") as f:
        f.write(out.getvalue())
    keep = [c for c in header if c not in ("class", "armour")]
    out = io.StringIO()
    writer = csv.writer(out, lineterminator="\n")
    writer.writerow(keep)
    writer.writerows([row[c] for c in keep] for row in rows)
    with open(args.csv, "w", encoding="utf-8", newline="") as f:
        f.write(out.getvalue())
    print(f"wrote {args.out} and {args.csv}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
