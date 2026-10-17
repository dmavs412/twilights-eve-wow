"""Where the wild camps of the early ring go: the terrain survey in, camp sites out.

`make_crossroads_low_camps.py` keeps its eleven hand-placed camps and asks `plan()` for the rest:

  * landmark camps, found by reading the ADTs: a hut, a tent, a wrecked caravan, a bone field, a
    stand of trees. A camp stands by each one that sits on open Barrens ground;
  * filler camps on a loose grid of flat ground until the ring holds `TARGET` new camps;
  * two rares, one west and one east, each alone.

Three sectors (wedges of 120 degrees round the Crossroads) each have a home species: boars,
scorpids and striders. A camp is mostly its sector's species (70%) with a few of another's, and
its level climbs with the distance from the Crossroads. A camp stands off the roads (the ADT's
road texture), off every WMO and on flat ground, and no camp is red to another one nearby: the
realm's keep-out rule (`tests/barrens.rs`) holds because `compatible()` says so.
"""

from __future__ import annotations

import math
import random

CROSSROADS = (-441.0, -2596.0)
# The realm's Hub (`content.rs` CROSSROADS): where the walking tests start from. A camp must pass
# their rings measured from here as well as from the town's middle above.
HUB = (-417.0, -2646.0)
HUB_RING_TOP = 597.0        # tests/barrens.rs: every camp's centre is within 600 yd of the Hub

RING = (235.0, 585.0)       # a camp's centre is this far from the Crossroads
# The two Hunting Grounds (tests/content_inner_ring.rs): a quest's targets all spawn in its giver's
# ground, so a species lives in one ground only. The Near Plains hold the levels 1-4 species out to
# NEAR_TOP, the Far Fields the levels 5-10 ones from FAR_FROM; a camp (its members stand up to
# MEMBER_RADIUS[1] off its centre) never straddles the line.
NEAR_TOP = 366.0
FAR_FROM = 394.0
NEAR_LEVELS = (1.0, 4.2)    # the target level at the ring's inner edge and at NEAR_TOP
FAR_LEVELS = (5.0, 10.0)    # ... and at FAR_FROM and the ring's outer edge
NEAR_SPECIES = {2955, 3098, 1984, 1985, 2966, 3124}
FAR_SPECIES = {2956, 2957, 3099, 3100, 3125, 3126, 3127}
SPACING = 100.0             # the nearest two camp centres stand to each other
LANDMARK_SPACING = 85.0
MEMBER_RADIUS = (9.0, 13.0)
TARGET = 34                 # new camps, rares not counted
WANDER = 8.0
RARE_WANDER = 25.0
ROAD_MARGIN = 30.0          # from a camp's centre
MEMBER_ROAD_MARGIN = 10.0
VIEW = 100.0                # the realm's view distance (population.rs VIEW_YD)
ROUTE_CLEAR = 40.0          # tests/barrens.rs: no red mob this near the way to a camp
RED_ABOVE = 5
STRAY_TOL = 3.5             # a member's ground may stand this far above or below its camp's
MAX_GROUND_Z = 110.0        # the plateau tops (z 150-210) are no place for a camp: the ground here is ~92

# Sector home species as (entry, level_min, level_max), by level.
SECTORS = {
    "striders": [(2955, 1, 2), (2956, 6, 7), (2957, 8, 9)],
    "boars": [(1984, 1, 2), (3098, 1, 2), (1985, 2, 3), (2966, 3, 4), (3099, 6, 7), (3100, 8, 9)],
    "scorpids": [(3124, 3, 3), (3125, 5, 6), (3126, 7, 8), (3127, 9, 10)],
}
# Each sector's wedge, as the bearing (degrees) of its middle: the Crossroads' south-west, then
# every 120 degrees round. Bearing is atan2(dy, dx) in the realm's x (north), y (west).
SECTOR_MIDDLE = {"striders": 150.0, "boars": 270.0, "scorpids": 30.0}
BLEND = 0.3                  # the share of a camp that is another sector's species
LEVEL_SLACK = 1.8            # a species this far from the target level still fits

