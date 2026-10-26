#!/usr/bin/env python3
"""Build the realm's Gear table from the gear sheet.

    python tools/twilights-eve/build_gear.py [--csv FILE] [--drops FILE] [--master FILE] [--out FILE]

Reads data/twilights-eve/gear.csv, the hand-edited source of every piece of Gear (ADR 0007), and
writes crates/twilight-realm/content/gear.toml, the item templates the realm loads (`--out -`
writes it to stdout). The sheet speaks in words (slot, type, hand, quality, one column per stat);
this maps them to 1.12's class, subclass, inventory type and quality ids.

Who drops what is not the sheet's: data/twilights-eve/gear_drops.csv gives the loot level
(`pool_level`) of each piece a world pool holds.

A Chest's Wardrobe (`wardrobe_*`, ItemDisplayInfo ids) becomes eight look templates: one
synthetic, statless item per body part and display, entries from 990000 up, which the realm
writes into the visible-item fields and the item query answers. No pool, vendor or reward
holds one.

Exits non-zero, writing nothing, on an unknown word, a duplicate entry, a wardrobe cell on a row
that is no Chest, a slot outside the ten, a spell the content does not know, or a drop row whose
entry is no Gear.

Columns besides the words: `speed` in seconds; `dmg_min`, `dmg_max` and `dmg_school` hold one
value per damage line, `;`-separated (a weapon with a bonus school hit has two); `equip`, `use`
and `on_hit` are `;`-separated spell ids.

`spell_power` (#272) is the flat Spell Power a Weapon, a held Offhand or a Wand carries; any
other slot with one is refused. The realm reads the column; the client sees it as the vanilla
"Increase Spell Dam N" item spell appended to `equip`, so the tooltip shows the line. `budget` is
the stat points a hand-authored row may spend (str + agi + sta + int + spi + ceil(spell_power / 2));
a row over it is refused, and a blank is unchecked. It is not written to gear.toml.

Northshire rows (entry 900000 and up, plus the outfit whites) are held to the tiers of #426: stat
points within 2 of the quality's budget (Common 2, Uncommon 6, Rare 13, Epic 18, doubled for a
two-hander), the `budget` cell equal to the points, and a weapon's DPS within 10% of its curve.
Every other row is unchecked.
"""

from __future__ import annotations

import argparse
import csv
import os
import sys

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
DEFAULT_CSV = os.path.join(REPO, "data", "twilights-eve", "gear.csv")
DEFAULT_DROPS = os.path.join(REPO, "data", "twilights-eve", "gear_drops.csv")
DEFAULT_MASTER = os.path.join(REPO, "data", "twilights-eve", "master.csv")
DEFAULT_OUT = os.path.join(REPO, "crates", "twilight-realm", "content", "gear.toml")

COLUMNS = (
    "entry", "name", "slot", "type", "hand", "quality", "item_level", "required_level",
    "str", "agi", "sta", "int", "spi", "armor", "block", "dmg_min", "dmg_max", "dmg_school",
    "speed", "res_holy", "res_fire", "res_nature", "res_frost", "res_shadow", "res_arcane",
    "attack_power", "crit", "defense", "spell_power", "budget", "line", "equip", "use", "on_hit",
    "display_id", "sell_price",
    "bonding", "durability", "sheath", "material", "wardrobe_head", "wardrobe_shoulders",
    "wardrobe_back", "wardrobe_wrists", "wardrobe_hands", "wardrobe_waist", "wardrobe_legs",
    "wardrobe_feet",
)

# The hidden body parts, in the order a Chest's `wardrobe` lists them, with the `InventoryType`
# their look templates carry.
WARDROBE = (
    ("head", 1), ("shoulders", 3), ("back", 16), ("wrists", 9), ("hands", 10), ("waist", 6),
    ("legs", 7), ("feet", 8),
)
# Look templates take entries from here up; the sheet's own entries stay below.
FIRST_LOOK = 990000

QUALITY = {"Poor": 0, "Common": 1, "Uncommon": 2, "Rare": 3, "Epic": 4}
BONDING = {"": 0, "Pickup": 1, "Equip": 2, "Use": 3}
SHEATH = {"": 0, "Back": 1, "Staff": 2, "Side": 3, "Shield": 4, "Rod": 5, "Offhand": 6, "Hip": 7}
MATERIAL = {"": 0, "Metal": 1, "Wood": 2, "Liquid": 3, "Jewelry": 4, "Chain": 5, "Plate": 6,
            "Cloth": 7, "Leather": 8}
