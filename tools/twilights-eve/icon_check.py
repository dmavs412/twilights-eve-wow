#!/usr/bin/env python3
"""List the items that share an inventory icon.

No two items in the game show the same icon. Two display ids can point at one icon, so this
compares icon names, not display ids. An item is a table under an `item` key in
crates/twilight-realm/content/*.toml with both `entry` and `display_id`; its icon is field 5 of
its ItemDisplayInfo.dbc row. Items are grouped by lowercased icon name, and an icon with two or
more distinct entries is a clash. One entry listed in several files is not a clash. A synthetic
display (995000 up) is not in the DBC; its icon is the stem of its `item_displays` row in
data/twilights-eve/client_dbc_patch.json.

icon_check_known.txt lists the clashes already in the content when the rule began, one per line,
`icon<TAB>entry,entry,...`. A listed clash passes until fixed. A listed line whose clash is gone
(or has changed its entries) is stale and fails until its line is deleted. `--write-known`
rewrites the file from the current clashes.

Exit 1 on a new clash or a stale known line, 0 otherwise.

    python3.15 tools/twilights-eve/icon_check.py [--data <dir>] [--write-known]

Reads the game data from $WOW_DATA (default C:\\Users\\davis\\wow-data\\Data).
"""
import argparse
import collections
import glob
import json
import os
import struct
import sys
import tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import mpq_dbc  # noqa: E402

CONTENT = os.path.normpath(os.path.join(HERE, "..", "..", "crates", "twilight-realm", "content"))
DEFAULT_DATA = r"C:\Users\davis\wow-data\Data"
KNOWN = os.path.join(HERE, "icon_check_known.txt")
PATCH = os.path.normpath(os.path.join(HERE, "..", "..", "data", "twilights-eve", "client_dbc_patch.json"))
ICON_FIELD = 5


def read_icons(data_dir):
    """Display id to icon name, from every ItemDisplayInfo.dbc row."""
    blob, _ = mpq_dbc.read_dbc_file(data_dir, "ItemDisplayInfo.dbc")
    _, nrec, nfield, recsize, _ = struct.unpack_from("<4sIIII", blob, 0)
    strings = blob[20 + nrec * recsize:]
    icons = {}
    for r in range(nrec):
        vals = struct.unpack_from("<%dI" % nfield, blob, 20 + r * recsize)
        o = vals[ICON_FIELD]
        icons[vals[0]] = strings[o:strings.index(b"\0", o)].decode("utf-8", "replace")
    return icons


def patch_icons(patch):
    """Synthetic display id to icon name, from the `item_displays` of the client patch."""
    icons = {}
    for row in patch.get("item_displays", []):
        icons[row["id"]] = row["icon"].replace("/", "\\").rsplit("\\", 1)[-1]
    return icons


def read_patch_icons(path=PATCH):
    if not os.path.exists(path):
        return {}
    with open(path, "rb") as f:
        return patch_icons(json.load(f))


def collect_items(content_dir):
    """(entry, name, display_id, file) for every item table in the content TOML."""
    rows = []

    def walk(o, key, fname):
        if isinstance(o, dict):
            if key == "item" and "entry" in o and "display_id" in o:
                rows.append((o["entry"], o.get("name", "?"), o["display_id"], fname))
            for k, v in o.items():
                walk(v, k, fname)
        elif isinstance(o, list):
            for v in o:
                walk(v, key, fname)

    for path in sorted(glob.glob(os.path.join(content_dir, "*.toml"))):
        with open(path, "rb") as f:
            walk(tomllib.load(f), "", os.path.basename(path))
    return rows


def find_clashes(rows, icons):
    """Icon name to the items on it, for each icon with two or more distinct entries.

    `rows` is (entry, name, display_id, file); `icons` maps display id to icon name. Each
    value is a list of (entry, name, file), one per entry, ordered by entry.
    """
    by_icon = collections.defaultdict(dict)
    for entry, name, display_id, fname in rows:
        icon = icons.get(display_id)
        if not icon:
            continue
        by_icon[icon.lower()].setdefault(entry, (entry, name, fname))
    return {icon: [ents[e] for e in sorted(ents)]
            for icon, ents in sorted(by_icon.items()) if len(ents) > 1}


def known_line(icon, items):
    return "%s\t%s" % (icon, ",".join(str(e) for e, _, _ in items))


def load_known(path=KNOWN):
    if not os.path.exists(path):
        return set()
    with open(path, encoding="utf-8") as f:
        return {line.rstrip("\n") for line in f if line.strip() and not line.startswith("#")}


def judge(clashes, known):
    """Split clashes into (new, known_count, stale): `new` maps icon to items, `stale` is lines."""
    new = {}
    seen = set()
    for icon, items in clashes.items():
        line = known_line(icon, items)
        if line in known:
            seen.add(line)
        else:
            new[icon] = items
    return new, len(seen), sorted(known - seen)


def show(icon, items):
    return "%s: %s" % (icon, "; ".join("%d %s (%s)" % (e, n, f) for e, n, f in items))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--data", default=os.environ.get("WOW_DATA", DEFAULT_DATA))
    ap.add_argument("--write-known", action="store_true",
                    help="rewrite icon_check_known.txt from the current clashes")
    args = ap.parse_args()

    icons = read_icons(args.data)
    icons.update(read_patch_icons())
    clashes = find_clashes(collect_items(CONTENT), icons)
    if args.write_known:
        with open(KNOWN, "wb") as f:
            for icon, items in clashes.items():
                f.write((known_line(icon, items) + "\n").encode("utf-8"))
        print("wrote %d clashes to %s" % (len(clashes), KNOWN))
        return 0

    new, known_count, stale = judge(clashes, load_known())
    for icon, items in new.items():
        print("clash " + show(icon, items))
    for line in stale:
        print("stale known line: " + line.replace("\t", " "))
    print("%d clashes, new %d, known %d, stale known %d" % (len(clashes), len(new), known_count,
                                                            len(stale)))
    return 1 if new or stale else 0


if __name__ == "__main__":
    sys.exit(main())
