#!/usr/bin/env python3
"""Check (and with --fix, rewrite) the spot of every placed unit in the realm content TOML.

A row is any table with a `position = [x, y, z]` line. Its Z is compared with the ADT terrain
height. A row inside a WMO placement box (with a 2 yd margin, Z range included) is interior: it
must stand on one of the building's floors (wmo_floor.py) or on the terrain, within the same
limits. `--fix` moves an interior row only onto a floor within 2 yd of its Z; farther off, a
building has several floors and only its author knows which one, so the row is flagged with the
nearest floors and its author writes the Z. Rows with `grounded = false` and maps without
terrain data are left alone.

A Rumour point (`{ x = ..., y = ... }` in a `points = [...]` array) has no Z: it stands on the
terrain, so a point inside a WMO box is flagged (rumour-indoor). Every row and point then passes
the body rules of ADR 0021:
- water: no water surface (ADT MCLQ or WMO MLIQ) more than 1.5 yd over its feet, unless it is a
  GameObject or a creature that swims (`swims = true` on our template, vmangos InhabitType & 2
  on an imported one). A Hub NPC is never exempt.
- wall: no colliding, unwalkable WMO triangle crosses its body, a 0.4 yd capsule from 0.3 to 2 yd
  over its feet.
- roof: it does not stand on a floor of an outdoor WMO group more than 3 yd over the terrain,
  unless the row says `on_top = true`.
- doodad: its body is not inside a placed M2's collision hull, a rock, trunk, fence or cart
  (doodad_check.py). Doodads a WMO places (MODD) are not checked.
Every row's model must also load (model): a creature's display (its own `display_id`, else its
template's) needs its CreatureDisplayInfo row, a positive scale, its CreatureModelData model file
and, without ExtendedDisplayInfo, its texture variations; a GameObject's `display` needs its
GameObjectDisplayInfo row and model file. `--fix` does not touch this rule.
`--fix` moves a row or point that fails one of these to the first spot in rings of 1 to 5 yd
(8 headings each, from +x) that passes every rule, and prints `nudge failed` when none does.

ground_check_known.txt lists the failures already in the content when each rule began, one per
line, `<rule> <file> <x> <y> <label>`. A listed failure passes until fixed, and a listed row that
passes its rule fails until its line is deleted.

Exit 1 when any row is flagged (or fixed), 0 otherwise.

    python3.15 tools/twilights-eve/ground_check.py [--fix] [--file <path>]

Reads the game data from $WOW_DATA (default C:\\Users\\davis\\wow-data\\Data) and imported
creature templates from the vmangos sqlite dump ($VMANGOS_SQLITE).
"""
import argparse
import collections
import glob
import math
import os
import re
import sqlite3
import sys
import tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import doodad_check  # noqa: E402
import wmo_floor  # noqa: E402
import wowmpq  # noqa: E402

