"""wmo_floor.py's vertical ray and ground_check.py's floor test, on a hand-made group.

    ~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests
"""

from __future__ import annotations

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, ".."))
import ground_check  # noqa: E402
import wmo_floor  # noqa: E402

RENDER = wmo_floor.MOPY_RENDER
DETAIL = wmo_floor.MOPY_DETAIL
COLLISION = wmo_floor.MOPY_COLLISION


def square(z, flags, size=4.0):
    """Two triangles covering x and y 0 to `size` at height `z`."""
    a, b, c, d = (0.0, 0.0, z), (size, 0.0, z), (size, size, z), (0.0, size, z)
    return [(a, b, c, flags), (a, c, d, flags)]


# An indoor room: a floor at z 2, a render-only rug at 2.1 the client walks through, and a ceiling
# at 6. A steep ramp beside it, rising 4 yd over 1 yd.
ROOM = wmo_floor.Group(0, wmo_floor.GROUP_INDOOR,
                       square(2.0, RENDER) + square(2.1, RENDER | DETAIL) + square(6.0, COLLISION))
RAMP = wmo_floor.Group(1, 0, [((10.0, 0.0, 0.0), (11.0, 0.0, 4.0), (11.0, 4.0, 4.0), RENDER)])


class Ray(unittest.TestCase):
    def test_it_finds_each_surface_over_the_point(self):
        hits = sorted((round(z, 2), grp.index) for z, grp, _, _ in wmo_floor.ray([ROOM, RAMP], 1.0, 3.0))
        self.assertEqual(hits, [(2.0, 0), (2.1, 0), (6.0, 0)])

    def test_nothing_over_a_point_beside_the_building(self):
        self.assertEqual(wmo_floor.ray([ROOM], 20.0, 20.0), [])

    def test_render_detail_does_not_collide(self):
        self.assertTrue(wmo_floor.collides(RENDER))
        self.assertTrue(wmo_floor.collides(COLLISION))
        self.assertFalse(wmo_floor.collides(RENDER | DETAIL))

    def test_a_steep_ramp_is_not_walkable(self):
        [(z, _, nz, _)] = wmo_floor.ray([RAMP], 10.5, 1.0)
        self.assertAlmostEqual(z, 2.0)
        self.assertLess(nz, wmo_floor.WALKABLE_NZ)


class StandsOn(unittest.TestCase):
    def test_a_unit_stands_within_a_yard_above_or_half_a_yard_below(self):
        self.assertTrue(ground_check.stands_on(82.4, 81.94))
        self.assertTrue(ground_check.stands_on(81.6, 81.94))
        self.assertFalse(ground_check.stands_on(82.4, 80.42))
        self.assertFalse(ground_check.stands_on(53.4, 57.16))


if __name__ == "__main__":
    unittest.main()
