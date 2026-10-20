#!/usr/bin/env python3
"""One-time migration of master.csv's talents to ids of their own (ADR 0011, #205).

    python tools/twilights-eve/migrate_talent_ids.py [--csv FILE] [--dry-run]

Every talent gets a fresh `talent_id` from 1,000,000 up, one per talent (all its ranks), in sheet
order, in place of the stock Talent.dbc id it borrowed. The Zealot, whose talents named their tab
by display name, gets the `talent_tab` rows of its TOML tree (Crusade, Blade, Faith) and its
talents name them by key. The old-to-new talent table joins the spell table in
data/twilights-eve/spell_id_remap.json as `talents`, beside `spells`; the realm's save loader
reads both. Run once; a sheet whose talent ids are all 1,000,000 up is refused.
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
REMAP = os.path.join(REPO, "data", "twilights-eve", "spell_id_remap.json")
FIRST_ID = 1_000_000
# The TOML Zealot's tabs (content/callings.toml): key, display name, art, order. Its sheet rows
# named them by display name before; the ids follow the sheet's tabs (900..911).
ZEALOT_TABS = (("crusade", "Crusade", "PaladinCombat", "903", "1"),
               ("blade", "Blade", "WarriorArms", "912", "2"),
               ("faith", "Faith", "PaladinProtection", "913", "3"))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--csv", default=DEFAULT_CSV)
    parser.add_argument("--remap", default=REMAP)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args(argv)

    with open(args.csv, newline="", encoding="utf-8") as f:
        header = next(csv.reader(f))
        f.seek(0)
        rows = [{k: v or "" for k, v in row.items() if k is not None} for row in csv.DictReader(f)]
    talents = [r for r in rows if r["kind"] == "talent"]
    if all(r["talent_id"] and int(r["talent_id"]) >= FIRST_ID for r in talents):
        print("migrate_talent_ids: every talent id is 1,000,000 up; the sheet is migrated already", file=sys.stderr)
        return 1

    remap: dict[int, int] = {}
    ids: dict[tuple[str, str], int] = {}
    for r in talents:
        new = ids.setdefault((r["calling"], r["key"]), FIRST_ID + len(ids))
        if r["talent_id"]:
            old = int(r["talent_id"])
            if remap.setdefault(old, new) != new:
                raise SystemExit(f"{r['calling']}/{r['key']}: talent id {old} is another talent's too")
        r["talent_id"] = str(new)

    by_name = {name: key for key, name, *_ in ZEALOT_TABS}
    for r in talents:
        if r["calling"] == "zealot":
            r["tab"] = by_name[r["tab"]]
    at = next(i for i, r in enumerate(rows) if r["kind"] == "calling" and r["calling"] == "zealot") + 1
    for n, (key, name, art, tab_id, order) in enumerate(ZEALOT_TABS):
        tab = dict.fromkeys(header, "")
        tab.update(kind="talent_tab", calling="zealot", **{"class": "1"}, key=key, name=name, tab_id=tab_id,
                   background=art, tab_order=order, status="placeholder")
        rows.insert(at + n, tab)

    print(f"{len(ids)} talents get ids {FIRST_ID}..{FIRST_ID + len(ids) - 1}; "
          f"{len(remap)} stock talent ids remapped; 3 Zealot tabs added")
    if args.dry_run:
        return 0
    out = io.StringIO()
    writer = csv.writer(out, lineterminator="\n")
    writer.writerow(header)
    writer.writerows([row[c] for c in header] for row in rows)
    with open(args.csv, "w", encoding="utf-8", newline="") as f:
        f.write(out.getvalue())
    with open(args.remap, encoding="utf-8") as f:
        table = json.load(f)
    spells = table.get("spells", table)
    with open(args.remap, "w", encoding="utf-8", newline="\n") as f:
        json.dump({"spells": spells, "talents": {str(old): new for old, new in sorted(remap.items())}}, f, indent=1)
        f.write("\n")
    print(f"wrote {args.csv} and {args.remap}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