# The rares: (entry, level, sector wedge middle (degrees), wander). One west, one east, each past
# the farthest the walking tests look (440 yd out and a view distance more).
RARES = [(5807, 10, 100.0), (3056, 12, 265.0)]
RARE_RING = (570.0, 596.0)  # from the Hub

LANDMARK_WMOS = ("taurenhuts/", "orctwostory", "orchut", "centaurtents/", "trollburrow", "orctower")
LANDMARK_DOODADS = ("tent", "hut", "wagon", "cage", "bone", "skull", "pyre", "totem", "mule", "sleepmat",
                    "banner", "catapult", "leanto", "windbreak", "roast", "weapon")
GROVE = ("barrenstree", "deadtreelog")
TOWN_YD = 235.0
LABELS = {"grove": "stand of trees (%s)", "ruin": "ruined camp (%s)"}


def dist(a, b):
    return math.hypot(a[0] - b[0], a[1] - b[1])


def bearing(x, y):
    return math.degrees(math.atan2(y - CROSSROADS[1], x - CROSSROADS[0])) % 360.0


def sector_of(x, y):
    b = bearing(x, y)
    return min(SECTOR_MIDDLE, key=lambda s: min(abs(b - SECTOR_MIDDLE[s]) % 360.0,
                                                 360.0 - abs(b - SECTOR_MIDDLE[s]) % 360.0))


def target_level(d):
    """The level a camp `d` yards out is for, or None on the line between the grounds."""
    if d <= NEAR_TOP:
        t = (max(d, RING[0]) - RING[0]) / (NEAR_TOP - RING[0])
        return NEAR_LEVELS[0] + (NEAR_LEVELS[1] - NEAR_LEVELS[0]) * t
    if d >= FAR_FROM:
        t = (min(d, RING[1]) - FAR_FROM) / (RING[1] - FAR_FROM)
        return FAR_LEVELS[0] + (FAR_LEVELS[1] - FAR_LEVELS[0]) * t
    return None


def to_segment(p, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    n = dx * dx + dy * dy
    t = 0.0 if n == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / n))
    return dist(p, (a[0] + t * dx, a[1] + t * dy))


class Site:
    """A camp as the compatibility check sees it."""

    def __init__(self, centre, members, wander):
        self.centre = centre
        self.members = members          # [(x, y, level_min, level_max)]
        self.wander = wander
        self.lmin = min(m[2] for m in members)
        self.lmax = max(m[3] for m in members)


def red_to(a, b):
    """Whether camp `b` holds a mob red to the players of camp `a`."""
    return b.lmax >= a.lmax + RED_ABOVE


def compatible(a, b):
    """No camp is red to the other's players in view of its centre or on the way to it."""
    for p, q in ((a, b), (b, a)):
        if not red_to(p, q):
            continue
        for x, y, lo, hi in q.members:
            if hi < p.lmax + RED_ABOVE:
                continue
            if dist((x, y), p.centre) - q.wander <= VIEW + 10.0:
                return False
            for origin in (CROSSROADS, HUB):
                if to_segment((x, y), origin, p.centre) - q.wander <= ROUTE_CLEAR + 25.0:
                    return False
    return True


def area_ok(terrain, area_rows, x, y, barrens=17):
    a = terrain.area_id(x, y)
    if a is None:
        return False
    parent = next((r[2] for r in area_rows if r[0] == a), None)
    return a == barrens or (parent == barrens and a not in (380, 458))


