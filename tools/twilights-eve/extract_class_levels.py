#!/usr/bin/env python3
"""Extract the per-level player tables for all nine Classes from a vmangos world DB.

Offline, one-off, re-runnable. Writes data/twilights-eve/<class>_level_stats.json for each
Class, which are committed, so building the game never needs the database.
`cargo run -p twilight-realm --example gen_tables` turns them into content/levels.toml.

    ~/.local/bin/python3.15 -I tools/twilights-eve/extract_class_levels.py --db mangos.sqlite

Inputs:
  --db    vmangos world database as SQLite (mangos.sqlite from the
          db-sqlite-<sha>.zip asset of the vmangos/core `db_latest` release)
  --out   output directory (default data/twilights-eve)

Tables read: player_classlevelstats (base health and mana by level), player_levelstats (naked
base stats by race and level), player_crit_per_agility and player_dodge_per_agility (agility
per 1% by level). The agility tables have gaps; ObjectMgr::LoadPlayerInfo fills them by linear
interpolation between the nearest levels on either side, and so does this script.

Only the races a Class can be appear under race_base_stats. The warrior file also carries
hand-derived naked_checks (test vectors); the script keeps that key when it rewrites the file.

Standard library only. Run with -I when the DB came from the internet.
"""

from __future__ import annotations

import argparse
import json
import os
import sqlite3

MAX_LEVEL = 60

# ChrClasses.dbc ids.
CLASSES = {
    1: "warrior",
    2: "paladin",
    3: "hunter",
    4: "rogue",
    5: "priest",
    7: "shaman",
    8: "mage",
    9: "warlock",
    11: "druid",
}

# ChrRaces.dbc ids, in player_levelstats' order.
RACES = {
    1: "human",
    2: "orc",
    3: "dwarf",
    4: "night_elf",
    5: "undead",
    6: "tauren",
    7: "gnome",
    8: "troll",
}


def interpolate(known: dict[int, float]) -> list[float]:
    """Levels 1..=MAX_LEVEL of a sparse table, gaps filled linearly between known neighbours."""
    levels = sorted(known)
    if levels[0] != 1 or levels[-1] != MAX_LEVEL:
        raise SystemExit(f"the agility table must cover levels 1 and {MAX_LEVEL}, has {levels}")
    out = []
    for level in range(1, MAX_LEVEL + 1):
        if level in known:
            out.append(known[level])
            continue
        lo = max(l for l in levels if l < level)
        hi = min(l for l in levels if l > level)
        t = (level - lo) / (hi - lo)
        out.append(round(known[lo] + (known[hi] - known[lo]) * t, 5))
    return out


def extract(db: sqlite3.Connection, class_id: int) -> dict:
    name = CLASSES[class_id]
    agility = {}
    interpolated = []
    for table in ("player_crit_per_agility", "player_dodge_per_agility"):
        rows = dict(db.execute(f"select level, rate from {table} where class=?", (class_id,)))
        missing = [l for l in range(1, MAX_LEVEL + 1) if l not in rows]
        interpolated.append(missing)
        agility[table] = interpolate(rows)
    base = db.execute(
        "select level, basehp, basemana from player_classlevelstats where class=? order by level",
        (class_id,),
    ).fetchall()
    if [r[0] for r in base] != list(range(1, MAX_LEVEL + 1)):
        raise SystemExit(f"{name}: player_classlevelstats does not cover every level")
    per_level = [
        {
            "level": level,
            "base_health": hp,
            "base_mana": mana,
            "agi_per_crit_pct": agility["player_crit_per_agility"][level - 1],
            "agi_per_dodge_pct": agility["player_dodge_per_agility"][level - 1],
        }
        for level, hp, mana in base
    ]
    race_base_stats = {}
    for race_id, race in RACES.items():
        rows = db.execute(
            "select level, str, agi, sta, inte, spi from player_levelstats"
            " where race=? and class=? order by level",
            (race_id, class_id),
        ).fetchall()
        if not rows:
            continue
        if [r[0] for r in rows] != list(range(1, MAX_LEVEL + 1)):
            raise SystemExit(f"{name}/{race}: player_levelstats does not cover every level")
        race_base_stats[race] = [
            {"level": l, "str": s, "agi": a, "sta": st, "int": i, "spi": sp}
            for l, s, a, st, i, sp in rows
        ]
    crit_gaps, dodge_gaps = interpolated
    return {
        "_source": {
            "per_level": (
                f"vmangos world DB: player_classlevelstats (class={class_id}) for"
                " base_health/base_mana; player_crit_per_agility and player_dodge_per_agility"
                f" (class={class_id}) for the agility divisors. Read by"
                " ObjectMgr::LoadPlayerInfo / ObjectMgr::GetPlayerCritPerAgility /"
                " GetPlayerDodgePerAgility."
            ),
            "race_base_stats": (
                f"vmangos world DB: player_levelstats (class={class_id}). Naked base stats per"
                " race per level, before gear/buffs. Only the races this Class can be."
            ),
            "formulas": "see docs/reference/vanilla-combat-rules.md section 5",
            "script": "tools/twilights-eve/extract_class_levels.py",
        },
        "_interpolated_note": (
            "Levels missing from the vmangos agility tables are filled by linear interpolation"
            " between the nearest neighbours, as ObjectMgr::LoadPlayerInfo does."
            f" Interpolated crit levels: {crit_gaps}; dodge levels: {dodge_gaps}."
        ),
        "class": name,
        "class_id": class_id,
        "per_level": per_level,
        "race_base_stats": race_base_stats,
    }


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--db", required=True)
    ap.add_argument("--out", default=os.path.join(
        os.path.dirname(os.path.abspath(__file__)), "..", "..", "data", "twilights-eve"))
    args = ap.parse_args()
    db = sqlite3.connect(f"file:{args.db}?mode=ro", uri=True)
    for class_id, name in CLASSES.items():
        path = os.path.join(args.out, f"{name}_level_stats.json")
        data = extract(db, class_id)
        if os.path.exists(path):
            with open(path, encoding="utf-8") as f:
                old = json.load(f)
            for key in ("_derived_check_rows_note", "naked_checks"):
                if key in old:
                    data[key] = old[key]
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            json.dump(data, f, indent=1)
            f.write("\n")
        print(f"wrote {path} ({len(data['race_base_stats'])} races)")


if __name__ == "__main__":
    main()
