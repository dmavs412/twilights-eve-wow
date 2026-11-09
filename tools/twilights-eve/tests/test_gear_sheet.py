"""gear_sheet.py driven through its command line against a small gear.csv and vmangos stand-in.

    ~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests
"""

from __future__ import annotations

import contextlib
import csv
import io
import os
import sqlite3
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, "..", "gear_sheet.py")
sys.path.insert(0, os.path.join(HERE, ".."))
import gear_sheet  # noqa: E402

# The real header and copies of five rows: a cloth Chest, a sword, a shield, a staff and an epic
# shield.
SHEET = (
    "entry,name,slot,type,hand,quality,item_level,required_level,str,agi,sta,int,spi,armor,block,"
    "dmg_min,dmg_max,dmg_school,speed,res_holy,res_fire,res_nature,res_frost,res_shadow,res_arcane,"
    "attack_power,crit,defense,spell_power,budget,line,equip,use,on_hit,display_id,icon,sell_price,"
    "bonding,durability,sheath,material,wardrobe_head,wardrobe_shoulders,wardrobe_back,"
    "wardrobe_wrists,wardrobe_hands,wardrobe_waist,wardrobe_legs,wardrobe_feet\n"
    "902080,Westbrook Wool Robe,Chest,Cloth,,Uncommon,28,27,,,7,16,9,420,,,,,,,,,,,,,,,,32,"
    "world-chest-cloth,,,,25889,INV_Chest_Leather_01,1430,Equip,70,,Cloth,,25887,25884,16805,"
    "25885,25881,10079,16802\n"
    "902084,Westbrook Cutlass,Weapon,Sword,1H,Uncommon,28,27,21,,12,,,,,56,86,Physical,2.0,,,,,,,"
    ",,,,33,world-sword,,,,20168,,3842,Equip,70,Side,Metal,,,,,,,,\n"
    "902085,Riverpaw Kite Shield,Offhand,Shield,,Uncommon,28,27,21,,12,,,567,8,,,,,,,,,,,,,,,33,"
    "world-shield,,,,23750,,2451,Equip,80,Shield,Metal,,,,,,,,\n"
    "902088,Westbrook Quarterstaff,Weapon,Staff,2H,Uncommon,28,27,,,,33,,,,114,169,Physical,3.0,"
    ",,,,,,,,,65,66,world-staff,,,,28228,,4414,Equip,80,Staff,Wood,,,,,,,,\n"
    "905906,Hogger's Pen Gate,Offhand,Shield,,Epic,30,27,72,,36,,,791,11,,,,,,,,,,,,,,,108,,,,,"
    "18816,,18815,Pickup,120,Shield,Metal,,,,,,,,\n"
)

# entry, name, class, subclass, inventory_type, item_level, display_id, quality, set_id
ITEMS = [
    # A mail family grouped by name: chest, coif, cloak (cloth subclass) and boots.
    (6411, "Chief Brigadier Armor", 4, 3, 5, 41, 25882, 2, 0),
    (4078, "Chief Brigadier Coif", 4, 3, 1, 39, 25904, 2, 0),
    (4726, "Chief Brigadier Cloak", 4, 1, 16, 36, 25900, 2, 0),
    (6412, "Chief Brigadier Boots", 4, 3, 8, 39, 25883, 2, 0),
    # A mail chest on a display the sheet uses as 902080's wardrobe_shoulders.
    (7001, "Used Look Hauberk", 4, 3, 5, 40, 25887, 2, 0),
    # A grey mail chest, and one too far from 40.
    (7002, "Grey Hauberk", 4, 3, 5, 40, 30001, 0, 0),
    (7003, "Ancient Hauberk", 4, 3, 5, 55, 30002, 2, 0),
    # A cloth set by set_id, with a chest and a robe: the robe is preferred.
    (7010, "Moonlit Vest", 4, 1, 5, 33, 30010, 2, 77),
    (7011, "Mooncloth Robe", 4, 1, 20, 34, 30011, 2, 77),
    (7012, "Starfall Gloves", 4, 1, 10, 33, 30012, 2, 77),
]


def make_db(path: str, items) -> None:
    db = sqlite3.connect(path)
    db.execute("create table item_template (entry integer, name text, class integer, "
               "subclass integer, inventory_type integer, item_level integer, display_id integer, "
               "quality integer, set_id integer)")
    db.executemany("insert into item_template values (?, ?, ?, ?, ?, ?, ?, ?, ?)", items)
    db.commit()
    db.close()


