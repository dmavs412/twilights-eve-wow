"""One function per rule in docs/balance/rules.md. Each takes (levels, fights, callings, run),
where `run` maps run_cell over a list of cells, and returns rows of dict(rule, calling, level,
gear, against, fights, win, health_left, dps, kill_s, deaths, notes) plus `holds` (True, False,
or None for a row the rule only compares against) and the kit and build (`rows`) the cell used.
pick_build picks each Calling's build a level, once a run."""
from __future__ import annotations

import json
import math
import random
import statistics
import zlib
from collections import defaultdict

import boss
import model
import multi

DAMAGE_CALLINGS = ("cutthroat", "ranger", "initiate")
TANK_CALLINGS = ("vanguard", "acolyte")
GENERIC_BOSS = (2.0, 1.1)   # rank 1 with these template multipliers and no spells
# The highest level the Calling rules run. docs/balance/rules.md words them for 10 to 30; past 30
# a run is for reading, not a gate (#661).
TOP_LEVEL = 50

# Rule -> (who, gear, against, what it must show), as docs/balance/rules.md words them.
RULES = {
    "T0-1": ("Basic Warrior, levels 1 to 9", "starter + whites", "1 normal on level", "win >= 97%"),
    "T0-2": ("Basic Warrior, levels 1 to 9", "starter + whites", "2 normals on level", "win >= 90%"),
    "P-2": ("each Calling, levels 10 to 30", "greens", "2 normals on level", "win >= 80%, health left >= 20%"),
    "P-3": ("each Calling, levels 10 to 30", "greens", "3 normals on level", "win between 15% and 50%"),
    "DPS": ("Cutthroat, Ranger, Initiate, levels 10 to 30", "greens", "1 normal on level",
            "each one's DPS above both the Vanguard's and the Acolyte's"),
    "E-dmg": ("Cutthroat, Ranger, Initiate, levels 10 to 30", "greens", "1 elite on level", "win between 35% and 65%"),
    "E-tank": ("Vanguard, Acolyte, levels 10 to 30", "greens", "1 elite on level",
               "win >= 70%, and above every damage Calling's win at that level"),
    "B-2E": ("each Calling, levels 10 to 30", "blues", "2 elites on level", "win >= 80%"),
    "B-boss": ("each Calling, levels 10 to 30", "blues", "generic boss on level", "win >= 80%"),
    "B-named": ("each Calling", "blues at the boss's level", "each named boss", "win >= 80%"),
    "G+5": ("each Calling, levels 15 to 30", "greens", "generic boss 5 levels below the character", "win >= 75%"),
    "G+5-named": ("each Calling", "greens, 5 levels above the boss", "each named boss", "win >= 75%"),
}


# ---------------------------------------------------------------- cells
def normals(level, n):
    return dict(kind="normal", level=level, n=n)


def elites(level, n):
    return dict(kind="elite", level=level, n=n)


def generic_boss(level):
    return dict(kind="generic_boss", level=level, n=1)


def named(b):
    return dict(kind="named", level=b["level"], n=1, entry=b["entry"], file=b["file"])


def cell(calling, level, gear, against, fights):
    return dict(calling=calling, level=level, gear=gear, against=against, fights=fights)


def describe(against):
    k, n, lv = against["kind"], against["n"], against["level"]
    if k == "named":
        b = _bosses()[(against["file"], against["entry"])]
        return f"{b['name']} ({b['entry']}) L{lv}"
    word = {"normal": "normal", "elite": "elite", "generic_boss": "generic boss"}[k]
    return f"{n} {word}{'s' if n > 1 else ''} L{lv}"


_BOSSES = {}
_CHARACTERS = {}
_BUILDS = {}   # (calling, level, fights) -> the picked build


def _bosses():
    if not _BOSSES:
        _BOSSES.update({(b["file"], b["entry"]): b for b in boss.named_bosses()})
    return _BOSSES


def _character(calling, level, gear, rows):
    key = (calling, level, gear, tuple(sorted(rows.items())))
    if key not in _CHARACTERS:
        _CHARACTERS[key] = model.character(calling, level, gear, rows)
    return _CHARACTERS[key]


def _fights(cls, calling, level, ch, mob, n, rng):
    """`n` fresh fights of the character `ch` against `mob` on `rng`: each one's result, with the
    damage it dealt as `dealt`."""
    out = []
    for _ in range(n):
        sim = cls(calling, level, ch["stats"], ch["rows"], rng, {}, mob=mob)
        r = sim.fight()
        r["dealt"] = sum(sim.dmg.values())
        out.append(r)
    return out


# ---------------------------------------------------------------- builds
TIE_PCT = 1.0       # choices within this share of the best DPS tie
PICK_FIGHTS = 150   # the most fights a pick gives a choice: at 300 the picks more than doubled a full run


