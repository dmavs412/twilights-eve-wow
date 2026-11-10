"""The balance model (tools/twilights-eve/balance/) against the realm's own numbers.

    ~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests
"""

from __future__ import annotations

import argparse
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
import run as run_balance  # noqa: E402
import scenarios  # noqa: E402


def swing(mob: dict) -> float:
    return (mob["dmin"] + mob["dmax"]) / 2


# The fights below were sized when mobs.toml's [difficulty] was 3.2 health and 3.0 damage, twice
# today's from level 10 up. Their mobs keep that strength, so a slow choice still has time to show.
SIZED_AT = 2.0

# They were sized in Elwynn's kits, too. The 30-50 arc's Quest Lines (#600) start at required level
# 30, so a level-30 kit would wear them and end the fights before Elusiveness's Evasion comes back.
# These tests measure the talent choices, so they fight without that gear.
SIZED_WITHOUT = "quest-"
_set_aside = {}


def setUpModule():
    _set_aside.update({e: r for e, r in model.GEAR.items() if r["line"].startswith(SIZED_WITHOUT)})
    for entry in _set_aside:
        del model.GEAR[entry]


def tearDownModule():
    model.GEAR.update(_set_aside)


def totals(calling: str, n: int, rows: dict, fights: int, rng: random.Random, level: int = 30,
           gear: str = "green", rank: int = 0) -> tuple:
    """(damage dealt, health lost, fight time) summed over `fights` fights of `calling` in `gear`
    and the build `rows` against `n` mobs of `rank` (0 normal, 1 elite) on level, at SIZED_AT
    times their health and damage. `rng` is one Random every fight draws from, or a function
    giving fight i its own."""
    mob = model.mob_stats(level, rank=rank, health_mul=SIZED_AT, damage_mul=SIZED_AT)
    cls = multi.fighter(calling, n)
    ch = model.character(calling, level, gear, rows)
    dealt = lost = spent = 0.0
    for i in range(fights):
        sim = cls(calling, level, ch["stats"], ch["rows"], rng(i) if callable(rng) else rng, {}, mob=mob)
        r = sim.fight()
        dealt += sum(sim.dmg.values())
        lost += 1 - r["health_left"]
        spent += r["time"]
    return dealt, lost, spent


def tier2_in_data() -> list:
    """The Tier 2 Callings server_spells.json already has as Tier 2."""
    return [c for c in model.TIER2_CALLINGS if model.SPELLS.get(c, {}).get("tier") == 2]


def fought(calling: str, rows: dict, fights: int, rng: random.Random, n: int = 2, **pull) -> tuple:
    """totals() against `n` mobs, 2 unless the pull says, in the pull's gear and rank."""
    return totals(calling, n, rows, fights, rng, **pull)