SCHOOL = {"Physical": 0, "Holy": 1, "Fire": 2, "Nature": 3, "Frost": 4, "Shadow": 5, "Arcane": 6}
STATS = (("str", 4), ("agi", 3), ("sta", 7), ("int", 5), ("spi", 6))
RESISTANCES = ("holy", "fire", "nature", "frost", "shadow", "arcane")

# (class, subclass) of each `type`; an axe, mace or sword's subclass follows its hand.
WEAPON_TYPES = {
    "Axe": (0, 1), "Mace": (4, 5), "Sword": (7, 8), "Dagger": (15, 15), "Fist": (13, 13),
    "Staff": (10, 10), "Polearm": (6, 6),
}
ARMOUR_TYPES = {"Cloth": 1, "Leather": 2, "Mail": 3, "Plate": 4, "Robe": 1}
RANGED_TYPES = {"Bow": (2, 15), "Gun": (3, 26), "Crossbow": (18, 26), "Thrown": (16, 25),
                "Wand": (19, 26)}
RELIC_TYPES = {"Libram": 7, "Idol": 8, "Totem": 9}
AMMO_TYPES = {"Arrow": 2, "Bullet": 3}
CLASS_WEAPON, CLASS_ARMOR, CLASS_PROJECTILE = 2, 4, 6
# `INVTYPE_*` of a hand-held weapon by `hand`.
WEAPON_HANDS = {"1H": 13, "MH": 21, "2H": 17}
# What a stack of ammo or thrown weapons holds.
AMMO_STACK = 200

# The vanilla client spells seeded rows carry as "Equip:" effects, which master.csv does not hold:
# their Spell.dbc names, which tests/content_spell_ids.rs checks.
VANILLA_SPELLS = {
    7517: "Increased Defense",
    9329: "Attack Power 16",
    13388: "Increased Defense",
    13390: "Increased Defense",
    13674: "Increased Block 1",
    18369: "Increased Defense",
}

# The vanilla "+N Spell Damage and Healing" item spells by N, from vmangos's spell_template
# (`Increase Spell Dam N`: aura 13 `SPELL_AURA_MOD_DAMAGE_DONE` on every magic school plus aura
# 135 `SPELL_AURA_MOD_HEALING_DONE`, both N). Only the tooltip uses them; the realm reads
# `spell_power`. No spell exists for 3, 10 or 17: such a row takes the nearest lower one.
SPELL_POWER_SPELLS = {
    1: 9392, 2: 9393, 4: 9394, 5: 9395, 6: 9396, 7: 9397, 8: 9398, 9: 9415, 11: 9416,
    12: 9417, 13: 9342, 14: 9343, 15: 9344, 16: 9345, 18: 9346, 19: 14254, 20: 14799,
    21: 14248, 22: 15714, 23: 14047, 24: 24595, 25: 15715, 26: 18049, 27: 14054, 28: 14127,
    29: 13881, 30: 14798,
}
SPELL_POWER_NAMES = {spell: f"Increase Spell Dam {n}" for n, spell in SPELL_POWER_SPELLS.items()}
# The slots and types that may carry Spell Power: `(slot, type)`, `None` for any type.
SPELL_POWER_SLOTS = (("Weapon", None), ("Offhand", "Held"), ("Ranged", "Wand"))


# The Northshire tier rule (#426). Stat points of a row sit within FLEX of its quality's budget,
# doubled for a two-hander; a weapon's DPS sits within DPS_FLEX of its curve at item level 10,
# scaled by min(item_level, 10) / 10. MH counts as 1H; a Bow, Gun or Crossbow is "bow".
BUDGET = {"Common": 2, "Uncommon": 6, "Rare": 13, "Epic": 18}
FLEX = 2
WEAPON_DPS = {("1H", "Common"): 6.3, ("1H", "Uncommon"): 7.9, ("1H", "Rare"): 10.3,
              ("1H", "Epic"): 11.9, ("2H", "Common"): 8.4, ("2H", "Uncommon"): 10.5,
              ("2H", "Rare"): 13.6, ("2H", "Epic"): 15.8, ("bow", "Common"): 4.8,
              ("bow", "Uncommon"): 6.0, ("bow", "Rare"): 7.8, ("bow", "Epic"): 9.0}
