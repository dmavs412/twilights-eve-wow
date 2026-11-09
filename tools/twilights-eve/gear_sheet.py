"""Read, set, scale and restyle rows of the gear sheet by entry, rewriting only the records touched.

    python3.15 tools/twilights-eve/gear_sheet.py get 902080 [column ...]
    python3.15 tools/twilights-eve/gear_sheet.py set 902080 name=X [column=value ...]
    python3.15 tools/twilights-eve/gear_sheet.py scale 902080 902100 item_level=33 required_level=31 \
        [quality=Rare] [name=N] [column=value ...] [--display D] [--data DIR]
    python3.15 tools/twilights-eve/gear_sheet.py looks Chest Mail 40 [--hand 2H] [--spread 5] [--db PATH]
    python3.15 tools/twilights-eve/gear_sheet.py look-check 902100-902179 [904024 ...] [--spread 5] [--db PATH]

`get` prints a row's non-empty cells, or the columns named. `set` changes cells of one row and
refuses an unknown entry or column. Untouched lines are written back byte for byte (`sheet.py`'s
`Sheet`).

`scale` copies row <source> to a new row <entry> at item level L and required level R, written
after the last row whose entry is below <entry>, and refuses an <entry> that exists. The budget
is the quality's budget x P(L), doubled for a two-hander, as build_gear.py's tier rule has it.
Stats and `spell_power` scale by the budget over the source's points, rounded half up, and the
remainder goes on or comes off the largest stat, so the points equal the budget and the `budget`
cell. A weapon's damage scales to its DPS curve x P(L), `speed` kept, within 1%; a weapon with no
curve (an off-hand weapon counts as one-handed, a wand or thrown has none) grows by
P(L) / P(source level). A Chest's armour is the armour rule (cloth and robe 100, leather 175, mail
300, plate 375, x 1.5 x P(L), x 1.15 on a Rare); an Epic or Common Chest is refused. A shield's
armour and block grow by P(L) / P(source level) and the ratio of the quality factors (Uncommon
1 / 1.15, Rare 1, Epic 1.13). `sell_price` grows by the square of P(L) / P(source level). Every
other cell copies. A column=value on the command line wins over the computed cell, and `slot`,
`type`, `hand` and `quality` given there shape the rules above. `--display D` sets `display_id`,
and on a source with a blank `icon` writes the icon its display shows (ItemDisplayInfo.dbc, from
$WOW_DATA or `--data`), so the row keeps its Line's icon.

`looks` lists vanilla items of the vmangos `item_template` whose model no gear row wears, of
quality 2 or better and within `--spread` item levels, nearest first. For a Chest it prints one
line per family (a shared nonzero `set_id`, else the name less its last word, among items of the
chest's armour type and cloaks): the chest's `display_id` and the eight `wardrobe_*` cells, ready
for `set`; a cloth family's robe is preferred. `--hand` picks a Weapon's hand (default 1H, 2H for
a Staff or Polearm).

`look-check` judges the Chest, Weapon, Offhand and Ranged rows named (entries or `a-b` ranges):
it prints each row's item level beside the nearest vanilla item with its `display_id`, and exits
1 when one is more than `--spread` away or has none (a Plate Chest below 41 passes on a plate look
from 41 to 50), when two named Chests of one armour type wear looks in the other order of their
item levels, when two named Chests share a `display_id`, or when a named Chest's Wardrobe equals
another Chest row's. Run `build_gear.py` after an edit to regenerate gear.toml.
"""

from __future__ import annotations

import argparse
import math
import os
import sqlite3
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from build_gear import (  # noqa: E402
    ARMOUR_TYPES, BUDGET, WARDROBE, WEAPON_DPS, WEAPON_TYPES, classify, power_curve,
)
from build_gear import SheetError as GearError  # noqa: E402
from icon_check import read_icons  # noqa: E402
from sheet import Sheet, SheetError, pairs  # noqa: E402

