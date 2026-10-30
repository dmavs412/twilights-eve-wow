"""ground_check.py's Rumour points, water, wall and roof rules and --fix's nudge, on hand-made
groups and a small TOML file.

    ~/.local/bin/python3.15 -m unittest discover tools/twilights-eve/tests
"""

from __future__ import annotations

import glob
import math
import struct
import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, ".."))
import doodad_check  # noqa: E402
import ground_check  # noqa: E402
import wmo_floor  # noqa: E402
import wowmpq  # noqa: E402

DATA = os.environ.get("WOW_DATA", ground_check.DEFAULT_DATA)

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
        self.assertTrue(all(p.z is None for p in points))

    def test_set_xy_rewrites_a_point_in_place(self):
        lines, points = ground_check.xy_points(self.path)
        for p in sorted(points[2:], key=lambda p: -p.span[0][0]):
            ground_check.set_xy(lines, p.idx, p.span, p.x + 1, p.y - 1)
        self.assertEqual(lines[points[2].idx],
                         "points = [{ x = -8991.0, y = -326.0 }, { x = -9009.5, y = -301.0 }]")


HERBS = """\
[[perk]]
key = "gloves"

[[zone]]
key = "elwynn"
zone = 12
live = 30
points = [{ x = -9001.5, y = 120.0, z = 56.25 }, { x = -9000.0, y = 130.0, z = 57.00 }]
"""


class HerbSpots(unittest.TestCase):
    def setUp(self):
        fd, self.path = tempfile.mkstemp(suffix=".toml")
        with os.fdopen(fd, "wb") as f:
            f.write(HERBS.encode("utf-8"))

    def tearDown(self):
        os.remove(self.path)

    def test_a_herb_spot_reads_its_z_under_its_zone_key(self):
        _, points = ground_check.xy_points(self.path)
        got = [(p.label, p.map, p.x, p.y, p.z) for p in points]
        self.assertEqual(got, [
            ("herb zone elwynn point 1", 0, -9001.5, 120.0, 56.25),
            ("herb zone elwynn point 2", 0, -9000.0, 130.0, 57.0),
        ])

    def test_set_xy_rewrites_a_herb_spots_z(self):
        lines, points = ground_check.xy_points(self.path)
        for p in sorted(points, key=lambda p: -p.span[0][0]):
            ground_check.set_xy(lines, p.idx, p.span, p.x, p.y, p.z + 1)
        self.assertEqual(lines[points[0].idx],
                         "points = [{ x = -9001.5, y = 120.0, z = 57.25 }, { x = -9000.0, y = 130.0, z = 58.00 }]")


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


def box_hull(sx, sy, sz):
    """A box of sx by sy by sz yd centred on the model origin: 8 vertices, 12 triangles."""
    verts = [(dx * sx / 2, dy * sy / 2, dz * sz / 2) for dx in (-1, 1) for dy in (-1, 1) for dz in (-1, 1)]
    faces = [(0, 1, 3, 2), (4, 6, 7, 5), (0, 4, 5, 1), (2, 3, 7, 6), (0, 2, 6, 4), (1, 5, 7, 3)]
    return verts, [t for a, b, c, d in faces for t in ((a, b, c), (a, c, d))]


