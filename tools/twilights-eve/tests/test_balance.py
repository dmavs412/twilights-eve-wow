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
import multi  # noqa: E402
import scenarios  # noqa: E402


def swing(mob: dict) -> float:
    return (mob["dmin"] + mob["dmax"]) / 2


def totals(calling: str, n: int, rows: dict, fights: int, rng: random.Random, level: int = 30,
           gear: str = "green", rank: int = 0) -> tuple:
    """(damage dealt, health lost, fight time) summed over `fights` fights of `calling` in `gear`
    and the build `rows` against `n` mobs of `rank` (0 normal, 1 elite) on level. `rng` is one
    Random every fight draws from, or a function giving fight i its own."""
    cls, mob = multi.fighter(calling, n), model.mob_stats(level, rank=rank)
    ch = model.character(calling, level, gear, rows)
    dealt = lost = spent = 0.0
    for i in range(fights):
        sim = cls(calling, level, ch["stats"], ch["rows"], rng(i) if callable(rng) else rng, {}, mob=mob)
        r = sim.fight()
        dealt += sum(sim.dmg.values())
        lost += 1 - r["health_left"]
        spent += r["time"]
    return dealt, lost, spent


# A pull of 2 elites in blues, B-2E's: it lasts about 55 s at 30, where 2 normals last about 30 s.
LONG_PULL = dict(gear="blue", rank=1)
# Choices that change nothing in 30 fights against 2 normals, so
# test_every_tier1_choice_changes_the_fight_or_is_ignored fights them in LONG_PULL: Elusiveness
# brings Evasion back after 60 s, and Blood Frenzy's stacks past 5 need a long run of droplets.
LONG_FIGHT = {("cutthroat", "elusiveness"), ("cutthroat", "blood_frenzy")}


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
                self.assertEqual(model.open_rows(calling, 35), model.open_rows(calling, 30))
                at_30, at_35 = model.character(calling, 30, "green"), model.character(calling, 35, "green")
                self.assertEqual(at_35["rows"], at_30["rows"])
                self.assertGreater(at_35["stats"].max_hp, at_30["stats"].max_hp)

    def test_rows_open_at_the_calling_level(self):
        for calling in model.CALLINGS:
            for level in (10, 20, 30):
                with self.subTest(calling=calling, level=level):
                    rows = scenarios.pick_build(calling, level, 2)
                    real = model.calling_level(calling, level)
                    want = [r["key"] for r in model.talent_rows(calling) if r["level"] <= real]
                    self.assertEqual(list(rows), want)
                    for r in model.talent_rows(calling):
                        if r["key"] in rows:
                            self.assertIn(rows[r["key"]], [c["key"] for c in r["choices"]])
        # The Tier 0 rules measure the bare kit.
        for level in (2, 5, 9):
            self.assertEqual(scenarios.pick_build("basic_warrior", level, 2), {})

    def test_a_set_choice_fights_with_its_own_spell(self):
        # Practiced Strike costs 25 energy, Sinister Strike 35.
        practiced = totals("cutthroat", 1, {"sinister_strike": "practiced_strike"}, 50, random.Random(1))
        plain = totals("cutthroat", 1, {}, 50, random.Random(1))
        self.assertGreater(practiced[0] / practiced[2], plain[0] / plain[2])

    def test_choose_breaks_ties_by_health_then_sheet_order(self):
        # A 2% lead wins outright, whatever the health left.
        self.assertEqual(scenarios.choose([("a", 100.0, 0.9), ("b", 102.0, 0.1), ("c", 90.0, 1.0)]), "b")
        # Within 1% the most health left wins.
        self.assertEqual(scenarios.choose([("a", 100.5, 0.3), ("b", 100.0, 0.6), ("c", 80.0, 0.9)]), "b")
        # Equal DPS and health: the first in the sheet.
        self.assertEqual(scenarios.choose([("a", 100.0, 0.5), ("b", 100.0, 0.5), ("c", 99.5, 0.4)]), "a")

    def test_every_tier1_choice_changes_the_fight_or_is_ignored(self):
        for calling in model.CALLINGS:
            plain = totals(calling, 2, {}, 30, random.Random(30))
            for row in model.talent_rows(calling):
                for c in row["choices"]:
                    if (calling, c["key"]) in model.IGNORED:
                        continue
                    with self.subTest(calling=calling, choice=c["key"]):
                        pull = LONG_PULL if (calling, c["key"]) in LONG_FIGHT else {}
                        base = totals(calling, 2, {}, 30, random.Random(30), **pull) if pull else plain
                        self.assertNotEqual(totals(calling, 2, {row["key"]: c["key"]}, 30, random.Random(30), **pull), base)
        choices = {(calling, c["key"]) for calling in model.CALLINGS for row in model.talent_rows(calling)
                   for c in row["choices"]}
        for key, why in model.IGNORED.items():
            with self.subTest(ignored=key):
                self.assertIn(key, choices)
                self.assertTrue(why)

    # What each Vanguard choice the sim models buys, from its description.
    VANGUARD_GAINS = {
        "iron_juggernaut": "damage", "vengeful_conviction": "damage", "penitents_smash": "survival",
        "searing_penance": "damage", "shield_specialization": "damage", "steadfast_penitent": "survival",
        "zealous_restoration": "survival", "tithe_of_zeal": "damage",
        "hammer_of_vanguard": "damage", "bulwark_slam": "survival", "fervent_charge": "damage",
        "avenging_light": "damage", "bulwark_of_faith": "survival", "ceaseless_zeal": "damage",
        "wrathful_iron": "damage", "seal_of_mending": "survival", "seal_of_fervor": "damage",
    }

    def test_vanguard_choices_move_the_fight(self):
        rows = {c["key"]: row["key"] for row in model.talent_rows("vanguard") for c in row["choices"]}
        self.assertEqual(set(self.VANGUARD_GAINS),
                         {k for k in rows if ("vanguard", k) not in model.IGNORED})
        # Fight i of every build starts from seed i, so a small gain (Tithe of Zeal's +0.6%) shows
        # over the luck. The plain Vanguard loses every fight here, so survival is the health lost
        # a second: it falls when the character dies later.
        plain = totals("vanguard", 2, {}, 50, random.Random)
        for key, gain in self.VANGUARD_GAINS.items():
            with self.subTest(choice=key, gain=gain):
                dealt, lost, spent = totals("vanguard", 2, {rows[key]: key}, 50, random.Random)
                if gain == "damage":
                    self.assertGreater(dealt / spent, plain[0] / plain[2])
                else:
                    self.assertLess(lost / spent, plain[1] / plain[2])

    def test_vanguard_has_no_unmodelled_choices(self):
        for (calling, key), why in model.IGNORED.items():
            if calling == "vanguard":
                with self.subTest(choice=key):
                    self.assertNotEqual(why, "not modelled yet")

    # What each Cutthroat choice the sim models buys, from its description. Brazen Garrote's
    # Garrote out of Stealth beats a Sinister Strike only when its whole bleed lands, and the
    # Cutthroat's fights at 30 end first: it changes the fight and buys nothing ("even").
    CUTTHROAT_GAINS = {
        "serrated_strikes": "damage", "flurry_of_blades": "damage", "practiced_strike": "damage",
        "phantom_guile": "damage", "elusiveness": "survival", "second_wind": "survival",
        "open_wounds": "damage", "executioners_toll": "survival", "kidney_shot": "survival",
        "bloodletting": "damage", "open_veins": "damage", "sanguine_rupture": "survival",
        "blood_frenzy": "damage", "thirst": "survival", "blood_rush": "damage",
        "brazen_garrote": "even", "choking_garrote": "survival", "initiative": "damage",
    }
    # The cases whose small gain shows only over more fights: Initiative's is +1.3% at 300 and
    # under the luck at 50. Elusiveness changes no fight against 2 normals.
    CUTTHROAT_FIGHTS = {"initiative": 300}
    CUTTHROAT_PULLS = {"elusiveness": LONG_PULL}

    def test_cutthroat_choices_move_the_fight(self):
        rows = {c["key"]: row["key"] for row in model.talent_rows("cutthroat") for c in row["choices"]}
        self.assertEqual(set(self.CUTTHROAT_GAINS),
                         {k for k in rows if ("cutthroat", k) not in model.IGNORED})
        # As the Vanguard's: fight i of every build starts from seed i, and the plain Cutthroat
        # loses nearly every fight here, so survival is the health lost a second.
        plains = {}
        for key, gain in self.CUTTHROAT_GAINS.items():
            with self.subTest(choice=key, gain=gain):
                fights, pull = self.CUTTHROAT_FIGHTS.get(key, 50), self.CUTTHROAT_PULLS.get(key, {})
                at = (fights, tuple(sorted(pull.items())))
                if at not in plains:
                    plains[at] = totals("cutthroat", 2, {}, fights, random.Random, **pull)
                plain = plains[at]
                got = totals("cutthroat", 2, {rows[key]: key}, fights, random.Random, **pull)
                dealt, lost, spent = got
                if gain == "damage":
                    self.assertGreater(dealt / spent, plain[0] / plain[2])
                elif gain == "survival":
                    self.assertLess(lost / spent, plain[1] / plain[2])
                else:
                    self.assertNotEqual(got, plain)

    def test_cutthroat_has_no_unmodelled_choices(self):
        for (calling, key), why in model.IGNORED.items():
            if calling == "cutthroat":
                with self.subTest(choice=key):
                    self.assertNotEqual(why, "not modelled yet")

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
                result = cls("vanguard", 30, ch["stats"], ch["rows"], random.Random(1), {}, mob=mob).fight()
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
