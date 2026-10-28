"""Named bosses: every `boss = true` template in content/*.toml, fought one at a time.

The Boss mixin goes ahead of a multi.py Calling class. The boss casts its template's spells (a
[[kit]]'s replace them) as creature_cast.rs does: one decision a second, the first ready spell
in list order, no swings while it casts. Each spell is modelled from its creature_spells.toml row:
a periodic damage or ground patch ticks on the character, a nova or gaze lands as direct damage,
a hit is a share of a swing, a heal restores a share of the boss's health, a stance changes what
it deals and takes. Any other effect (fear, shield, root, snare, charge, armour debuff, a spell
that needs a corpse) is skipped and listed as `unmodelled`. The character kicks with its Calling's
Kick row on its cooldown: every heal cast, and otherwise the longest damage cast.
"""
from __future__ import annotations

import glob
import os
import re
import tomllib

import model
import multi

THINK_S = 1.0            # creature_cast.rs THINK_SECS
KICK_REACTION_S = 0.5    # the kick lands this long into a cast it targets
BOSS_CAP_S = 300.0
MODELLED = {"periodic_damage", "ground_patch", "nova", "gaze", "hit", "heal", "stance"}
DAMAGE = {"periodic_damage", "ground_patch", "nova", "gaze", "hit"}
NORTHSHIRE_WORDS = ("Northshire", "Abbey", "Echo Ridge")


def _comments(text):
    """Each template's entry and the comment lines above and inside its block. A blank line ends
    the block's inside, so the comment over the next block is that block's."""
    out, pending, current, inside = {}, [], None, False
    for line in text.splitlines():
        s = line.strip()
        if s.startswith("[["):
            current = None
            if s == "[[template]]":
                current = {"comments": pending}
            pending, inside = [], current is not None
        elif s.startswith("#"):
            (current["comments"] if inside else pending).append(s)
        elif not s:
            if inside:
                inside = False
            else:
                pending = []
        else:
            m = re.match(r"entry\s*=\s*(\d+)", s)
            if current is not None and m and "entry" not in current:
                current["entry"] = int(m.group(1))
                out[current["entry"]] = current["comments"]
    return out


def _spawn_comments(text):
    """Each entry a non-template block places (one with a `position`) and the comment lines
    between the key line before that block and the block's own: its paragraph and any section
    heading over it."""
    out, since, block = {}, [], None
    for line in text.splitlines():
        s = line.strip()
        if s.startswith("[["):
            block = None if s == "[[template]]" else {"comments": since, "entry": None}
            since = []
        elif s.startswith("#"):
            since.append(s)
        elif s:
            since = []
            m = re.match(r"entry\s*=\s*(\d+)", s)
            if block is not None and m and block["entry"] is None:
                block["entry"] = int(m.group(1))
            if block is not None and block["entry"] is not None and s.startswith("position"):
                out.setdefault(block["entry"], []).extend(block["comments"])
    return out


def _band_levels():
    """level_overrides.toml: each creature's levels from its Level Band, with the band's name."""
    out = {}
    for area in model.DATA.toml("level_overrides.toml").get("area", []):
        for c in area.get("creature", []):
            out[c["entry"]] = (c["levels"], area["name"])
    return out


def kick_row(calling):
    """The Calling's Kick (spec #334): its first interrupt that works on bosses."""
    for sk in model.SPELLS[calling]["skills"]:
        for e in sk.get("effects", []):
            if e["kind"] == "interrupt" and e["params"].get("bosses", True) and e["params"].get("lockout_ms"):
                return dict(key=sk["key"], cooldown_s=sk["realm_cooldown_ms"] / 1000,
                            lockout_s=e["params"]["lockout_ms"] / 1000)
    return None


