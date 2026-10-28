"""The balance model (tools/twilights-eve/balance/) against the realm's own numbers.

    ~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests
"""

from __future__ import annotations

import math
import os
import random
import sys
import unittest
from collections import Counter

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "balance"))
import boss  # noqa: E402
import model  # noqa: E402
import scenarios  # noqa: E402


def swing(mob: dict) -> float:
    return (mob["dmin"] + mob["dmax"]) / 2


class BalanceModel(unittest.TestCase):
    def test_mob_stats_match_the_realm(self):
        at_ten = model.DATA.mob_class["warrior"]
        health, damage = at_ten["health"][9], at_ten["melee_damage_avg"][9]
        # mobs.rs a_level_12_warrior_in_elwynn_forest_has_half_again_northshires_health.
        twelve = model.mob_stats(12)
        self.assertEqual(twelve["hp"], 1140)
        self.assertAlmostEqual(swing(twelve), 71.32, delta=0.05)
        # Level 20 in Elwynn, as mobs.rs difficulty_puts_health_and_damage_on_the_power_curve
        # computes a creature: the level-10 row x P(20) = 2.0, x3.2 health and x3.0 damage, then
        # Elwynn's 1.5 and 1.3.
        twenty = model.mob_stats(20)
        self.assertLessEqual(abs(twenty["hp"] - round(health * 2.0 * 3.2 * 1.5)), 1)
        self.assertAlmostEqual(swing(twenty), damage * 2.0 * 3.0 * 1.3, places=3)
        # That test's own mob: level 8, no Zone Factor, damage faded in to x2.67.
        eight = model.mob_stats(8)
        self.assertLessEqual(abs(eight["hp"] - round(health * 0.8 * 3.2)), 1)
        self.assertAlmostEqual(swing(eight) / (damage * 0.8), 1.5 + 1.5 * 7.0 / 9.0, places=3)
        self.assertAlmostEqual(swing(eight) / (damage * 0.8), 2.67, delta=0.01)
        self.assertEqual(eight["armor"], at_ten["armor"][7])

    def test_every_calling_fights_once_at_every_level(self):
        cases = [(c, (10, 20, 30), "green") for c in model.CALLINGS]
        cases.append(("basic_warrior", (1, 5, 9), "white"))
        for calling, levels, gear in cases:
            for level in levels:
                with self.subTest(calling=calling, level=level):
                    row = scenarios.run_cell(scenarios.cell(calling, level, gear, scenarios.normals(level, 1), 1))
                    self.assertIn(row["win"], (0, 1))
                    self.assertTrue(math.isfinite(row["dps"]))

    def test_a_calling_above_30_keeps_its_cap(self):
        # G+5-named fights Hogger (30) at 35: the Calling stays at its cap, the base stats grow.
        for calling in model.CALLINGS:
            with self.subTest(calling=calling):
                self.assertEqual(model.calling_level(calling, 35), model.calling_level(calling, 30))
                at_30, at_35 = model.character(calling, 30, "green"), model.character(calling, 35, "green")
                self.assertEqual(at_35["talents"], at_30["talents"])
                self.assertGreater(at_35["stats"].max_hp, at_30["stats"].max_hp)

    def test_kits_fill_every_slot(self):
        for calling in model.CALLINGS:
            for level in (15, 25):
                for tier in ("green", "blue"):
                    with self.subTest(calling=calling, level=level, tier=tier):
                        entries, off, _ = model.kit(calling, level, tier)
                        slots = Counter(model.GEAR[e]["slot"] for e in entries + ([off] if off else []))
                        self.assertEqual(slots["Chest"], 1)
                        self.assertGreaterEqual(slots["Weapon"], 1)
                        self.assertEqual(slots["Amulet"], 1)
                        self.assertEqual(slots["Ring"], 2)

    def test_named_bosses_load(self):
        bosses = boss.named_bosses()
        self.assertGreaterEqual(len(bosses), 10)
        for b in bosses:
            with self.subTest(boss=b["name"]):
                cls, mob = boss.fight_class("vanguard", b)
                ch = model.character("vanguard", 30, "blue")
                result = cls("vanguard", 30, ch["stats"], ch["talents"], random.Random(1), {}, mob=mob).fight()
                self.assertIn(result["won"], (True, False))
                self.assertIsInstance(b["unmodelled"], list)
        # The Shadow of Morganth's Terrify is a fear: listed, not raised.
        self.assertTrue(any("terrify_creature" in b["unmodelled"] for b in bosses))
        # Each takes the Zone Factor of where it spawns, not of the template above or below it:
        # the Abbot in the Abbey, Vurx in Echo Ridge Mine, the Eye Below in Jasperlode (zone 12).
        areas = {b["name"]: b["area"] for b in bosses}
        self.assertEqual(areas["Abbot Ashworth"], model.NORTHSHIRE_VALLEY)
        self.assertEqual(areas["Geomancer Vurx"], model.NORTHSHIRE_VALLEY)
        self.assertEqual(areas["The Eye Below"], model.ELWYNN_FOREST)


if __name__ == "__main__":
    unittest.main()
