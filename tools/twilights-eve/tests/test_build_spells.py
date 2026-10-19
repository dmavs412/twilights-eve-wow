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
    "client_spell_id": "78", "client_cost": "15", "realm_cost": "15", "target": "enemy",
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
AURA = {"kind": "aura", "calling": "vanguard", "class": "1", "key": "buff", "name": "Buff", "rank": "1", "client_spell_id": "14177",
        "duration_ms": "10000", "charges": "1", "effect1_kind": "skill_crit_pct",
        "effect1_params": "skill=strike;pct=50", "modelled": "no"}


class BuildSpells(unittest.TestCase):
    def run_build(self, rows: list[dict], flags: dict | None = None,
                  extra_columns: tuple = ()) -> tuple[subprocess.CompletedProcess, list | None]:
        tmp = tempfile.mkdtemp()
        path = os.path.join(tmp, "master.csv")
        # A fixture that leaves `status` or a talent's `max_rank` out gets the usual value; one that
        # sets the cell, even to "", keeps it, so a test can aim at the missing case.
        ranks: dict[tuple, int] = {}
        for row in rows:
            if row["kind"] == "talent":
                ranks[(row["calling"], row["key"])] = ranks.get((row["calling"], row["key"]), 0) + 1
        rows = [{"status": "placeholder",
                 **({"max_rank": str(ranks[(row["calling"], row["key"])])} if row["kind"] == "talent" else {}),
                 **row} for row in rows]
        with open(path, "w", newline="", encoding="utf-8") as f:
            writer = csv.DictWriter(f, [*COLUMNS, *extra_columns], restval="")
            writer.writeheader()
            writer.writerows(rows)
        command = [sys.executable, SCRIPT, "--csv", path, "--out-dir", tmp, "--no-dbc-check"]
        if flags is not None:
            flags_path = os.path.join(tmp, "flags.json")
            with open(flags_path, "w", encoding="utf-8") as f:
                json.dump(flags, f)
            command += ["--spell-flags", flags_path]
        done = subprocess.run(command, capture_output=True, text=True)
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
        self.assert_fails([STRIKE, {**STRIKE, "key": "stab", "class": "4", "client_spell_id": "79"}], "disagree on class")

    def test_a_missing_or_unknown_class_fails(self):
        self.assert_fails([{**STRIKE, "class": ""}], "class is required")
        self.assert_fails([{**STRIKE, "class": "6"}], "is not one of")
        self.assert_fails([{**STRIKE, "class": "1.5"}], "is not one of")

    def test_every_patch_row_carries_the_power_type_of_its_class(self):
        for cls, power_type in (("1", 1), ("4", 3), ("8", 0)):
            rows = [{**STRIKE, "class": cls}, talent("cruelty", 1, 0, 1, **{"class": cls, "client_spell_id": "12320"})]
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

    def test_aura_stacks_effects_name_an_aura_row_of_the_calling(self):
        aura = {**AURA, "key": "conviction", "effect1_kind": "", "effect1_params": "", "modelled": "yes",
                "max_stacks": "5"}
        add = {**STRIKE, "effect1_kind": "add_aura_stacks", "effect1_params": "aura=conviction;stacks=2"}
        done, data = self.run_build([CALLING, aura, add])
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(data[0]["skills"][0]["effects"][0]["params"], {"aura": "conviction", "stacks": 2})
        consume = {**STRIKE, "effect1_kind": "consume_aura_stacks",
                   "effect1_params": "aura=conviction;school=holy;per_stack.base=5;per_stack.per_level=1"}
        done, _ = self.run_build([CALLING, aura, consume])
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assert_fails([CALLING, aura, {**add, "effect1_params": "aura=nothing"}], "not an aura row")
        self.assert_fails([CALLING, aura, {**add, "effect1_params": "aura=conviction;stacks=0"}],
                          "positive whole stacks")
        self.assert_fails([CALLING, aura, {**consume, "effect1_params": "aura=conviction"}], "per_stack formula")

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
               "cast_kind": "next_swing", "effect2_kind": "stun", "effect2_params": "duration_ms=500",
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
        mod = {"spellmod1_type": "flat", "spellmod1_op": "COST", "spellmod1_target": "shout"}
        rows = [{**STRIKE, "key": "shout", "client_spell_id": "6192"},
                talent("cruelty", 1, 0, 1, client_spell_id="12320", talent_id="138", spellmod1_value="-1", **mod),
                talent("cruelty", 2, 0, 1, client_spell_id="12852", talent_id="138", spellmod1_value="-2", **mod),
                talent("fury", 1, 1, 1, client_spell_id="12300", prereq_key="cruelty", prereq_rank="2")]
        done, _ = self.run_build(rows, flags={"6192": 1 << 16})
        self.assertEqual(done.returncode, 0, done.stderr)
        mods = {o["spell_id"]: o.get("spellmods") for o in self.patch["spells"]}
        self.assertEqual(mods[12320], [{"type": "flat", "op": "COST", "family_bit": 16, "value": -1}])
        self.assertEqual(mods[12852][0]["value"], -2)
        self.assertIsNone(mods[12300])
        cruelty, fury = self.patch["talents"]
        self.assertEqual(cruelty, {"calling": "vanguard", "key": "cruelty", "tab": "Arms", "row": 0, "column": 1,
                                   "talent_id": 138, "rank_spell_ids": [12320, 12852]})
        self.assertEqual(fury["prereq"], {"key": "cruelty", "rank": 2})

    def borrowed(self, key: str, bit: int, spell_id: str) -> dict:
        return {**STRIKE, "key": key, "client_spell_id": spell_id, "borrowed_from_class": "2",
                "borrowed_family_bit": str(bit)}

    def mod_talent(self, target: str, **extra: str) -> dict:
        return talent("boon", 1, 0, 0, client_spell_id="12320", talent_id="138", spellmod1_type="pct",
                      spellmod1_op="DAMAGE", spellmod1_target=target, spellmod1_value="10", **extra)

    def test_spellmod_mask_is_built_from_skill_keys_for_both_outputs(self):
        rows = [self.borrowed("a", 33, "20375"), self.borrowed("b", 34, "20305"),
                {**STRIKE, "key": "c", "client_spell_id": "6546"}, self.mod_talent("a|c")]
        done, data = self.run_build(rows, flags={"6546": 1 << 5})
        self.assertEqual(done.returncode, 0, done.stderr)
        want = {"type": "pct", "op": "DAMAGE", "family_mask": (1 << 33) | (1 << 5), "value": 10}
        client = next(o for o in self.patch["spells"] if o["spell_id"] == 12320)
        self.assertEqual(client["spellmods"], [want])
        rank = data[0]["talent_tabs"][0]["talents"][0]["ranks"][0]
        self.assertEqual(rank["spellmods"], [{**want, "target": ["a", "c"]}])

    def test_a_single_skill_is_a_bit_index(self):
        done, _ = self.run_build([self.borrowed("a", 33, "20375"), self.mod_talent("a")], flags={})
        self.assertEqual(done.returncode, 0, done.stderr)
        client = next(o for o in self.patch["spells"] if o["spell_id"] == 12320)
        self.assertEqual(client["spellmods"][0]["family_bit"], 33)

    def test_a_mask_reaching_an_unnamed_skill_fails(self):
        shared = {**self.borrowed("b", 33, "20305")}
        self.assert_fails([self.borrowed("a", 33, "20375"), shared, self.mod_talent("a")],
                          "spellmod1_target 'a' also reaches 'b'")

    def test_spellbook_no_keeps_a_visible_skill_off_the_book(self):
        done, data = self.run_build([{**STRIKE, "spellbook": "no"}])
        self.assertEqual(done.returncode, 0, done.stderr)
        skill = data[0]["skills"][0]
        self.assertEqual((skill["spell_id"], skill["spellbook"]), (78, False))
        self.assertIn(78, [o["spell_id"] for o in self.patch["spells"]])

    def test_spellbook_on_a_talent_fails(self):
        self.assert_fails([talent("boon", 1, 0, 0, client_spell_id="12320", talent_id="138", spellbook="no")],
                          "spellbook is set on a talent row")

    def test_an_unknown_op_fails(self):
        row = {**self.mod_talent("a"), "spellmod1_op": "MAKE_BIGGER"}
        self.assert_fails([self.borrowed("a", 33, "20375"), row], "spellmod1_op is 'MAKE_BIGGER'")

    def test_an_unknown_target_key_fails(self):
        self.assert_fails([self.borrowed("a", 33, "20375"), self.mod_talent("zz")], "'zz' is not a skill")

    def test_a_stock_target_without_flags_fails(self):
        self.assert_fails([{**STRIKE, "key": "c", "client_spell_id": "6546"}, self.mod_talent("c")],
                          "needs --data or --spell-flags")

    def test_five_groups_are_read_and_a_sixth_column_fails(self):
        rows = [self.borrowed(k, b, str(i)) for i, (k, b) in enumerate(
            [("a", 33), ("b", 34), ("c", 35), ("d", 36), ("e", 37)], start=100)]
        groups = {}
        for n, key in enumerate("abcde", start=1):
            groups |= {f"spellmod{n}_type": "pct", f"spellmod{n}_op": "DAMAGE", f"spellmod{n}_target": key,
                       f"spellmod{n}_value": str(n)}
        done, _ = self.run_build(rows + [talent("boon", 1, 0, 0, client_spell_id="12320", **groups)], flags={})
        self.assertEqual(done.returncode, 0, done.stderr)
        client = next(o for o in self.patch["spells"] if o["spell_id"] == 12320)
        self.assertEqual([m["family_bit"] for m in client["spellmods"]], [33, 34, 35, 36, 37])
        done, _ = self.run_build([STRIKE], extra_columns=("spellmod6_type",))
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("spellmod6_type: a row has at most 5 spellmod groups", done.stderr)

    def test_partial_spellmod_fails(self):
        self.assert_fails([{**STRIKE, "spellmod1_type": "pct"}], "spellmod1 needs")

    def test_fractional_id_fails(self):
        self.assert_fails([{**STRIKE, "client_spell_id": "78.0"}], "not a whole id")

    def test_dbc_check_without_data_fails(self):
        tmp = tempfile.mkdtemp()
        path = os.path.join(tmp, "master.csv")
        with open(path, "w", newline="", encoding="utf-8") as f:
            writer = csv.DictWriter(f, COLUMNS, restval="")
            writer.writeheader()
            writer.writerow({**STRIKE, "status": "placeholder"})
        env = {k: v for k, v in os.environ.items() if k != "WOW_DATA"}
        done = subprocess.run([sys.executable, SCRIPT, "--csv", path, "--out-dir", tmp],
                              capture_output=True, text=True, env=env)
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("needs --data", done.stderr)
        self.assertFalse(os.path.exists(os.path.join(tmp, "server_spells.json")))

    def test_build_prints_rows_that_carry_assumptions(self):
        guessed = {**STRIKE, "key": "guess", "client_spell_id": "79", "assumptions": "damage is a placeholder"}
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
        done, data = self.run_build([CALLING, TAB, talent("cruelty", 1, 0, 1, tab="arms")])
        self.assertEqual(done.returncode, 0, done.stderr)
        tab = data[0]["talent_tabs"][0]
        self.assertEqual((tab["name"], tab["id"], tab["background"], tab["order"]), ("Arms", 900, "WarriorArms", 0))
        self.assertEqual(tab["talents"][0]["key"], "cruelty")

    def test_talent_in_an_undeclared_tab_fails_when_the_calling_declares_tabs(self):
        self.assert_fails([TAB, talent("cruelty", 1, 0, 1, tab="fury")], "not a talent_tab key")

    def test_a_tab_naming_a_display_name_fails(self):
        self.assert_fails([TAB, talent("cruelty", 1, 0, 1, tab="Arms")], "cruelty tab 'Arms' is not a talent_tab key")

    def test_a_calling_with_no_talent_tab_rows_keeps_the_bare_name(self):
        done, data = self.run_build([CALLING, talent("cruelty", 1, 0, 1, tab="Arms")])
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(data[0]["talent_tabs"][0]["name"], "Arms")

    def test_client_visible_no_emits_the_realm_id_and_nothing_to_the_patch(self):
        hidden = {"kind": "aura", "calling": "vanguard", "class": "1", "key": "bleed", "name": "Bleed", "rank": "1",
                  "client_visible": "no", "realm_spell_id": "1000001", "modelled": "no"}
        done, data = self.run_build([CALLING, hidden])
        self.assertEqual(done.returncode, 0, done.stderr)
        aura = data[0]["auras"][0]
        self.assertEqual((aura["spell_id"], aura["status"]), (1000001, "placeholder"))
        self.assertEqual(self.patch["spells"], [])

    def test_a_visible_row_emits_its_client_spell_id_as_spell_id(self):
        done, data = self.run_build([{**STRIKE, "client_visible": "yes"}])
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(data[0]["skills"][0]["spell_id"], 78)
        self.assertEqual(self.patch["spells"][0]["spell_id"], 78)

    def hidden(self, **extra: str) -> dict:
        return {"kind": "skill", "calling": "vanguard", "class": "1", "key": "bleed", "name": "Bleed", "rank": "1",
                "client_visible": "no", "realm_spell_id": "1000001", "modelled": "no", **extra}

    def test_a_client_cell_on_a_hidden_row_fails(self):
        self.assert_fails([self.hidden(client_cost="5")], "bleed): a client_visible=no row sets client_cost")

    def test_realm_spell_id_below_range_fails(self):
        self.assert_fails([self.hidden(realm_spell_id="999999")], "realm_spell_id 999999 is below")

    def test_a_hidden_row_needs_a_realm_spell_id(self):
        self.assert_fails([self.hidden(realm_spell_id="")], "needs realm_spell_id")

    def test_duplicate_realm_spell_id_fails(self):
        self.assert_fails([self.hidden(), self.hidden(key="bleed2")], "realm_spell_id 1000001 is already used")

    def test_two_skills_on_one_client_spell_id_fail(self):
        self.assert_fails([STRIKE, {**STRIKE, "key": "smash", "name": "Smash"}], "client_spell_id 78 is already claimed")

    def test_two_callings_on_one_client_spell_id_fail(self):
        self.assert_fails([STRIKE, {**STRIKE, "calling": "zealot"}], "client_spell_id 78 is already claimed")

    def test_an_aura_may_share_its_callings_client_spell_id(self):
        done, _ = self.run_build([STRIKE, {**AURA, "client_spell_id": "78"}])
        self.assertEqual(done.returncode, 0, done.stderr)

    def test_two_talents_on_one_talent_id_fail(self):
        rows = [talent("a", 1, 0, 0, talent_id="27"), talent("b", 1, 0, 1, talent_id="27")]
        self.assert_fails([CALLING, TAB, *rows], "talent_id 27 is already claimed")

    def test_realm_spell_id_on_a_visible_row_fails(self):
        self.assert_fails([{**STRIKE, "realm_spell_id": "1000001"}], "realm_spell_id is set on a row that is visible")

    def test_max_rank_is_required_on_a_talent(self):
        self.assert_fails([talent("cruelty", 1, 0, 1, max_rank="")], "cruelty): a talent rank row needs max_rank")

    def test_max_rank_above_five_fails(self):
        self.assert_fails([talent("cruelty", 1, 0, 1, max_rank="6")], "max_rank 6 is not 1 to 5")

    def test_max_rank_mismatch_fails(self):
        self.assert_fails([talent("cruelty", 1, 0, 1, max_rank="2"), talent("cruelty", 2, 0, 1, max_rank="3")],
                          "disagree on max_rank")

    def test_a_rank_gap_fails(self):
        self.assert_fails([talent("cruelty", 1, 0, 1, max_rank="3"), talent("cruelty", 3, 0, 1, max_rank="3")],
                          "ranks are not 1..max_rank")

    def test_a_skill_leaves_max_rank_empty(self):
        self.assert_fails([{**STRIKE, "max_rank": "1"}], "only talent ranks carry it")

    def test_a_missing_or_unknown_status_fails(self):
        self.assert_fails([{**STRIKE, "status": ""}], "status is ''")
        self.assert_fails([{**STRIKE, "status": "finished"}], "status is 'finished'")

    def test_status_counts_and_wip_rows_are_printed(self):
        done, _ = self.run_build([STRIKE, {**STRIKE, "key": "other", "client_spell_id": "79", "status": "wip"}])
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertIn("status: 1 placeholder, 1 wip, 0 done", done.stdout)
        self.assertIn("wip: vanguard/other", done.stdout)

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
        self.assert_fails([{**base, "effect1_params": "on=cast"}], "at least one of")
        self.assert_fails([{**base, "effect1_params": "on=kill;heal_pct=10;apply_aura=buff"}], "not both")
        self.assert_fails([{**base, "effect1_params": "on=sneeze;heal_pct=10"}], "proc on")
        for on in ("dodge", "finisher"):
            params = f"on={on};chance=50;energize=10;add_combo_points=1"
            done, _ = self.run_build([CALLING, {**base, "effect1_params": params}])
            self.assertEqual(done.returncode, 0, done.stderr)

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
        done, _ = self.run_build([STRIKE, {**STRIKE, "key": "plain", "client_spell_id": "79", "description": ""}])
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

    def test_the_unavoidable_param_is_replaced_by_outcomes(self):
        params = "weapon=normalized;bonus.base=1;bonus.per_level=1"
        self.assert_fails([{**STRIKE, "effect1_params": params + ";unavoidable=yes"}],
                          "vanguard/strike) effect1: the unavoidable param is gone")
        # dodge=no and parry=no together still reach the realm as the effect's unavoidable flag
        done, data = self.run_build([{**STRIKE, "effect1_params": params, "outcomes": "dodge=no;parry=no"}])
        self.assertEqual(done.returncode, 0, done.stderr)
        skill = data[0]["skills"][0]
        self.assertIs(skill["effects"][0]["params"]["unavoidable"], True)
        self.assertEqual(list(skill["effects"][0]["params"])[:2], ["weapon", "unavoidable"])
        self.assertEqual(skill["outcomes"], {"dodge": False, "parry": False})
        self.assertIs(self.patch["spells"][0]["fields"]["effects"][0]["params"]["unavoidable"], True)
        skill = self.built_skill(effect1_params=params, outcomes="crit=no")
        self.assertNotIn("unavoidable", skill["effects"][0]["params"])

    def test_cost_kind_and_its_patch_fields(self):
        self.assertEqual(self.built_skill()["cost_kind"], "flat")
        self.assertNotIn("cost_extra_max", self.built_skill())
        skill = self.built_skill(cost_kind="all_remaining", cost_extra_max="30")
        self.assertEqual((skill["cost_kind"], skill["cost_extra_max"]), ("all_remaining", 30))
        fields = self.patch["spells"][0]["fields"]
        self.assertEqual(fields["cost"], 15)
        self.built_skill(cost_kind="pct_base", client_cost="5")
        fields = self.patch["spells"][0]["fields"]
        self.assertEqual((fields["cost_percent"], "cost" in fields), (5, False))
        self.built_skill(cost_kind="per_sec", client_cost="8")
        self.assertEqual(self.patch["spells"][0]["fields"]["cost_per_second"], 8)
        self.built_skill(cost_kind="health")
        self.assertEqual(self.patch["spells"][0]["fields"]["power_type"], 0xFFFFFFFE)

    def test_cost_kind_failures(self):
        self.assert_fails([{**STRIKE, "cost_kind": "mana"}], "vanguard/strike) cost_kind: 'mana' is not one of")
        self.assert_fails([{**STRIKE, "cost_extra_max": "30"}], "vanguard/strike) cost_extra_max: only with cost_kind=all_remaining")
        self.assert_fails([{**STRIKE, "cost_kind": "all_remaining", "cost_extra_max": "0"}],
                          "cost_extra_max: must be above 0")

    def test_cast_kind_and_its_patch_bits(self):
        self.assertEqual(self.built_skill()["cast_kind"], "instant")
        self.built_skill(cast_kind="next_swing")
        self.assertIs(self.patch["spells"][0]["fields"]["on_next_swing"], True)
        self.built_skill(cast_kind="auto_repeat")
        self.assertIs(self.patch["spells"][0]["fields"]["auto_repeat"], True)
        skill = self.built_skill(cast_kind="cast", client_cast_time_ms="2000")
        self.assertEqual(skill["cast_kind"], "cast")
        skill = self.built_skill(cast_kind="channel", client_duration_ms="8000", channel_tick_ms="1000")
        self.assertEqual((skill["cast_kind"], skill["channel_tick_ms"]), ("channel", 1000))
        self.assertIs(self.patch["spells"][0]["fields"]["channeled"], True)
        self.assertNotIn("on_next_swing", self.patch["spells"][0]["fields"])

    def test_cast_kind_failures(self):
        self.assert_fails([{**STRIKE, "cast_kind": "sprint"}], "vanguard/strike) cast_kind: 'sprint' is not one of")
        self.assert_fails([{**STRIKE, "cast_kind": "cast"}], "vanguard/strike) cast_kind: cast needs a client_cast_time_ms")
        self.assert_fails([{**STRIKE, "cast_kind": "cast", "client_cast_time_ms": "0"}], "cast needs a client_cast_time_ms")
        self.assert_fails([{**STRIKE, "cast_kind": "channel", "channel_tick_ms": "1000"}], "channel needs a client_duration_ms")
        self.assert_fails([{**STRIKE, "cast_kind": "channel", "client_duration_ms": "5000"}], "channel needs channel_tick_ms")
        self.assert_fails([{**STRIKE, "channel_tick_ms": "1000"}], "vanguard/strike) channel_tick_ms: only with cast_kind=channel")
        self.assert_fails([{**STRIKE, "cast_kind": "channel", "client_duration_ms": "5000", "channel_tick_ms": "0"}],
                          "channel_tick_ms: must be above 0")
        done, _ = self.run_build([STRIKE], extra_columns=("client_on_next_swing",))
        self.assertIn("client_on_next_swing is gone", done.stderr)

    def test_interrupt_on_defaults_and_overrides(self):
        self.assertEqual(self.built_skill()["interrupt_on"], [])
        self.assertEqual(self.built_skill(cast_kind="next_swing")["interrupt_on"], [])
        cast = {"cast_kind": "cast", "client_cast_time_ms": "2000"}
        self.assertEqual(self.built_skill(**cast)["interrupt_on"], ["move", "pushback", "kick"])
        self.assertNotIn("interrupt_flags", self.patch["spells"][0]["fields"])
        channel = {"cast_kind": "channel", "client_duration_ms": "5000", "channel_tick_ms": "1000"}
        self.assertEqual(self.built_skill(**channel)["interrupt_on"], ["move", "pushback", "kick"])
        self.assertEqual(self.built_skill(**cast, interrupt_on="none")["interrupt_on"], [])
        self.assertEqual(self.built_skill(**cast, interrupt_on="damage|kick")["interrupt_on"], ["damage", "kick"])
        self.assertEqual(self.patch["spells"][0]["fields"]["interrupt_flags"], 0x10 | 0x08)
        self.built_skill(**channel, interrupt_on="move")
        self.assertEqual(self.patch["spells"][0]["fields"]["channel_interrupt_flags"], 0x01)
        self.assert_fails([{**STRIKE, "interrupt_on": "sneeze"}], "vanguard/strike) interrupt_on: 'sneeze' is not one of")
        self.assert_fails([{**STRIKE, "interrupt_on": "none|move"}], "vanguard/strike) interrupt_on:")
        self.assert_fails([{**STRIKE, "interrupt_on": "move|move"}], "vanguard/strike) interrupt_on:")

    def test_defense_type_defaults_and_overrides(self):
        self.assertEqual(self.built_skill()["defense_type"], "melee")
        self.assertEqual(self.built_skill(school="fire")["defense_type"], "magic")
        self.assertEqual(self.built_skill(target="self")["defense_type"], "none")
        self.assertEqual(self.built_skill(target="friend", school="holy")["defense_type"], "none")
        self.assertNotIn("dmg_class", self.patch["spells"][0]["fields"])
        self.assertEqual(self.built_skill(defense_type="ranged")["defense_type"], "ranged")
        self.assertEqual(self.patch["spells"][0]["fields"]["dmg_class"], 3)
        self.assert_fails([{**STRIKE, "defense_type": "psychic"}], "vanguard/strike) defense_type: 'psychic' is not one of")

    def test_on_cast_defaults_and_overrides(self):
        self.assertEqual(self.built_skill()["on_cast"],
                         {"breaks_stealth": True, "starts_auto_attack": True, "resets_swing": False})
        self.assertEqual(self.built_skill(target="self")["on_cast"]["starts_auto_attack"], False)
        self.assertEqual(self.built_skill(school="fire")["on_cast"]["starts_auto_attack"], False)
        self.assertEqual(self.built_skill(defense_type="ranged")["on_cast"]["starts_auto_attack"], False)
        self.assertEqual(self.built_skill(on_cast="breaks_stealth=no;resets_swing=yes")["on_cast"],
                         {"breaks_stealth": False, "starts_auto_attack": True, "resets_swing": True})
        for cell, message in (("breaks_stealth=maybe", "breaks_stealth is 'maybe'"), ("hides=yes", "unknown key 'hides'"),
                              ("resets_swing", "malformed pair"), ("resets_swing=yes;resets_swing=no", "'resets_swing' given twice")):
            self.assert_fails([{**STRIKE, "on_cast": cell}], f"vanguard/strike) on_cast: {message}")

    def test_outcomes_cell(self):
        self.assertNotIn("outcomes", self.built_skill())
        self.assertEqual(self.built_skill(outcomes="dodge=no;crit=no")["outcomes"], {"dodge": False, "crit": False})
        for cell, message in (("glance=no", "unknown key 'glance'"), ("dodge=never", "dodge is 'never'"),
                              ("dodge", "malformed pair"), ("dodge=no;dodge=no", "'dodge' given twice")):
            self.assert_fails([{**STRIKE, "outcomes": cell}], f"vanguard/strike) outcomes: {message}")

    def test_weapon_hand_default_and_failure(self):
        self.assertEqual(self.built_skill()["weapon_hand"], "main")
        self.assertEqual(self.built_skill(weapon_hand="both")["weapon_hand"], "both")
        self.assert_fails([{**STRIKE, "weapon_hand": "left"}], "vanguard/strike) weapon_hand: 'left' is not one of")

    def test_area_columns_and_max_targets_default(self):
        skill = self.built_skill()
        self.assertEqual(skill["max_targets"], 1)
        self.assertNotIn("area_shape", skill)
        skill = self.built_skill(realm_radius_yd="8")
        self.assertIsNone(skill["max_targets"])
        skill = self.built_skill(area_shape="cone", area_center="caster", max_targets="3")
        self.assertEqual((skill["area_shape"], skill["area_center"], skill["max_targets"]), ("cone", "caster", 3))
        self.assertEqual(self.built_skill(realm_radius_yd="8", max_targets="2")["max_targets"], 2)
        self.assertEqual(self.built_skill(max_targets="1")["max_targets"], 1)
        self.assert_fails([{**STRIKE, "area_shape": "square"}], "vanguard/strike) area_shape: 'square' is not one of")
        self.assert_fails([{**STRIKE, "area_shape": "circle", "area_center": "moon"}], "area_center: 'moon' is not one of")
        self.assert_fails([{**STRIKE, "area_center": "caster"}], "vanguard/strike) area_center: needs area_shape")
        self.assert_fails([{**STRIKE, "max_targets": "3"}], "vanguard/strike) max_targets: 3 needs area_shape or realm_radius_yd")
        self.assert_fails([{**STRIKE, "max_targets": "0"}], "max_targets: must be above 0")

    def test_shape_columns_fail_on_an_aura_row(self):
        for column, value in (("cost_kind", "health"), ("cast_kind", "channel"), ("outcomes", "crit=no"),
                              ("weapon_hand", "off"), ("max_targets", "2")):
            self.assert_fails([{**AURA, column: value}], f"vanguard/buff) {column}: set on an aura row")

    def test_a_hidden_cast_row_needs_no_client_cast_time(self):
        done, data = self.run_build([{**self.hidden(), "cast_kind": "cast"}])
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(data[0]["skills"][0]["cast_kind"], "cast")

    def built_skill(self, **cells: str) -> dict:
        done, data = self.run_build([{**STRIKE, **cells}])
        self.assertEqual(done.returncode, 0, done.stderr)
        return data[0]["skills"][0]

    def test_requires_weapon_maps_to_item_class_and_masks(self):
        skill = self.built_skill(requires_weapon="dagger")
        self.assertEqual(skill["requires_weapon"], ["dagger"])
        fields = self.patch["spells"][0]["fields"]
        self.assertEqual(fields["equipped_item_class"], 2)
        self.assertEqual(fields["equipped_subclass_mask"], 1 << 15)
        self.assertEqual(fields["equipped_inventory_mask"], (1 << 13) | (1 << 21) | (1 << 22))
        self.built_skill(requires_weapon="shield")
        fields = self.patch["spells"][0]["fields"]
        self.assertEqual((fields["equipped_item_class"], fields["equipped_subclass_mask"],
                          fields["equipped_inventory_mask"]), (4, 1 << 6, 1 << 14))
        self.built_skill(requires_weapon="any_ranged")
        self.assertEqual(self.patch["spells"][0]["fields"]["equipped_subclass_mask"],
                         (1 << 2) | (1 << 3) | (1 << 16) | (1 << 18) | (1 << 19))
        self.built_skill(requires_weapon="one_hand|two_hand")
        self.assertEqual(self.patch["spells"][0]["fields"]["equipped_inventory_mask"],
                         (1 << 13) | (1 << 17) | (1 << 21) | (1 << 22))
        self.built_skill()
        self.assertNotIn("equipped_item_class", self.patch["spells"][0]["fields"])

    def test_requires_weapon_failures(self):
        self.assert_fails([{**STRIKE, "requires_weapon": "club"}], "vanguard/strike) requires_weapon: 'club'")
        self.assert_fails([{**STRIKE, "requires_weapon": "shield|dagger"}], "requires_weapon: shield")

    def test_cooldown_category_numbers_names_and_shares_the_cooldown(self):
        other = {**STRIKE, "key": "other", "client_spell_id": "79", "cooldown_category": "poison",
                 "category_cooldown_ms": "6000"}
        done, data = self.run_build([{**STRIKE, "cooldown_category": "poison", "category_cooldown_ms": "6000"},
                                     other, {**STRIKE, "key": "third", "client_spell_id": "80",
                                             "cooldown_category": "blind", "category_cooldown_ms": "9000"}])
        self.assertEqual(done.returncode, 0, done.stderr)
        first, second, third = (s["fields"] for s in self.patch["spells"])
        self.assertEqual(first["category"], second["category"])
        self.assertNotEqual(first["category"], third["category"])
        self.assertGreaterEqual(first["category"], 10_000)
        self.assertEqual(first["category_cooldown_ms"], 6000)
        self.assertEqual(data[0]["skills"][0]["cooldown_category"], "poison")

    def test_cooldown_category_failures(self):
        self.assert_fails([{**STRIKE, "cooldown_category": "poison"}], "vanguard/strike) cooldown_category")
        self.assert_fails([{**STRIKE, "category_cooldown_ms": "6000"}], "vanguard/strike) cooldown_category")
        self.assert_fails([{**STRIKE, "cooldown_category": "p", "category_cooldown_ms": "0"}], "category_cooldown_ms")
        self.assert_fails([{**STRIKE, "cooldown_category": "p", "category_cooldown_ms": "6000"},
                           {**STRIKE, "key": "b", "client_spell_id": "79", "cooldown_category": "p",
                            "category_cooldown_ms": "7000"}],
                          "vanguard/b) category_cooldown_ms")

    def test_school_and_prevention_defaults(self):
        skill = self.built_skill()
        self.assertEqual((skill["school"], skill["prevention"]), ("physical", "pacify"))
        self.assertEqual(self.patch["spells"][0]["fields"]["school"], 0)
        self.assertEqual(self.patch["spells"][0]["fields"]["prevention_type"], 2)
        skill = self.built_skill(school="fire")
        self.assertEqual((skill["school"], skill["prevention"]), ("fire", "silence"))
        self.assertEqual(self.patch["spells"][0]["fields"]["school"], 2)
        self.assertEqual(self.patch["spells"][0]["fields"]["prevention_type"], 1)
        skill = self.built_skill(school="frost", prevention="none")
        self.assertEqual(skill["prevention"], "none")
        skill = self.built_skill(prevention="silence")
        self.assertEqual(skill["prevention"], "silence")

    def test_school_and_prevention_failures(self):
        self.assert_fails([{**STRIKE, "school": "light"}], "vanguard/strike) school: 'light'")
        self.assert_fails([{**STRIKE, "prevention": "stun"}], "vanguard/strike) prevention: 'stun'")

    def test_school_damage_takes_the_rows_school(self):
        params = "amount.base=5;amount.per_level=0"
        skill = self.built_skill(effect1_kind="school_damage", effect1_params=params, school="fire")
        self.assertEqual(skill["effects"][0]["params"]["school"], "fire")
        skill = self.built_skill(effect1_kind="school_damage", effect1_params=params)
        self.assertNotIn("school", skill["effects"][0]["params"])
        self.assert_fails([{**STRIKE, "effect1_kind": "school_damage",
                            "effect1_params": "school=fire;" + params}], "school column")

    def test_aura_states_map_to_numbers(self):
        skill = self.built_skill(requires_target_state="health_below_20", requires_caster_state="defense")
        self.assertEqual(skill["requires_target_state"], "health_below_20")
        fields = self.patch["spells"][0]["fields"]
        self.assertEqual((fields["target_aura_state"], fields["caster_aura_state"]), (2, 1))
        self.assert_fails([{**STRIKE, "requires_target_state": "bleeding"}], "vanguard/strike) requires_target_state")
        self.assert_fails([{**STRIKE, "requires_caster_state": "stealth"}], "vanguard/strike) requires_caster_state")

    def test_every_requires_key_passes(self):
        for cell, expected in (
            ("position=behind", {"position": "behind"}), ("position=front", {"position": "front"}),
            ("facing=no", {"facing": False}), ("stealth=yes", {"stealth": True}),
            ("combat=in", {"combat": "in"}), ("target_state=bleeding", {"target_state": "bleeding"}),
            ("caster_state=after_dodge;window_ms=5000", {"caster_state": "after_dodge", "window_ms": 5000}),
            ("offhand=weapon", {"offhand": "weapon"}), ("los=no", {"los": False}),
            ("usable_while=stunned|feared", {"usable_while": ["stunned", "feared"]}),
            ("position=behind;stealth=yes", {"position": "behind", "stealth": True}),
        ):
            self.assertEqual(self.built_skill(requires=cell)["requires"], expected, cell)

    def test_requires_failures(self):
        for cell, message in (
            ("position=above", "position is 'above'"), ("facing=maybe", "facing is 'maybe'"),
            ("stealth=no", "stealth is 'no'"), ("combat=always", "combat is 'always'"),
            ("target_state=asleep", "target_state is 'asleep'"),
            ("caster_state=after_miss;window_ms=5", "caster_state is 'after_miss'"),
            ("caster_state=after_crit", "go together"), ("window_ms=500", "go together"),
            ("caster_state=after_crit;window_ms=0", "window_ms must be above 0"),
            ("offhand=shield", "offhand is 'shield'"), ("los=sometimes", "los is 'sometimes'"),
            ("usable_while=asleep", "usable_while 'asleep'"), ("colour=red", "unknown key 'colour'"),
            ("position", "malformed pair"), ("combat=in;combat=out", "given twice"),
        ):
            self.assert_fails([{**STRIKE, "requires": cell}], message)

    def test_combat_out_sets_the_realm_flag(self):
        skill = self.built_skill(requires="combat=out")
        self.assertTrue(skill["requires_out_of_combat"])
        self.assertEqual(skill["requires"], {"combat": "out"})
        self.assertNotIn("requires_out_of_combat", self.built_skill(requires="combat=in"))

    def test_client_not_in_combat_needs_combat_out(self):
        self.built_skill(client_not_in_combat="yes", requires="combat=out")
        self.assertTrue(self.patch["spells"][0]["fields"]["not_in_combat"])
        self.assert_fails([{**STRIKE, "client_not_in_combat": "yes"}],
                          "vanguard/strike) client_not_in_combat: yes needs requires: combat=out")
        self.assert_fails([{**STRIKE, "client_not_in_combat": "yes", "requires": "combat=in"}],
                          "client_not_in_combat")

    def test_gate_columns_fail_on_an_aura_row(self):
        self.assert_fails([{**AURA, "requires": "stealth=yes"}], "vanguard/buff) requires: set on an aura row")
        done, data = self.run_build([{**AURA, "school": "nature"}])
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(data[0]["auras"][0]["school"], "nature")

    def aura(self, **cells: str) -> dict:
        """The aura the first Calling of a build carries, built from AURA plus `cells`."""
        done, data = self.run_build([{**AURA, **cells}])
        self.assertEqual(done.returncode, 0, done.stderr)
        return data[0]["auras"][0]

    def test_aura_only_columns_default(self):
        aura = self.aura()
        self.assertEqual(
            {k: aura[k] for k in ("max_stacks", "stack_rule", "unique_per", "group_scope", "dispel_type",
                                  "aura_positive", "aura_hidden", "cancelable", "persist_death")},
            {"max_stacks": 1, "stack_rule": "refresh", "unique_per": "caster", "group_scope": "caster",
             "dispel_type": "none", "aura_positive": True, "aura_hidden": False, "cancelable": True,
             "persist_death": False})
        for absent in ("exclusive_group", "mechanic", "aura_description", "break_on", "grants_form"):
            self.assertNotIn(absent, aura)

    def test_an_enemy_target_makes_a_debuff_that_cannot_be_cancelled(self):
        aura = self.aura(target="enemy")
        self.assertEqual((aura["target"], aura["aura_positive"], aura["cancelable"]), ("enemy", False, False))
        aura = self.aura(target="enemy", aura_positive="yes", cancelable="yes")
        self.assertEqual((aura["aura_positive"], aura["cancelable"]), (True, True))
        aura = self.aura(aura_positive="no")
        self.assertEqual((aura["aura_positive"], aura["cancelable"]), (False, False))

    def test_aura_only_columns_are_read(self):
        aura = self.aura(max_stacks="5", stack_rule="replace", unique_per="target", exclusive_group="seal",
                         group_scope="target", dispel_type="magic", mechanic="snare", aura_hidden="yes",
                         persist_death="yes", aura_description="Slowed by $s1%.", grants_form="30")
        self.assertEqual((aura["max_stacks"], aura["stack_rule"], aura["unique_per"]), (5, "replace", "target"))
        self.assertEqual((aura["exclusive_group"], aura["group_scope"]), ("seal", "target"))
        self.assertEqual((aura["dispel_type"], aura["mechanic"]), ("magic", "snare"))
        self.assertEqual((aura["aura_hidden"], aura["persist_death"]), (True, True))
        self.assertEqual((aura["aura_description"], aura["grants_form"]), ("Slowed by $s1%.", 30))

    def test_break_on_is_read(self):
        self.assertEqual(self.aura(break_on="damage=yes")["break_on"], {"damage": True})
        self.assertEqual(self.aura(break_on="damage_threshold=30;move=yes")["break_on"],
                         {"damage_threshold": 30, "move": True})
        self.assertEqual(self.aura(break_on="attack=yes;stealth_break=yes;leave_combat=yes")["break_on"],
                         {"attack": True, "stealth_break": True, "leave_combat": True})

    def test_aura_column_failures_name_the_row_and_the_column(self):
        for column, value in (("max_stacks", "0"), ("max_stacks", "1.5"), ("stack_rule", "pile"),
                              ("unique_per", "world"), ("group_scope", "world"), ("dispel_type", "holy"),
                              ("mechanic", "dizzy"), ("aura_positive", "maybe"), ("aura_hidden", "1"),
                              ("cancelable", "sure"), ("persist_death", "true"), ("grants_form", "0"),
                              ("grants_form", "stealth"), ("break_on", "damage"), ("break_on", "damage=no"),
                              ("break_on", "sneeze=yes"), ("break_on", "damage=yes;damage=yes"),
                              ("break_on", "damage=yes;damage_threshold=5"), ("break_on", "damage_threshold=0")):
            with self.subTest(column=column, value=value):
                extra = {"exclusive_group": "g"} if column == "group_scope" else {}
                done, data = self.run_build([{**AURA, **extra, column: value}])
                self.assertNotEqual(done.returncode, 0)
                self.assertRegex(done.stderr, rf"vanguard/buff\) {column}|vanguard/buff\) {column} ")
                self.assertIsNone(data)

    def test_a_group_scope_needs_an_exclusive_group(self):
        self.assert_fails([{**AURA, "group_scope": "target"}], "vanguard/buff) group_scope: needs exclusive_group")

    def test_aura_only_columns_fail_on_other_rows(self):
        values = {"max_stacks": "2", "stack_rule": "replace", "unique_per": "target", "exclusive_group": "g",
                  "group_scope": "caster", "dispel_type": "magic", "mechanic": "stun", "aura_positive": "yes",
                  "aura_hidden": "no", "cancelable": "yes", "persist_death": "no", "aura_description": "x",
                  "break_on": "damage=yes", "grants_form": "30"}
        for column, value in values.items():
            for row, kind in ((STRIKE, "skill"), (talent("rage", 1, 0, 0), "talent"), (CALLING, "calling"),
                              (TAB, "talent_tab")):
                with self.subTest(column=column, kind=kind):
                    done, data = self.run_build([{**row, column: value}])
                    self.assertNotEqual(done.returncode, 0)
                    self.assertIn(f"{column}: set on a {kind} row; only aura rows carry it", done.stderr)
                    self.assertIsNone(data)

    def test_aura_dbc_fields_reach_the_patch(self):
        done, _ = self.run_build([CALLING, {**AURA, "dispel_type": "poison", "mechanic": "stun",
                                            "aura_description": "Poisoned."}])
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(self.patch["spells"], [{
            "spell_id": 14177, "calling": "vanguard", "key": "buff", "rank": 1,
            "fields": {"dispel_type": 4, "mechanic": 12, "aura_description": "Poisoned."}}])

    def test_an_aura_that_sets_none_of_them_adds_no_override(self):
        done, _ = self.run_build([CALLING, {**AURA, "max_stacks": "3"}])
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(self.patch["spells"], [])

    def test_an_aura_on_a_skills_spell_joins_the_skills_override(self):
        aura = {**AURA, "client_spell_id": STRIKE["client_spell_id"], "mechanic": "incapacitate"}
        for rows in ([CALLING, STRIKE, aura], [CALLING, aura, STRIKE]):
            done, _ = self.run_build(rows)
            self.assertEqual(done.returncode, 0, done.stderr)
            self.assertEqual(len(self.patch["spells"]), 1)
            entry = self.patch["spells"][0]
            self.assertEqual((entry["key"], entry["fields"]["mechanic"], entry["fields"]["cost"]), ("strike", 14, 15))

    def test_two_auras_disagreeing_on_one_spell_fail(self):
        other = {**AURA, "key": "other", "mechanic": "snare"}
        self.assert_fails([CALLING, {**AURA, "mechanic": "stun"}, other], "(vanguard/other): mechanic 11 differs from 12")

    def test_a_hidden_aura_adds_no_override(self):
        hidden = {**AURA, "client_visible": "no", "realm_spell_id": "1000001", "client_spell_id": "", "duration_ms": "",
                  "mechanic": "stun"}
        done, data = self.run_build([CALLING, hidden])
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(self.patch["spells"], [])
        self.assertEqual(data[0]["auras"][0]["mechanic"], "stun")

    def test_apply_aura_names_an_aura_with_an_effect_of_the_kind(self):
        sleep = {**AURA, "key": "sleep", "effect1_kind": "incapacitate", "effect1_params": "drop_combat=yes",
                 "modelled": "yes"}
        hit = {**STRIKE, "effect4_kind": "", "effect4_params": "", "effect2_kind": "incapacitate",
               "effect2_params": "apply_aura=sleep"}
        done, data = self.run_build([CALLING, sleep, hit])
        self.assertEqual(done.returncode, 0, done.stderr)
        effect = data[0]["skills"][0]["effects"][1]
        self.assertEqual((effect["kind"], effect["params"]), ("incapacitate", {"apply_aura": "sleep"}))
        self.assert_fails([CALLING, sleep, {**hit, "effect2_params": "apply_aura=ghost"}],
                          "(vanguard/strike) effect2: apply_aura 'ghost' is not an aura row")
        self.assert_fails([CALLING, sleep, {**hit, "effect2_kind": "stun"}],
                          "(vanguard/strike) effect2: aura 'sleep' has no stun effect")
        self.assert_fails([CALLING, sleep, {**hit, "effect2_params": "apply_aura=sleep|buff"}],
                          "apply_aura names one aura key")

    def test_the_old_combat_column_is_refused(self):
        done, _ = self.run_build([STRIKE], extra_columns=("requires_out_of_combat",))
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("requires_out_of_combat is gone", done.stderr)

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


class PointBlank(unittest.TestCase):
    """Point Blank's ranks carry the Twilight MIN_RANGE cell (ADR 0010) on every ranged skill of
    the Ranger the client can see; a hidden skill has no client row to modify."""

    def test_every_rank_sends_min_range_on_every_ranged_skill(self):
        with open(MASTER, encoding="utf-8", newline="") as f:
            rows = list(csv.DictReader(f))
        ranged = {r["key"] for r in rows if r["calling"] == "ranger" and r["kind"] == "skill"
                  and r["defense_type"] == "ranged" and r["client_visible"] != "no"}
        ranks = [r for r in rows if r["calling"] == "ranger" and r["key"] == "point_blank"]
        self.assertEqual(len(ranks), 2)
        for r in ranks:
            self.assertEqual((r["spellmod1_type"], r["spellmod1_op"], r["spellmod1_value"]),
                             ("pct", "MIN_RANGE", "-100"))
            self.assertEqual(set(r["spellmod1_target"].split("|")), ranged)


if __name__ == "__main__":
    unittest.main()