DPS_FLEX = 0.10
# Above item level 10 budgets and weapon DPS compound 5% a level (ADR 0026, spec #448).
LEVEL_GROWTH = 1.05
# Only these rows are checked: entries from here up, and the vanilla outfit whites.
FIRST_CHECKED = 900000
OUTFIT_WHITES = (2108, 2109, 2110, 3260, 25, 37, 4565, 1388, 2055, 2362, 8179)


class SheetError(Exception):
    pass


def classify(row: dict) -> tuple[int, int, int]:
    """(class, subclass, inventory type) of a row's slot, type and hand."""
    slot, kind, hand = row["slot"], row["type"], row["hand"]
    where = f"{row['entry']} {row['name']}"

    def no_hand():
        if hand:
            raise SheetError(f"{where}: hand {hand!r} on a {slot}")

    if slot == "Weapon":
        if kind not in WEAPON_TYPES or hand not in WEAPON_HANDS:
            raise SheetError(f"{where}: a Weapon of type {kind!r} and hand {hand!r}")
        one, two = WEAPON_TYPES[kind]
        if kind in ("Staff", "Polearm") and hand != "2H":
            raise SheetError(f"{where}: a {kind} is two-handed")
        return CLASS_WEAPON, two if hand == "2H" else one, WEAPON_HANDS[hand]
    if slot == "Offhand":
        if kind in WEAPON_TYPES and hand == "OH":
            return CLASS_WEAPON, WEAPON_TYPES[kind][0], 22
        no_hand()
        if kind == "Shield":
            return CLASS_ARMOR, 6, 14
        if kind == "Held":
            return CLASS_ARMOR, 0, 23
        raise SheetError(f"{where}: an Offhand of type {kind!r}")
    if slot == "Ranged":
        no_hand()
        if kind in RANGED_TYPES:
            sub, inv = RANGED_TYPES[kind]
            return CLASS_WEAPON, sub, inv
        if kind in RELIC_TYPES:
            return CLASS_ARMOR, RELIC_TYPES[kind], 28
        raise SheetError(f"{where}: a Ranged of type {kind!r}")
    if slot == "Ammo":
        no_hand()
        if kind not in AMMO_TYPES:
            raise SheetError(f"{where}: Ammo of type {kind!r}")
        return CLASS_PROJECTILE, AMMO_TYPES[kind], 24
    if slot in ("Ring", "Trinket", "Amulet"):
        no_hand()
        if kind != "Misc":
            raise SheetError(f"{where}: a {slot} of type {kind!r}")
        return CLASS_ARMOR, 0, {"Ring": 11, "Trinket": 12, "Amulet": 2}[slot]
    if slot == "Chest":
        no_hand()
        if kind not in ARMOUR_TYPES:
            raise SheetError(f"{where}: a Chest of type {kind!r}")
        return CLASS_ARMOR, ARMOUR_TYPES[kind], 20 if kind == "Robe" else 5
    raise SheetError(f"{where}: slot {slot!r} is none of the ten Gear Slots")


def is_checked(entry: int) -> bool:
    """Whether the Northshire tier rule applies to this entry."""
    return entry >= FIRST_CHECKED or entry in OUTFIT_WHITES


def check_tier(row: dict, entry: int, where: str, points: int) -> None:
    """Refuse a checked row whose stat points stray from its quality's budget by more than FLEX."""
    quality = row["quality"]
    if quality not in BUDGET:
        return
    level = integer(row["item_level"], "item_level", where)
    base = BUDGET[quality]
    if level > 10:
        base = int(base * LEVEL_GROWTH ** (level - 10) + 0.5)
    budget = base * (2 if row["hand"] == "2H" else 1)
    if abs(points - budget) > FLEX:
        raise SheetError(f"{where}: a {quality} row spends {points} stat points, "
                         f"{budget} +/- {FLEX} allowed")
    if integer(row["budget"], "budget", where) != points:
        raise SheetError(f"{where}: the budget cell {row['budget']!r} is not the {points} "
                         "points the row spends")


