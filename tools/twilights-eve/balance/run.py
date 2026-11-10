#!/usr/bin/env python3
"""The balance model's command line: runs the rules of docs/balance/rules.md and writes a Markdown
report, one table per rule with a `holds` column, then the failures, the kits, the picked builds
and the Talent Row choices the model ignores.

    python3.15 tools/twilights-eve/balance/run.py --scenario <T0|pulls|dps|elites|blues|named|plus5|heal|all>
        --levels 1-30 --fights 300 --callings all --data-root <dir> --jobs 4 --out <file.md>

--levels takes levels and ranges (`10 20`, `1-30`); the named-boss rules ignore it, since each
boss sets its own level. The default runs Tier 1; `--levels 30-50` runs Tier 2, whose rows the
report keeps under their own heading. --callings takes Calling keys (Tier 1 or Tier 2) or `all`;
the Tier 0 rules always run the Basic Warrior. A Tier 2 Calling with no sim yet gets one "no sim"
row a rule. --data-root is a folder holding data/twilights-eve/ and
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
    for tier, heading in ((1, "## Tiers 0 and 1"), (2, "## Tier 2")):
        by_rule = {}
        for r in rows:
            if scenarios.tier(r["calling"]) == tier:
                by_rule.setdefault(r["rule"], []).append(r)
        if not by_rule:
            continue
        out += [heading, ""]
        for rule, rs in by_rule.items():
            who, gear, against, band = scenarios.rule_words(rule, rs[0]["calling"])
            out += [f"### {rule}: {who}, {gear}, {against}", "", f"Must show: {band}.", "",
                    "| Calling | Level | Gear | Against | Mob health | Win | Health left | DPS | Kill s | Deaths | Holds | Notes |",
                    "|---|---|---|---|---|---|---|---|---|---|---|---|"]
            for r in rs:
                out.append(f"| {r['calling']} | {r['level']} | {r['gear']} | {r['against']} | {r['mob_hp']} "
                           f"| {pct(r['win'])} | {pct(r['health_left'])} | {num(r['dps'])} | {num(r['kill_s'])} "
                           f"| {pct(r['deaths'])} | {holds(r['holds'])} | {r['notes']} |")
            out.append("")
    out += ["## Failures", ""]
    failed = [r for r in rows if r["holds"] is False]
    for r in failed:
        out.append(f"- {r['rule']} {r['calling']} {r['level']} vs {r['against']}: win {pct(r['win'])}, health left "
                   f"{pct(r['health_left'])}, DPS {num(r['dps'])} (must show "
                   f"{scenarios.rule_words(r['rule'], r['calling'])[3]})")
    if not failed:
        out.append("- none")
    out += ["", "## Kits", "", "A blue kit wears a green in the slots `Green in` names: no Rare row there yet. "
            "`Nothing in` names the slots with no usable row at all.", "",
            "| Calling | Level | Gear | Kit | Green in | Nothing in |", "|---|---|---|---|---|---|"]
    seen = set()
    simmed = [r for r in rows if not r.get("no_sim")]
    for r in sorted(simmed, key=lambda r: (r["calling"], r["level"], r["gear"])):
        key = (r["calling"], r["level"], r["gear"])
        if key in seen:
            continue
        seen.add(key)
        green_in = ", ".join(r["fell_back"]) or "-"
        empty = ", ".join(r["empty"]) or "-"
        out.append(f"| {r['calling']} | {r['level']} | {r['gear']} | {r['kit']} | {green_in} | {empty} |")
    out += ["", "## Builds", "", "Each level where a Calling's picked build differs from its build at the level before: "
            "every open Talent Row and its choice.", "", "| Calling | Level | Build |", "|---|---|---|"]
    builds = {(r["calling"], r["level"]): r["rows"] for r in simmed}
    order = {c: i for i, c in enumerate(model.CALLINGS + model.TIER2_CALLINGS)}
    before = {}
    for calling, level in sorted(builds, key=lambda k: (order.get(k[0], -1), k[1])):
        b = builds[(calling, level)]
        if before.get(calling) != b:
            text = ", ".join(f"{k}: {v}" for k, v in b.items()) or "none"
            out.append(f"| {calling} | {level} | {text} |")
        before[calling] = b
    out += ["", "## Ignored choices", "", "Talent Row choices that change nothing in the model's fights, in sheet "
            "order, and why. A choice named as its row is the row's own skill, unset.", "",
            "| Calling | Row | Choice | Reason |", "|---|---|---|---|"]
    simmed_callings = [c for c in model.CALLINGS + model.TIER2_CALLINGS if scenarios.has_sim(c)]
    ignored = [(c, row["key"], key) for c in simmed_callings for row in model.talent_rows(c)
               for key in [row["key"]] + [ch["key"] for ch in row["choices"]] if (c, key) in model.IGNORED]
    for calling, row, key in ignored:
        out.append(f"| {calling} | {row} | {key} | {model.IGNORED[(calling, key)]} |")
    if not ignored:
        out.append("| - | - | none | - |")
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
    every = model.CALLINGS + model.TIER2_CALLINGS
    callings = list(every) if "all" in args.callings else args.callings
    unknown = [c for c in callings if c not in every]
    if unknown:
        p.error(f"unknown Calling(s): {', '.join(unknown)}")
    model.use(args.data_root)
    rules = [f for name, fs in scenarios.SCENARIOS.items() if args.scenario in (name, "all") for f in fs]
    # The builds are picked here, once a Calling and level, before their cells go out.
    pool = None
    if args.jobs > 1:
        pool = concurrent.futures.ProcessPoolExecutor(max_workers=args.jobs, initializer=model.use,
                                                      initargs=(args.data_root,))
        run = lambda cells: list(pool.map(scenarios.run_cell, scenarios.with_builds(cells, pool.map)))  # noqa: E731
    else:
        run = lambda cells: [scenarios.run_cell(c) for c in scenarios.with_builds(cells)]  # noqa: E731
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
