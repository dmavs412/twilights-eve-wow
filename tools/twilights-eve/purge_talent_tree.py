#!/usr/bin/env python3
"""One-time purge of the Talent Tree from master.csv (ADR 0028, #502).

    python tools/twilights-eve/purge_talent_tree.py [--csv FILE] [--dry-run]

Drops every `talent_tab` row and every `talent` row with neither `variant_of` nor `row_key` (a tree
talent's rank), and prints the counts. Every other row is written back as it was read, LF line
ends. The tree columns stay in the header, empty; build_spells.py refuses a filled one. Run once;
a sheet with nothing to drop is refused.
"""

from __future__ import annotations

import argparse
import csv
import io
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_CSV = os.path.join(HERE, "..", "..", "data", "twilights-eve", "master.csv")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--csv", default=DEFAULT_CSV)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args(argv)

    with open(args.csv, newline="", encoding="utf-8") as f:
        header, *rows = list(csv.reader(f))
    cell = lambda row, column: row[header.index(column)] if header.index(column) < len(row) else ""
    tabs = [r for r in rows if cell(r, "kind") == "talent_tab"]
    tree = [r for r in rows if cell(r, "kind") == "talent" and not cell(r, "variant_of") and not cell(r, "row_key")]
    print(f"{len(tabs)} talent_tab rows and {len(tree)} tree talent rows dropped")
    if not tabs and not tree:
        print("purge_talent_tree: no tree rows; the sheet is purged already", file=sys.stderr)
        return 1
    if args.dry_run:
        return 0
    dropped = {id(r) for r in tabs + tree}
    out = io.StringIO()
    writer = csv.writer(out, lineterminator="\n")
    writer.writerow(header)
    writer.writerows(r for r in rows if id(r) not in dropped)
    with open(args.csv, "wb") as f:
        f.write(out.getvalue().encode("utf-8"))
    print(f"wrote {args.csv}: {len(rows) - len(dropped)} rows")
    return 0


if __name__ == "__main__":
    sys.exit(main())