def check_dps(row: dict, where: str, damage: list, speed_ms: int) -> None:
    """Refuse a checked weapon whose DPS is more than DPS_FLEX off its curve."""
    if row["slot"] == "Weapon":
        shape = "2H" if row["hand"] == "2H" else "1H"
    elif row["slot"] == "Ranged" and row["type"] in ("Bow", "Gun", "Crossbow"):
        shape = "bow"
    else:
        return
    curve = WEAPON_DPS.get((shape, row["quality"]))
    if curve is None or not damage or not speed_ms:
        return
    level = integer(row["item_level"], "item_level", where)
    if row["quality"] == "Common" and level <= 10:
        return  # Northshire's whites predate the white curve (#426, #448)
    target = curve * LEVEL_GROWTH ** (level - 10) if level > 10 else curve * level / 10
    dps = sum((lo + hi) / 2 for lo, hi, _ in damage) / (speed_ms / 1000)
    if abs(dps - target) > DPS_FLEX * target:
        raise SheetError(f"{where}: a {row['quality']} {shape} weapon of item level {level} "
                         f"does {dps:.1f} DPS, {target:.1f} +/- {DPS_FLEX:.0%} allowed")


def word(table: dict, value: str, column: str, where: str) -> int:
    if value not in table:
        raise SheetError(f"{where}: unknown {column} {value!r}")
    return table[value]


def integer(value: str, column: str, where: str) -> int:
    if value == "":
        return 0
    try:
        return int(value)
    except ValueError:
        raise SheetError(f"{where}: {column} {value!r} is not a whole number") from None


def ids(value: str, column: str, where: str) -> list[int]:
    return [integer(v, column, where) for v in value.split(";")] if value else []


def spell_power_spell(n: int, where: str) -> int:
    """The item spell whose tooltip says "+N Spell Damage and Healing", or the nearest lower."""
    top = max(SPELL_POWER_SPELLS)
    if n > top:
        raise SheetError(f"{where}: spell_power {n} is above {top}, the last tooltip spell")
    if n not in SPELL_POWER_SPELLS:
        lower = max(k for k in SPELL_POWER_SPELLS if k < n)
        print(f"build_gear: {where}: no tooltip spell for spell_power {n}; the tooltip says {lower}",
              file=sys.stderr)
        n = lower
    return SPELL_POWER_SPELLS[n]


def spell_name(spell: int, spells: dict[int, str]) -> str:
    return spells.get(spell) or VANILLA_SPELLS.get(spell) or SPELL_POWER_NAMES[spell]


