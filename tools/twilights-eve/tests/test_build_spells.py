"""build_spells.py driven through its command line against small fixture CSVs.

    ~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests
"""

from __future__ import annotations

import csv
import json
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, "..", "build_spells.py")
MASTER = os.path.join(HERE, "..", "..", "..", "data", "twilights-eve", "master.csv")
with open(MASTER, newline="", encoding="utf-8") as _f:
    COLUMNS = next(csv.reader(_f))

STRIKE = {
    "kind": "skill", "calling": "vanguard", "class": "1", "key": "strike", "name": "Strike", "rank": "1",
    "spell_id": "78", "client_cost": "15", "realm_cost": "15", "target": "enemy",
    "effect1_kind": "weapon_strike",
    "effect1_params": "weapon=normal;bonus.base=9.5;bonus.per_level=1.4;bonus.per_ap=0;tags=a|b",
    "effect4_kind": "stun", "effect4_params": "duration_ms=1000",
    "modelled": "yes", "description": "Hits.",
}


def talent(key: str, rank: int, row: int, column: int, **extra: str) -> dict:
    return {"kind": "talent", "calling": "vanguard", "class": "1", "key": key, "name": key.title(),
            "rank": str(rank), "tab": "Arms", "tab_row": str(row), "tab_column": str(column),
            "modelled": "yes", **extra}


CALLING = {
    "kind": "calling", "calling": "vanguard", "class": "1", "key": "vanguard", "name": "Vanguard",
    "tier": "0",
    "tier_cap": "10", "tier_bonus": "strength=1;stamina=2", "shapeshift_form": "17",
    "armour": "cloth|mail",
}
TAB = {"kind": "talent_tab", "calling": "vanguard", "class": "1", "key": "arms", "name": "Arms", "tab_id": "900",
       "background": "WarriorArms", "tab_order": "0"}
AURA = {"kind": "aura", "calling": "vanguard", "class": "1", "key": "buff", "name": "Buff", "rank": "1", "spell_id": "14177",
        "duration_ms": "10000", "charges": "1", "effect1_kind": "skill_crit_pct",
        "effect1_params": "skill=strike;pct=50", "modelled": "no"}


