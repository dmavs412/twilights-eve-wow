#!/usr/bin/env python3
"""One-time migration of master.csv to from-scratch spell rows (ADR 0011, #204).

    python tools/twilights-eve/migrate_from_scratch.py [--csv FILE] [--data DIR] [--dry-run]

Every skill, talent and aura row that borrows a stock spell (`client_spell_id`) gets a fresh
`spell_id` from 1,000,000 up, in sheet order; an aura sharing its skill's or talent's stock id
shares the new one. `visual` is the stock id when that spell has a SpellVisual, `icon` its
SpellIcon path. The client-only cells the stock row supplied while it was borrowed are written
into the sheet where empty, so the client shows what it showed before: range, cast time,
duration, cooldown, global cooldown, cost in the Calling's power, stances (a row borrowed from
another class had none), out-of-combat only, and an empty description. Realm columns (school,
cast_kind, an aura's dispel type and mechanic) stay the sheet's.

`client_spell_id`, `realm_spell_id`, `borrowed_from_class` and `borrowed_family_bit` leave the
sheet. The old-to-new table goes to data/twilights-eve/spell_id_remap.json, which the realm's save
loader reads. Run once; a sheet without `client_spell_id` is refused.
"""

from __future__ import annotations

import argparse
import csv
import io
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from extract_spells import FORMS, Tables  # noqa: E402

REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
DEFAULT_CSV = os.path.join(REPO, "data", "twilights-eve", "master.csv")
REMAP = os.path.join(REPO, "data", "twilights-eve", "spell_id_remap.json")
FIRST_ID = 1_000_000
GONE = ("client_spell_id", "realm_spell_id", "borrowed_from_class", "borrowed_family_bit")
# build_spells.CLASS_POWER_TYPE: the power each class id's Callings spend.
CLASS_POWER = {1: "rage", 2: "mana", 3: "mana", 4: "energy", 5: "mana", 7: "mana", 8: "mana", 9: "mana", 11: "mana"}
SPELL_VISUAL, SPELL_GCD = 115, 158


def text(value: float) -> str:
    """A number as the sheet writes it: no trailing `.0`."""
    return str(int(value)) if float(value).is_integer() else str(round(value, 2))


def active(row: dict) -> bool:
    """A row the player casts: a skill, or a talent with a target. Passive talent ranks and auras
    have no cast for the client to show."""
    return row["kind"] == "skill" or (row["kind"] == "talent" and bool(row["target"]))


def fill(row: dict, column: str, value: str, filled: dict) -> None:
    if not row[column] and value:
        row[column] = value
        filled[column] = filled.get(column, 0) + 1


def migrate(rows: list[dict], tables: Tables) -> tuple[dict[int, int], dict, list[str]]:
    remap: dict[int, int] = {}
    filled: dict[str, int] = {}
    notes: list[str] = []
    for row in rows:
        stock = row["client_spell_id"]
        if row["kind"] not in ("skill", "talent", "aura") or not stock:
            continue
        old = int(stock)
        new = remap.setdefault(old, FIRST_ID + len(remap))
        row["spell_id"] = str(new)
        f = tables.spell_fields(old)
        where = f"{row['calling']}/{row['key']} ({old} {f['name']})"
        r = tables.spell.row(old)
        if tables.spell.u32(r, SPELL_VISUAL):
            fill(row, "visual", str(old), filled)
        fill(row, "icon", f["icon"] or "", filled)
        fill(row, "description", f["description"], filled)
        if not active(row):
            continue
        if not row["client_range_min"] and not row["client_range_max"] and f["range_yd"]:
            low, high = f["range_yd"]
            if high:
                row["client_range_min"], row["client_range_max"] = text(low), text(high)
                filled["client_range"] = filled.get("client_range", 0) + 1
        if f["cast_time_ms"]:
            fill(row, "client_cast_time_ms", text(f["cast_time_ms"]), filled)
        if f["duration_ms"] and f["duration_ms"] > 0:
            fill(row, "client_duration_ms", text(f["duration_ms"]), filled)
        if f["cooldown_ms"]:
            fill(row, "client_cooldown_ms", text(f["cooldown_ms"]), filled)
        gcd = tables.spell.u32(r, SPELL_GCD)
        if gcd != 1500:
            fill(row, "client_gcd_ms", text(gcd), filled)
        if f["cost_raw"] and not row["client_cost"]:
            if f["power_type"] == CLASS_POWER[int(row["class"])]:
                fill(row, "client_cost", text(f["cost"]), filled)
            else:
                notes.append(f"{where}: stock cost {f['cost']} {f['power_type']} not carried "
                             f"(the Calling spends {CLASS_POWER[int(row['class'])]})")
        if f["cost_pct"] and not row["client_cost"]:
            notes.append(f"{where}: stock cost {f['cost_pct']}% of base not carried")
        if f["stances"] and not row["borrowed_from_class"]:
            fill(row, "client_stances", "|".join(f["stances"]), filled)
        elif f["stances_mask"] and not row["borrowed_from_class"]:
            notes.append(f"{where}: stock stances {f['stances_mask']:#x} name no warrior form")
        if f["not_in_combat"]:
            fill(row, "client_not_in_combat", "yes", filled)
    return remap, filled, notes


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--csv", default=DEFAULT_CSV)
    parser.add_argument("--data", default=os.environ.get("WOW_DATA") or r"C:\Users\davis\wow-data\Data")
    parser.add_argument("--remap", default=REMAP)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args(argv)

    with open(args.csv, newline="", encoding="utf-8") as f:
        header = next(csv.reader(f))
        f.seek(0)
        rows = [{k: v or "" for k, v in row.items() if k is not None} for row in csv.DictReader(f)]
    if "client_spell_id" not in header:
        print("migrate_from_scratch: the sheet has no client_spell_id; it is migrated already", file=sys.stderr)
        return 1
    for row in rows:
        for column in header:
            row.setdefault(column, "")
    remap, filled, notes = migrate(rows, Tables(args.data))
    print(f"{len(remap)} stock ids get from-scratch ids {FIRST_ID}..{FIRST_ID + len(remap) - 1}")
    for column, n in sorted(filled.items()):
        print(f"  filled {column}: {n} rows")
    for note in notes:
        print(f"  note: {note}")
    if args.dry_run:
        return 0
    keep = [c for c in header if c not in GONE]
    out = io.StringIO()
    writer = csv.writer(out, lineterminator="\n")
    writer.writerow(keep)
    writer.writerows([row[c] for c in keep] for row in rows)
    with open(args.csv, "w", encoding="utf-8", newline="") as f:
        f.write(out.getvalue())
    with open(args.remap, "w", encoding="utf-8", newline="\n") as f:
        json.dump({str(old): new for old, new in sorted(remap.items())}, f, indent=1)
        f.write("\n")
    print(f"wrote {args.csv} and {args.remap}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