# A pull of 2 elites in blues, B-2E's: it lasts about 55 s at 30, where 2 normals last about 30 s.
LONG_PULL = dict(gear="blue", rank=1)
# One normal, the pick's own fight: the plain Initiate lasts about 18 s there. Against 2 normals at
# 30 it dies in about 8 s, before its first Ley Spark.
ONE_NORMAL = dict(n=1)
# Choices that change nothing in 30 fights against 2 normals, and the pull
# test_every_tier1_choice_changes_the_fight_or_is_ignored fights them in instead: Elusiveness
# brings Evasion back after 60 s, Blood Frenzy's stacks past 5 need a long run of droplets, and
# the six Initiate choices act through Ley Spark (its crit, a buff, the mana it crits with). The
# plain Acolyte dies in about 6 s against 2 normals and lasts about 23 s against 1: Gorging Siphon
# shields it while a lone mob runs in, which a pull never does, and Virulence's faster Blood Lance,
# Sanguine Barrier's shield off a Lance and Necrotic Detonation's damage need the Lances and Blight
# ticks of the longer fight.
LONG_FIGHT = {
    ("cutthroat", "elusiveness"): LONG_PULL, ("cutthroat", "blood_frenzy"): LONG_PULL,
    ("initiate", "ley_line_tap"): ONE_NORMAL, ("initiate", "arcane_substrate"): ONE_NORMAL,
    ("initiate", "warding_spark"): ONE_NORMAL, ("initiate", "siphoning_ward"): ONE_NORMAL,
    ("initiate", "kindling_ember"): ONE_NORMAL, ("initiate", "arcane_momentum"): ONE_NORMAL,
    ("ranger", "explosive_payload"): ONE_NORMAL, ("ranger", "twin_arrows"): ONE_NORMAL,
    ("ranger", "jagged_barbs"): ONE_NORMAL, ("ranger", "flurry"): ONE_NORMAL,
    ("ranger", "rapid_assault"): ONE_NORMAL,
    ("acolyte", "gorging_siphon"): ONE_NORMAL, ("acolyte", "virulence"): ONE_NORMAL,
    ("acolyte", "sanguine_barrier"): ONE_NORMAL, ("acolyte", "necrotic_detonation"): ONE_NORMAL,
}
# A pull starts in melee, and the Ranger shoots again only past its 8 yd minimum range: a Vault
# puts it at 15 yd, inside 8 again before its global cooldown ends. Against 2 normals it never
# shoots but its auto shot; against 1 it fires its opener while the mob runs in. It starts a Shaft
# only when the cast ends before the mob reaches 8 yd, never after the opener, so the Shaft choices
# fight on top of Point Blank, which lifts the minimum range.
POINT_BLANK = {"talents": "point_blank"}
ON_TOP = {("ranger", k): POINT_BLANK for k in ("piercing_force", "staggering_shaft", "killshot")}
# The every-choice test's cases that need more than 30 fights: mana lifts Ley Spark's crit by a
# hair while the bar stays nearly full, and over 30 fights no roll lands in that hair.
MORE_FIGHTS = {("initiate", "siphoning_ward"): 150, ("initiate", "kindling_ember"): 150}


