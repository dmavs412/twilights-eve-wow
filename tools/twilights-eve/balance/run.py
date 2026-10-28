#!/usr/bin/env python3
"""The balance model's command line: runs the rules of docs/balance/rules.md and writes a Markdown
report, one table per rule with a `holds` column, then the failures, then the kits and builds.

    python3.15 tools/twilights-eve/balance/run.py --scenario <T0|pulls|dps|elites|blues|named|plus5|all>
        --levels 1-30 --fights 300 --callings all --data-root <dir> --jobs 4 --out <file.md>

--levels takes levels and ranges (`10 20`, `1-30`); the named-boss rules ignore it, since each
boss sets its own level. --callings takes Calling keys or `all`; the Tier 0 rules always run the
Basic Warrior. --data-root is a folder holding data/twilights-eve/ and
crates/twilight-realm/content/ (default the repo). Stdlib only.
"""
from __future__ import annotations

import argparse
import concurrent.futures
import math
import os
import sys

import model
import scenarios


def parse_levels(items):
    out = set()
    for item in items:
        for part in item.split(","):
            if "-" in part:
                lo, hi = part.split("-")
                out.update(range(int(lo), int(hi) + 1))
            elif part:
                out.add(int(part))
    return sorted(out)


def pct(v):
    return "-" if v is None or (isinstance(v, float) and math.isnan(v)) else f"{v:.0%}"


def num(v, fmt="{:.1f}"):
    return "-" if v is None or (isinstance(v, float) and math.isnan(v)) else fmt.format(v)


def holds(v):
    return "-" if v is None else ("yes" if v else "no")


def report(rows, args, levels):
    out = ["# Balance model report", "",
           f"Data root `{os.path.abspath(args.data_root)}`; {args.fights} fights a cell; levels "
           f"{levels[0]}-{levels[-1] if levels else ''}; scenario `{args.scenario}`. Rules: docs/balance/rules.md.", ""]
    by_rule = {}
    for r in rows:
        by_rule.setdefault(r["rule"], []).append(r)
    for rule, rs in by_rule.items():
        who, gear, against, band = scenarios.RULES[rule]
        out += [f"## {rule}: {who}, {gear}, {against}", "", f"Must show: {band}.", "",
                "| Calling | Level | Gear | Against | Mob health | Win | Health left | DPS | Kill s | Deaths | Holds | Notes |",
                "|---|---|---|---|---|---|---|---|---|---|---|---|"]
        for r in rs:
            out.append(f"| {r['calling']} | {r['level']} | {r['gear']} | {r['against']} | {r['mob_hp']} | {pct(r['win'])} "
                       f"| {pct(r['health_left'])} | {num(r['dps'])} | {num(r['kill_s'])} | {pct(r['deaths'])} "
                       f"| {holds(r['holds'])} | {r['notes']} |")
        out.append("")
    out += ["## Failures", ""]
    failed = [r for r in rows if r["holds"] is False]
    for r in failed:
        out.append(f"- {r['rule']} {r['calling']} {r['level']} vs {r['against']}: win {pct(r['win'])}, health left "
                   f"{pct(r['health_left'])}, DPS {num(r['dps'])} (must show {scenarios.RULES[r['rule']][3]})")
    if not failed:
        out.append("- none")
    out += ["", "## Kits and builds", "", "A blue kit wears a green in the slots `Green in` names: no Rare row there yet.", "",
            "| Calling | Level | Gear | Kit | Green in | Talents |", "|---|---|---|---|---|---|"]
    seen = set()
    for r in sorted(rows, key=lambda r: (r["calling"], r["level"], r["gear"])):
        key = (r["calling"], r["level"], r["gear"])
        if key in seen:
            continue
        seen.add(key)
        talents = ", ".join(f"{k} {v}" for k, v in r["talents"].items()) or "none"
        green_in = ", ".join(r["fell_back"]) or "-"
        out.append(f"| {r['calling']} | {r['level']} | {r['gear']} | {r['kit']} | {green_in} | {talents} |")
    return "\n".join(out) + "\n"


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("--scenario", default="all", choices=[*scenarios.SCENARIOS, "all"])
    p.add_argument("--levels", nargs="+", default=["1-30"])
    p.add_argument("--fights", type=int, default=300)
    p.add_argument("--callings", nargs="+", default=["all"])
    p.add_argument("--data-root", default=model.REPO)
    p.add_argument("--jobs", type=int, default=4)
    p.add_argument("--out")
    args = p.parse_args(argv)
    levels = parse_levels(args.levels)
    callings = list(model.CALLINGS) if "all" in args.callings else args.callings
    unknown = [c for c in callings if c not in model.CALLINGS]
    if unknown:
        p.error(f"unknown Calling(s): {', '.join(unknown)}")
    model.use(args.data_root)
    rules = [f for name, fs in scenarios.SCENARIOS.items() if args.scenario in (name, "all") for f in fs]
    pool = None
    if args.jobs > 1:
        pool = concurrent.futures.ProcessPoolExecutor(max_workers=args.jobs, initializer=model.use,
                                                      initargs=(args.data_root,))
        run = lambda cells: list(pool.map(scenarios.run_cell, cells))  # noqa: E731
    else:
        run = lambda cells: [scenarios.run_cell(c) for c in cells]  # noqa: E731
    try:
        rows = [r for rule in rules for r in rule(levels, args.fights, callings, run)]
    finally:
        if pool:
            pool.shutdown()
    text = report(rows, args, levels)
    if args.out:
        with open(args.out, "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
    else:
        sys.stdout.write(text)
    failed = sum(r["holds"] is False for r in rows)
    print(f"{len(rows)} cells, {failed} failing", file=sys.stderr)


if __name__ == "__main__":
    main()