def choose(results):
    """The choice a row is set to, from [(choice key, dps, health left)] in sheet order: the highest
    DPS. Every choice within TIE_PCT of the best ties, and a tie goes to the most health left, then
    to the first in the sheet."""
    best = max(dps for _, dps, _ in results)
    tied = [r for r in results if r[1] >= best * (100 - TIE_PCT) / 100]
    return max(tied, key=lambda r: r[2])[0]


def pick_build(calling, level, fights):
    """`calling`'s build at Power Level `level` (docs/balance/rules.md, "The character"): its open
    rows in level order, each set to choose's pick of its three choices in the DPS rule's fight (1
    normal on level, greens), `fights` fights a choice with the earlier rows set and the later rows
    empty. A row's three choices fight on one seed, from the Calling, the level and the row."""
    rows = {}
    cls, mob = multi.fighter(calling, 1), model.mob_stats(level)
    for row in model.open_rows(calling, level):
        seed = zlib.crc32(json.dumps([calling, level, row["key"]]).encode())
        results = []
        for c in row["choices"]:
            ch = _character(calling, level, "green", {**rows, row["key"]: c["key"]})
            rs = _fights(cls, calling, level, ch, mob, fights, random.Random(seed))
            spent = sum(r["time"] for r in rs)
            dps = sum(r["dealt"] for r in rs) / spent if spent else 0.0
            results.append((c["key"], dps, statistics.mean(r["health_left"] for r in rs)))
        rows[row["key"]] = choose(results)
    return rows


def picked(key):
    """pick_build for one (calling, level, fights): what with_builds sends a pool."""
    return pick_build(*key)


def _build_key(c):
    """The pick a cell fights in: its Calling, its level, and its fights up to PICK_FIGHTS."""
    return c["calling"], c["level"], min(c["fights"], PICK_FIGHTS)


def with_builds(cells, run_map=map):
    """`cells`, each carrying its Calling and level's picked build as `rows`. The builds no cell
    has needed yet are picked here, through `run_map` (a pool's map runs them in parallel)."""
    want = sorted({_build_key(c) for c in cells} - set(_BUILDS))
    _BUILDS.update(zip(want, run_map(picked, want)))
    return [{**c, "rows": _BUILDS[_build_key(c)]} for c in cells]


def run_cell(c):
    """`fights` fresh fights of one character, in its level's picked build (the cell's `rows`, else
    picked here), against one pull. The seed comes from the cell, so a rerun repeats it."""
    calling, level, gear, against, fights = c["calling"], c["level"], c["gear"], c["against"], c["fights"]
    rows = c["rows"] if "rows" in c else with_builds([c])[0]["rows"]
    rng = random.Random(zlib.crc32(json.dumps({k: v for k, v in c.items() if k != "rows"}, sort_keys=True).encode()))
    ch = _character(calling, level, gear, rows)
    notes = []
    kind, lv = against["kind"], against["level"]
    if kind == "named":
        b = _bosses()[(against["file"], against["entry"])]
        cls, mob = boss.fight_class(calling, b)
        if b["unmodelled"]:
            notes.append("unmodelled: " + ", ".join(b["unmodelled"]))
    else:
        cls = multi.fighter(calling, against["n"])
        if kind == "normal":
            mob = model.mob_stats(lv)
        elif kind == "elite":
            mob = model.mob_stats(lv, rank=1)
        else:
            mob = model.mob_stats(lv, rank=1, health_mul=GENERIC_BOSS[0], damage_mul=GENERIC_BOSS[1])
    if ch["fell_back"]:
        notes.append("green in " + ", ".join(ch["fell_back"]))
    won, left, kills, deaths, dealt, spent = 0, [], [], 0, 0.0, 0.0
    for r in _fights(cls, calling, level, ch, mob, fights, rng):
        dealt += r["dealt"]
        spent += r["time"]
        deaths += r["died"]
        if r["won"]:
            won += 1
            left.append(r["health_left"])
            kills.append(r["time"])
    return dict(calling=calling, level=level, gear=gear, against=describe(against), fights=fights,
                win=won / fights, health_left=statistics.mean(left) if left else math.nan,
                dps=dealt / spent if spent else 0.0, kill_s=statistics.median(kills) if kills else math.nan,
                deaths=deaths / fights, notes="; ".join(notes), mob_hp=mob["hp"],
                kit=model.describe_kit(ch["entries"], ch["off"]), fell_back=ch["fell_back"], rows=ch["rows"])


def _rows(rule, cells, run, holds):
    rows = run(cells)
    for r in rows:
        r["rule"] = rule
        r["holds"] = holds(r)
    return rows


def _levels(levels, lo, hi):
    return [lv for lv in levels if lo <= lv <= hi]


def _pick(callings, among):
    return [c for c in among if c in callings]


# ---------------------------------------------------------------- the rules
def t0_1(levels, fights, callings, run):
    cells = [cell("basic_warrior", lv, "white", normals(lv, 1), fights) for lv in _levels(levels, 1, 9)]
    return _rows("T0-1", cells, run, lambda r: r["win"] >= 0.97)


