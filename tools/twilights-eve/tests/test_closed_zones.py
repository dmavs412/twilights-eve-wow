"""closed_zones.py roll-up, grid index and output, on a fixture AreaTable and chunk map.

    ~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests
"""

from __future__ import annotations

import json
import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, ".."))
import closed_zones  # noqa: E402

# area -> parent. 40 is a closed zone, 41 its subzone, 42 under 41, 12 an unrelated zone with
# subzone 13. 90 -> 91 -> ... -> 99 -> 90 is a cycle, longer than the step cap.
PARENT = {40: 0, 41: 40, 42: 41, 12: 0, 13: 12}
PARENT.update({90 + i: 90 + (i + 1) % 10 for i in range(10)})


class RollUp(unittest.TestCase):
    def test_zone_is_its_own_top(self):
        self.assertEqual(closed_zones.roll_up(40, PARENT), 40)

    def test_subzone_two_levels_deep_reaches_the_zone(self):
        self.assertEqual(closed_zones.roll_up(42, PARENT), 40)

    def test_unknown_area_is_its_own_top(self):
        self.assertEqual(closed_zones.roll_up(7, PARENT), 7)

    def test_cycle_stops_at_the_step_cap(self):
        top = closed_zones.roll_up(90, PARENT)
        self.assertEqual(top, 90 + closed_zones.MAX_STEPS % 10)


class GridIndex(unittest.TestCase):
    def test_tile_31_49_chunk_3_15(self):
        self.assertEqual(closed_zones.grid_index(31, 49, 3, 15), (499, 799))


class Chunks(unittest.TestCase):
    TILES = {
        (31, 49): {(3, 15): 42, (0, 0): 13, (1, 0): 40},
        (30, 49): {(15, 15): 41, (2, 2): 12},
    }

    def test_deep_subzone_closed_and_sibling_zone_open(self):
        chunks, _ = closed_zones.closed_chunks(self.TILES, PARENT, {40})
        self.assertIn([499, 799], chunks)
        self.assertNotIn([496, 784], chunks)
        self.assertNotIn([482, 786], chunks)

    def test_sorted_and_counted_per_area(self):
        chunks, per_area = closed_zones.closed_chunks(self.TILES, PARENT, {40})
        self.assertEqual(chunks, sorted(chunks))
        self.assertEqual(len(chunks), 3)
        self.assertEqual(per_area, {42: 1, 40: 1, 41: 1})

    def test_output_parses_back_to_the_same_set(self):
        chunks, _ = closed_zones.closed_chunks(self.TILES, PARENT, {40})
        text = closed_zones.render(0, [40], chunks)
        data = json.loads(text)
        self.assertEqual(data["map"], 0)
        self.assertEqual(data["zones"], [40])
        self.assertEqual(data["chunks"], chunks)
        self.assertEqual(len(text.splitlines()), len(chunks) + 2)

    def test_one_chunk_per_line(self):
        text = closed_zones.render(0, [40], [[1, 2], [3, 4]])
        self.assertEqual(text.splitlines(), ['{"map": 0, "zones": [40], "chunks": [', "[1, 2],", "[3, 4]", "]}"])


if __name__ == "__main__":
    unittest.main()