CONTENT = os.path.normpath(os.path.join(HERE, "..", "..", "crates", "twilight-realm", "content"))
DEFAULT_DATA = r"C:\Users\davis\wow-data\Data"
DEFAULT_VMANGOS = r"C:\Users\davis\AppData\Local\Temp\vmangos\sqlite-dump\mangos.sqlite"
# Same table as import_barrens_spawns.MAP_DIRS (that module runs its import on load).
MAP_DIRS = {0: "Azeroth", 1: "Kalimdor"}
# Files whose rows stand on Kalimdor unless the row says otherwise (hub.rs and mobs.rs default
# to it); every other file defaults to its top-level `map`, else 0.
FILE_DEFAULT_MAP = {"hub.toml": 1, "mobs.toml": 1}
# Test dungeons whose realm tests (creature_gaze, phased_wards, test_grounds and others) stand on a
# flat plane at z 47.2 and hard-code that height; their Z is a test convention, not terrain.
TEST_FIXTURES = {"gaze_hall.toml", "ward_test.toml", "test_grounds.toml"}
# Files of imported vmangos templates: a creature on one of these swims by vmangos InhabitType.
IMPORTED_TEMPLATES = {"elwynn.toml", "barrens.toml"}
MARGIN = 2.0
ABOVE_LIMIT = 1.0
BELOW_LIMIT = 0.5
# `--fix` moves an interior row onto the nearest floor only within this many yards.
FIX_INTERIOR_YD = 2.0
# A land unit may stand in water this deep, and on an outdoor WMO floor this far over the terrain.
WATER_YD = 1.5
ROOF_YD = 3.0
# `--fix` tries rings of these radii, 8 headings each, before giving up.
NUDGE_RINGS = (1, 2, 3, 4, 5)
# The failures in the content when each rule began, one per line: `<rule> <file> <x> <y> <label>`.
# Delete a line when its row is fixed.
KNOWN = os.path.join(HERE, "ground_check_known.txt")
RULES = ("off-floor", "water", "wall", "roof", "doodad", "model", "rumour-indoor")
# The rules a row is checked against after its Z, and a Rumour point after its terrain.
BODY_RULES = ("water", "wall", "roof", "doodad")

# Table kinds by what they place. Another kind with a position is a creature when its `entry`
# names a template, else a GameObject.
GAMEOBJECT_KINDS = {"gameobject", "portal", "portal.door", "inner_portal", "cover", "entrance", "exit"}
CREATURE_KINDS = {"creature", "creature.on_health", "creature.every", "spawn", "added", "moved", "npc"}

HEADER = re.compile(r"^\s*\[\[?\s*([^\]\s]+)\s*\]")
NAME = re.compile(r'^\s*name\s*=\s*"([^"]*)"')
GUID = re.compile(r"^\s*guid\s*=\s*(\d+)")
MAP = re.compile(r"^\s*map\s*=\s*(\d+)")
ENTRY = re.compile(r"^\s*entry\s*=\s*(\d+)")
GROUNDED = re.compile(r"^\s*grounded\s*=\s*(true|false)")
ON_TOP = re.compile(r"^\s*on_top\s*=\s*(true|false)")
DISPLAY = re.compile(r"^\s*display(?:_id)?\s*=\s*(\d+)")
POSITION = re.compile(
    r"^(?P<head>\s*position\s*=\s*\[\s*)(?P<x>[^,\]\s]+)(?P<s1>\s*,\s*)(?P<y>[^,\]\s]+)"
    r"(?P<s2>\s*,\s*)(?P<z>[^,\]\s]+)(?P<tail>.*)$",
    re.S,
)
ID = re.compile(r"^\s*id\s*=\s*(\d+)")
QUEST = re.compile(r"^\s*quest\s*=\s*(\d+)")
GIVER = re.compile(r"^\s*giver\s*=\s*(\d+)")
POINTS = re.compile(r"^\s*points\s*=\s*\[")
XY = re.compile(r"\{\s*x\s*=\s*(?P<x>-?[\d.]+)\s*,\s*y\s*=\s*(?P<y>-?[\d.]+)")
STRING = re.compile(r'"[^"]*"')

Row = collections.namedtuple("Row", "idx label map grounded x y z kind entry on_top display")
Point = collections.namedtuple("Point", "idx span label map x y kind entry")


def _read_lines(path):
    with open(path, encoding="utf-8", newline="") as f:
        return f.read().split("\n")