def named_bosses():
    """Every named boss with the level, unit class, multipliers, Zone Factor area and spells the
    model fights it at, and the spells it leaves out."""
    content = model.DATA.content
    files = sorted(glob.glob(os.path.join(content, "*.toml")))
    parsed = []
    kits = {}
    spawns = {}      # entry -> [(its spawn file's name, that file's zone or None, comments)]
    for path in files:
        with open(path, "rb") as f:
            raw = f.read()
        data = tomllib.loads(raw.decode("utf-8"))
        parsed.append((path, raw.decode("utf-8"), data))
        for k in data.get("kit", []):
            if "spells" in k:
                kits[k["entry"]] = k["spells"]
        zone = data.get("zone") if isinstance(data.get("zone"), int) else None
        name = os.path.splitext(os.path.basename(path))[0].replace("_", " ")
        for entry, comments in _spawn_comments(raw.decode("utf-8")).items():
            spawns.setdefault(entry, []).append((name, zone, comments))
    bands = _band_levels()
    bosses = []
    for path, text, data in parsed:
        notes = _comments(text)
        for t in data.get("template", []):
            if not t.get("boss"):
                continue
            entry = t["entry"]
            band = bands.get(entry)
            level = max(band[0]) if band else t["level_max"]
            placed = spawns.get(entry, [])
            words = " ".join([*notes.get(entry, []), band[1] if band else "",
                              *(f"{name} {' '.join(comments)}" for name, _, comments in placed)]).lower()
            dungeon = next((zone for _, zone, _ in placed if zone is not None), None)
            if dungeon is not None:
                area, why = dungeon, f"its spawn's file's zone {dungeon}"
            elif isinstance(data.get("zone"), int):
                area, why = data["zone"], f"its file's zone {data['zone']}"
            elif any(w.lower() in words for w in NORTHSHIRE_WORDS):
                area, why = model.NORTHSHIRE_VALLEY, "Northshire, from its comments, band or spawn"
            else:
                area, why = model.ELWYNN_FOREST, "Elwynn Forest, the default"
            keys = kits.get(entry, t.get("spells", []))
            spells, unmodelled = [], []
            for key in keys:
                sp = model.DATA.creature_spells.get(key)
                if sp and sp["effect"]["kind"] in MODELLED and "corpse_within" not in sp:
                    spells.append(sp)
                else:
                    unmodelled.append(key)
            bosses.append(dict(entry=entry, name=t["name"], file=os.path.basename(path), level=level,
                               unit_class=t["unit_class"], rank=t.get("rank", 0),
                               health_mul=t["health_multiplier"], damage_mul=t["damage_multiplier"],
                               armor_mul=t.get("armor_multiplier", 1.0),
                               swing_s=t.get("base_attack_time", 2000) / 1000, area=area, area_why=why,
                               spells=spells, unmodelled=unmodelled))
    return bosses


def boss_mob(b):
    return model.mob_stats(b["level"], b["unit_class"], b["rank"], b["area"], b["health_mul"], b["damage_mul"],
                           b["armor_mul"], b["swing_s"], b["name"])


def kickable(spells):
    """The keys the character kicks: every heal with a cast, and the longest damage cast."""
    keys = {s["key"] for s in spells if s["effect"]["kind"] == "heal" and s["cast_time_ms"]}
    hurts = [s for s in spells if s["effect"]["kind"] in DAMAGE and s["cast_time_ms"]]
    if hurts:
        keys.add(max(hurts, key=lambda s: s["cast_time_ms"])["key"])
    return frozenset(keys)


def fight_class(calling, b):
    """(class, mob row) for `calling` against the named boss `b`."""
    cls = type(f"{calling}_vs_{b['entry']}", (Boss, multi.CLASSES[calling]),
               dict(boss_spells=tuple(b["spells"]), kick_keys=kickable(b["spells"]), kick=kick_row(calling)))
    return cls, boss_mob(b)


def _cooldown_s(sp, rng, shortest=False):
    cd = sp["cooldown_ms"]
    if isinstance(cd, list):
        cd = cd[0] if shortest else rng.randint(cd[0], cd[1])
    return cd / 1000


