"""build_gear.py's Northshire tier check, fed small in-memory sheets.

    ~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests
"""

from __future__ import annotations

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
import build_gear  # noqa: E402


def row(**cells) -> dict:
    base = {col: "" for col in build_gear.COLUMNS}
    base.update(item_level="10", required_level="1", display_id="1")
    base.update(cells)
    return base


def ring(entry: int, quality: str, points: int, budget: int | None = None) -> dict:
    return row(entry=str(entry), name="Test Ring", slot="Ring", type="Misc", quality=quality,
               sta=str(points), budget=str(points if budget is None else budget))


def sword(quality: str, lo: float, hi: float, speed: float) -> dict:
    return row(entry="900100", name="Test Sword", slot="Weapon", type="Sword", hand="1H",
               quality=quality, dmg_min=str(lo), dmg_max=str(hi), dmg_school="Physical",
               speed=str(speed), sta="13", budget="13")


class TierCheck(unittest.TestCase):
    def test_green_ring_at_budget_passes(self):
        build_gear.build([ring(900001, "Uncommon", 6)], [], {})

    def test_green_ring_over_flex_is_refused(self):
        with self.assertRaises(build_gear.SheetError) as e:
            build_gear.build([ring(900001, "Uncommon", 9)], [], {})
        self.assertIn("900001", str(e.exception))

    def test_blue_one_hander_on_the_curve_passes(self):
        # 10.3 DPS at speed 2.0
        build_gear.build([sword("Rare", 18.0, 23.2, 2.0)], [], {})

    def test_blue_one_hander_off_the_curve_is_refused(self):
        # 8.0 DPS at speed 2.0
        with self.assertRaises(build_gear.SheetError) as e:
            build_gear.build([sword("Rare", 14.0, 18.0, 2.0)], [], {})
        self.assertIn("DPS", str(e.exception))

    def test_budget_cell_must_equal_the_points(self):
        with self.assertRaises(build_gear.SheetError):
            build_gear.build([ring(900001, "Uncommon", 6, budget=7)], [], {})

    def test_vanilla_row_outside_the_outfit_is_unchecked(self):
        vanilla = ring(1234, "Common", 1)
        vanilla["budget"] = ""
        build_gear.build([vanilla], [], {})


def ring_at(level: int, points: int, **cells) -> dict:
    return {**ring(900001, "Uncommon", points), "item_level": str(level), **cells}


def blue_sword_at_30(lo: float, hi: float) -> dict:
    return {**sword("Rare", lo, hi, 2.0), "item_level": "30", "sta": "34", "budget": "34"}


class LevelCurves(unittest.TestCase):
    def test_green_ring_at_22_spends_11(self):
        build_gear.build([ring_at(22, 11)], [], {})

    def test_green_ring_at_22_with_14_is_refused(self):
        with self.assertRaises(build_gear.SheetError) as e:
            build_gear.build([ring_at(22, 14)], [], {})
        self.assertIn("900001", str(e.exception))

    def test_blue_one_hander_at_30_on_the_curve_passes(self):
        # 27.3 DPS at speed 2.0
        build_gear.build([blue_sword_at_30(43.7, 65.5)], [], {})

    def test_blue_one_hander_at_30_on_the_level_10_curve_is_refused(self):
        # 21.0 DPS at speed 2.0
        with self.assertRaises(build_gear.SheetError) as e:
            build_gear.build([blue_sword_at_30(33.6, 50.4)], [], {})
        self.assertIn("DPS", str(e.exception))

    def test_green_staff_at_15_carries_double_of_8(self):
        staff = {**ring_at(15, 16), "slot": "Weapon", "type": "Staff", "hand": "2H"}
        build_gear.build([staff], [], {})


class LineColumn(unittest.TestCase):
    def test_line_on_an_outfit_white_is_refused(self):
        white = {**ring(2108, "Common", 2), "line": "x"}
        with self.assertRaises(build_gear.SheetError) as e:
            build_gear.build([white], [], {})
        self.assertIn("2108", str(e.exception))

    def test_two_rows_of_a_line_with_different_slots_are_refused(self):
        a = {**ring(900001, "Uncommon", 6), "line": "x"}
        b = {**ring(900002, "Uncommon", 6), "line": "x", "slot": "Amulet"}
        with self.assertRaises(build_gear.SheetError) as e:
            build_gear.build([a, b], [], {})
        self.assertIn("900002", str(e.exception))

    def test_rows_of_one_line_with_one_slot_pass(self):
        a = {**ring(900001, "Uncommon", 6), "line": "x"}
        b = {**ring_at(22, 11), "entry": "900002", "line": "x"}
        build_gear.build([a, b], [], {})


if __name__ == "__main__":
    unittest.main()