def rows(path):
    """(lines, [Row]) for each 3D position line.

    A table's `map`, `grounded`, `entry` and `on_top` may sit on either side of its position
    line, so a row is emitted when the next header or the end of the file closes the table."""
    lines = _read_lines(path)
    file_map = FILE_DEFAULT_MAP.get(os.path.basename(path), 0)
    label = None
    cur = None  # dict for the open table
    out = []

    def close():
        if cur and cur["pos"] is not None:
            out.append(Row(cur["pos"], cur["label"], cur["map"] if cur["map"] is not None else file_map,
                           cur["grounded"], *cur["xyz"], cur["kind"], cur["entry"], cur["on_top"],
                           cur["display"]))

    seen_header = False
    for i, raw in enumerate(lines):
        line = raw.rstrip("\r")
        m = HEADER.match(line)
        if m:
            close()
            seen_header = True
            label = None
            cur = {"pos": None, "label": None, "map": None, "grounded": True, "xyz": None,
                   "kind": m.group(1), "entry": None, "on_top": False, "display": None}
            continue
        m = MAP.match(line)
        if m:
            if seen_header:
                cur["map"] = int(m.group(1))
            else:
                file_map = int(m.group(1))
            continue
        if not seen_header:
            continue
        m = NAME.match(line) or None
        if m:
            label = m.group(1)
            continue
        m = GUID.match(line)
        if m:
            label = "guid " + m.group(1)
            continue
        m = ENTRY.match(line)
        if m:
            cur["entry"] = int(m.group(1))
            continue
        m = GROUNDED.match(line)
        if m:
            cur["grounded"] = m.group(1) == "true"
            continue
        m = ON_TOP.match(line)
        if m:
            cur["on_top"] = m.group(1) == "true"
            continue
        m = DISPLAY.match(line)
        if m:
            cur["display"] = int(m.group(1))
            continue
        m = POSITION.match(line)
        if m:
            try:
                xyz = (float(m.group("x")), float(m.group("y")), float(m.group("z")))
            except ValueError:
                continue
            cur["pos"] = i
            cur["label"] = label
            cur["xyz"] = xyz
    close()
    return lines, out


def xy_points(path):
    """(lines, [Point]) for every `{ x = <num>, y = <num>` inside a `points = [` array: a
    `[[rumour]]`'s spots for its giver, and a `[[rumour.spawn]]`'s. A point's `span` is the
    (start, end) columns of its x and of its y. A spawn table takes its map from its Rumour."""
    lines = _read_lines(path)
    file_map = FILE_DEFAULT_MAP.get(os.path.basename(path), 0)
    out = []
    table = {"kind": None}
    rumour = {"id": None, "map": None, "giver": None}
    depth = 0
    found = []

    def close():
        rumour_map = rumour["map"] if rumour["map"] is not None else file_map
        tmap = table.get("map")
        if table["kind"] == "rumour.spawn":
            base = "rumour %s spawn %s" % (rumour["id"], table.get("quest"))
            entry = table.get("entry")
        else:
            base = "rumour %s" % (table.get("id") if table["kind"] == "rumour" else table["kind"])
            entry = table.get("giver")
        for n, (i, span, x, y) in enumerate(found, 1):
            out.append(Point(i, span, "%s point %d" % (base, n), tmap if tmap is not None else rumour_map,
                             x, y, table["kind"], entry))
        found.clear()

    for i, raw in enumerate(lines):
        line = raw.rstrip("\r")
        if depth == 0:
            m = HEADER.match(line)
            if m:
                close()
                table = {"kind": m.group(1)}
                if table["kind"] == "rumour":
                    rumour = {"id": None, "map": None, "giver": None}
                continue
            if line.lstrip().startswith("#"):
                continue
            for key, rx in (("id", ID), ("map", MAP), ("giver", GIVER), ("quest", QUEST), ("entry", ENTRY)):
                m = rx.match(line)
                if m:
                    table[key] = int(m.group(1))
                    if table["kind"] == "rumour" and key in rumour:
                        rumour[key] = int(m.group(1))
                    break
            m = POINTS.match(line)
            if not m:
                continue
            start = m.end() - 1
        else:
            if line.lstrip().startswith("#"):
                continue
            start = 0
        for m in XY.finditer(line, start):
            found.append((i, (m.span("x"), m.span("y")), float(m.group("x")), float(m.group("y"))))
        code = STRING.sub("", line[start:]).split("#")[0]
        depth += code.count("[") - code.count("]")
    close()
    return lines, out