class BuildSpells(unittest.TestCase):
    def run_build(self, rows: list[dict]) -> tuple[subprocess.CompletedProcess, list | None]:
        tmp = tempfile.mkdtemp()
        path = os.path.join(tmp, "master.csv")
        with open(path, "w", newline="", encoding="utf-8") as f:
            writer = csv.DictWriter(f, COLUMNS, restval="")
            writer.writeheader()
            writer.writerows(rows)
        done = subprocess.run([sys.executable, SCRIPT, "--csv", path, "--out-dir", tmp, "--no-dbc-check"],
                              capture_output=True, text=True)
        self.patch = None
        data = None
        for name in ("server_spells.json", "client_dbc_patch.json"):
            out = os.path.join(tmp, name)
            if os.path.exists(out):
                with open(out, encoding="utf-8") as f:
                    if name == "server_spells.json":
                        data = json.load(f)
                    else:
                        self.patch = json.load(f)
        return done, data

    def assert_fails(self, rows: list[dict], message: str) -> None:
        done, data = self.run_build(rows)
        self.assertNotEqual(done.returncode, 0)
        self.assertIn(message, done.stderr)
        self.assertIsNone(data)
        self.assertIsNone(self.patch)

    def test_skill_carries_realm_columns_and_effects(self):
        done, data = self.run_build([STRIKE])
        self.assertEqual(done.returncode, 0, done.stderr)
        skill = data[0]["skills"][0]
        self.assertEqual(data[0]["key"], "vanguard")
        self.assertEqual(skill["realm_cost"], 15)
        self.assertTrue(skill["modelled"])
        strike, stun = skill["effects"]
        self.assertEqual(strike["params"]["bonus"], {"base": 9.5, "per_level": 1.4, "per_ap": 0})
        self.assertEqual(strike["params"]["tags"], ["a", "b"])
        self.assertFalse(strike["realm_only"])
        self.assertTrue(stun["realm_only"])

    def test_unmodelled_row_is_emitted_with_empty_effects(self):
        row = {**STRIKE, "modelled": "no", "effect1_kind": "not_yet_a_kind"}
        done, data = self.run_build([row])
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(data[0]["skills"][0]["effects"], [])
        self.assertFalse(data[0]["skills"][0]["modelled"])
        fields = self.patch["spells"][0]["fields"]
        self.assertNotIn("effects", fields)
        self.assertEqual(fields["cost"], 15)

    def test_unknown_kind_fails_a_modelled_row(self):
        self.assert_fails([{**STRIKE, "effect1_kind": "not_yet_a_kind"}], "unknown effect kind")

    def test_malformed_params_fail(self):
        self.assert_fails([{**STRIKE, "effect1_params": "weapon"}], "malformed param")

    def test_formula_missing_a_field_fails(self):
        self.assert_fails([{**STRIKE, "effect1_params": "bonus.base=9.5"}], "lacks per_level")

    def test_formula_with_text_fails(self):
        self.assert_fails([{**STRIKE, "effect1_params": "bonus.base=9.5;bonus.per_level=L*2"}], "not a number")

    def test_realm_cost_below_client_cost_fails(self):
        self.assert_fails([{**STRIKE, "realm_cost": "10"}], "below client_cost")

    def test_a_callings_rows_must_agree_on_class(self):
        self.assert_fails([STRIKE, {**STRIKE, "key": "stab", "class": "4"}], "disagree on class")

    def test_a_missing_or_unknown_class_fails(self):
        self.assert_fails([{**STRIKE, "class": ""}], "class is required")
        self.assert_fails([{**STRIKE, "class": "6"}], "is not one of")
        self.assert_fails([{**STRIKE, "class": "1.5"}], "is not one of")

    def test_every_patch_row_carries_the_power_type_of_its_class(self):
        for cls, power_type in (("1", 1), ("4", 3), ("8", 0)):
            rows = [{**STRIKE, "class": cls}, talent("cruelty", 1, 0, 1, **{"class": cls, "spell_id": "12320"})]
            done, data = self.run_build(rows)
            self.assertEqual(done.returncode, 0, done.stderr)
            self.assertEqual(data[0]["class"], int(cls))
            self.assertNotIn("class", data[0]["skills"][0])
            self.assertEqual(len(self.patch["spells"]), 2)
            for override in self.patch["spells"]:
                self.assertEqual(override["fields"]["power_type"], power_type, cls)

    def test_energize_names_its_power(self):
        row = {**STRIKE, "effect1_kind": "energize",
               "effect1_params": "power=energy;amount.base=20;amount.per_level=0"}
        done, data = self.run_build([row])
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(data[0]["skills"][0]["effects"][0]["params"]["power"], "energy")

    def test_energize_without_a_known_power_fails(self):
        self.assert_fails([{**STRIKE, "effect1_kind": "energize",
                            "effect1_params": "amount.base=20;amount.per_level=0"}], "energize needs power")
        self.assert_fails([{**STRIKE, "effect1_kind": "energize",
                            "effect1_params": "power=focus;amount.base=20;amount.per_level=0"}],
                          "energize needs power")
        self.assert_fails([{**STRIKE, "effect1_kind": "energize", "effect1_params": "power=rage"}],
                          "lacks an amount")

    def test_energize_rage_is_no_longer_a_kind(self):
        self.assert_fails([{**STRIKE, "effect1_kind": "energize_rage",
                            "effect1_params": "amount.base=20;amount.per_level=0"}], "unknown effect kind")

    def test_add_combo_points_needs_a_positive_whole_amount(self):
        row = {**STRIKE, "effect1_kind": "add_combo_points", "effect1_params": "amount=1"}
        done, data = self.run_build([row])
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(data[0]["skills"][0]["effects"][0]["params"], {"amount": 1})
        for bad in ("amount=0", "amount=-1", "amount=1.5", ""):
            self.assert_fails([{**STRIKE, "effect1_kind": "add_combo_points", "effect1_params": bad}],
                              "positive whole amount")

    def test_per_combo_is_a_formula_block(self):
        row = {**STRIKE, "effect1_kind": "school_damage",
               "effect1_params": "amount.base=5;amount.per_level=0;per_combo.base=5;per_combo.per_level=0.5"}
        done, data = self.run_build([row])
        self.assertEqual(done.returncode, 0, done.stderr)
        params = data[0]["skills"][0]["effects"][0]["params"]
        self.assertEqual(params["per_combo"], {"base": 5, "per_level": 0.5})
        self.assert_fails([{**STRIKE, "effect1_kind": "school_damage",
                            "effect1_params": "amount.base=5;amount.per_level=0;per_combo.base=5"}],
                          "lacks per_level")
        self.assert_fails([{**STRIKE, "effect1_kind": "school_damage",
                            "effect1_params": "amount.base=5;amount.per_level=0;per_combo=5"}],
                          "formula block")

    def test_talents_group_into_tabs_with_ranks_and_prereqs(self):
        rows = [talent("cruelty", 1, 0, 1), talent("cruelty", 2, 0, 1),
                talent("fury", 1, 1, 1, prereq_key="cruelty", prereq_rank="2")]
        done, data = self.run_build(rows)
        self.assertEqual(done.returncode, 0, done.stderr)
        tab = data[0]["talent_tabs"][0]
        self.assertEqual(tab["name"], "Arms")
        cruelty, fury = tab["talents"]
        self.assertEqual([r["rank"] for r in cruelty["ranks"]], [1, 2])
        self.assertEqual(fury["seat"], {"row": 1, "column": 1})
        self.assertEqual(fury["prereq"], {"key": "cruelty", "rank": 2})

    def test_unresolved_prereq_fails(self):
        self.assert_fails([talent("fury", 1, 1, 1, prereq_key="cruelty", prereq_rank="1")], "prereq")

    def test_prereq_rank_beyond_the_talent_fails(self):
        rows = [talent("cruelty", 1, 0, 1), talent("fury", 1, 1, 1, prereq_key="cruelty", prereq_rank="3")]
        self.assert_fails(rows, "has no rank 3")

    def test_shared_seat_fails(self):
        self.assert_fails([talent("cruelty", 1, 0, 1), talent("fury", 1, 0, 1)], "seat")

    def test_missing_seat_fails(self):
        self.assert_fails([talent("cruelty", 1, 0, 1, tab_row="")], "tab_row")

    def test_patch_carries_client_columns_and_only_effects_1_to_3(self):
        row = {**STRIKE, "client_cost": "10", "client_stances": "Battle Stance|Defensive Stance",
               "client_on_next_swing": "yes", "effect2_kind": "stun", "effect2_params": "duration_ms=500",
               "effect3_kind": "stun", "effect3_params": "duration_ms=700",
               "effect5_kind": "stun", "effect5_params": "duration_ms=2000"}
        done, data = self.run_build([row])
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(len(data[0]["skills"][0]["effects"]), 5)
        override = self.patch["spells"][0]
        self.assertEqual(override["spell_id"], 78)
        fields = override["fields"]
        self.assertEqual(fields["cost"], 10)
        self.assertEqual(fields["stances"], ["Battle Stance", "Defensive Stance"])
        self.assertTrue(fields["on_next_swing"])
        self.assertEqual([e["params"].get("duration_ms") for e in fields["effects"]], [None, 500, 700])
        self.assertNotIn("realm_cost", fields)

    def test_patch_emits_spellmods_and_seats_per_rank(self):
        mod = {"spellmod1_type": "flat", "spellmod1_op": "COST", "spellmod1_family_bit": "16"}
        rows = [talent("cruelty", 1, 0, 1, spell_id="12320", talent_id="138", spellmod1_value="-1", **mod),
                talent("cruelty", 2, 0, 1, spell_id="12852", talent_id="138", spellmod1_value="-2", **mod),
                talent("fury", 1, 1, 1, spell_id="12300", prereq_key="cruelty", prereq_rank="2")]
        done, _ = self.run_build(rows)
        self.assertEqual(done.returncode, 0, done.stderr)
        mods = {o["spell_id"]: o.get("spellmods") for o in self.patch["spells"]}
        self.assertEqual(mods[12320], [{"type": "flat", "op": "COST", "family_bit": 16, "value": -1}])
        self.assertEqual(mods[12852][0]["value"], -2)
        self.assertIsNone(mods[12300])
        cruelty, fury = self.patch["talents"]
        self.assertEqual(cruelty, {"calling": "vanguard", "key": "cruelty", "tab": "Arms", "row": 0, "column": 1,
                                   "talent_id": 138, "rank_spell_ids": [12320, 12852]})
        self.assertEqual(fury["prereq"], {"key": "cruelty", "rank": 2})

    def test_partial_spellmod_fails(self):
        self.assert_fails([{**STRIKE, "spellmod1_type": "pct"}], "spellmod1 needs")

    def test_fractional_id_fails(self):
        self.assert_fails([{**STRIKE, "spell_id": "78.0"}], "not a whole id")

    def test_dbc_check_without_data_fails(self):
        tmp = tempfile.mkdtemp()
        path = os.path.join(tmp, "master.csv")
        with open(path, "w", newline="", encoding="utf-8") as f:
            writer = csv.DictWriter(f, COLUMNS, restval="")
            writer.writeheader()
            writer.writerow(STRIKE)
        env = {k: v for k, v in os.environ.items() if k != "WOW_DATA"}
        done = subprocess.run([sys.executable, SCRIPT, "--csv", path, "--out-dir", tmp],
                              capture_output=True, text=True, env=env)
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("needs --data", done.stderr)
        self.assertFalse(os.path.exists(os.path.join(tmp, "server_spells.json")))

    def test_build_prints_rows_that_carry_assumptions(self):
        guessed = {**STRIKE, "key": "guess", "spell_id": "79", "assumptions": "damage is a placeholder"}
        done, _ = self.run_build([STRIKE, guessed])
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertIn("vanguard/guess", done.stdout)
        self.assertIn("damage is a placeholder", done.stdout)
        self.assertNotIn("vanguard/strike", done.stdout)

    def test_assumptions_never_reach_the_outputs(self):
        done, data = self.run_build([{**STRIKE, "assumptions": "secret guess"}])
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertNotIn("secret guess", json.dumps(data) + json.dumps(self.patch))

    def test_calling_row_carries_tier_stats_form_armour_and_class(self):
        done, data = self.run_build([CALLING, STRIKE])
        self.assertEqual(done.returncode, 0, done.stderr)
        calling = data[0]
        self.assertEqual(calling["name"], "Vanguard")
        self.assertEqual((calling["tier"], calling["tier_cap"]), (0, 10))
        self.assertEqual(calling["tier_bonus"], {"strength": 1, "stamina": 2})
        self.assertEqual(calling["shapeshift_form"], 17)
        self.assertEqual(calling["armour"], ["cloth", "mail"])
        self.assertEqual(calling["class"], 1)
        self.assertEqual(len(calling["skills"]), 1)

    def test_calling_row_needs_tier_and_cap(self):
        self.assert_fails([{**CALLING, "tier_cap": ""}], "tier_cap")
        self.assert_fails([{**CALLING, "tier": ""}], "tier")

    def test_calling_row_rejects_unknown_armour_and_stat(self):
        self.assert_fails([{**CALLING, "armour": "cloth|robe"}], "armour")
        self.assert_fails([{**CALLING, "tier_bonus": "luck=1"}], "tier_bonus")

    def test_second_calling_row_fails(self):
        self.assert_fails([CALLING, {**CALLING, "key": "again"}], "second calling row")

    def test_talent_tab_row_names_id_background_and_order(self):
        done, data = self.run_build([CALLING, TAB, talent("cruelty", 1, 0, 1, tab="Arms")])
        self.assertEqual(done.returncode, 0, done.stderr)
        tab = data[0]["talent_tabs"][0]
        self.assertEqual((tab["name"], tab["id"], tab["background"], tab["order"]), ("Arms", 900, "WarriorArms", 0))
        self.assertEqual(tab["talents"][0]["key"], "cruelty")

    def test_talent_in_an_undeclared_tab_fails_when_the_calling_declares_tabs(self):
        self.assert_fails([TAB, talent("cruelty", 1, 0, 1, tab="Fury")], "no talent_tab row")

    def test_duplicate_talent_tab_fails(self):
        self.assert_fails([TAB, {**TAB, "key": "arms2"}], "duplicate talent_tab")

    def test_talent_tab_needs_a_whole_id(self):
        self.assert_fails([{**TAB, "tab_id": "9.5"}], "tab_id")

    def test_aura_row_is_listed_with_duration_charges_and_effects(self):
        done, data = self.run_build([CALLING, AURA])
        self.assertEqual(done.returncode, 0, done.stderr)
        aura = data[0]["auras"][0]
        self.assertEqual((aura["key"], aura["spell_id"], aura["duration_ms"], aura["charges"]),
                         ("buff", 14177, 10000, 1))
        self.assertEqual(aura["modelled"], False)
        self.assertEqual(aura["effects"], [])
        self.assertEqual(self.patch["spells"], [])

    def test_modelled_aura_effects_are_checked(self):
        self.assert_fails([{**AURA, "modelled": "yes", "effect1_kind": "nonsense"}], "unknown effect kind")

    def test_aura_duration_must_be_a_number(self):
        self.assert_fails([{**AURA, "duration_ms": "long"}], "duration_ms")

    def test_proc_applies_auras_that_exist(self):
        proc = {**STRIKE, "key": "momentum", "effect1_kind": "proc", "effect2_kind": "", "effect4_kind": "",
                "effect4_params": "", "effect1_params": "on=cast;skill=strike;chance=100;apply_aura=buff"}
        done, data = self.run_build([CALLING, AURA, proc])
        self.assertEqual(done.returncode, 0, done.stderr)
        params = data[0]["skills"][0]["effects"][0]["params"]
        self.assertEqual((params["on"], params["chance"], params["apply_aura"]), ("cast", 100, "buff"))
        self.assert_fails([CALLING, AURA, {**proc, "effect1_params": "on=cast;apply_aura=buff|ghost"}], "ghost")

    def test_proc_needs_one_action_and_a_known_event(self):
        base = {**STRIKE, "effect1_kind": "proc", "effect4_kind": "", "effect4_params": ""}
        self.assert_fails([{**base, "effect1_params": "on=cast"}], "exactly one of")
        self.assert_fails([{**base, "effect1_params": "on=kill;heal_pct=10;apply_aura=buff"}], "exactly one of")
        self.assert_fails([{**base, "effect1_params": "on=sneeze;heal_pct=10"}], "proc on")

    def test_calling_row_with_only_a_name_and_class_gives_the_class_and_no_tier(self):
        bare = {"kind": "calling", "calling": "vanguard", "key": "vanguard", "name": "Vanguard",
                "class": "1"}
        done, data = self.run_build([bare, STRIKE])
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(data[0]["class"], 1)
        self.assertNotIn("tier", data[0])
        self.assertEqual(self.patch["spells"][0]["fields"]["power_type"], 1)
        self.assert_fails([{**bare, "tier": "1"}], "tier_cap")

    def test_patch_carries_the_description_only_when_the_row_has_one(self):
        done, _ = self.run_build([STRIKE, {**STRIKE, "key": "plain", "spell_id": "79", "description": ""}])
        self.assertEqual(done.returncode, 0, done.stderr)
        fields = {o["spell_id"]: o["fields"] for o in self.patch["spells"]}
        self.assertEqual(fields[78]["description"], "Hits.")
        self.assertNotIn("description", fields[79])

    def test_seats_are_whole_numbers(self):
        done, data = self.run_build([talent("cruelty", 1, 0, 1, tab_row="2", tab_column="3")])
        self.assertEqual(done.returncode, 0, done.stderr)
        seat = data[0]["talent_tabs"][0]["talents"][0]["seat"]
        self.assertEqual(json.dumps(seat), '{"row": 2, "column": 3}')
        self.assert_fails([talent("cruelty", 1, 0, 1, tab_row="1.0")], "not a whole number")

    def test_weapon_rolls_use_the_realms_names(self):
        self.assert_fails([{**STRIKE, "effect1_params": "weapon=Wn;bonus.base=1;bonus.per_level=1"}], "weapon is")

    def test_unavoidable_is_a_boolean(self):
        params = "weapon=normalized;unavoidable=dodge|parry;bonus.base=1;bonus.per_level=1"
        self.assert_fails([{**STRIKE, "effect1_params": params}], "unavoidable")
        done, data = self.run_build([{**STRIKE, "effect1_params": params.replace("dodge|parry", "yes")}])
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertIs(data[0]["skills"][0]["effects"][0]["params"]["unavoidable"], True)

    def test_duplicate_rank_fails(self):
        self.assert_fails([STRIKE, STRIKE], "duplicate")