class GearSheetTool(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.mkdtemp()
        self.path = os.path.join(self.dir, "gear.csv")
        with open(self.path, "wb") as f:
            f.write(SHEET.encode("utf-8"))

    def run_tool(self, *args: str) -> subprocess.CompletedProcess:
        return subprocess.run([sys.executable, SCRIPT, "--csv", self.path, *args],
                              capture_output=True, text=True)

    def ok(self, *args: str) -> str:
        done = self.run_tool(*args)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        return done.stdout

    def text(self) -> str:
        with open(self.path, "rb") as f:
            return f.read().decode("utf-8")

    def rows(self) -> list[dict]:
        return list(csv.DictReader(io.StringIO(self.text(), newline="")))

    def row(self, entry: int) -> dict:
        return next(r for r in self.rows() if r["entry"] == str(entry))

    def points(self, row: dict) -> int:
        stats = sum(int(row[c] or 0) for c in ("str", "agi", "sta", "int", "spi"))
        return stats + -(-int(row["spell_power"] or 0) // 2)

    def dps(self, row: dict) -> float:
        return (int(row["dmg_min"]) + int(row["dmg_max"])) / 2 / float(row["speed"])

    def db(self, items=ITEMS) -> str:
        path = os.path.join(self.dir, "mangos.sqlite")
        make_db(path, items)
        return path

    def test_get_prints_the_row_or_the_columns_named(self):
        self.assertIn("  armor: 420", self.ok("get", "902080"))
        self.assertEqual(self.ok("get", "902080", "armor").splitlines()[1:], ["  armor: 420"])
        self.assertNotEqual(self.run_tool("get", "999999").returncode, 0)

    def test_set_rewrites_only_the_cell_it_touches(self):
        self.ok("set", "902080", "name=X")
        self.assertEqual(self.text(), SHEET.replace("Westbrook Wool Robe", "X"))

    def test_set_refuses_an_unknown_entry_or_column(self):
        self.assertIn("no row", self.run_tool("set", "999999", "name=X").stderr)
        self.assertIn("not a column", self.run_tool("set", "902080", "colour=red").stderr)
        self.assertEqual(self.text(), SHEET)

    def test_scale_a_chest(self):
        self.ok("scale", "902080", "902100", "item_level=33", "required_level=31")
        # After 902080, and after the last row below it, so the sheet stays in entry order.
        entries = [int(r["entry"]) for r in self.rows()]
        self.assertEqual(entries, [902080, 902084, 902085, 902088, 902100, 905906])
        row = self.row(902100)
        self.assertEqual((row["item_level"], row["required_level"]), ("33", "31"))
        self.assertEqual(row["budget"], "39")
        self.assertEqual(self.points(row), 39)
        self.assertEqual(row["armor"], "492")
        self.assertEqual(row["wardrobe_feet"], "16802")
        self.assertEqual(row["line"], "world-chest-cloth")

    def test_scale_a_staff(self):
        self.ok("scale", "902088", "902108", "item_level=33", "required_level=31")
        row = self.row(902108)
        self.assertEqual(row["budget"], "78")
        self.assertEqual(self.points(row), 78)
        self.assertAlmostEqual(int(row["spell_power"]) / int(row["int"]), 2, delta=0.1)
        self.assertLess(abs(self.dps(row) - 55.1), 0.01 * 55.1)
        self.assertEqual(row["speed"], "3.0")
        # In entry order: after 902088, before 905906.
        self.assertEqual([r["entry"] for r in self.rows()][-2:], ["902108", "905906"])

    def test_scale_a_shield(self):
        self.ok("scale", "902085", "902105", "item_level=33", "required_level=31")
        row = self.row(902105)
        ratio = gear_sheet.power_curve(33) / gear_sheet.power_curve(28)
        self.assertEqual(row["armor"], str(int(567 * ratio + 0.5)))
        self.assertEqual(row["block"], str(int(8 * ratio + 0.5)))
        self.assertEqual(row["sell_price"], str(int(2451 * ratio ** 2 + 0.5)))

    def test_scale_a_shield_to_another_quality(self):
        self.ok("scale", "905906", "905907", "item_level=30", "required_level=27", "quality=Rare")
        self.assertEqual(self.row(905907)["armor"], str(int(791 / 1.13 + 0.5)))

    def test_scale_to_rare(self):
        self.ok("scale", "902084", "904100", "item_level=38", "required_level=35", "quality=Rare",
                "name=Blade")
        row = self.row(904100)
        self.assertEqual((row["quality"], row["name"]), ("Rare", "Blade"))
        self.assertEqual(row["budget"], "99")
        self.assertEqual(self.points(row), 99)
        self.assertLess(abs(self.dps(row) - 62.7), 0.01 * 62.7)

    def test_scale_refuses_an_existing_entry_and_an_epic_chest(self):
        done = self.run_tool("scale", "902080", "902084", "item_level=33", "required_level=31")
        self.assertIn("already", done.stderr)
        done = self.run_tool("scale", "902080", "902100", "item_level=33", "required_level=31",
                             "quality=Epic")
        self.assertIn("Epic Chest", done.stderr)
        self.assertEqual(self.text(), SHEET)

    def test_display_with_a_blank_icon_writes_the_display_icon(self):
        real = gear_sheet.read_icons
        gear_sheet.read_icons = lambda data: {20168: "INV_Sword_04.tga", 25000: "INV_Sword_99"}
        try:
            with contextlib.redirect_stdout(io.StringIO()):
                code = gear_sheet.main(["--csv", self.path, "scale", "902084", "902104",
                                        "item_level=33", "required_level=31", "--display",
                                        "25000", "--data", self.dir])
        finally:
            gear_sheet.read_icons = real
        self.assertEqual(code, 0)
        row = self.row(902104)
        self.assertEqual((row["display_id"], row["icon"]), ("25000", "INV_Sword_04"))

    def test_looks_groups_a_family_and_leaves_out_used_displays(self):
        out = self.ok("looks", "Chest", "Mail", "40", "--db", self.db())
        self.assertIn("display_id=25882", out)
        for cell in ("wardrobe_head=25904", "wardrobe_shoulders=0", "wardrobe_back=25900",
                     "wardrobe_feet=25883"):
            self.assertIn(cell, out)
        for left_out in ("25887", "30001", "30002"):
            self.assertNotIn(left_out, out)

    def test_looks_prefers_a_cloth_family_robe(self):
        out = self.ok("looks", "Chest", "Cloth", "33", "--db", self.db())
        self.assertIn("display_id=30011", out)
        self.assertIn("wardrobe_hands=30012", out)
        self.assertNotIn("display_id=30010", out)

    def test_looks_lists_weapon_models(self):
        items = ITEMS + [(8001, "Long Sword", 2, 7, 13, 34, 31001, 2, 0),
                         (8002, "Great Sword", 2, 8, 17, 34, 31002, 2, 0)]
        out = self.ok("looks", "Weapon", "Sword", "33", "--db", self.db(items))
        self.assertIn("31001", out)
        self.assertNotIn("31002", out)

    def test_look_check_passes_a_near_model(self):
        db = self.db(ITEMS + [(8001, "Cutlass", 2, 7, 13, 30, 20168, 2, 0)])
        self.assertIn("902084", self.ok("look-check", "902084", "--db", db))

    def test_look_check_fails_a_far_model(self):
        db = self.db(ITEMS + [(8001, "Cutlass", 2, 7, 13, 50, 20168, 2, 0)])
        done = self.run_tool("look-check", "902084", "--db", db)
        self.assertEqual(done.returncode, 1, done.stdout)
        self.assertIn("902084", done.stdout)
        done = self.run_tool("look-check", "902085", "--db", db)
        self.assertEqual(done.returncode, 1)
        self.assertIn("no vanilla item", done.stdout)

    def test_look_check_fails_an_out_of_order_pair(self):
        self.ok("scale", "902080", "902100", "item_level=33", "required_level=31",
                "display_id=30020", "wardrobe_head=1")
        db = self.db(ITEMS + [(8001, "Old Robe", 4, 1, 20, 30, 25889, 2, 0),
                              (8002, "Older Robe", 4, 1, 20, 29, 30020, 2, 0)])
        done = self.run_tool("look-check", "902080-902100", "--db", db)
        self.assertEqual(done.returncode, 1, done.stdout)
        self.assertIn("out of order", done.stdout)
        self.assertNotIn("wardrobe", done.stdout)

    def test_look_check_fails_a_shared_wardrobe(self):
        self.ok("scale", "902080", "902100", "item_level=33", "required_level=31",
                "display_id=30020")
        db = self.db(ITEMS + [(8002, "Newer Robe", 4, 1, 20, 34, 30020, 2, 0)])
        done = self.run_tool("look-check", "902100", "--db", db)
        self.assertEqual(done.returncode, 1, done.stdout)
        self.assertIn("wardrobe", done.stdout)

    def test_look_check_passes_a_low_plate_chest_on_a_41_to_50_look(self):
        self.ok("scale", "902080", "902100", "item_level=33", "required_level=31",
                "display_id=30030", "type=Plate", "wardrobe_head=1")
        db = self.db(ITEMS + [(8003, "First Plate", 4, 4, 5, 45, 30030, 2, 0)])
        self.ok("look-check", "902100", "--db", db)


if __name__ == "__main__":
    unittest.main()
