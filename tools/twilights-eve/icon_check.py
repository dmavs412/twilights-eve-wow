#!/usr/bin/env python3
"""List the items that share an inventory icon.

No two items in the game show the same icon. Two display ids can point at one icon, so this
compares icon names, not display ids. An item is a table under an `item` key in
crates/twilight-realm/content/*.toml with both `entry` and `display_id`; its icon is field 5 of
its ItemDisplayInfo.dbc row. Items are grouped by lowercased icon name with a trailing `.tga` or
`.blp` removed (one stock display names `INV_Chest_Fur.tga`), and an icon with two or more
distinct entries is a clash. One entry listed in several files is not a clash. A table with
a `line` (ADR 0026) counts as the item named by its line: every row of one Line is one item, so
a Line on one icon is no clash and the clash listing names the line, not the entry. A synthetic
display (995000 up) is not in the DBC; its icon is the last part of its `item_displays` row's
path in data/twilights-eve/client_dbc_patch.json. A Wardrobe look template (gear.toml, 990000
up) is no item: no bag holds one and no one sees its icon, so it is skipped.

An `item_displays` row with a stock icon (any path not under `Interface\\Icons\\Twilight\\`) must
name an icon some ItemDisplayInfo.dbc row has; one that does not would draw as a missing icon,
and fails.

icon_check_known.txt lists the clashes already in the content when the rule began, one per line,
`icon<TAB>entry,entry,...`. A listed clash passes until fixed. A listed line whose clash is gone
(or has changed its entries) is stale and fails until its line is deleted. `--write-known`
rewrites the file from the current clashes.

Exit 1 on a new clash, a stale known line or an unknown stock icon, 0 otherwise.

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
FIRST_LOOK = 990000  # build_gear.py's first look template
PAINTED_DIR = "interface\\icons\\twilight\\"  # build_spells.py's ART_ICON_DIR, lowercased


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


def icon_key(icon):
    """An icon name as the clash check compares it: lowercased, a trailing `.tga` or `.blp` cut."""
    icon = icon.lower()
    return icon[:-4] if icon.endswith((".tga", ".blp")) else icon


def patch_icons(patch):
    """Synthetic display id to icon name, from the `item_displays` of the client patch."""
    icons = {}
    for row in patch.get("item_displays", []):
        icons[row["id"]] = row["icon"].replace("/", "\\").rsplit("\\", 1)[-1]
    return icons


def unknown_stock_icons(patch, icons):
    """(display id, icon path) of each patch `item_displays` row whose stock icon no display of
    `icons` (display id to icon name, the DBC's) names. A painted icon is never one."""
    names = {icon_key(name) for name in icons.values()}
    unknown = []
    for row in patch.get("item_displays", []):
        path = row["icon"].replace("/", "\\")
        if path.lower().startswith(PAINTED_DIR):
            continue
        if icon_key(path.rsplit("\\", 1)[-1]) not in names:
            unknown.append((row["id"], row["icon"]))
    return unknown


def read_patch(path=PATCH):
    if not os.path.exists(path):
        return {}
    with open(path, "rb") as f:
        return json.load(f)


def collect_items(content_dir):
    """(entry, name, display_id, file, line) for every item table in the content TOML."""
    rows = []

    def walk(o, key, fname):
        if isinstance(o, dict):
            if key == "item" and "entry" in o and "display_id" in o:
                rows.append((o["entry"], o.get("name", "?"), o["display_id"], fname,
                             o.get("line")))
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

    `rows` is (entry, name, display_id, file, line), the line optional; `icons` maps display id
    to icon name. Every row of one Line counts as one item, keyed by the line's name. A look
    template is skipped. Each value
    is a list of (entry or line name, name, file), one per item, entries first in entry order,
    then Lines by name.
    """
    by_icon = collections.defaultdict(dict)
    for entry, name, display_id, fname, *rest in rows:
        icon = icons.get(display_id)
        if not icon or entry >= FIRST_LOOK:
            continue
        line = rest[0] if rest else None
        key = line or entry
        by_icon[icon_key(icon)].setdefault(key, (key, line or name, fname))
    order = lambda key: (isinstance(key, str), key)  # noqa: E731
    return {icon: [ents[k] for k in sorted(ents, key=order)]
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
    return "%s: %s" % (icon, "; ".join(
        "line %s (%s)" % (e, f) if isinstance(e, str) else "%d %s (%s)" % (e, n, f)
        for e, n, f in items))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--data", default=os.environ.get("WOW_DATA", DEFAULT_DATA))
    ap.add_argument("--write-known", action="store_true",
                    help="rewrite icon_check_known.txt from the current clashes")
    args = ap.parse_args()

    dbc = read_icons(args.data)
    patch = read_patch()
    icons = {**dbc, **patch_icons(patch)}
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
    unknown = unknown_stock_icons(patch, dbc)
    for display_id, icon in unknown:
        print("unknown stock icon: display %d names %s, which no ItemDisplayInfo.dbc row has"
              % (display_id, icon))
    print("%d clashes, new %d, known %d, stale known %d, unknown stock icons %d"
          % (len(clashes), len(new), known_count, len(stale), len(unknown)))
    return 1 if new or stale or unknown else 0


if __name__ == "__main__":
    sys.exit(main())