def interior(terrain, x, y, z):
    """True when (x, y, z) lies in a WMO placement box on its tile, margin included."""
    t, _, _ = terrain._tile_at(x, y)
    if not t:
        return False
    for _path, _ns, xs, ys, zs in t["wmos"]:
        if (xs[0] - MARGIN <= x <= xs[1] + MARGIN and ys[0] - MARGIN <= y <= ys[1] + MARGIN
                and zs[0] - MARGIN <= z <= zs[1] + MARGIN):
            return True
    return False


def stands_on(z, height):
    """True when a unit at `z` stands on a surface at `height`, within the limits."""
    d = z - height
    return -BELOW_LIMIT <= d <= ABOVE_LIMIT


def interior_floors(floors, terrain, x, y):
    """The heights an interior row may stand on: the buildings' floors over it, and the terrain."""
    heights = [s.z for s in floors.surfaces(x, y, MARGIN) if s.floor]
    g = terrain.ground_z(x, y)
    return heights + ([g] if g is not None else [])


def too_deep(water, feet):
    """True when a water surface at `water` stands more than WATER_YD over feet at `feet`."""
    return water is not None and water - feet > WATER_YD


def is_roof(surface, ground, on_top=False):
    """True when a unit standing on WMO `surface` is on a roof: an outdoor group's floor more than
    ROOF_YD over the terrain at `ground`, which only a row saying `on_top = true` may do."""
    return (not on_top and surface is not None and not surface.indoor and ground is not None
            and surface.z - ground > ROOF_YD)


def water_checked(kind, entry, templates, swimmers):
    """Whether the water rule applies to a unit of table `kind` naming template `entry`.
    `templates` maps an entry to {"ours", "swims"}; `swimmers` holds the imported entries
    vmangos lets swim."""
    if kind == "npc":
        return True
    if kind in GAMEOBJECT_KINDS or (kind not in CREATURE_KINDS and entry not in templates):
        return False
    t = templates.get(entry)
    if t and t["ours"]:
        return not t["swims"]
    return entry not in swimmers


def nudge_candidates(x, y):
    """The spots `--fix` tries, in order: rings of 1 to 5 yd, 8 headings each from +x, rounded
    to the 0.1 yd the content writes."""
    for r in NUDGE_RINGS:
        for k in range(8):
            a = math.radians(45 * k)
            yield round(x + r * math.cos(a), 1), round(y + r * math.sin(a), 1)


def nudge(x, y, passes):
    """The first candidate (nudge_candidates) where `passes(cx, cy)` is truthy, as
    (cx, cy, result); None when none passes."""
    for cx, cy in nudge_candidates(x, y):
        got = passes(cx, cy)
        if got:
            return cx, cy, got
    return None


def load_templates(paths):
    """entry -> {"ours", "swims"} over every `[[template]]` in the content files."""
    out = {}
    for path in paths:
        with open(path, "rb") as f:
            doc = tomllib.load(f)
        ours = os.path.basename(path) not in IMPORTED_TEMPLATES
        for t in doc.get("template", []):
            if "entry" in t:
                out[t["entry"]] = {"ours": ours, "swims": bool(t.get("swims", False)),
                                  "display": t.get("display_id")}
    return out


def load_swimmers():
    """The vmangos creature_template entries whose InhabitType has the water bit (2)."""
    path = os.environ.get("VMANGOS_SQLITE", DEFAULT_VMANGOS)
    if not os.path.exists(path):
        print("warning: no vmangos sqlite at %s; imported creatures count as land units" % path)
        return set()
    con = sqlite3.connect(path)
    try:
        return {r[0] for r in con.execute("SELECT entry FROM creature_template WHERE inhabit_type & 2")}
    finally:
        con.close()