def t0_2(levels, fights, callings, run):
    cells = [cell("basic_warrior", lv, "white", normals(lv, 2), fights) for lv in _levels(levels, 1, 9)]
    return _rows("T0-2", cells, run, lambda r: r["win"] >= 0.90)


def p_2(levels, fights, callings, run):
    cells = [cell(c, lv, "green", normals(lv, 2), fights) for lv in _levels(levels, 10, TOP_LEVEL) for c in callings]
    return _rows("P-2", cells, run, lambda r: r["win"] >= 0.80 and r["health_left"] >= 0.20)


def p_3(levels, fights, callings, run):
    cells = [cell(c, lv, "green", normals(lv, 3), fights) for lv in _levels(levels, 10, TOP_LEVEL) for c in callings]
    return _rows("P-3", cells, run, lambda r: 0.15 <= r["win"] <= 0.50)


def dps(levels, fights, callings, run):
    """The damage Callings asked for, each against the Vanguard and the Acolyte, who are run
    whatever `callings` says and shown with no verdict of their own."""
    who = _pick(callings, DAMAGE_CALLINGS) + list(TANK_CALLINGS)
    cells = [cell(c, lv, "green", normals(lv, 1), fights) for lv in _levels(levels, 10, TOP_LEVEL) for c in who]
    rows = _rows("DPS", cells, run, lambda r: None)
    tanks = defaultdict(list)
    for r in rows:
        if r["calling"] in TANK_CALLINGS:
            tanks[r["level"]].append(r["dps"])
    for r in rows:
        if r["calling"] in DAMAGE_CALLINGS:
            r["holds"] = r["dps"] > max(tanks[r["level"]])
    return rows


def e_dmg(levels, fights, callings, run):
    cells = [cell(c, lv, "green", elites(lv, 1), fights)
             for lv in _levels(levels, 10, TOP_LEVEL) for c in _pick(callings, DAMAGE_CALLINGS)]
    return _rows("E-dmg", cells, run, lambda r: 0.35 <= r["win"] <= 0.65)


def e_tank(levels, fights, callings, run):
    """The Vanguard and Acolyte asked for, against every damage Calling's elite win at the level
    (those are run here too, shown with no verdict)."""
    who = _pick(callings, TANK_CALLINGS) + list(DAMAGE_CALLINGS)
    cells = [cell(c, lv, "green", elites(lv, 1), fights) for lv in _levels(levels, 10, TOP_LEVEL) for c in who]
    rows = _rows("E-tank", cells, run, lambda r: None)
    dealers = defaultdict(list)
    for r in rows:
        if r["calling"] in DAMAGE_CALLINGS:
            dealers[r["level"]].append(r["win"])
    for r in rows:
        if r["calling"] in TANK_CALLINGS:
            r["holds"] = r["win"] >= 0.70 and r["win"] > max(dealers[r["level"]])
    return rows


def b_2e(levels, fights, callings, run):
    cells = [cell(c, lv, "blue", elites(lv, 2), fights) for lv in _levels(levels, 10, TOP_LEVEL) for c in callings]
    return _rows("B-2E", cells, run, lambda r: r["win"] >= 0.80)


def b_boss(levels, fights, callings, run):
    cells = [cell(c, lv, "blue", generic_boss(lv), fights) for lv in _levels(levels, 10, TOP_LEVEL) for c in callings]
    return _rows("B-boss", cells, run, lambda r: r["win"] >= 0.80)


def _named(rule, gear, above, need, fights, callings, run):
    """Every named boss, whatever the levels asked for: the boss sets the level. A Tier 1
    character starts at 10, so a boss below 10 is fought at 10. Above 30 the character is fought at
    the level it wants (G+5-named puts it at 35 against Hogger), on the provisional level table."""
    cells = []
    for b in _bosses().values():
        lv = max(b["level"] + above, 10)
        cells += [cell(c, lv, gear, named(b), fights) for c in callings]
    rows = _rows(rule, cells, run, lambda r: r["win"] >= need)
    for r, c in zip(rows, cells):
        want = c["against"]["level"] + above
        if want != r["level"]:
            r["notes"] = "; ".join(filter(None, [f"wants level {want}", r["notes"]]))
    return rows


def b_named(levels, fights, callings, run):
    return _named("B-named", "blue", 0, 0.80, fights, callings, run)


def g_5(levels, fights, callings, run):
    cells = [cell(c, lv, "green", generic_boss(lv - 5), fights) for lv in _levels(levels, 15, TOP_LEVEL) for c in callings]
    return _rows("G+5", cells, run, lambda r: r["win"] >= 0.75)


def g_5_named(levels, fights, callings, run):
    return _named("G+5-named", "green", 5, 0.75, fights, callings, run)


SCENARIOS = {
    "T0": [t0_1, t0_2],
    "pulls": [p_2, p_3],
    "dps": [dps],
    "elites": [e_dmg, e_tank],
    "blues": [b_2e, b_boss],
    "named": [b_named],
    "plus5": [g_5, g_5_named],
}
