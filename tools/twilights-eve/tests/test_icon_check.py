"""icon_check.py's grouping and known-list rules, on a fake display-to-icon map and rows.

    ~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests
"""

from __future__ import annotations

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, ".."))
import icon_check  # noqa: E402

ICONS = {10: "INV_Misc_Bag_01", 11: "inv_misc_bag_01", 12: "INV_Sword_01"}


class FindClashes(unittest.TestCase):
    def test_two_displays_one_icon_clash(self):
        rows = [(1, "Bag", 10, "a.toml"), (2, "Pouch", 11, "b.toml"), (3, "Sword", 12, "a.toml")]
        clashes = icon_check.find_clashes(rows, ICONS)
        self.assertEqual(list(clashes), ["inv_misc_bag_01"])
        self.assertEqual([e for e, _, _ in clashes["inv_misc_bag_01"]], [1, 2])

    def test_one_entry_in_two_files_is_not_a_clash(self):
        rows = [(1, "Bag", 10, "a.toml"), (1, "Bag", 10, "b.toml"), (3, "Sword", 12, "a.toml")]
        self.assertEqual(icon_check.find_clashes(rows, ICONS), {})

    def test_unknown_display_is_skipped(self):
        rows = [(1, "Bag", 99, "a.toml"), (2, "Pouch", 98, "a.toml")]
        self.assertEqual(icon_check.find_clashes(rows, ICONS), {})


class Judge(unittest.TestCase):
    clashes = {"inv_misc_bag_01": [(1, "Bag", "a.toml"), (2, "Pouch", "b.toml")]}

    def test_known_clash_passes(self):
        new, known, stale = icon_check.judge(self.clashes, {"inv_misc_bag_01\t1,2"})
        self.assertEqual((new, known, stale), ({}, 1, []))

    def test_unknown_clash_is_new(self):
        new, known, stale = icon_check.judge(self.clashes, set())
        self.assertEqual(list(new), ["inv_misc_bag_01"])
        self.assertEqual((known, stale), (0, []))

    def test_known_line_without_clash_is_stale(self):
        new, known, stale = icon_check.judge({}, {"inv_sword_01\t3,4"})
        self.assertEqual((new, known, stale), ({}, 0, ["inv_sword_01\t3,4"]))


if __name__ == "__main__":
    unittest.main()