def inside(failed):
    """The flag line's ` inside <model>` when a doodad rule failed."""
    return " inside " + failed["doodad"] if failed.get("doodad") else ""


class Models:
    """Whether the client can draw a display: its CreatureDisplayInfo or GameObjectDisplayInfo
    row and the model and texture files it names. `gd` gives `dbc(name)` and `has(path)`; each
    table and file is looked up once."""

    def __init__(self, gd):
        self.gd = gd
        self.tables = {}
        self.files = {}

    def _table(self, name):
        if name not in self.tables:
            rows, block = self.gd.dbc(name)
            self.tables[name] = ({r[0]: r for r in rows}, block)
        return self.tables[name]

    def _has(self, path):
        key = path.lower()
        if key not in self.files:
            self.files[key] = self.gd.has(path)
        return self.files[key]

    def _model(self, path):
        """Why the model file at `path` (an .mdx name, read as .m2) is missing, or None."""
        m2 = doodad_check.model_path(path)
        return None if path and self._has(m2) else "no model file %s" % (m2 or "(empty)")

    def creature(self, display):
        """None when the client can draw creature display `display`, else why not. A display
        with ExtendedDisplayInfo (a humanoid NPC) needs only its model; any other needs each
        texture variation it names beside its model."""
        rows, block = self._table("CreatureDisplayInfo")
        r = rows.get(display)
        if r is None:
            return "no CreatureDisplayInfo row"
        scale = wowmpq.dbc_float(r[4])
        if not scale > 0:
            return "scale %g" % scale
        models, mblock = self._table("CreatureModelData")
        m = models.get(r[1])
        if m is None:
            return "no CreatureModelData row %d" % r[1]
        path = wowmpq.dbc_string(mblock, m[2])
        why = self._model(path)
        if why or r[3]:
            return why
        folder = path.rsplit("\\", 1)[0]
        for off in r[6:9]:
            tex = wowmpq.dbc_string(block, off)
            if tex and not self._has("%s\\%s.blp" % (folder, tex)):
                return "no texture %s\\%s.blp" % (folder, tex)
        return None

    def gameobject(self, display):
        """None when the client can draw GameObject display `display`, else why not."""
        if not display:
            return "display 0"
        rows, block = self._table("GameObjectDisplayInfo")
        r = rows.get(display)
        if r is None:
            return "no GameObjectDisplayInfo row"
        return self._model(wowmpq.dbc_string(block, r[1]))


def model_failure(r, templates, models):
    """(display, why) when the client cannot draw row `r`'s model, None when it can or the row
    places none of its own (a cover, a portal, an explore point). A creature row's display is its
    own `display_id`, else its template's; a GameObject's is its `display`."""
    if r.kind in GAMEOBJECT_KINDS or (r.kind not in CREATURE_KINDS and r.entry not in templates):
        if r.display is None and r.kind != "gameobject":
            return None
        why = models.gameobject(r.display or 0)
        return (r.display or 0, why) if why else None
    if r.display is not None:
        display = r.display
    elif r.entry not in templates:
        return None, "no template"
    else:
        display = templates[r.entry]["display"]
        if display is None:
            return None, "no display_id on template %d" % r.entry
    why = models.creature(display)
    return (display, why) if why else None


def known_key(rule, name, label, x, y):
    return "%s %s %.1f %.1f %s" % (rule, name, x, y, label or "?")


def load_known():
    if not os.path.exists(KNOWN):
        return set()
    with open(KNOWN, encoding="utf-8") as f:
        return {line.strip() for line in f if line.strip() and not line.startswith("#")}


def set_z(lines, idx, z):
    set_xyz(lines, idx, None, None, z)