class Doodad(unittest.TestCase):
    def test_a_scaled_cube_hits_its_centre_and_misses_a_point_outside(self):
        cube = doodad_check.place("cube", 1, box_hull(1, 1, 1), (100.0, 200.0, 50.0), (0.0, 30.0, 0.0), 2.0)
        self.assertTrue(doodad_check.body_hits(cube, 100.0, 200.0, 49.0))
        # The cube turns by 30 - 180 degrees; 2.5 yd along a face normal is 1.5 yd outside it.
        a = math.radians(30.0 - 180.0)
        self.assertFalse(doodad_check.body_hits(cube, 100.0 + 2.5 * math.cos(a), 200.0 + 2.5 * math.sin(a), 49.0))

    def test_a_body_wholly_inside_a_big_rock_hits(self):
        rock = doodad_check.place("rock", 1, box_hull(1, 1, 1), (0.0, 0.0, 0.0), (0.0, 0.0, 0.0), 20.0)
        self.assertTrue(doodad_check.body_hits(rock, 0.0, 0.0, 0.0))

    def test_a_quarter_turn_moves_the_long_side_to_the_other_axis(self):
        hull = box_hull(4, 1, 1)
        along_x = doodad_check.place("box", 1, hull, (0.0, 0.0, 0.5), (0.0, 0.0, 0.0), 1.0)
        along_y = doodad_check.place("box", 2, hull, (0.0, 0.0, 0.5), (0.0, 90.0, 0.0), 1.0)
        self.assertTrue(doodad_check.body_hits(along_x, 1.5, 0.0, 0.0))
        self.assertFalse(doodad_check.body_hits(along_x, 0.0, 1.5, 0.0))
        self.assertFalse(doodad_check.body_hits(along_y, 1.5, 0.0, 0.0))
        self.assertTrue(doodad_check.body_hits(along_y, 0.0, 1.5, 0.0))

    @unittest.skipUnless(os.path.isdir(DATA), "no game data under $WOW_DATA")
    def test_a_real_cliff_rock_on_the_northshire_rim(self):
        # elwynncliffrock02.m2, MDDF uniqueId 10122, on the east rim of Northshire Valley: its
        # hull spans x -8765.6 to -8727.6 and y -429.1 to -398.0, so x + 29 is 10 yd north of it.
        gd = wowmpq.GameData(DATA)
        self.addCleanup(lambda: [a.f.close() for a in gd.archives])
        terrain = wowmpq.Terrain(gd, "Azeroth")
        doodads = doodad_check.Doodads(gd, terrain)
        rock = "world/azeroth/elwynn/passivedoodads/cliffrocks/elwynncliffrock02.m2"
        x, y = -8746.6, -413.6
        self.assertIn(rock, doodads.hits(x, y, terrain.ground_z(x, y)))
        self.assertNotIn(rock, doodads.hits(x + 29.0, y, terrain.ground_z(x + 29.0, y)))


def bits(f):
    """A float's int32 bits, as wowmpq.GameData.dbc returns a float cell."""
    return struct.unpack("<i", struct.pack("<f", f))[0]


class FakeData:
    """Three display DBCs and a file set: dbc(name) gives int32 rows and a string block."""

    def __init__(self, files):
        self.block = b"\0"
        self.files = {f.lower() for f in files}
        model = self.s("Creature\\Wolf\\Wolf.mdx")
        skin = self.s("WolfSkinGrey")
        lost = self.s("WolfSkinLost")
        door = self.s("World\\Generic\\Door.mdx")
        self.tables = {
            # ID, ModelID, Sound, Extended, Scale, Alpha, Texture 0..2, SizeClass, Blood, NPCSound
            "CreatureDisplayInfo": [
                (10, 1, 0, 0, bits(1.0), 255, skin, 0, 0, 1, 0, 0),
                (11, 1, 0, 0, bits(0.0), 255, skin, 0, 0, 1, 0, 0),
                (12, 1, 0, 0, bits(1.0), 255, lost, 0, 0, 1, 0, 0),
                (13, 2, 0, 0, bits(1.0), 255, 0, 0, 0, 1, 0, 0),
                (14, 1, 0, 7, bits(1.0), 255, lost, 0, 0, 1, 0, 0),
            ],
            "CreatureModelData": [(1, 0, model), (2, 0, self.s("Creature\\Gone\\Gone.mdx"))],
            "GameObjectDisplayInfo": [(20, door)],
        }

    def s(self, text):
        off = len(self.block)
        self.block += text.encode() + b"\0"
        return off

    def dbc(self, name):
        return self.tables[name], self.block

    def has(self, path):
        return path.lower() in self.files