DEFAULT_CSV = os.path.join(HERE, "..", "..", "data", "twilights-eve", "gear.csv")
DEFAULT_DB = r"C:\Users\davis\wow-data\vmangos\mangos.sqlite"
DEFAULT_DATA = r"C:\Users\davis\wow-data\Data"

STATS = ("str", "agi", "sta", "int", "spi")
# The armour rule of a Chest: base armour by type, x 1.5 x P(item level), x the quality's factor.
CHEST_ARMOUR = {"Cloth": 100, "Robe": 100, "Leather": 175, "Mail": 300, "Plate": 375}
CHEST_QUALITY = {"Uncommon": 1.0, "Rare": 1.15}
# A shield's armour and block by quality, relative to a Rare (Hogger's Pen Gate over Hogger's
# Bulwark, the sheet's epic and blue shields at 30).
SHIELD_QUALITY = {"Uncommon": 1 / 1.15, "Rare": 1.0, "Epic": 1.13}
# A weapon's DPS lands within this of its curve.
DPS_CLOSE = 0.01
CLASS_WEAPON = 2
CHEST_INVENTORY = (5, 20)
ROBE_INVENTORY = 20
BACK_INVENTORY = 16
# Vanilla has no plate chest below this item level; a lower Plate Chest may wear one up to 50.
FIRST_PLATE_LOOK, LAST_PLATE_LOOK = 41, 50
LOOK_SLOTS = ("Chest", "Weapon", "Offhand", "Ranged")


def round_half_up(x: float) -> int:
    return math.floor(x + 0.5)


def number(row: dict, column: str) -> int:
    try:
        return int(row[column] or 0)
    except ValueError:
        raise SheetError(f"{row['entry']}: {column} {row[column]!r} is not a whole number") from None