class ResolveRows(unittest.TestCase):
    """The patch carries SpellRange, SpellCastTimes and SpellDuration row indexes, not values."""

    def setUp(self):
        sys.path.insert(0, os.path.join(HERE, ".."))
        import build_spells
        self.module = build_spells
        # (id, min, max), (id, base_ms, per_level_ms), (id, base_ms): the stock rows' shape.
        self.tables = {
            "ranges": [(2, 0.0, 5.0), (12, 0.0, 5.0), (3, 0.0, 20.0)],
            "cast_times": [(1, 0, 0), (191, 0, 0), (5, 1500, 0), (6, 1500, 50)],
            "durations": [(1, 10000), (21, 4000), (30, 4000)],
        }

    def resolved(self, **fields):
        patch = {"spells": [{"spell_id": 1, "fields": fields}], "talents": []}
        self.module.resolve_dbc_rows(patch, **self.tables)
        return patch["spells"][0]["fields"]

    def test_values_become_the_lowest_matching_row_index(self):
        fields = self.resolved(range_max=5.0, cast_time_ms=1500, duration_ms=4000, cost=15)
        self.assertEqual(fields, {"range_index": 2, "cast_time_index": 5, "duration_index": 21, "cost": 15})

    def test_a_missing_minimum_is_zero_and_an_instant_cast_is_row_one(self):
        fields = self.resolved(range_min=0, range_max=20, cast_time_ms=0)
        self.assertEqual((fields["range_index"], fields["cast_time_index"]), (3, 1))

    def test_no_matching_row_fails_naming_the_value(self):
        with self.assertRaisesRegex(self.module.BuildError, "range 3.0-7.0"):
            self.resolved(range_min=3.0, range_max=7.0)
        with self.assertRaisesRegex(self.module.BuildError, "cast time 1234 ms"):
            self.resolved(cast_time_ms=1234)
        with self.assertRaisesRegex(self.module.BuildError, "duration 999 ms"):
            self.resolved(duration_ms=999)


if __name__ == "__main__":
    unittest.main()
