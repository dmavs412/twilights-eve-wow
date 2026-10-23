"""ground_check.py's Rumour points, water, wall and roof rules and --fix's nudge, on hand-made
groups and a small TOML file.

    ~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests
"""

from __future__ import annotations

import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, ".."))
import ground_check  # noqa: E402
import wmo_floor  # noqa: E402

RUMOURS = """\
[[rumour]]
id = 4
map = 0
giver = 97010
points = [
  { x = -8975.0, y = -185.0, landmark = "the river bank [below]", note = 980030 },
  # A comment line { x = 1.0, y = 2.0 } inside the array is not a point.
  { x = -8829.0, y = -160.0, landmark = "the wolf glade", note = 980031 },
]
quests = [94901]

[[rumour.spawn]]
quest = 94903
entry = 97011
points = [{ x = -8992.0, y = -325.0 }, { x = -9010.5, y = -300.0 }]
"""


class XyPoints(unittest.TestCase):
    def setUp(self):
        fd, self.path = tempfile.mkstemp(suffix=".toml")
        with os.fdopen(fd, "wb") as f:
            f.write(RUMOURS.encode("utf-8"))

    def tearDown(self):
        os.remove(self.path)

    def test_it_finds_both_point_layouts_with_their_labels_and_maps(self):
        _, points = ground_check.xy_points(self.path)
        got = [(p.label, p.map, p.x, p.y, p.entry) for p in points]
        self.assertEqual(got, [
            ("rumour 4 point 1", 0, -8975.0, -185.0, 97010),
            ("rumour 4 point 2", 0, -8829.0, -160.0, 97010),
            ("rumour 4 spawn 94903 point 1", 0, -8992.0, -325.0, 97011),
            ("rumour 4 spawn 94903 point 2", 0, -9010.5, -300.0, 97011),
        ])

    def test_set_xy_rewrites_a_point_in_place(self):
        lines, points = ground_check.xy_points(self.path)
        for p in sorted(points[2:], key=lambda p: -p.span[0][0]):
            ground_check.set_xy(lines, p.idx, p.span, p.x + 1, p.y - 1)
        self.assertEqual(lines[points[2].idx],
                         "points = [{ x = -8991.0, y = -326.0 }, { x = -9009.5, y = -301.0 }]")


class Water(unittest.TestCase):
    TEMPLATES = {97010: {"ours": True, "swims": False}, 97020: {"ours": True, "swims": True},
                 80: {"ours": False, "swims": False}, 123: {"ours": False, "swims": False}}

    def test_a_land_unit_stands_in_water_up_to_a_yard_and_a_half(self):
        self.assertFalse(ground_check.too_deep(51.4, 50.0))
        self.assertTrue(ground_check.too_deep(51.6, 50.0))
        self.assertFalse(ground_check.too_deep(None, 50.0))

    def test_swimmers_and_gameobjects_are_exempt(self):
        check = ground_check.water_checked
        self.assertTrue(check("creature", 97010, self.TEMPLATES, set()))
        self.assertFalse(check("creature", 97020, self.TEMPLATES, set()))
        self.assertFalse(check("gameobject", 97010, self.TEMPLATES, set()))
        self.assertFalse(check("added", 80, self.TEMPLATES, {80}))
        self.assertTrue(check("added", 123, self.TEMPLATES, {80}))

    def test_a_hub_npc_is_never_exempt(self):
        self.assertTrue(ground_check.water_checked("npc", 97020, self.TEMPLATES, set()))

    def test_another_kind_follows_its_entry(self):
        self.assertTrue(ground_check.water_checked("rumour", 97010, self.TEMPLATES, set()))
        self.assertFalse(ground_check.water_checked("rumour", 555, self.TEMPLATES, set()))

    def test_a_wmo_pool_reads_its_tile(self):
        step = wmo_floor.MLIQ_STEP
        pool = wmo_floor.Liquid(2, 2, (0.0, 0.0, 0.0), [3.0, 3.0, 3.0, 3.0], bytes([0x04]))
        self.assertAlmostEqual(pool.surface(step / 2, step / 2), 3.0)
        self.assertIsNone(pool.surface(step + 1, 1.0))
        lava = wmo_floor.Liquid(2, 2, (0.0, 0.0, 0.0), [3.0] * 4, bytes([0x04]), group_liquid=2)
        self.assertIsNone(lava.surface(1.0, 1.0))


def wall_at(x):
    """A vertical wall in the plane model x = `x`, y -5 to 5, z 0 to 4, that collides."""
    a, b, c, d = (x, -5.0, 0.0), (x, 5.0, 0.0), (x, 5.0, 4.0), (x, -5.0, 4.0)
    return wmo_floor.Group(0, wmo_floor.GROUP_INDOOR,
                           [(a, b, c, wmo_floor.MOPY_RENDER), (a, c, d, wmo_floor.MOPY_RENDER)])


class Wall(unittest.TestCase):
    def test_a_wall_closer_than_the_body_radius_fails(self):
        self.assertTrue(wmo_floor.walls_at([wall_at(0.3)], 0.0, 0.0, 0.0))
        self.assertFalse(wmo_floor.walls_at([wall_at(0.5)], 0.0, 0.0, 0.0))

    def test_a_wall_below_the_knees_or_over_the_head_passes(self):
        self.assertFalse(wmo_floor.walls_at([wall_at(0.3)], 0.0, 0.0, 4.5))
        self.assertFalse(wmo_floor.walls_at([wall_at(0.3)], 0.0, 0.0, -2.5))

    def test_a_render_detail_wall_does_not_collide(self):
        a, b, c = (0.2, -5.0, 0.0), (0.2, 5.0, 0.0), (0.2, 5.0, 4.0)
        grp = wmo_floor.Group(0, 0, [(a, b, c, wmo_floor.MOPY_RENDER | wmo_floor.MOPY_DETAIL)])
        self.assertFalse(wmo_floor.walls_at([grp], 0.0, 0.0, 0.0))


def floor(indoor):
    return wmo_floor.Surface(54.0, (0, 0, 4.0), 0, indoor, True, True)


class Roof(unittest.TestCase):
    def test_an_outdoor_floor_four_yards_over_the_terrain_is_a_roof(self):
        self.assertTrue(ground_check.is_roof(floor(False), 50.0))

    def test_on_top_allows_it(self):
        self.assertFalse(ground_check.is_roof(floor(False), 50.0, on_top=True))

    def test_the_same_floor_indoors_is_a_storey(self):
        self.assertFalse(ground_check.is_roof(floor(True), 50.0))


class Nudge(unittest.TestCase):
    def test_rings_go_out_a_yard_at_a_time_from_plus_x(self):
        got = list(ground_check.nudge_candidates(10.0, 20.0))
        self.assertEqual(len(got), 40)
        self.assertEqual(got[0], (11.0, 20.0))
        self.assertEqual(got[2], (10.0, 21.0))
        self.assertEqual(got[8], (12.0, 20.0))

    def test_it_picks_the_nearer_passing_spot(self):
        def passes(cx, cy):
            return cy == 20.0 and cx in (12.0, 13.0)

        self.assertEqual(ground_check.nudge(10.0, 20.0, passes), (12.0, 20.0, True))

    def test_none_when_nothing_passes(self):
        self.assertIsNone(ground_check.nudge(10.0, 20.0, lambda cx, cy: False))


if __name__ == "__main__":
    unittest.main()