class Model(unittest.TestCase):
    FILES = ["Creature\\Wolf\\Wolf.m2", "Creature\\Wolf\\WolfSkinGrey.blp", "World\\Generic\\Door.m2"]

    def setUp(self):
        self.models = ground_check.Models(FakeData(self.FILES))

    def test_a_good_creature_display_passes(self):
        self.assertIsNone(self.models.creature(10))

    def test_a_scale_of_zero_fails(self):
        self.assertEqual(self.models.creature(11), "scale 0")

    def test_a_missing_texture_variation_fails(self):
        self.assertIn("no texture Creature\\Wolf\\WolfSkinLost.blp", self.models.creature(12))

    def test_a_missing_model_file_fails(self):
        self.assertIn("no model file Creature\\Gone\\Gone.m2", self.models.creature(13))

    def test_a_humanoid_display_needs_only_its_model(self):
        self.assertIsNone(self.models.creature(14))

    def test_a_missing_display_row_fails(self):
        self.assertEqual(self.models.creature(99), "no CreatureDisplayInfo row")

    def test_a_gameobject_with_display_0_fails(self):
        self.assertEqual(self.models.gameobject(0), "display 0")
        self.assertIsNone(self.models.gameobject(20))

    def test_a_row_takes_its_template_display_and_a_missing_template_fails(self):
        templates = {96011: {"ours": True, "swims": False, "display": 13}}
        row = ground_check.Row(0, "guid 1", 0, True, 0.0, 0.0, 0.0, "creature", 96011, False, None)
        self.assertEqual(ground_check.model_failure(row, templates, self.models)[0], 13)
        self.assertIsNone(ground_check.model_failure(row._replace(display=10), templates, self.models))
        self.assertEqual(ground_check.model_failure(row._replace(entry=5), templates, self.models),
                         (None, "no template"))

    def test_a_portal_without_a_display_is_not_checked(self):
        row = ground_check.Row(0, None, 0, True, 0.0, 0.0, 0.0, "portal", None, False, None)
        self.assertIsNone(ground_check.model_failure(row, {}, self.models))

    @unittest.skipUnless(os.path.isdir(DATA), "no game data under $WOW_DATA")
    def test_the_defias_bandit_display_loads(self):
        # mobs.toml template 96011, the Defias Bandit, display 2358.
        gd = wowmpq.GameData(DATA)
        self.addCleanup(lambda: [a.f.close() for a in gd.archives])
        self.assertIsNone(ground_check.Models(gd).creature(2358))


class TableKinds(unittest.TestCase):
    def test_every_table_kind_the_gate_reads_is_a_creature_or_a_gameobject(self):
        # A new kind would fall through the water and model rules unchecked: name it in
        # CREATURE_KINDS or GAMEOBJECT_KINDS.
        known = ground_check.CREATURE_KINDS | ground_check.GAMEOBJECT_KINDS
        for path in sorted(glob.glob(os.path.join(ground_check.CONTENT, "*.toml"))):
            for r in ground_check.rows(path)[1]:
                self.assertIn(r.kind, known, "%s:%d" % (os.path.basename(path), r.idx + 1))


def quat_matrix(x, y, z, w):
    """The rotation matrix of unit quaternion (x, y, z, w), rows first."""
    return (
        (1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)),
        (2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)),
        (2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)),
    )


# WoW to Bevy, bevy = B wow: (x, y, z) -> (-y, z, -x) (benilla-assets coords::wow_to_bevy).
WOW_TO_BEVY = ((0, -1, 0), (0, 0, 1), (-1, 0, 0))


def transpose(m):
    return tuple(tuple(m[j][i] for j in range(3)) for i in range(3))


class PlacementRotation(unittest.TestCase):
    # crates/benilla-assets/src/coords.rs, placement_rotation_goldens: the MDDF rotation in
    # degrees and the Bevy-space quaternion (x, y, z, w) the client places a model with. The
    # server's collision.rs Placement turns models the same way.
    GOLDENS = [
        ((86.0, 252.0, 93.5), (-0.691160, -0.107332, -0.156292, 0.697388)),
        ((0.0, 0.0, 90.0), (math.sqrt(0.5), -math.sqrt(0.5), 0.0, 0.0)),
        ((30.0, 45.0, 60.0), (0.360423, -0.822363, -0.391904, 0.200562)),
    ]

    def test_the_python_placement_matches_the_clients_goldens(self):
        b = WOW_TO_BEVY
        for deg, q in self.GOLDENS:
            # The WoW-frame rotation of the Bevy one: B^T R B.
            want = doodad_check._mul(doodad_check._mul(transpose(b), quat_matrix(*q)), b)
            got = doodad_check.placement_matrix(deg)
            for i in range(3):
                for j in range(3):
                    self.assertAlmostEqual(got[i][j], want[i][j], places=4, msg="%s [%d][%d]" % (deg, i, j))


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
