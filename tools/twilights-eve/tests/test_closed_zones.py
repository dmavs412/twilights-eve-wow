"""closed_zones.py roll-up, grid index and output, on a fixture AreaTable and chunk map.

    ~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests
"""

from __future__ import annotations

import json
import os
import sys
import tempfile
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
        self.assertEqual(
            text.splitlines(),
            ['{"map": 0, "zones": [40], "chunks": [', "[1, 2],", "[3, 4]", '], "gated": []}'],
        )

    def test_no_opens_rows_writes_an_empty_gated_list(self):
        data = json.loads(closed_zones.render(0, [40], [[1, 2]]))
        self.assertEqual(data["gated"], [])

    def test_gated_rows_carry_key_zone_and_sorted_chunks_one_per_line(self):
        chunks, _ = closed_zones.closed_chunks(self.TILES, PARENT, {12})
        text = closed_zones.render(0, [40], [[1, 2]], [("steppes_open", 12, chunks)])
        data = json.loads(text)
        self.assertEqual(data["chunks"], [[1, 2]])
        self.assertEqual(data["gated"], [{"key": "steppes_open", "zone": 12, "chunks": [[482, 786], [496, 784]]}])
        self.assertEqual(
            text.splitlines(),
            [
                '{"map": 0, "zones": [40], "chunks": [',
                "[1, 2]",
                '], "gated": [',
                '{"key": "steppes_open", "zone": 12, "chunks": [',
                "[482, 786],",
                "[496, 784]",
                "]}",
                "]}",
            ],
        )


class Config(unittest.TestCase):
    def load(self, text):
        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, "closed_zones.toml")
            with open(path, "w", encoding="utf-8") as f:
                f.write(text)
            return closed_zones.load_config(path)

    def test_opens_rows_come_back_with_their_milestone(self):
        cfg = self.load('map = 0\nzones = [40, 3]\n[[opens]]\nzone = 46\nmilestone = "steppes_open"\n')
        self.assertEqual(cfg, (0, [40, 3], [(46, "steppes_open")]))

    def test_no_opens_rows_is_an_empty_list(self):
        self.assertEqual(self.load("map = 0\nzones = [40]\n"), (0, [40], []))

    def test_a_zone_in_zones_and_in_opens_is_an_error_naming_it(self):
        with self.assertRaisesRegex(ValueError, "zone 46"):
            self.load('map = 0\nzones = [40, 46]\n[[opens]]\nzone = 46\nmilestone = "steppes_open"\n')

    def test_a_zone_in_two_opens_rows_is_an_error_naming_it(self):
        with self.assertRaisesRegex(ValueError, "zone 46"):
            self.load(
                'map = 0\nzones = [40]\n[[opens]]\nzone = 46\nmilestone = "a"\n[[opens]]\nzone = 46\nmilestone = "b"\n'
            )


if __name__ == "__main__":
    unittest.main()