def points(stats: dict[str, int], spell_power: int) -> int:
    """Stat points as build_gear.py counts them: the stats plus half the spell power, rounded up."""
    return sum(stats.values()) + -(-spell_power // 2)


class GearSheet(Sheet):
    """The gear sheet, its rows found by `entry`."""

    def entry(self, i: int) -> int:
        return int(self.cells(i)[0])

    def find_entry(self, entry: int) -> int | None:
        for i in range(1, len(self.records)):
            if self.entry(i) == entry:
                return i
        return None

    def at(self, entry: int) -> int:
        i = self.find_entry(entry)
        if i is None:
            raise SheetError(f"{entry}: no row has this entry")
        return i

    def rows(self) -> list[dict[str, str]]:
        return [self.row(i) for i in range(1, len(self.records))]

    def add_copy(self, source: int, entry: int, values: dict[str, str]) -> int:
        """Copy record `source` to a new record after the last row whose entry is below `entry`,
        then set `values` on it. The new record's index."""
        at = 1
        for i in range(1, len(self.records)):
            if self.entry(i) < entry:
                at = i + 1
        self.records.insert(at, self.records[source])
        self.set(at, values)
        return at


def budget(quality: str, level: int, hand: str) -> int:
    """The stat points of a quality at an item level, as build_gear.py's `check_tier` has them."""
    base = BUDGET[quality]
    if level > 10:
        base = round_half_up(base * power_curve(level))
    return base * (2 if hand == "2H" else 1)


def dps_shape(row: dict) -> str | None:
    """The WEAPON_DPS shape of a row with damage: an off-hand weapon counts as one-handed."""
    if row["slot"] == "Weapon":
        return "2H" if row["hand"] == "2H" else "1H"
    if row["slot"] == "Offhand" and row["type"] in WEAPON_TYPES:
        return "1H"
    if row["slot"] == "Ranged" and row["type"] in ("Bow", "Gun", "Crossbow"):
        return "bow"
    return None


def scaled_stats(src: dict, target: dict, level: int) -> dict[str, str]:
    """The stat, `spell_power` and `budget` cells of `src` scaled to `target`'s budget."""
    want = budget(target["quality"], level, target["hand"])
    stats = {c: number(src, c) for c in STATS}
    spell_power = number(src, "spell_power")
    have = points(stats, spell_power)
    if have == 0:
        raise SheetError(f"{src['entry']} spends no stat points, so it has no shape to scale")
    new = {c: round_half_up(v * want / have) for c, v in stats.items()}
    new_power = round_half_up(spell_power * want / have)
    rest = want - points(new, new_power)
    if any(new.values()):
        new[max(STATS, key=lambda c: new[c])] += rest
    else:
        new_power += 2 * rest
    # A zero stays as the source wrote it: blank or 0.
    cell = lambda column, v: src[column] if v == 0 and number(src, column) == 0 else str(v)  # noqa: E731
    out = {c: cell(c, v) for c, v in new.items()}
    out["spell_power"] = cell("spell_power", new_power)
    out["budget"] = str(points(new, new_power))
    return out


def scaled_damage(src: dict, target: dict, level: int, growth: float) -> dict[str, str]:
    """`dmg_min` and `dmg_max` of `src` scaled to `target`'s DPS curve at `level`, speed kept."""
    if not src["dmg_min"] or not src["speed"]:
        return {}
    try:
        lows = [float(v) for v in src["dmg_min"].split(";")]
        highs = [float(v) for v in src["dmg_max"].split(";")]
        speed = float(src["speed"])
    except ValueError:
        raise SheetError(f"{src['entry']}: a damage or speed that is no number") from None
    have = sum((lo + hi) / 2 for lo, hi in zip(lows, highs)) / speed
    curve = WEAPON_DPS.get((dps_shape(target), target["quality"]))
    want = curve * power_curve(level) if curve else have * growth
    lows = [round_half_up(v * want / have) for v in lows]
    highs = [round_half_up(v * want / have) for v in highs]
    dps = lambda: sum((lo + hi) / 2 for lo, hi in zip(lows, highs)) / speed  # noqa: E731
    # Whole numbers can miss the curve on a small weapon: walk the last line's top to it.
    for _ in range(1000):
        if abs(dps() - want) <= DPS_CLOSE * want:
            break
        highs[-1] += 1 if dps() < want else -1
    return {"dmg_min": ";".join(map(str, lows)), "dmg_max": ";".join(map(str, highs))}


def scaled_armour(src: dict, target: dict, level: int, growth: float) -> dict[str, str]:
    """`armor` (and a shield's `block`) of `src` at `target`'s quality and `level`."""
    quality = target["quality"]
    if target["slot"] == "Chest":
        if quality not in CHEST_QUALITY:
            raise SheetError(f"no armour rule covers a {quality} Chest yet")
        if target["type"] not in CHEST_ARMOUR:
            raise SheetError(f"a Chest of type {target['type']!r}")
        armour = CHEST_ARMOUR[target["type"]] * 1.5 * power_curve(level) * CHEST_QUALITY[quality]
        return {"armor": str(round_half_up(armour))}
    if target["slot"] == "Offhand" and target["type"] == "Shield":
        factor = 1.0
        if quality != src["quality"]:
            if quality not in SHIELD_QUALITY or src["quality"] not in SHIELD_QUALITY:
                raise SheetError(f"no shield factor takes a {src['quality']} shield to {quality}")
            factor = SHIELD_QUALITY[quality] / SHIELD_QUALITY[src["quality"]]
        return {c: str(round_half_up(number(src, c) * growth * factor))
                for c in ("armor", "block") if src[c]}
    return {}


def icon_name(icon: str) -> str:
    """An ItemDisplayInfo icon as an `icon` cell: no folder, no `.tga` or `.blp`."""
    icon = icon.replace("/", "\\").rsplit("\\", 1)[-1]
    return icon[:-4] if icon.lower().endswith((".tga", ".blp")) else icon


def scale(sheet: GearSheet, source: int, entry: int, values: dict[str, str],
          display: int | None, data: str) -> int:
    """Write row `source` scaled to a new row `entry`; the new record's index."""
    src_i = sheet.at(source)
    if sheet.find_entry(entry) is not None:
        raise SheetError(f"{entry} is already a row")
    sheet.check_columns(values)
    for column in ("item_level", "required_level"):
        if column not in values:
            raise SheetError(f"scale needs {column}=")
    src = sheet.row(src_i)
    target = {**src, **values}
    if target["quality"] not in BUDGET:
        raise SheetError(f"quality {target['quality']!r} has no budget")
    level = number(target, "item_level")
    growth = power_curve(level) / power_curve(number(src, "item_level"))
    out = {"entry": str(entry)}
    out.update(scaled_stats(src, target, level))
    out.update(scaled_damage(src, target, level, growth))
    out.update(scaled_armour(src, target, level, growth))
    if src["sell_price"]:
        out["sell_price"] = str(round_half_up(number(src, "sell_price") * growth ** 2))
    if display is not None:
        out["display_id"] = str(display)
        if not src["icon"]:
            icons = read_icons(data)
            shown = number(src, "display_id")
            if shown not in icons:
                raise SheetError(f"display {shown} of {source} is in no ItemDisplayInfo.dbc row")
            out["icon"] = icon_name(icons[shown])
    out.update(values)
    return sheet.add_copy(src_i, entry, out)


def vanilla_items(path: str) -> list[dict]:
    """The vmangos `item_template` rows, the latest patch of each entry."""
    if not os.path.exists(path):
        raise SheetError(f"{path}: no vmangos database (pass --db)")
    db = sqlite3.connect(path)
    try:
        have = {r[1] for r in db.execute("pragma table_info(item_template)")}
        columns = ["entry", "name", "class", "subclass", "inventory_type", "item_level",
                   "display_id", "quality", "set_id"]
        patch = ["patch"] if "patch" in have else []
        rows = db.execute(f"select {', '.join(columns + patch)} from item_template").fetchall()
    finally:
        db.close()
    latest = {}
    for r in rows:
        item = dict(zip(columns + patch, r))
        old = latest.get(item["entry"])
        if old is None or item.get("patch", 0) >= old.get("patch", 0):
            latest[item["entry"]] = item
    return list(latest.values())


def used_displays(sheet: GearSheet) -> set[int]:
    """Every model a gear row wears, as its `display_id` or in a `wardrobe_*` cell."""
    used = set()
    for row in sheet.rows():
        for column in ["display_id"] + [f"wardrobe_{part}" for part, _ in WARDROBE]:
            if number(row, column):
                used.add(number(row, column))
    return used


def family_key(item: dict) -> tuple:
    """A shared nonzero set, else the name less its last word, else the item alone."""
    if item["set_id"]:
        return ("set", item["set_id"])
    words = item["name"].rsplit(" ", 1)
    return ("name", words[0]) if len(words) == 2 else ("item", item["entry"])


def looks(sheet: GearSheet, items: list[dict], slot: str, kind: str, level: int, hand: str | None,
          spread: int) -> list[str]:
    """The lines `looks` prints."""
    if hand is None:
        if slot == "Weapon":
            hand = "2H" if kind in ("Staff", "Polearm") else "1H"
        else:
            hand = "OH" if slot == "Offhand" and kind in WEAPON_TYPES else ""
    cls, sub, inv = classify({"slot": slot, "type": kind, "hand": hand, "entry": "looks",
                              "name": f"{slot} {kind}"})

    def fits(item):
        if cls == CLASS_WEAPON:
            return item["class"] == cls and item["subclass"] == sub
        if slot == "Chest":
            return (item["class"] == cls and item["subclass"] == sub
                    and item["inventory_type"] in CHEST_INVENTORY)
        return (item["class"], item["subclass"], item["inventory_type"]) == (cls, sub, inv)

    used = used_displays(sheet)
    near = lambda item: (abs(item["item_level"] - level), item["item_level"], item["entry"])  # noqa: E731
    found = sorted((i for i in items if fits(i) and i["quality"] >= 2 and i["display_id"]
                    and i["display_id"] not in used and abs(i["item_level"] - level) <= spread),
                   key=near)
    lines = []
    if slot != "Chest":
        shown = set()
        for item in found:
            if item["display_id"] not in shown:
                shown.add(item["display_id"])
                lines.append(f"display_id={item['display_id']}  # item level {item['item_level']}, "
                             f"{item['name']} ({item['entry']})")
        return lines
    robe = kind in ("Cloth", "Robe")
    families = {}
    for item in found:
        families.setdefault(family_key(item), []).append(item)
    chosen = [(min(chests, key=lambda i: ((i["inventory_type"] == ROBE_INVENTORY) != robe,
                                          *near(i))), key)
              for key, chests in families.items()]
    chosen.sort(key=lambda pair: near(pair[0]))
    for chest, key in chosen:
        members = [i for i in items if i["class"] == cls and family_key(i) == key
                   and (i["subclass"] == sub or i["inventory_type"] == BACK_INVENTORY)]
        cells = []
        for part, part_inv in WARDROBE:
            pieces = sorted((i for i in members if i["inventory_type"] == part_inv),
                            key=lambda i: (abs(i["item_level"] - chest["item_level"]), i["entry"]))
            cells.append(f"wardrobe_{part}={pieces[0]['display_id'] if pieces else 0}")
        family = f"set {key[1]}" if key[0] == "set" else f"family {key[1]}"
        lines.append(f"display_id={chest['display_id']} {' '.join(cells)}  # item level "
                     f"{chest['item_level']}, {chest['name']} ({chest['entry']}), {family}")
    return lines


def named_rows(sheet: GearSheet, targets: list[str]) -> list[dict]:
    """The rows the targets name, in sheet order: single entries or `a-b` ranges."""
    spans = []
    for target in targets:
        try:
            lo, hi = (int(v) for v in target.split("-", 1)) if "-" in target else (int(target),) * 2
        except ValueError:
            raise SheetError(f"{target!r}: name an entry or an a-b range") from None
        spans.append((lo, hi))
        if lo == hi:
            sheet.at(lo)
    rows = [r for r in sheet.rows() if any(lo <= int(r["entry"]) <= hi for lo, hi in spans)]
    if not rows:
        raise SheetError(f"no row in {' '.join(targets)}")
    return rows


def wardrobe(row: dict) -> tuple[int, ...]:
    return tuple(number(row, f"wardrobe_{part}") for part, _ in WARDROBE)


def look_check(sheet: GearSheet, items: list[dict], targets: list[str], spread: int) -> tuple[list[str], bool]:
    """The lines `look-check` prints, and whether every check passed."""
    levels = {}
    for item in items:
        levels.setdefault(item["display_id"], []).append(item["item_level"])
    lines, ok, chests = [], True, []
    for row in named_rows(sheet, targets):
        if row["slot"] not in LOOK_SLOTS:
            continue
        level, display = number(row, "item_level"), number(row, "display_id")
        where = f"{row['entry']} {row['name']}"
        found = levels.get(display)
        if not found:
            lines.append(f"fail: {where}: no vanilla item has display {display}")
            ok = False
            continue
        look = min(found, key=lambda v: (abs(v - level), v))
        good = abs(look - level) <= spread
        plate = [v for v in found if FIRST_PLATE_LOOK <= v <= LAST_PLATE_LOOK]
        if (row["slot"], row["type"]) == ("Chest", "Plate") and level < FIRST_PLATE_LOOK and plate:
            look, good = min(plate), True
        lines.append(f"{'ok' if good else 'fail'}: {where}: item level {level}, look {look} "
                     f"(display {display})")
        ok = ok and good
        if row["slot"] == "Chest":
            chests.append((row, level, look))
    for n, (a, a_level, a_look) in enumerate(chests):
        for b, b_level, b_look in chests[n + 1:]:
            if a["display_id"] == b["display_id"]:
                lines.append(f"fail: {a['entry']} and {b['entry']} share display {a['display_id']}")
                ok = False
            if ARMOUR_TYPES.get(a["type"]) != ARMOUR_TYPES.get(b["type"]):
                continue
            if (a_level - b_level) * (a_look - b_look) < 0:
                lines.append(f"fail: {a['entry']} (item level {a_level}, look {a_look}) and "
                             f"{b['entry']} (item level {b_level}, look {b_look}) are out of order")
                ok = False
    reported = set()
    others = [r for r in sheet.rows() if r["slot"] == "Chest"]
    for row, _, _ in chests:
        cells = wardrobe(row)
        if not any(cells):
            continue
        for other in others:
            pair = frozenset((row["entry"], other["entry"]))
            if other["entry"] != row["entry"] and wardrobe(other) == cells and pair not in reported:
                reported.add(pair)
                lines.append(f"fail: {row['entry']} and {other['entry']} wear one wardrobe")
                ok = False
    return lines, ok


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--csv", default=DEFAULT_CSV)
    commands = parser.add_subparsers(dest="command", required=True)
    get = commands.add_parser("get", help="print a row's non-empty cells, or the columns named")
    get.add_argument("entry", type=int)
    get.add_argument("columns", nargs="*")
    put = commands.add_parser("set", help="set cells of a row: column=value ...")
    put.add_argument("entry", type=int)
    put.add_argument("values", nargs="+")
    grow = commands.add_parser("scale", help="copy a row to a new entry at another item level")
    grow.add_argument("source", type=int)
    grow.add_argument("entry", type=int)
    grow.add_argument("values", nargs="+", help="item_level=L required_level=R [column=value ...]")
    grow.add_argument("--display", type=int, help="the new row's display_id")
    grow.add_argument("--data", default=os.environ.get("WOW_DATA", DEFAULT_DATA))
    find = commands.add_parser("looks", help="list free vanilla models near an item level")
    find.add_argument("slot")
    find.add_argument("type")
    find.add_argument("level", type=int)
    find.add_argument("--hand")
    check = commands.add_parser("look-check", help="judge the models of the rows named")
    check.add_argument("targets", nargs="+", help="entries or a-b ranges")
    for p in (find, check):
        p.add_argument("--spread", type=int, default=5)
        p.add_argument("--db", default=DEFAULT_DB)
    args = parser.parse_args(argv)
    try:
        sheet = GearSheet(args.csv)
        if args.command == "get":
            sheet.check_columns(args.columns)
            row = sheet.row(sheet.at(args.entry))
            print(f"{args.entry} {row['name']}")
            for column in args.columns or [c for c in sheet.header if row[c]]:
                print(f"  {column}: {row[column]}")
        elif args.command == "set":
            values = pairs(args.values)
            sheet.set(sheet.at(args.entry), values)
            sheet.save()
            print(f"set {', '.join(values)} on {args.entry}")
        elif args.command == "scale":
            i = scale(sheet, args.source, args.entry, pairs(args.values), args.display, args.data)
            sheet.save()
            row = sheet.row(i)
            print(f"added {args.entry} {row['name']} at line {i + 1}: budget {row['budget']}")
        elif args.command == "looks":
            for line in looks(sheet, vanilla_items(args.db), args.slot, args.type, args.level,
                              args.hand, args.spread):
                print(line)
        else:
            lines, ok = look_check(sheet, vanilla_items(args.db), args.targets, args.spread)
            for line in lines:
                print(line)
            return 0 if ok else 1
    except (SheetError, GearError, sqlite3.Error) as error:
        print(f"gear_sheet: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