def known_spells(master_path: str) -> dict[int, str]:
    """Every spell id master.csv names, with its row's name."""
    spells = {}
    with open(master_path, newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            if row.get("spell_id"):
                spells.setdefault(int(row["spell_id"]), row["name"])
    return spells


def toml_string(s: str) -> str:
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def fmt_float(v: float) -> str:
    return repr(float(v))


def build(rows: list[dict], drops: list[dict], spells: dict[int, str]) -> str:
    seen = set()
    items, wardrobes, lines = [], {}, {}
    for row in rows:
        entry = integer(row["entry"], "entry", row["name"])
        where = f"{entry} {row['name']}"
        if entry in seen:
            raise SheetError(f"{where}: a duplicate entry")
        if entry <= 0 or entry >= FIRST_LOOK:
            raise SheetError(f"{where}: entries run from 1 to {FIRST_LOOK - 1}")
        seen.add(entry)
        cls, sub, inv = classify(row)
        if row["crit"]:
            raise SheetError(f"{where}: the realm applies no crit from gear yet")
        if row["on_hit"]:
            raise SheetError(f"{where}: the realm models no chance-on-hit effects yet")
        spell_power = integer(row["spell_power"], "spell_power", where)
        if spell_power and not any(row["slot"] == slot and kind in (None, row["type"])
                                   for slot, kind in SPELL_POWER_SLOTS):
            raise SheetError(f"{where}: spell_power on a {row['slot']} of type {row['type']!r}; "
                             "only a Weapon, a held Offhand or a Wand carries it")
        if row["line"]:
            if entry < FIRST_CHECKED:
                raise SheetError(f"{where}: a line on an entry under {FIRST_CHECKED}")
            shape = (row["slot"], row["type"])
            if lines.setdefault(row["line"], shape) != shape:
                raise SheetError(f"{where}: line {row['line']!r} is a {shape[0]} of type "
                                 f"{shape[1]!r}, but its other rows are a "
                                 f"{lines[row['line']][0]} of type {lines[row['line']][1]!r}")
        spent = sum(integer(row[col], col, where) for col, _ in STATS) + -(-spell_power // 2)
        if is_checked(entry):
            check_tier(row, entry, where, spent)
        elif row["budget"]:
            budget = integer(row["budget"], "budget", where)
            if spent > budget:
                raise SheetError(f"{where}: spends {spent} stat points of a budget of {budget}")
        cells = {part: integer(row[f"wardrobe_{part}"], f"wardrobe_{part}", where)
                 for part, _ in WARDROBE}
        if row["slot"] != "Chest" and any(cells.values()):
            raise SheetError(f"{where}: a wardrobe column on a {row['slot']}")
        if row["slot"] == "Chest":
            wardrobes[entry] = cells
        for column in ("equip", "use"):
            for spell in ids(row[column], column, where):
                if spell not in spells and spell not in VANILLA_SPELLS:
                    raise SheetError(f"{where}: {column} spell {spell} is in neither master.csv "
                                     "nor the seeded vanilla spells")
        uses = ids(row["use"], "use", where)
        if len(uses) > 1:
            raise SheetError(f"{where}: one use spell at most")
        mins = row["dmg_min"].split(";") if row["dmg_min"] else []
        maxs = row["dmg_max"].split(";") if row["dmg_max"] else []
        schools = row["dmg_school"].split(";") if row["dmg_school"] else []
        if not len(mins) == len(maxs) == len(schools):
            raise SheetError(f"{where}: dmg_min, dmg_max and dmg_school disagree in length")
        try:
            damage = [(float(lo), float(hi), word(SCHOOL, s, "dmg_school", where))
                      for lo, hi, s in zip(mins, maxs, schools)]
            speed_ms = round(float(row["speed"]) * 1000) if row["speed"] else 0
        except ValueError:
            raise SheetError(f"{where}: a damage or speed that is no number") from None
        if is_checked(entry):
            check_dps(row, where, damage, speed_ms)
        items.append({
            "entry": entry, "row": row, "class": cls, "subclass": sub, "inventory_type": inv,
            "damage": damage, "speed_ms": speed_ms, "use": uses, "where": where,
            "spell_power": spell_power,
        })

    pool_level = {}
    for d in drops:
        entry = integer(d["entry"], "entry", "gear_drops.csv")
        if entry not in seen:
            raise SheetError(f"gear_drops.csv: {entry} is no Gear in the sheet")
        if entry in pool_level:
            raise SheetError(f"gear_drops.csv: {entry} twice")
        pool_level[entry] = integer(d["pool_level"], "pool_level", f"gear_drops.csv {entry}")

    # One look template per body part and display, in that order.
    looks = sorted({(i, display) for cells in wardrobes.values()
                    for i, (part, _) in enumerate(WARDROBE) if (display := cells[part])})
    look_entry = {key: FIRST_LOOK + n for n, key in enumerate(looks)}

    out = [
        "# Every piece of Gear, generated from data/twilights-eve/gear.csv (ADR 0007) by\n"
        "# `tools/twilights-eve/build_gear.py`; edit the sheet, never this file. tests/content.rs\n"
        "# checks the tool reproduces it byte for byte.\n"
        "#\n"
        "# The fields and defaults of content/items.toml. `pool_level` comes from\n"
        "# data/twilights-eve/gear_drops.csv. A Chest's `wardrobe` is the look template worn on\n"
        "# each hidden body part (head, shoulders, back, wrists, hands, waist, legs, feet), 0 for\n"
        "# bare; the look templates, from 990000 up, close the table. `spell_power` is the flat\n"
        "# Spell Power the realm adds to spells; its tooltip spell is among `equip_spells`.\n\n"
        "item = [\n"
    ]
    for item in items:
        out.append(row_line(item, spells, pool_level.get(item["entry"], 0),
                            wardrobes.get(item["entry"]), look_entry))
    for (i, display), entry in look_entry.items():
        part, inv = WARDROBE[i]
        out.append("  { " + ", ".join([
            f"entry = {entry}", f"name = {toml_string('Wardrobe ' + part)}",
            f"class = {CLASS_ARMOR}", f"inventory_type = {inv}", f"display_id = {display}",
        ]) + " },\n")
    out.append("]\n")
    return "".join(out)


def row_line(item: dict, spells: dict[int, str], pool: int, wardrobe: dict | None,
             look_entry: dict) -> str:
    row, where = item["row"], item["where"]
    num = lambda column: integer(row[column], column, where)  # noqa: E731
    stack = AMMO_STACK if row["slot"] == "Ammo" or row["type"] == "Thrown" else 1
    parts = [f"entry = {item['entry']}", f"name = {toml_string(row['name'])}"]
    ints = [
        ("quality", word(QUALITY, row["quality"], "quality", where)),
        ("item_level", num("item_level")),
        ("required_level", num("required_level")),
        ("class", item["class"]),
        ("subclass", item["subclass"]),
        ("inventory_type", item["inventory_type"]),
        ("display_id", num("display_id")),
        ("armor", num("armor")),
        ("block", num("block")),
        ("speed_ms", item["speed_ms"]),
        ("durability", num("durability")),
        ("sell_price", num("sell_price")),
        ("bonding", word(BONDING, row["bonding"], "bonding", where)),
        ("stackable", stack),
        ("sheath", word(SHEATH, row["sheath"], "sheath", where)),
        ("material", word(MATERIAL, row["material"], "material", where)),
        ("pool_level", pool),
    ]
    parts += [f"{k} = {v}" for k, v in ints if v]
    if row["line"]:
        parts.append(f"line = {toml_string(row['line'])}")
    stats = [f"[{sid}, {num(col)}]" for col, sid in STATS if num(col)]
    if stats:
        parts.append(f"stats = [{', '.join(stats)}]")
    res = [num(f"res_{r}") for r in RESISTANCES]
    if any(res):
        parts.append(f"resistances = [{', '.join(map(str, res))}]")
    if item["damage"]:
        damage = ", ".join(f"[{fmt_float(lo)}, {fmt_float(hi)}, {s}]" for lo, hi, s in item["damage"])
        parts.append(f"damage = [{damage}]")
    equip = ids(row["equip"], "equip", where)
    if item["spell_power"]:
        equip.append(spell_power_spell(item["spell_power"], where))
    if equip:
        names = [toml_string(spell_name(s, spells)) for s in equip]
        parts.append(f"equip_spells = [{', '.join(map(str, equip))}]")
        parts.append(f"equip_spell_names = [{', '.join(names)}]")
    for column in ("attack_power", "defense", "spell_power"):
        if num(column):
            parts.append(f"{column} = {num(column)}")
    if item["use"]:
        spell = item["use"][0]
        parts.append(f"use_spell = {spell}")
        parts.append(f"use_spell_name = {toml_string(spell_name(spell, spells))}")
    if wardrobe is not None:
        looks = [str(look_entry.get((i, wardrobe[part]), 0)) for i, (part, _) in enumerate(WARDROBE)]
        parts.append(f"wardrobe = [{', '.join(looks)}]")
    return "  { " + ", ".join(parts) + " },\n"


def read_csv(path: str, columns: tuple[str, ...] | None = None) -> list[dict]:
    with open(path, newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        if columns is not None and tuple(reader.fieldnames or ()) != columns:
            raise SheetError(f"{path}: the columns are not the sheet's (see this tool's COLUMNS)")
        return [{k: (v or "").strip() for k, v in row.items()} for row in reader]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--csv", default=DEFAULT_CSV)
    parser.add_argument("--drops", default=DEFAULT_DROPS)
    parser.add_argument("--master", default=DEFAULT_MASTER)
    parser.add_argument("--out", default=DEFAULT_OUT, help="the generated table; - for stdout")
    args = parser.parse_args(argv)
    try:
        text = build(read_csv(args.csv, COLUMNS), read_csv(args.drops, ("entry", "pool_level")),
                     known_spells(args.master))
    except SheetError as e:
        print(f"build_gear: {e}", file=sys.stderr)
        return 1
    data = text.encode("utf-8")
    if args.out == "-":
        sys.stdout.buffer.write(data)
    else:
        with open(args.out, "wb") as f:
            f.write(data)
        print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