class Planner:
    def __init__(self, survey, area_rows, templates, existing, seed=1712):
        self.s = survey
        self.t = survey.terrain
        self.area_rows = area_rows
        self.templates = templates
        self.rng = random.Random(seed)
        self.sites = list(existing)     # Site, for every camp so far
        self.centres = [s.centre for s in existing]
        self.wmos = [w for w in survey.wmos if not w[0].startswith("world/wmo/dungeon")]

    # ---- ground

    def near_wmo(self, x, y, margin):
        return any(xs[0] - margin <= x <= xs[1] + margin and ys[0] - margin <= y <= ys[1] + margin
                   for _, xs, ys in self.wmos)

    def spot_ok(self, x, y, road_margin, half=5.0):
        z = self.t.ground_z(x, y)
        return (z is not None and z <= MAX_GROUND_Z
                and area_ok(self.t, self.area_rows, x, y)
                and not self.s.near_road(x, y, road_margin)
                and not self.near_wmo(x, y, 2.5)
                and self.s.flat(x, y, half, STRAY_TOL))

    # ---- species

    def pick_species(self, sector, level, n):
        """`n` species entries for a camp of `sector` near `level`: mostly home, some other."""
        ground = NEAR_SPECIES if level < 4.5 else FAR_SPECIES

        def fits(e):
            lo, hi = self.templates[e]["level_min"], self.templates[e]["level_max"]
            return e in ground and abs((lo + hi) / 2.0 - level) <= LEVEL_SLACK and hi <= 10
        home = [e for e, *_ in SECTORS[sector] if fits(e)]
        other = [e for s, ladder in SECTORS.items() if s != sector for e, *_ in ladder if fits(e)]
        if not home:
            home, other = other, []
        if not home:
            return None
        out = []
        for _ in range(n):
            out.append(self.rng.choice(other) if other and self.rng.random() < BLEND else self.rng.choice(home))
        if len(out) >= 3 and other and not any(e not in home for e in out):
            out[-1] = self.rng.choice(other)
        # the home species leads the camp, so the camp's first member is the camp's own
        out.sort(key=lambda e: e not in home)
        return out

    # ---- a camp

    def camp_at(self, cx, cy, spacing, ring=RING):
        d = dist((cx, cy), CROSSROADS)
        if not ring[0] <= d <= ring[1] or dist((cx, cy), HUB) > HUB_RING_TOP:
            return None
        if any(dist((cx, cy), c) < spacing for c in self.centres):
            return None
        if not self.spot_ok(cx, cy, ROAD_MARGIN, half=7.0):
            return None
        level = target_level(d)
        if level is None:
            return None
        n = self.rng.choice((2, 3, 3, 4, 4))
        entries = self.pick_species(sector_of(cx, cy), level, n)
        if not entries:
            return None
        members, spin = [], self.rng.uniform(0, math.tau)
        for i, e in enumerate(entries):
            if i == 0 and len(entries) == 1:
                members.append((e, 0, 0))
                continue
            r = self.rng.uniform(*MEMBER_RADIUS)
            a = spin + math.tau * i / len(entries) + self.rng.uniform(-0.3, 0.3)
            dx, dy = round(r * math.cos(a)), round(r * math.sin(a))
            if not self.spot_ok(cx + dx, cy + dy, MEMBER_ROAD_MARGIN, half=3.0):
                return None
            members.append((e, dx, dy))
        return self.accept(cx, cy, members, WANDER)

    def accept(self, cx, cy, members, wander):
        site = Site((cx, cy), [(cx + dx, cy + dy, self.templates[e]["level_min"], self.templates[e]["level_max"])
                               for e, dx, dy in members], wander)
        if any(not compatible(site, o) for o in self.sites):
            return None
        self.sites.append(site)
        self.centres.append((cx, cy))
        return members, site

    # ---- landmarks

    def landmarks(self):
        """(x, y, what) of the landmarks inside the ring, the biggest first."""
        found = []
        for path, xs, ys in self.wmos:
            if any(w in path for w in LANDMARK_WMOS):
                p = ((xs[0] + xs[1]) / 2, (ys[0] + ys[1]) / 2)
                found.append((p[0], p[1], "building (%s)" % path.rsplit("/", 1)[-1][:-4], 10))
        cells = {}
        for x, y, z, sc, name in self.s.doodads:
            kind = ("grove" if any(g in name for g in GROVE)
                    else "ruin" if any(k in name for k in LANDMARK_DOODADS) else None)
            if kind and RING[0] - 40 <= dist((x, y), CROSSROADS) <= RING[1] + 40 and "fence" not in name:
                cells.setdefault((kind, int(x // 70), int(y // 70)), []).append((x, y, name))
        for (kind, _, _), v in cells.items():
            if len(v) < (3 if kind == "grove" else 4):
                continue
            x, y = sum(a[0] for a in v) / len(v), sum(a[1] for a in v) / len(v)
            names = sorted({a[2] for a in v}, key=lambda n: -sum(1 for a in v if a[2] == n))
            found.append((x, y, LABELS[kind] % names[0].rsplit("/", 1)[-1].removesuffix(".mdx"), len(v)))
        found.sort(key=lambda f: (-f[3], f[0], f[1]))
        return found

    def landmark_camp(self, lx, ly):
        """A camp as near the landmark as the ground allows, or None."""
        for r in range(0, 61, 6):
            for k in range(1 if r == 0 else 12):
                a = math.tau * k / 12
                cx, cy = round(lx + r * math.cos(a)), round(ly + r * math.sin(a))
                got = self.camp_at(cx, cy, LANDMARK_SPACING)
                if got:
                    return got
        return None

    # ---- the plan

    def plan(self):
        """[(key, cx, cy, members, wander, note)] for the landmark camps, the fillers and the rares."""
        out = [self.rare(*r) for r in RARES]
        n = 0
        for lx, ly, what, _ in self.landmarks():
            if n >= TARGET // 2 + 4:
                break
            got = self.landmark_camp(lx, ly)
            if not got:
                continue
            members, site = got
            n += 1
            out.append(("landmark_%d" % n, site.centre[0], site.centre[1], members, WANDER,
                        "%s, by a %s at (%d, %d), level %d-%d." % (self.describe(members), what, lx, ly,
                                                                    site.lmin, site.lmax)))
        grid = [(x, y) for x in range(-1100, 300, 22) for y in range(-3300, -1900, 22)]
        self.rng.shuffle(grid)
        for x, y in grid:
            if n >= TARGET:
                break
            got = self.camp_at(x, y, SPACING)
            if not got:
                continue
            members, site = got
            n += 1
            out.append(("wild_%d" % n, x, y, members, WANDER,
                        "%s, on open ground, level %d-%d." % (self.describe(members), site.lmin, site.lmax)))
        return out

    def describe(self, members):
        """'Dire Mottled Boar x2, Adult Plainstrider' for a camp's members."""
        counts = {}
        for e, _, _ in members:
            counts[e] = counts.get(e, 0) + 1
        return ", ".join(self.templates[e]["name"] + (" x%d" % n if n > 1 else "") for e, n in counts.items())

    def rare(self, entry, level, middle):
        best = None
        for k in range(720):
            b = math.radians(middle + (k % 2 * 2 - 1) * (k // 2) * 0.5)
            for d in range(int(RARE_RING[0]), int(RARE_RING[1]) + 1, 5):
                cx = round(HUB[0] + d * math.cos(b))
                cy = round(HUB[1] + d * math.sin(b))
                if not self.spot_ok(cx, cy, ROAD_MARGIN + 10.0, half=7.0):
                    continue
                t = self.templates[entry]
                site = Site((cx, cy), [(cx, cy, level, level)], RARE_WANDER)
                if any(dist((cx, cy), c) < 2 * SPACING for c in self.centres):
                    continue
                if any(not compatible(site, o) for o in self.sites):
                    continue
                best = (cx, cy, site)
                break
            if best:
                break
        assert best, "no place for rare %d" % entry
        cx, cy, site = best
        self.sites.append(site)
        self.centres.append((cx, cy))
        return ("rare_%s" % self.templates[entry]["name"].lower().replace(" ", "_"), cx, cy,
                [(entry, 0, 0)], RARE_WANDER,
                "The rare %s (level %d), alone %d yd %s of the Hub. It takes ten minutes to come back."
                % (self.templates[entry]["name"], level, dist((cx, cy), HUB),
                   "west" if cy > CROSSROADS[1] else "east"))
