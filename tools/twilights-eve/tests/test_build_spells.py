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
    "kind": "skill", "calling": "vanguard", "key": "strike", "name": "Strike", "rank": "1",
    "spell_id": "78", "client_cost": "15", "realm_cost": "15", "target": "enemy",
    "effect1_kind": "weapon_strike",
    "effect1_params": "weapon=W;bonus.base=9.5;bonus.per_level=1.4;bonus.per_ap=0;tags=a|b",
    "effect4_kind": "stun", "effect4_params": "duration_ms=1000",
    "modelled": "yes", "description": "Hits.",
}


def talent(key: str, rank: int, row: int, column: int, **extra: str) -> dict:
    return {"kind": "talent", "calling": "vanguard", "key": key, "name": key.title(),
            "rank": str(rank), "tab": "Arms", "tab_row": str(row), "tab_column": str(column),
            "modelled": "yes", **extra}


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
        out = os.path.join(tmp, "server_spells.json")
        data = None
        if os.path.exists(out):
            with open(out, encoding="utf-8") as f:
                data = json.load(f)
        return done, data

    def assert_fails(self, rows: list[dict], message: str) -> None:
        done, data = self.run_build(rows)
        self.assertNotEqual(done.returncode, 0)
        self.assertIn(message, done.stderr)
        self.assertIsNone(data)

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

    def test_duplicate_rank_fails(self):
        self.assert_fails([STRIKE, STRIKE], "duplicate")


if __name__ == "__main__":
    unittest.main()
