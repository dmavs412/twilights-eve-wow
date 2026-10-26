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


if __name__ == "__main__":
    unittest.main()