class Boss:
    """Mob 0 casts `boss_spells`; the character kicks `kick_keys` with `kick`."""
    boss_spells = ()
    kick_keys = frozenset()
    kick = None

    def fight(self):
        t = self.t
        self.next_think = t
        self.casting = None          # (spell, starts, lands)
        self.ready_at = {sp["key"]: t + sp.get("first_cast_ms", 0) / 1000 for sp in self.boss_spells}
        self.locked_until = {}       # school -> time
        self.kick_ready = t
        self.boss_dots = {}          # key -> [until, next, tick, every]
        self.stance = None           # (dealt pct, taken pct)
        return super().fight()

    def cap_s(self):
        return BOSS_CAP_S

    def mob_swing(self, *a, **kw):
        if self.casting:
            return 0, "casting"
        d, o = super().mob_swing(*a, **kw)
        if self.stance and d:
            d = int(d * (100 + self.stance[0]) / 100)
        return d, o

    def hit_mob(self, raw, src, *a, **kw):
        if self.stance:
            raw = raw * (100 + self.stance[1]) / 100
        return super().hit_mob(raw, src, *a, **kw)

    def hurt(self, d):
        d = self.take(int(d))
        if hasattr(self, "rage_taken"):
            self.rage_taken(d)

    def land(self, sp):
        e, t, level = sp["effect"], self.t, self.m["level"]
        kind = e["kind"]
        if kind in ("periodic_damage", "ground_patch"):
            every = e["tick_ms"] / 1000
            self.boss_dots[sp["key"]] = [t + e["duration_ms"] / 1000, t + every, model.linear(e["tick"], level), every]
        elif kind in ("nova", "gaze"):
            self.hurt(e["damage"])
        elif kind == "hit":
            swing = (self.m["dmin"] + self.m["dmax"]) / 2 * e["pct_of_swing"] / 100
            self.hurt(model.armored(swing, self.player_armor(), level))
        elif kind == "heal":
            self.mobs[0] = min(self.mobs[0] + self.m["hp"] * e["pct_max_health"] // 100, self.m["hp"])
        elif kind == "stance":
            self.stance = (e["damage_dealt_pct"], e["damage_taken_pct"])

    def after_step(self):
        t = self.t
        for key, dot in list(self.boss_dots.items()):
            if t >= dot[0]:
                del self.boss_dots[key]
            elif t >= dot[1]:
                dot[1] += dot[3]
                self.hurt(dot[2])
        if self.mobs[0] <= 0:
            self.casting = None
            return
        if self.casting:
            sp, starts, lands = self.casting
            if (self.kick and sp["key"] in self.kick_keys and t >= self.kick_ready
                    and t >= starts + KICK_REACTION_S):
                self.kick_ready = t + self.kick["cooldown_s"]
                self.casting = None
                self.ready_at[sp["key"]] = t + _cooldown_s(sp, self.rng, shortest=True)
                self.locked_until[sp["school"]] = t + self.kick["lockout_s"]
            elif t >= lands:
                self.casting = None
                self.ready_at[sp["key"]] = t + _cooldown_s(sp, self.rng)
                self.land(sp)
            return
        if t < self.next_think:
            return
        self.next_think = t + THINK_S
        health, most = self.mobs[0], self.m["hp"]
        for sp in self.boss_spells:
            pct = sp.get("below_health_pct")
            if (t >= self.ready_at[sp["key"]] and t >= self.locked_until.get(sp["school"], -1)
                    and (pct is None or health * 100 < most * pct)
                    and not (sp["effect"]["kind"] == "stance" and self.stance)):
                break
        else:
            return
        if self.rng.random() * 100 >= sp.get("chance", 100):
            return
        if sp["cast_time_ms"]:
            self.casting = (sp, t, t + sp["cast_time_ms"] / 1000)
        else:
            self.ready_at[sp["key"]] = t + _cooldown_s(sp, self.rng)
            self.land(sp)