class BalanceModel(unittest.TestCase):
    def test_mob_stats_match_the_realm(self):
        at_ten = model.DATA.mob_class["warrior"]
        health, damage = at_ten["health"][9], at_ten["melee_damage_avg"][9]
        # mobs.rs a_level_12_warrior_in_elwynn_forest_has_half_again_northshires_health.
        twelve = model.mob_stats(12)
        self.assertEqual(twelve["hp"], 570)
        self.assertAlmostEqual(swing(twelve), 35.66, delta=0.05)
        # Level 20 in Elwynn, as mobs.rs difficulty_puts_health_and_damage_on_the_power_curve
        # computes a creature: the level-10 row x P(20) = 2.0, x1.6 health and x1.5 damage, then
        # Elwynn's 1.5 and 1.3.
        twenty = model.mob_stats(20)
        self.assertLessEqual(abs(twenty["hp"] - round(health * 2.0 * 1.6 * 1.5)), 1)
        self.assertAlmostEqual(swing(twenty), damage * 2.0 * 1.5 * 1.3, places=3)
        # That test's own mob: level 8, no Zone Factor, damage on the flat fade at x1.5.
        eight = model.mob_stats(8)
        self.assertLessEqual(abs(eight["hp"] - round(health * 0.8 * 1.6)), 1)
        self.assertAlmostEqual(swing(eight) / (damage * 0.8), 1.5, places=3)
        self.assertEqual(eight["armor"], at_ten["armor"][7])

    def test_health_pct_scales_base_health(self):
        # callings.csv health_pct, as rules/stats.rs scale_base_pools applies it in the realm.
        row = model.CALLINGS_CSV["cutthroat"]
        was = row["health_pct"]
        try:
            row["health_pct"] = "100"
            plain = model.character("cutthroat", 20, "green")["stats"].base_health
            row["health_pct"] = "200"
            doubled = model.character("cutthroat", 20, "green")["stats"].base_health
        finally:
            row["health_pct"] = was
        self.assertEqual(doubled, 2 * plain)

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

    def test_calling_rules_run_past_30(self):
        # #661: the Calling rules run up to 50, and a run of 1 to 30 keeps its old cells.
        def levels_of(levels):
            seen = []
            scenarios.p_2(levels, 1, ["vanguard"], lambda cells: seen.extend(cells) or [])
            return sorted({c["level"] for c in seen})

        self.assertEqual(levels_of([33, 48]), [33, 48])
        self.assertEqual(levels_of(range(1, 31)), list(range(10, 31)))

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
                        pull = LONG_FIGHT.get((calling, c["key"]), {})
                        fights = MORE_FIGHTS.get((calling, c["key"]), 30)
                        on = ON_TOP.get((calling, c["key"]), {})
                        base = fought(calling, on, fights, random.Random(30), **pull) if pull or on or fights != 30 else plain
                        got = fought(calling, {**on, row["key"]: c["key"]}, fights, random.Random(30), **pull)
                        self.assertNotEqual(got, base)
        choices = {(calling, c["key"]) for calling in model.CALLINGS for row in model.talent_rows(calling)
                   for c in row["choices"]}
        for key, why in model.IGNORED.items():
            if key[0] not in model.CALLINGS:
                continue
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

    # What each Initiate choice the sim models buys, from its description. The plain Initiate dies
    # in every pull at 30 with about 95% of its mana, so Ley-Line Tap's refund buys nothing and its
    # roll moves the luck ("even"). Siphoning Ward and Kindling Ember buy only Ley Spark's crit,
    # which grows with the mana bar.
    INITIATE_GAINS = {
        "ley_line_tap": "even", "arcane_substrate": "damage", "warding_spark": "survival",
        "siphoning_ward": "damage", "volatile_ward": "damage",
        "thermal_feedback": "damage", "smoldering_ember": "survival", "kindling_ember": "damage",
        "combustion_engine": "damage", "arcane_momentum": "damage", "catalytic_resonance": "damage",
        "thermal_shock": "damage", "deep_freeze": "survival", "catalytic_chain": "damage",
    }

    def test_initiate_choices_move_the_fight(self):
        rows = {c["key"]: row["key"] for row in model.talent_rows("initiate") for c in row["choices"]}
        self.assertEqual(set(self.INITIATE_GAINS),
                         {k for k in rows if ("initiate", k) not in model.IGNORED})
        # As the Vanguard's: fight i of every build starts from seed i, and survival is the health
        # lost a second. A choice that changes nothing against 2 normals fights 1 (LONG_FIGHT).
        plains = {}
        for key, gain in self.INITIATE_GAINS.items():
            with self.subTest(choice=key, gain=gain):
                pull = LONG_FIGHT.get(("initiate", key), {})
                at = tuple(sorted(pull.items()))
                if at not in plains:
                    plains[at] = fought("initiate", {}, 50, random.Random, **pull)
                plain = plains[at]
                got = fought("initiate", {rows[key]: key}, 50, random.Random, **pull)
                dealt, lost, spent = got
                if gain == "damage":
                    self.assertGreater(dealt / spent, plain[0] / plain[2])
                elif gain == "survival":
                    self.assertLess(lost / spent, plain[1] / plain[2])
                else:
                    self.assertNotEqual(got, plain)

    def test_initiate_has_no_unmodelled_choices(self):
        for (calling, key), why in model.IGNORED.items():
            if calling == "initiate":
                with self.subTest(choice=key):
                    self.assertNotEqual(why, "not modelled yet")

    # What each Ranger choice the sim models buys, from its description.
    RANGER_GAINS = {
        "explosive_payload": "damage", "point_blank": "damage", "twin_arrows": "damage",
        "parting_shot": "damage", "jagged_barbs": "damage", "piercing_force": "damage",
        "staggering_shaft": "survival", "killshot": "damage", "flurry": "damage", "rapid_assault": "damage",
    }

    def test_ranger_choices_move_the_fight(self):
        rows = {c["key"]: row["key"] for row in model.talent_rows("ranger") for c in row["choices"]}
        self.assertEqual(set(self.RANGER_GAINS), {k for k in rows if ("ranger", k) not in model.IGNORED})
        # As the Vanguard's: fight i of every build starts from seed i, and survival is the health
        # lost a second. A choice that changes nothing against 2 normals fights 1 (LONG_FIGHT), and
        # a Shaft choice fights on top of Point Blank (ON_TOP).
        plains = {}
        for key, gain in self.RANGER_GAINS.items():
            with self.subTest(choice=key, gain=gain):
                pull, on = LONG_FIGHT.get(("ranger", key), {}), ON_TOP.get(("ranger", key), {})
                at = (tuple(sorted(pull.items())), tuple(sorted(on.items())))
                if at not in plains:
                    plains[at] = fought("ranger", on, 50, random.Random, **pull)
                plain = plains[at]
                dealt, lost, spent = fought("ranger", {**on, rows[key]: key}, 50, random.Random, **pull)
                if gain == "damage":
                    self.assertGreater(dealt / spent, plain[0] / plain[2])
                else:
                    self.assertLess(lost / spent, plain[1] / plain[2])

    def test_ranger_has_no_unmodelled_choices(self):
        for (calling, key), why in model.IGNORED.items():
            if calling == "ranger":
                with self.subTest(choice=key):
                    self.assertNotEqual(why, "not modelled yet")

    # What each Acolyte choice the sim models buys, from its description.
    ACOLYTE_GAINS = {
        "epidemic": "damage", "virulence": "damage", "withering_blight": "survival",
        "exsanguinate": "damage", "splintering_lance": "damage", "sanguine_barrier": "survival",
        "necrotic_detonation": "damage", "masochism": "survival", "will_to_survive": "survival",
        "feeding_siphon": "damage", "gorging_siphon": "survival", "coagulation": "survival",
        "dark_surplus": "damage", "sanguine_flow": "damage", "noxious_vapors": "survival",
        "breeding_ground": "damage",
    }

    def test_acolyte_choices_move_the_fight(self):
        rows = {c["key"]: row["key"] for row in model.talent_rows("acolyte") for c in row["choices"]}
        self.assertEqual(set(self.ACOLYTE_GAINS), {k for k in rows if ("acolyte", k) not in model.IGNORED})
        # As the Vanguard's: fight i of every build starts from seed i, and survival is the health
        # lost a second. A choice that pays off only in a longer fight fights 1 normal (LONG_FIGHT).
        plains = {}
        for key, gain in self.ACOLYTE_GAINS.items():
            with self.subTest(choice=key, gain=gain):
                pull = LONG_FIGHT.get(("acolyte", key), {})
                at = tuple(sorted(pull.items()))
                if at not in plains:
                    plains[at] = fought("acolyte", {}, 50, random.Random, **pull)
                plain = plains[at]
                dealt, lost, spent = fought("acolyte", {rows[key]: key}, 50, random.Random, **pull)
                if gain == "damage":
                    self.assertGreater(dealt / spent, plain[0] / plain[2])
                else:
                    self.assertLess(lost / spent, plain[1] / plain[2])

    def test_acolyte_has_no_unmodelled_choices(self):
        for (calling, key), why in model.IGNORED.items():
            if calling == "acolyte":
                with self.subTest(choice=key):
                    self.assertNotEqual(why, "not modelled yet")

    def test_kits_fill_every_slot(self):
        # The Tier 2 Callings in the data wear the 30-50 arc's rows (#615), up to Power Level 50.
        cases = [(c, (15, 25)) for c in model.CALLINGS]
        cases += [(c, (30, 40, 50)) for c in tier2_in_data()]
        for calling, levels in cases:
            for level in levels:
                for tier in ("green", "blue"):
                    with self.subTest(calling=calling, level=level, tier=tier):
                        entries, off, _, empty = model.kit(calling, level, tier)
                        slots = Counter(model.GEAR[e]["slot"] for e in entries + ([off] if off else []))
                        self.assertEqual(slots["Chest"], 1)
                        self.assertGreaterEqual(slots["Weapon"], 1)
                        self.assertEqual(slots["Amulet"], 1)
                        self.assertEqual(slots["Ring"], 2)
                        self.assertEqual(empty, [])

    def test_a_slot_with_no_row_is_named(self):
        amulets = {e: r for e, r in model.GEAR.items() if r["slot"] == "Amulet" and r["quality"] == "Uncommon"}
        try:
            for e in amulets:
                del model.GEAR[e]
            entries, off, _, empty = model.kit("vanguard", 40, "green")
        finally:
            model.GEAR.update(amulets)
        self.assertEqual(empty, ["Amulet"])
        self.assertNotIn("Amulet", [model.GEAR[e]["slot"] for e in entries])

    def test_tier2_levels_run_power_levels_30_to_50(self):
        # rules/stats.rs power_level: a Tier 2 Calling's real levels 1 to 50 are Power Levels 30 to
        # 50, and real level 25 is Power Level 40.
        self.assertEqual(model.power_level(2, 1), 30)
        self.assertEqual(model.power_level(2, 25), 40)
        self.assertEqual(model.power_level(2, 50), 50)
        self.assertTrue(tier2_in_data())
        for calling in tier2_in_data():
            with self.subTest(calling=calling):
                self.assertEqual(model.calling_level(calling, 30), 1)
                self.assertEqual(model.calling_level(calling, 40), 25)
                self.assertEqual(model.calling_level(calling, 50), 50)
                want = [r["key"] for r in model.talent_rows(calling) if r["level"] <= 25]
                self.assertEqual([r["key"] for r in model.open_rows(calling, 40)], want)

    def test_mobs_take_the_tier_step_above_30(self):
        d = model.DATA.difficulty
        step = d["tier_step"]
        at_ten = model.DATA.mob_class["warrior"]
        health, damage = at_ten["health"][9], at_ten["melee_damage_avg"][9]
        # On level above 30 is Redridge, whose Zone Factor is Duskwood's.
        self.assertEqual(model.on_level_area(30), model.ELWYNN_FOREST)
        self.assertEqual(model.on_level_area(31), model.REDRIDGE_MOUNTAINS)
        self.assertEqual(model.zone_factor(model.REDRIDGE_MOUNTAINS), model.zone_factor(model.DUSKWOOD))
        for level, (step_h, step_d) in ((30, (1.0, 1.0)), (31, (step["health"], step["damage"]))):
            with self.subTest(level=level):
                zone_h, zone_d = model.zone_factor(model.on_level_area(level))
                p = model.curve_at(level)
                mob = model.mob_stats(level)
                self.assertLessEqual(abs(mob["hp"] - round(health * p * d["health"] * step_h * zone_h)), 1)
                self.assertAlmostEqual(swing(mob), damage * p * d["damage"] * step_d * zone_d, places=3)

    def test_a_tier2_run_with_no_sims_lists_each_calling_as_no_sim(self):
        levels = run_balance.parse_levels(["31-50"])
        tier2 = list(model.TIER2_CALLINGS)
        sims = {c: multi.CLASSES.pop(c) for c in tier2 if c in multi.CLASSES}
        try:
            serial = lambda cells: [scenarios.run_cell(c) for c in scenarios.with_builds(cells)]  # noqa: E731
            rows = [r for fs in scenarios.SCENARIOS.values() for f in fs for r in f(levels, 1, tier2, serial)]
        finally:
            multi.CLASSES.update(sims)
        self.assertTrue(rows)
        for r in rows:
            # The named-boss rules take the bosses' levels, not the run's.
            if r["rule"] not in ("B-named", "G+5-named"):
                self.assertEqual(r["notes"], "no sim at levels 31 to 50", r)
            self.assertTrue(r["notes"].startswith("no sim at levels"), r)
            self.assertIsNone(r["holds"])
        self.assertEqual({r["calling"] for r in rows}, set(tier2))
        args = argparse.Namespace(data_root=model.REPO, fights=1, scenario="all")
        text = run_balance.report(rows, args, levels)
        self.assertIn("## Tier 2", text)
        self.assertIn("## Failures\n\n- none\n", text)

    def test_h1_reports_the_friends_win_and_health_left(self):
        # A fixture healer: the Acolyte's data as a Tier 2 Blightcaller, which mends the friend to
        # full every second, and the same Blightcaller that never heals it.
        calling = "blightcaller"

        class Mender(multi.CLASSES["acolyte"]):
            def after_step(self):
                super().after_step()
                if self.friend and self.t >= getattr(self, "mend_t", 0.0):
                    self.mend_t = self.t + 1.0
                    self.heal_friend(self.friend["hp"])

        added = dict(spells=model.SPELLS, callings=model.CALLINGS_CSV)
        kept = {name: table.get(calling) for name, table in added.items()}
        model.SPELLS[calling] = {**model.SPELLS["acolyte"], "key": calling, "tier": 2, "tier_cap": 50}
        model.CALLINGS_CSV[calling] = model.CALLINGS_CSV["acolyte"]
        had = multi.CLASSES.get(calling)
        try:
            rows = {}
            for name, cls in (("mends", Mender), ("idle", multi.CLASSES["acolyte"])):
                multi.CLASSES[calling] = cls
                scenarios._BUILDS.clear()
                scenarios._CHARACTERS.clear()
                rows[name] = scenarios.h_1([40], 5, [calling], lambda cells: [scenarios.run_cell(c) for c in cells])
        finally:
            # The Blightcaller's own data and sim come back once the fixture is done.
            for name, table in added.items():
                if kept[name] is None:
                    del table[calling]
                else:
                    table[calling] = kept[name]
            if had:
                multi.CLASSES[calling] = had
            else:
                del multi.CLASSES[calling]
            scenarios._BUILDS.clear()
            scenarios._CHARACTERS.clear()
        mends, idle = rows["mends"][0], rows["idle"][0]
        for r in (mends, idle):
            self.assertEqual((r["rule"], r["calling"], r["level"]), ("H-1", calling, 40))
            self.assertIn("friend", r["against"])
        self.assertEqual(mends["win"], 1.0)
        self.assertGreaterEqual(mends["health_left"], 0.30)
        self.assertTrue(mends["holds"])
        self.assertLess(idle["win"], 0.80)
        self.assertFalse(idle["holds"])

    # The sturdy Tier 2 Callings' sims: the Ironclad and the Zealot in the form of the Vanguard,
    # the Blightcaller and the Bloodmender in the form of the Acolyte.
    STURDY_TIER2 = ("ironclad", "zealot", "blightcaller", "bloodmender")

    def test_sturdy_tier2_sims_fight_at_30_and_50(self):
        for calling in self.STURDY_TIER2:
            for level in (30, 50):
                with self.subTest(calling=calling, level=level):
                    self.assertIn(calling, multi.CLASSES)
                    ch = model.character(calling, level, "green", scenarios.pick_build(calling, level, 1))
                    sim = multi.fighter(calling, 1)(calling, level, ch["stats"], ch["rows"], random.Random(7), {},
                                                    mob=model.mob_stats(level))
                    r = sim.fight()
                    self.assertLess(r["time"], model.FIGHT_CAP_S)
                    self.assertTrue(r["won"] or r["died"])
                    self.assertGreater(sum(sim.dmg.values()), 0)
                    self.assertTrue(0 <= r["health_left"] <= 1)
                    row = scenarios.run_cell(scenarios.cell(calling, level, "green", scenarios.normals(level, 1), 1))
                    self.assertIn(row["win"], (0, 1))
                    self.assertGreater(row["dps"], 0)
                    self.assertTrue(row["win"] == 0 or 0 < row["health_left"] <= 1)

    def test_ironclad_spends_conviction_with_shield_slam(self):
        level = 40
        ch = model.character("ironclad", level, "green")
        cls = multi.fighter("ironclad", 1)
        slams = []
        for seed in range(5):
            sim = cls("ironclad", level, ch["stats"], ch["rows"], random.Random(seed), {},
                      mob=model.mob_stats(level, rank=1))
            sim.fight()
            slams += [entry for entry in sim.fight_log if entry[1] == "shield_slam"]
        self.assertTrue(slams)
        for at, key, stacks in slams:
            self.assertGreaterEqual(stacks, 3)

    def test_zealot_holy_wrath_doubles_on_undead(self):
        level = 40
        ch = model.character("zealot", level, "green")
        dealt, stunned = {}, {}
        for kind in ("humanoid", "undead"):
            mob = model.mob_stats(level, creature_type=kind)
            sim = model.Zealot("zealot", level, ch["stats"], ch["rows"], random.Random(1), {}, mob=mob)
            sim.mob_hp, sim.m_armor = mob["hp"], mob["armor"]
            sim.setup_fight()
            swing_at = sim.mob_swing_t
            sim.holy_wrath()
            dealt[kind] = sim.dmg["holy_wrath"]
            stunned[kind] = sim.mob_swing_t > swing_at
        self.assertGreater(dealt["humanoid"], 0)
        self.assertEqual(dealt["undead"], 2 * dealt["humanoid"])
        self.assertEqual(stunned, dict(humanoid=False, undead=True))
        # A named boss fights as its template's creature_type; on-level mobs are humanoid.
        self.assertEqual(model.mob_stats(level)["type"], "humanoid")
        types = {boss.boss_mob(b)["type"] for b in boss.named_bosses()}
        self.assertIn("undead", types)

    def test_healers_h1_cells_report_the_friend(self):
        serial = lambda cells: [scenarios.run_cell(c) for c in scenarios.with_builds(cells)]  # noqa: E731
        for calling in ("blightcaller", "bloodmender"):
            with self.subTest(calling=calling):
                rows = scenarios.h_1([30, 50], 3, [calling], serial)
                self.assertEqual([r["level"] for r in rows], [30, 50])
                for r in rows:
                    self.assertFalse(r.get("no_sim"))
                    self.assertIn("friend", r["against"])
                    self.assertTrue(0 <= r["win"] <= 1)
                    self.assertTrue(r["win"] == 0 or 0.30 <= r["health_left"] <= 1)
                    self.assertIsNotNone(r["holds"])
                # The healer's heals reach the friend.
                ch = model.character(calling, 40, "green")
                cls = multi.guard(calling, dict(hp=ch["stats"].max_hp, armor=ch["stats"].armor))
                sim = cls(calling, 40, ch["stats"], ch["rows"], random.Random(3), {}, mob=model.mob_stats(40, rank=1))
                sim.fight()
                self.assertGreater(sim.friend_healed, 0)

    def test_every_tier2_ignored_entry_names_a_choice_or_row(self):
        for (calling, key), why in model.IGNORED.items():
            if calling in model.CALLINGS:
                continue
            with self.subTest(calling=calling, key=key):
                rows = model.talent_rows(calling)
                self.assertIn(key, {r["key"] for r in rows} | {c["key"] for r in rows for c in r["choices"]})
                self.assertTrue(why)

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
