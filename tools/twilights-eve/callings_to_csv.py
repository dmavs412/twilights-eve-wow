#!/usr/bin/env python3
"""One-time migration: flatten data/twilights-eve/callings.json into the master CSV.

    python tools/twilights-eve/callings_to_csv.py [--callings FILE] [--out FILE]

One row per skill. The column set is documented in
docs/spell_pipeline.md. Effect params are `key=value` pairs joined by `;` with nested keys dotted
and lists joined by `|`. After this run, master.csv is the hand-edited truth and callings.json is
history.
"""

from __future__ import annotations

import argparse
import csv
import json
import os

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
DEFAULT_IN = os.path.join(REPO, "data", "twilights-eve", "callings.json")
DEFAULT_OUT = os.path.join(REPO, "data", "twilights-eve", "master.csv")
MAX_EFFECTS = 5

COLUMNS = [
    "kind", "calling", "class", "key", "name", "rank",
    "spell_id", "talent_id", "source_tab_id", "icon_id",
    "tab", "tab_row", "tab_column", "prereq_key", "prereq_rank", "exceptional",
    "borrowed_from_class", "borrowed_family_bit",
    "client_cost", "client_cooldown_ms", "client_gcd_ms", "client_cast_time_ms",
    "client_range_min", "client_range_max", "client_duration_ms", "client_stances",
    "client_on_next_swing", "client_not_in_combat",
    "realm_cost", "realm_cooldown_ms", "realm_gcd_ms", "realm_range_min", "realm_range_max",
    "realm_radius_yd", "target", "requires_out_of_combat",
]
for i in range(1, MAX_EFFECTS + 1):
    COLUMNS += [f"effect{i}_kind", f"effect{i}_params"]
for i in (1, 2):
    COLUMNS += [f"spellmod{i}_type", f"spellmod{i}_op", f"spellmod{i}_family_bit", f"spellmod{i}_value"]
COLUMNS += ["modelled", "description", "notes"]


def flat(value, prefix=""):
    """Flatten a nested dict into dotted key=value pairs."""
    out = []
    if isinstance(value, dict):
        for k, v in value.items():
            out += flat(v, f"{prefix}{k}.")
    elif isinstance(value, list):
        out.append((prefix[:-1], "|".join(str(x) for x in value)))
    elif isinstance(value, bool):
        out.append((prefix[:-1], "yes" if value else "no"))
    else:
        out.append((prefix[:-1], str(value)))
    return out


def params(effect):
    return ";".join(f"{k}={v}" for k, v in flat({k: v for k, v in effect.items() if k != "kind"}))


def yn(v):
    return "" if v is None else ("yes" if v else "no")


def pair(v):
    if v is None:
        return "", ""
    if isinstance(v, list):
        return v[0], v[1]
    return 0, v


def skill_row(calling, s):
    c, r = s.get("client", {}), s.get("realm", {})
    cmin, cmax = pair(c.get("range_yd"))
    rmin, rmax = pair(r.get("range_yd"))
    b = s.get("borrowed") or {}
    row = {
        "kind": "skill", "calling": calling, "class": 1, "key": s["key"], "name": s.get("name", ""), "rank": 1,
        "spell_id": s.get("spell_id", ""), "icon_id": s.get("icon_id", ""),
        "borrowed_from_class": b.get("from_class", ""), "borrowed_family_bit": b.get("family_bit", ""),
        "client_cost": c.get("cost", ""), "client_cooldown_ms": c.get("cooldown_ms", ""),
        "client_gcd_ms": c.get("gcd_ms", ""), "client_cast_time_ms": c.get("cast_time_ms", ""),
        "client_range_min": cmin, "client_range_max": cmax, "client_duration_ms": c.get("duration_ms", ""),
        "client_stances": "|".join(c.get("stances", [])),
        "client_on_next_swing": yn(c.get("on_next_swing")), "client_not_in_combat": yn(c.get("not_in_combat")),
        "realm_cost": r.get("rage_cost", ""), "realm_cooldown_ms": r.get("cooldown_ms", ""),
        "realm_gcd_ms": r.get("gcd_ms", ""), "realm_range_min": rmin, "realm_range_max": rmax,
        "realm_radius_yd": r.get("radius_yd", ""), "target": r.get("target", ""),
        "requires_out_of_combat": yn(r.get("requires_out_of_combat")),
        "modelled": "yes",
        "description": r.get("effects_note", ""),
        "notes": " / ".join(x for x in (s.get("vanilla_reference"), s.get("tooltip_note"), s.get("client_note"), r.get("cooldown_note"), r.get("requires")) if x),
    }
    effects = r.get("effects", [])
    if len(effects) > MAX_EFFECTS:
        raise SystemExit(f"{calling}/{s['key']}: {len(effects)} effects, the CSV holds {MAX_EFFECTS}")
    for i, e in enumerate(effects, 1):
        row[f"effect{i}_kind"] = e["kind"]
        row[f"effect{i}_params"] = params(e)
    return row


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--callings", default=DEFAULT_IN)
    ap.add_argument("--out", default=DEFAULT_OUT)
    a = ap.parse_args()
    with open(a.callings, encoding="utf-8") as f:
        d = json.load(f)
    rows = []
    for cname, c in d["callings"].items():
        for s in c.get("skills", []):
            rows.append(skill_row(cname, s))
    with open(a.out, "w", encoding="utf-8", newline="") as f:
        w = csv.DictWriter(f, fieldnames=COLUMNS, lineterminator="\n")
        w.writeheader()
        for r in rows:
            w.writerow({k: r.get(k, "") for k in COLUMNS})
    print(f"{a.out}: {len(rows)} skill rows, {len(COLUMNS)} columns")


if __name__ == "__main__":
    main()
