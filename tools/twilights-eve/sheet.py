"""Read and edit rows of the master CSV by calling/key, rewriting only the records touched.

    python3.15 tools/twilights-eve/sheet.py get acolyte/miasma [column ...]
    python3.15 tools/twilights-eve/sheet.py set acolyte/miasma visual=11433 [column=value ...]
    python3.15 tools/twilights-eve/sheet.py add-row vanguard/consecration kind=skill name=Consecration \
        [--after vanguard/bulwark_charge]
    python3.15 tools/twilights-eve/sheet.py add-column client_radius_index

`get` prints a row's non-empty cells, or the columns named. A calling/key can match several rows
(a talent's ranks, an aura sharing a skill's key): `--kind` and `--rank` narrow it, and `set`
refuses more than one match unless `--all`. `add-row` needs `kind`, and writes the row after the
last record of `--after`, or at the end. `add-column` appends to the header only; the build reads
a row shorter than the header as empty cells. Untouched lines are written back byte for byte, a
quoted cell's line breaks included. Run `build_spells.py` after an edit to regenerate the outputs.
"""

from __future__ import annotations

import argparse
import csv
import io
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_CSV = os.path.join(HERE, "..", "..", "data", "twilights-eve", "master.csv")


class SheetError(Exception):
    pass


class Sheet:
    """The CSV as its header and its records, each record the raw text of one row."""

    def __init__(self, path: str):
        self.path = path
        raw = open(path, "rb").read().decode("utf-8")
        self.eol = "\r\n" if "\r\n" in raw else "\n"
        self.trailing = raw.endswith(self.eol)
        lines = raw.split(self.eol)
        if self.trailing:
            lines.pop()
        # A record ends where its quotes balance: a quoted cell may hold line breaks.
        self.records: list[str] = []
        pending = None
        for line in lines:
            pending = line if pending is None else pending + self.eol + line
            if pending.count('"') % 2 == 0:
                self.records.append(pending)
                pending = None
        if pending is not None:
            raise SheetError(f"{path}: the last record has an unclosed quote")
        self.header = self.cells(0)

    def cells(self, i: int) -> list[str]:
        return next(csv.reader(io.StringIO(self.records[i], newline="")))

    def row(self, i: int) -> dict[str, str]:
        cells = self.cells(i)
        return {c: (cells[n] if n < len(cells) else "") for n, c in enumerate(self.header)}

    def find(self, name: str, kind: str | None = None, rank: str | None = None) -> list[int]:
        """The record indexes of `calling/key`, narrowed by kind and rank."""
        if name.count("/") != 1:
            raise SheetError(f"{name!r}: name a row as calling/key")
        calling, key = name.split("/")
        found = []
        for i in range(1, len(self.records)):
            row = self.row(i)
            if (row["calling"], row["key"]) != (calling, key):
                continue
            if kind is not None and row["kind"] != kind:
                continue
            if rank is not None and row["rank"] != rank:
                continue
            found.append(i)
        return found

    def check_columns(self, columns) -> None:
        unknown = [c for c in columns if c not in self.header]
        if unknown:
            raise SheetError(f"not a column of the sheet: {', '.join(unknown)}")

    def write_cells(self, i: int, cells: list[str]) -> None:
        buf = io.StringIO()
        csv.writer(buf, lineterminator=self.eol).writerow(cells)
        self.records[i] = buf.getvalue()[: -len(self.eol)]

    def set(self, i: int, values: dict[str, str]) -> None:
        self.check_columns(values)
        cells = self.cells(i)
        for column, value in values.items():
            n = self.header.index(column)
            if n >= len(cells):
                cells += [""] * (n + 1 - len(cells))
            cells[n] = value
        self.write_cells(i, cells)

    def add_row(self, name: str, values: dict[str, str], after: str | None) -> None:
        calling, key = name.split("/") if name.count("/") == 1 else (None, None)
        if calling is None:
            raise SheetError(f"{name!r}: name a row as calling/key")
        if not values.get("kind"):
            raise SheetError("add-row needs kind=")
        self.check_columns(values)
        row = {"calling": calling, "key": key, **values}
        cells = [row.get(c, "") for c in self.header]
        while cells and not cells[-1]:
            cells.pop()
        at = len(self.records)
        if after:
            found = self.find(after)
            if not found:
                raise SheetError(f"--after {after}: no such row")
            at = found[-1] + 1
        self.records.insert(at, "")
        self.write_cells(at, cells)

    def add_column(self, column: str) -> None:
        if column in self.header:
            raise SheetError(f"{column} is already a column")
        self.header.append(column)
        self.write_cells(0, self.header)

    def save(self) -> None:
        text = self.eol.join(self.records) + (self.eol if self.trailing else "")
        open(self.path, "wb").write(text.encode("utf-8"))


def pairs(items: list[str]) -> dict[str, str]:
    out = {}
    for item in items:
        if "=" not in item:
            raise SheetError(f"{item!r}: write column=value")
        column, value = item.split("=", 1)
        out[column] = value
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--csv", default=DEFAULT_CSV)
    commands = parser.add_subparsers(dest="command", required=True)
    get = commands.add_parser("get", help="print a row's non-empty cells, or the columns named")
    put = commands.add_parser("set", help="set cells of a row: column=value ...")
    for p in (get, put):
        p.add_argument("name", help="calling/key")
        p.add_argument("--kind")
        p.add_argument("--rank")
    get.add_argument("columns", nargs="*")
    put.add_argument("values", nargs="+")
    put.add_argument("--all", action="store_true", help="set every row that matches")
    add = commands.add_parser("add-row", help="add a row: column=value ..., kind required")
    add.add_argument("name", help="calling/key")
    add.add_argument("values", nargs="+")
    add.add_argument("--after", help="calling/key the new row follows")
    col = commands.add_parser("add-column", help="append a column to the header")
    col.add_argument("column")
    args = parser.parse_args(argv)
    try:
        sheet = Sheet(args.csv)
        if args.command == "get":
            sheet.check_columns(args.columns)
            found = sheet.find(args.name, args.kind, args.rank)
            if not found:
                raise SheetError(f"{args.name}: no such row")
            for i in found:
                row = sheet.row(i)
                # The record number `build_spells.py` names in its errors.
                print(f"line {i + 1} ({args.name}, {row['kind']}"
                      + (f" rank {row['rank']}" if row["rank"] else "") + ")")
                for column in args.columns or [c for c in sheet.header if row[c]]:
                    print(f"  {column}: {row[column]}")
        elif args.command == "set":
            values = pairs(args.values)
            found = sheet.find(args.name, args.kind, args.rank)
            if not found:
                raise SheetError(f"{args.name}: no such row")
            if len(found) > 1 and not args.all:
                kinds = ", ".join(f"{sheet.row(i)['kind']} rank {sheet.row(i)['rank'] or '-'}" for i in found)
                raise SheetError(f"{args.name} matches {len(found)} rows ({kinds}); narrow with --kind "
                                 "and --rank, or pass --all")
            for i in found:
                sheet.set(i, values)
            sheet.save()
            print(f"set {', '.join(values)} on {len(found)} row(s) of {args.name}")
        elif args.command == "add-row":
            sheet.add_row(args.name, pairs(args.values), args.after)
            sheet.save()
            print(f"added {args.name}")
        else:
            sheet.add_column(args.column)
            sheet.save()
            print(f"added column {args.column}")
    except SheetError as error:
        print(f"sheet: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
