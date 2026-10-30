"""import_herb_spots.py's points-line rewrite, terrain Z, spacing rule and lock selection, on strings.

    ~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests
"""

from __future__ import annotations

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, ".."))
import import_herb_spots as ihs  # noqa: E402

TOML = """\
# header comment
[[zone]]
key = "elwynn"
zone = 12
live = 30
points = []

[[zone]]
key = "redridge"
zone = 44
live = 20
points = [{ x = 1.0, y = 2.0, z = 3.00 }]

[[plant]]
zone = "elwynn"
points = [ keep, me ]
"""


class RewriteTest(unittest.TestCase):
    def test_only_the_points_line_of_a_zone_row_changes(self):
        out = ihs.rewrite_points(TOML, {12: [(5.0, 6.0, 40.0)], 44: [(7.0, 8.0, 41.25)]})
        old, new = TOML.split("\n"), out.split("\n")
        self.assertEqual(len(old), len(new))
        changed = [i for i, (a, b) in enumerate(zip(old, new)) if a != b]
        self.assertEqual(len(changed), 2)
        self.assertEqual(new[changed[0]], "points = [{ x = 5.0, y = 6.0, z = 40.00 }]")
        self.assertEqual(new[changed[1]], "points = [{ x = 7.0, y = 8.0, z = 41.25 }]")
        self.assertIn("points = [ keep, me ]", new)

    def test_a_zone_without_points_is_left_alone(self):
        out = ihs.rewrite_points(TOML, {12: [(5.0, 6.0, 40.0)]})
        self.assertIn("points = [{ x = 1.0, y = 2.0, z = 3.00 }]", out)

    def test_points_sort_by_x_then_y(self):
        line = ihs.points_line([(3.0, 9.0, 1.0), (1.0, 5.0, 2.0), (3.0, 2.0, 3.0)])
        self.assertEqual(line, "points = [{ x = 1.0, y = 5.0, z = 2.00 }, { x = 3.0, y = 2.0, z = 3.00 }, { x = 3.0, y = 9.0, z = 1.00 }]")

    def test_x_and_y_round_to_one_decimal_place_and_z_to_two(self):
        self.assertEqual(ihs.points_line([(-8975.04, 12.26, 56.789)]),
                         "points = [{ x = -8975.0, y = 12.3, z = 56.79 }]")

    def test_a_rerun_gives_the_same_text(self):
        once = ihs.rewrite_points(TOML, {12: [(5.0, 6.0, 40.0), (1.0, 1.0, 41.0)]})
        self.assertEqual(ihs.rewrite_points(once, {12: [(1.0, 1.0, 41.0), (5.0, 6.0, 40.0)]}), once)

    def test_crlf_line_endings_survive(self):
        out = ihs.rewrite_points(TOML.replace("\n", "\r\n"), {12: [(5.0, 6.0, 40.0)]})
        self.assertIn("points = [{ x = 5.0, y = 6.0, z = 40.00 }]\r\n", out)


class TerrainZTest(unittest.TestCase):
    def test_z_is_the_terrain_at_the_rounded_x_and_y(self):
        asked = []

        def ground_z(x, y):
            asked.append((x, y))
            return x + y

        self.assertEqual(ihs.with_z([(1.04, 2.06)], ground_z), [(1.0, 2.1, 3.1)])
        self.assertEqual(asked, [(1.0, 2.1)])

    def test_a_point_with_no_terrain_is_dropped(self):
        got = ihs.with_z([(1.0, 2.0), (5.0, 6.0)], lambda x, y: None if x < 3 else 40.0)
        self.assertEqual(got, [(5.0, 6.0, 40.0)])


class DedupeTest(unittest.TestCase):
    def test_a_point_within_5_yards_of_a_kept_one_is_dropped(self):
        self.assertEqual(ihs.dedupe([(0.0, 0.0), (3.0, 3.0), (10.0, 0.0)]), [(0.0, 0.0), (10.0, 0.0)])

    def test_exactly_5_yards_apart_is_kept(self):
        self.assertEqual(ihs.dedupe([(0.0, 0.0), (3.0, 4.0)]), [(0.0, 0.0), (3.0, 4.0)])

    def test_the_first_point_wins(self):
        self.assertEqual(ihs.dedupe([(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)]), [(0.0, 0.0)])


class LockTest(unittest.TestCase):
    def test_a_lock_with_a_herbalism_slot_counts(self):
        def row(lock_id, slot_type, index):
            r = [0] * 33
            r[0], r[1], r[9] = lock_id, slot_type, index
            return tuple(r)

        rows = [row(29, 2, 2), row(30, 2, 3), row(31, 1, 2)]
        self.assertEqual(ihs.herbalism_locks(rows), {29})


if __name__ == "__main__":
    unittest.main()
