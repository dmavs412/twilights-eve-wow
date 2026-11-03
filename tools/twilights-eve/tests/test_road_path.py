"""road_path.py's tracer steps and carts.toml rewrite, wowmpq's texture layers, road_textures.py's
list and ground_check.py's cart route points, on hand-made grids, chunks and TOML text.

    ~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests
"""

from __future__ import annotations

import math
import os
import sys
import tempfile
import tomllib
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, ".."))
import ground_check  # noqa: E402
import road_path  # noqa: E402
import road_textures  # noqa: E402
import wowmpq  # noqa: E402

CARTS = """\
# Carts: a comment the rewrite keeps.
[cart]
speed_yd_s = 24

[[route]]
key = "north"
name = "North"
# Kept too.
stops = [
  { key = "a", name = "A", position = [-10.0, 5.0, 0.0] },
  { key = "b", name = "B", position = [40.0, 5.0, 0.0] },
]
path = []

[[route]]
key = "east"
stops = [
  { key = "c", name = "C", position = [0.0, 0.0, 1.0] },
  { key = "d", name = "D", position = [0.0, -50.0, 2.0] },
]
path = [  # road_path.py east: 50.0 yd
  [0.0, 0.0, 1.00],
  [0.0, -50.0, 2.00],
]
"""


