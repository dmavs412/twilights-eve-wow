"""sheet.py driven through its command line against a small CSV.

    ~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests
"""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, "..", "sheet.py")

# A quoted cell with a line break and a comma, a talent with two ranks, and a short row.
SHEET = (
    "kind,calling,key,name,rank,visual,notes\n"
    'skill,acolyte,miasma,Miasma,1,2120,"Borrowed, then\nmoved"\n'
    "talent,vanguard,hallowed,Hallowed,1,,\n"
    "talent,vanguard,hallowed,Hallowed,2,,\n"
    "skill,ranger,auto_shot,Auto Shot,1\n"
)


class SheetTool(unittest.TestCase):
    def setUp(self):
        self.path = os.path.join(tempfile.mkdtemp(), "master.csv")
        with open(self.path, "wb") as f:
            f.write(SHEET.encode("utf-8"))

    def run_sheet(self, *args: str) -> subprocess.CompletedProcess:
        return subprocess.run([sys.executable, SCRIPT, "--csv", self.path, *args],
                              capture_output=True, text=True)

    def text(self) -> str:
        with open(self.path, "rb") as f:
            return f.read().decode("utf-8")

    def test_get_prints_the_non_empty_cells(self):
        done = self.run_sheet("get", "acolyte/miasma")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertIn("line 2 (acolyte/miasma, skill rank 1)", done.stdout)
        self.assertIn("  visual: 2120", done.stdout)
        self.assertNotIn("  rank:", self.run_sheet("get", "acolyte/miasma", "visual").stdout)

    def test_set_rewrites_only_the_record_it_touches(self):
        done = self.run_sheet("set", "acolyte/miasma", "visual=11433")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(self.text(), SHEET.replace(",2120,", ",11433,"))

    def test_set_pads_a_short_row(self):
        done = self.run_sheet("set", "ranger/auto_shot", "visual=none")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertIn("skill,ranger,auto_shot,Auto Shot,1,none\n", self.text())

    def test_several_matches_need_a_rank_or_all(self):
        done = self.run_sheet("set", "vanguard/hallowed", "visual=5")
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("matches 2 rows", done.stderr)
        self.assertEqual(self.text(), SHEET)
        done = self.run_sheet("set", "vanguard/hallowed", "--rank", "2", "visual=5")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertIn("talent,vanguard,hallowed,Hallowed,2,5,\n", self.text())

    def test_an_unknown_column_fails_and_writes_nothing(self):
        done = self.run_sheet("set", "acolyte/miasma", "colour=red")
        self.assertIn("not a column of the sheet: colour", done.stderr)
        self.assertEqual(self.text(), SHEET)

    def test_add_row_goes_after_the_named_row(self):
        done = self.run_sheet("add-row", "acolyte/cloud", "kind=skill", "name=Cloud", "--after", "acolyte/miasma")
        self.assertEqual(done.returncode, 0, done.stderr)
        lines = self.text().split("\n")
        self.assertEqual(lines[3], "skill,acolyte,cloud,Cloud")
        self.assertIn("kind=", self.run_sheet("add-row", "acolyte/x", "name=X").stderr)

    def test_add_column_extends_the_header_only(self):
        done = self.run_sheet("add-column", "client_radius_index")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(self.text(), SHEET.replace("notes\n", "notes,client_radius_index\n", 1))
        self.assertIn("already a column", self.run_sheet("add-column", "visual").stderr)


if __name__ == "__main__":
    unittest.main()