def set_xyz(lines, idx, x, y, z):
    """Rewrite a position line's x, y (when given) and z, keeping the rest of the line."""
    raw = lines[idx]
    cr = "\r" if raw.endswith("\r") else ""
    m = POSITION.match(raw.rstrip("\r"))
    xs = m.group("x") if x is None else "%.1f" % x
    ys = m.group("y") if y is None else "%.1f" % y
    lines[idx] = (m.group("head") + xs + m.group("s1") + ys + m.group("s2") + "%.2f" % z
                  + m.group("tail") + cr)


def set_xy(lines, idx, span, x, y):
    """Rewrite one point's x and y at `span` on line `idx`. Spans of points to its right move, so
    a caller rewrites a line's points right to left."""
    (x0, x1), (y0, y1) = span
    line = lines[idx]
    lines[idx] = line[:x0] + "%.1f" % x + line[x1:y0] + "%.1f" % y + line[y1:]


class World:
    """The game data for one run: terrain and WMO floors per map, templates and swimmers."""

    def __init__(self, gd, templates, swimmers):
        self.gd = gd
        self.templates = templates
        self.swimmers = swimmers
        self.models = Models(gd)
        self.terrains = {}
        self.floors = {}
        self.doodads = {}

    def terrain(self, map_id):
        if map_id not in self.terrains:
            self.terrains[map_id] = wowmpq.Terrain(self.gd, MAP_DIRS[map_id])
        return self.terrains[map_id]

    def floors_of(self, map_id):
        if map_id not in self.floors:
            self.floors[map_id] = wmo_floor.Floors(self.gd, self.terrain(map_id))
        return self.floors[map_id]

    def doodads_of(self, map_id):
        if map_id not in self.doodads:
            self.doodads[map_id] = doodad_check.Doodads(self.gd, self.terrain(map_id))
        return self.doodads[map_id]

    def body_rules(self, map_id, x, y, z, kind, entry, on_top):
        """The water, wall, roof and doodad rules a unit standing at (x, y, z) fails, each with
        what it is in: {rule: detail}, the detail empty but for a doodad's model names."""
        terrain = self.terrain(map_id)
        floors = self.floors_of(map_id)
        failed = {}
        if water_checked(kind, entry, self.templates, self.swimmers):
            ws = [w for w in (terrain.water_z(x, y), floors.water(x, y)) if w is not None]
            if ws and too_deep(max(ws), z):
                failed["water"] = ""
        if floors.walls_near(x, y, z, margin=MARGIN):
            failed["wall"] = ""
        stood = [s for s in floors.surfaces(x, y, MARGIN) if s.floor and stands_on(z, s.z)]
        g = terrain.ground_z(x, y)
        if stood and not (g is not None and stands_on(z, g)):
            if is_roof(min(stood, key=lambda s: abs(z - s.z)), g, on_top):
                failed["roof"] = ""
        inside = self.doodads_of(map_id).hits(x, y, z)
        if inside:
            failed["doodad"] = " ".join(inside)
        return failed

    def point_rules(self, p, x, y):
        """The rules a Rumour point at (x, y) fails, standing on the terrain; None without terrain."""
        terrain = self.terrain(p.map)
        g = terrain.ground_z(x, y)
        if g is None:
            return None
        failed = {"rumour-indoor": ""} if interior(terrain, x, y, g) else {}
        failed.update(self.body_rules(p.map, x, y, g, p.kind, p.entry, False))
        return failed

    def row_spot(self, r, x, y, z_from):
        """The Z a row nudged to (x, y) stands at: the floor or terrain nearest `z_from` inside a
        building (within FIX_INTERIOR_YD), the terrain outside; None when there is none."""
        terrain = self.terrain(r.map)
        if interior(terrain, x, y, z_from):
            heights = interior_floors(self.floors_of(r.map), terrain, x, y)
            near = min(heights, key=lambda h: abs(z_from - h), default=None)
            return near if near is not None and abs(z_from - near) <= FIX_INTERIOR_YD else None
        return terrain.ground_z(x, y)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--fix", action="store_true")
    ap.add_argument("--file")
    a = ap.parse_args()

    data = os.environ.get("WOW_DATA", DEFAULT_DATA)
    all_paths = sorted(glob.glob(os.path.join(CONTENT, "*.toml")))
    world = World(wowmpq.GameData(data), load_templates(all_paths), load_swimmers())
    paths = [a.file] if a.file else all_paths
    verb = "fixed" if a.fix else "flagged"
    counts = collections.Counter()
    known = load_known()
    per_file = collections.Counter()

    def known_or_flag(rules_failed, name, label, x, y, where):
        """Split `rules_failed` into new failures, counting known ones and stale known lines."""
        new = []
        for rule in RULES[1:]:
            key = known_key(rule, name, label, x, y)
            if rule in rules_failed:
                if key in known:
                    counts["known"] += 1
                else:
                    new.append(rule)
            elif key in known and rule in where:
                counts["stale"] += 1
                print("passes %s now %s %s: delete its line from %s" % (rule, name, label or "?",
                                                                        os.path.basename(KNOWN)))
        return new

    for path in paths:
        name = os.path.basename(path)
        if name in TEST_FIXTURES:
            print("skipped %s (test fixture on a flat plane)" % name)
            continue
        lines, found = rows(path)
        changed = False
        for r in found:
            bad = model_failure(r, world.templates, world.models)
            if known_or_flag({"model": ""} if bad else {}, name, r.label, r.x, r.y, ("model",)):
                counts["model"] += 1
                per_file[name] += 1
                print("model %s:%d %s display %s: %s"
                      % (name, r.idx + 1, r.label or "?", "-" if bad[0] is None else bad[0], bad[1]))
            if not r.grounded:
                continue
            if r.map not in MAP_DIRS:
                counts["interior"] += 1
                continue
            terrain = world.terrain(r.map)
            x, y, z = r.x, r.y, r.z
            label = r.label or "?"
            z_now = z  # the Z the body rules see: the row's, or the one --fix writes
            if interior(terrain, x, y, z):
                counts["interior"] += 1
                key = known_key("off-floor", name, r.label, x, y)
                heights = interior_floors(world.floors_of(r.map), terrain, x, y)
                if any(stands_on(z, h) for h in heights):
                    if key in known:
                        counts["stale"] += 1
                        print("on a floor now %s:%d %s: delete its line from %s"
                              % (name, r.idx + 1, label, os.path.basename(KNOWN)))
                else:
                    nearest = min(heights, key=lambda h: abs(z - h), default=None)
                    if a.fix and key not in known and nearest is not None and abs(z - nearest) <= FIX_INTERIOR_YD:
                        counts[verb] += 1
                        per_file[name] += 1
                        print("fixed %s:%d %s z=%.2f floor=%.2f diff=%+.2f"
                              % (name, r.idx + 1, label, z, nearest, z - nearest))
                        set_z(lines, r.idx, nearest)
                        changed = True
                        z_now = nearest
                    elif key in known:
                        counts["known"] += 1
                        continue
                    else:
                        counts["off-floor"] += 1
                        per_file[name] += 1
                        below = max((h for h in heights if h < z), default=None)
                        above = min((h for h in heights if h > z), default=None)
                        print("off-floor %s:%d %s z=%.2f: nearest floor below %s, above %s (wmo_floor.py %.1f %.1f)"
                              % (name, r.idx + 1, label, z,
                                 "none" if below is None else "%.2f" % below,
                                 "none" if above is None else "%.2f" % above, x, y))
                        continue
            else:
                g = terrain.ground_z(x, y)
                if g is None:
                    counts["no-terrain"] += 1
                    continue
                counts["checked"] += 1
                d = z - g
                if d > ABOVE_LIMIT or -d > BELOW_LIMIT:
                    counts[verb] += 1
                    per_file[name] += 1
                    print("%s %s:%d %s z=%.2f terrain=%.2f diff=%+.2f" % (verb, name, r.idx + 1, label, z, g, d))
                    if not a.fix:
                        continue
                    set_z(lines, r.idx, g)
                    changed = True
                    z_now = g

            failed = world.body_rules(r.map, x, y, z_now, r.kind, r.entry, r.on_top)
            new = known_or_flag(failed, name, r.label, x, y, BODY_RULES)
            if not new:
                continue
            if not a.fix:
                counts[verb] += 1
                per_file[name] += 1
                print("%s %s:%d %s%s (%.1f %.1f %.2f)" % (" ".join(new), name, r.idx + 1, label, inside(failed), x, y, z_now))
                continue

            def passes(cx, cy, r=r, z_now=z_now):
                cz = world.row_spot(r, cx, cy, z_now)
                if cz is None or world.body_rules(r.map, cx, cy, cz, r.kind, r.entry, r.on_top):
                    return None
                return cz

            moved = nudge(x, y, passes)
            if moved is None:
                counts["nudge-failed"] += 1
                per_file[name] += 1
                print("nudge failed %s:%d %s %s" % (name, r.idx + 1, label, " ".join(new)))
                continue
            cx, cy, cz = moved
            counts[verb] += 1
            per_file[name] += 1
            print("nudged %s:%d %s %s: (%.1f %.1f) to (%.1f %.1f %.2f)"
                  % (name, r.idx + 1, label, " ".join(new), x, y, cx, cy, cz))
            set_xyz(lines, r.idx, cx, cy, cz)
            changed = True

        # Row edits touch only position lines, so the points' spans still hold in `lines`.
        _, points = xy_points(path)
        moves = []
        for p in points:
            if p.map not in MAP_DIRS:
                continue
            failed = world.point_rules(p, p.x, p.y)
            if failed is None:
                counts["no-terrain"] += 1
                continue
            counts["points"] += 1
            new = known_or_flag(failed, name, p.label, p.x, p.y, BODY_RULES + ("rumour-indoor",))
            if not new:
                continue
            if not a.fix:
                counts[verb] += 1
                per_file[name] += 1
                print("%s %s:%d %s%s (%.1f %.1f)" % (" ".join(new), name, p.idx + 1, p.label, inside(failed), p.x, p.y))
                continue
            moved = nudge(p.x, p.y, lambda cx, cy, p=p: world.point_rules(p, cx, cy) == {})
            if moved is None:
                counts["nudge-failed"] += 1
                per_file[name] += 1
                print("nudge failed %s:%d %s %s" % (name, p.idx + 1, p.label, " ".join(new)))
                continue
            counts[verb] += 1
            per_file[name] += 1
            print("nudged %s:%d %s %s: (%.1f %.1f) to (%.1f %.1f)"
                  % (name, p.idx + 1, p.label, " ".join(new), p.x, p.y, moved[0], moved[1]))
            moves.append((p.idx, p.span, moved[0], moved[1]))
        for idx, span, cx, cy in sorted(moves, key=lambda m: (m[0], -m[1][0][0])):
            set_xy(lines, idx, span, cx, cy)
            changed = True
        if changed:
            with open(path, "w", encoding="utf-8", newline="") as f:
                f.write("\n".join(lines))

    for n, c in sorted(per_file.items()):
        print("  %s: %d" % (n, c))
    print("checked %d rows and %d points, %s %d, interior %d (off-floor %d), known %d, stale known %d, "
          "nudge failed %d, model %d, no-terrain %d"
          % (counts["checked"], counts["points"], verb, counts[verb], counts["interior"],
             counts["off-floor"], counts["known"], counts["stale"], counts["nudge-failed"], counts["model"],
             counts["no-terrain"]))
    return 1 if (counts[verb] or counts["off-floor"] or counts["stale"] or counts["nudge-failed"]
                 or counts["model"]) else 0


if __name__ == "__main__":
    sys.exit(main())