class TracerTest(unittest.TestCase):
    def test_a_name_loses_its_extension_only(self):
        self.assertEqual(road_path.strip_ext("Tileset\\Elwynn\\ElwynnDirtBase2.blp"), "Tileset\\Elwynn\\ElwynnDirtBase2")
        self.assertEqual(road_path.strip_ext("Tileset\\A.B\\Road"), "Tileset\\A.B\\Road")

    def test_road_names_are_per_map_and_case_free(self):
        roads = {"map": [{"id": 0, "textures": ["Tileset\\Elwynn\\ElwynnRoad.blp"]}, {"id": 1, "textures": ["X"]}]}
        self.assertEqual(road_path.road_names(roads, 0), {"tileset\\elwynn\\elwynnroad"})
        self.assertEqual(road_path.road_names(roads, 7), set())

    def test_astar_takes_the_road_round_a_field(self):
        # A 20 x 20 field with a road along three sides: the U costs 57 cells of road, the straight
        # line 19 cells of field (25 each), so the path keeps to the road.
        nx = ny = 20
        mask = bytearray(nx * ny)
        for i in range(nx):
            mask[i * ny + 0] = mask[i * ny + ny - 1] = 1
        for j in range(ny):
            mask[(nx - 1) * ny + j] = 1
        path = road_path.astar(mask, nx, ny, 0, ny - 1)
        self.assertEqual(path[0], 0)
        self.assertEqual(path[-1], ny - 1)
        self.assertTrue(all(mask[k] for k in path), "every cell is road")

    def test_astar_bridges_a_gap_in_the_road(self):
        nx, ny = 1, 30
        mask = bytearray([1] * ny)
        mask[10] = mask[11] = 0
        self.assertEqual(road_path.astar(mask, nx, ny, 0, ny - 1), list(range(ny)))

    def test_douglas_peucker_drops_the_line_and_keeps_the_corner(self):
        pts = [(float(k), 0.0) for k in range(10)] + [(9.0, float(k)) for k in range(1, 10)]
        self.assertEqual(road_path.douglas_peucker(pts, 0.75), [(0.0, 0.0), (9.0, 0.0), (9.0, 9.0)])

    def test_a_bend_turns_through_one_midpoint(self):
        out = road_path.round_bends([(0.0, 0.0), (20.0, 0.0), (20.0, 20.0)], cut=4.0)
        self.assertEqual(out[0], (0.0, 0.0))
        self.assertEqual(out[-1], (20.0, 20.0))
        p, m, q = out[1:4]
        self.assertEqual(p, (16.0, 0.0))
        self.assertEqual(q, (20.0, 4.0))
        self.assertAlmostEqual(m[0], 19.0)
        self.assertAlmostEqual(m[1], 1.0)

    def test_bends_half_a_leg_apart_share_their_cut(self):
        out = road_path.round_bends([(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (8.0, 4.0)], cut=4.0)
        self.assertTrue(all(math.dist(a, b) > 1e-6 for a, b in zip(out, out[1:])), out)

    def test_densify_cuts_long_segments_and_drops_repeats(self):
        out = road_path.densify([(0.0, 0.0), (0.0, 0.0), (0.0, 13.0)], step=6.0)
        self.assertEqual(len(out), 4)
        self.assertTrue(all(0 < math.dist(a, b) <= 6.0 for a, b in zip(out, out[1:])))
        self.assertEqual(out[-1], (0.0, 13.0))

    def test_a_snapped_stop_snaps_to_itself(self):
        def mask(grid):
            """One road cell, the lattice cell holding (-9465.3, 82.1)."""
            out = bytearray(grid.nx * grid.ny)
            for k in range(len(out)):
                cx, cy = grid.center(k)
                out[k] = abs(cx - -9465.25) < 0.01 and abs(cy - 82.25) < 0.01
            return out

        grid = road_path.Grid([-9468.0], [80.0], 10.0)
        at = road_path.snap(mask(grid), grid, -9468.0, 80.0)
        self.assertEqual(at, (-9465.25, 82.25))
        again = road_path.Grid([at[0]], [at[1]], 10.0)
        self.assertEqual(road_path.snap(mask(again), again, *at), at)
        self.assertIsNone(road_path.snap(mask(grid), grid, -9468.0, 80.0, radius=1.0))

    def test_write_route_rewrites_one_route_and_keeps_the_rest(self):
        text = road_path.write_route(CARTS, "north", [(-9.9, 5.1, 57.123), (40.0, 5.0, 60.0)],
                                     [(-9.9, 5.1, 57.123), (15.0, 5.0, 58.5), (40.0, 5.0, 60.0)], "note")
        self.assertIn("# Carts: a comment the rewrite keeps.", text)
        self.assertIn("# Kept too.", text)
        self.assertIn('position = [-9.9, 5.1, 57.12] }', text)
        self.assertIn("path = [  # note\n  [-9.9, 5.1, 57.12],\n", text)
        doc = tomllib.loads(text)
        north, east = doc["route"]
        self.assertEqual(north["path"], [[-9.9, 5.1, 57.12], [15.0, 5.0, 58.5], [40.0, 5.0, 60.0]])
        self.assertEqual(east["path"], [[0.0, 0.0, 1.0], [0.0, -50.0, 2.0]])
        # A traced path is replaced whole on the next run.
        again = road_path.write_route(text, "north", [(-9.9, 5.1, 1.0), (40.0, 5.0, 2.0)],
                                      [(-9.9, 5.1, 1.0), (40.0, 5.0, 2.0)], "again")
        self.assertEqual(tomllib.loads(again)["route"][0]["path"], [[-9.9, 5.1, 1.0], [40.0, 5.0, 2.0]])
        self.assertNotIn("note", again)


class TextureLayersTest(unittest.TestCase):
    def test_four_bit_alpha_reads_low_nibble_first(self):
        a = wowmpq.Terrain._alpha_layer(bytes([0x8F]) * 2048, 0, 0)
        self.assertEqual((a[0], a[1]), (255, 136))

    def test_rle_alpha_fills_and_copies(self):
        # A fill token (bit 7) of 100 cells of 200, a copy token of the next 2 bytes, then nothing.
        a = wowmpq.Terrain._alpha_layer(bytes([0x80 | 100, 200, 0x02, 7, 9]), 0, 0x200)
        self.assertEqual(len(a), 4096)
        self.assertEqual((a[0], a[99], a[100], a[101], a[102]), (200, 200, 7, 9, 0))

    def test_the_top_layer_is_the_strongest_overlay_from_128(self):
        t = wowmpq.Terrain(None, "Test")
        weak = bytes([0x77]) * 2048     # 7 * 17 = 119 everywhere: under 128
        strong = bytes([0xFF]) * 2048   # 255 everywhere
        mcly = ((0, 0, 0), (1, 0, 0), (2, 0, 2048))
        t.tiles[(32, 32)] = {"chunks": {}, "layers": {(0, 0): (mcly, weak + strong), (1, 0): (mcly[:2], weak)},
                             "textures": ["Base.blp", "Weak.blp", "Road.blp"], "alpha": {}}
        names, top = t.texture_cells(32, 32, 0, 0)
        self.assertEqual(names, ["Base.blp", "Weak.blp", "Road.blp"])
        self.assertEqual(set(top), {2})
        self.assertEqual(set(t.texture_cells(32, 32, 1, 0)[1]), {0}, "no overlay reaches 128: the base")
        self.assertEqual(t.top_texture(-1.0, -1.0), "Road.blp")

    def test_road_textures_counts_each_name_once_per_tile(self):
        counts = road_textures.texture_counts([["B.blp", "A.blp", "A.blp"], ["A.blp"]])
        self.assertEqual(counts, [("A.blp", 2), ("B.blp", 1)])
        text = road_textures.render(["Elwynn Forest"], 2, counts)
        self.assertTrue(all(line.startswith("#") for line in text.split("\n")))
        self.assertIn("#     2  A.blp", text)


class RoutePointsTest(unittest.TestCase):
    def test_stops_and_path_points_are_placed_units_with_their_z(self):
        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d, "carts.toml")
            with open(path, "w", encoding="utf-8", newline="") as f:
                f.write(CARTS)
            lines, points = ground_check.route_points(path)
        self.assertEqual([p.label for p in points], ["route north stop a", "route north stop b", "route east stop c",
                                                     "route east stop d", "route east path point 1",
                                                     "route east path point 2"])
        self.assertEqual((points[2].x, points[2].y, points[2].z), (0.0, 0.0, 1.0))
        p = points[5]
        ground_check.set_xy(lines, p.idx, p.span, p.x, p.y, 7.25)
        self.assertEqual(lines[p.idx], "  [0.0, -50.0, 7.25],")


if __name__ == "__main__":
    unittest.main()
