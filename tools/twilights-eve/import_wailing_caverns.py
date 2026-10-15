#!/usr/bin/env python3
"""Import the Wailing Caverns (instance map 43) for Twilight's Eve ticket #16.

Offline, re-runnable. Reads:
  * a vmangos world database as SQLite (the `mangos.sqlite` of a vmangos sqlite dump), and
  * the 1.12.1 client's MPQs (AreaTrigger, WorldSafeLocs, Map, LoadingScreens DBCs and the WDT),
and writes:
  * data/twilights-eve/wailing_caverns_spawns_full.json - every creature spawn on map 43
  * data/twilights-eve/wailing_caverns_run.json         - the slice's run: entrance, exit,
    death respawn, ~10 hand-picked trash mobs, one boss and the route graph through the tunnels.

Usage (paths default to this machine's layout; pass your own otherwise):

  python -I tools/twilights-eve/import_wailing_caverns.py \
      --db   <vmangos mangos.sqlite> \
      --data <WoW 1.12.1 Data dir> \
      --out  data/twilights-eve

Only the standard library is used. The MPQ reader handles what 1.12.1 ships: hashed names,
encrypted hash/block tables, sectored files with zlib (0x02) sectors, and stored files.
See docs/reference/wailing-caverns.md for what the numbers mean and why these mobs.
"""

import argparse
import json
import math
import os
import sqlite3
import struct
import sys
import zlib

MAP_ID = 43                # Map.dbc "WailingCaverns"
OUTSIDE_MAP_ID = 1         # Kalimdor
ZONE_ID = 718              # AreaTable "Wailing Caverns", the instance's zone (map_template.linked_zone)
ENTRANCE_TRIGGER = 228     # AreaTrigger.dbc on map 1: "Wailing Caverns - Entrance"
EXIT_TRIGGER = 226         # AreaTrigger.dbc on map 43: "Wailing Caverns - Exit"
ESCORT_ENTRY = 3678        # Disciple of Naralex: script_waypoint walks the tunnels from the entrance
VMANGOS_PATCH = 10         # 1.12 content patch in vmangos' patch_min/patch_max scheme

# ---------------------------------------------------------------------------------------------
# The slice's hand-picked run (spawn guids from the vmangos `creature` table).
#
# Boss: Lady Anacondra at her classic room, guid 300999. vmangos authors four Anacondra spawns
# (27366, 300999, 301000, 301001) and the instance script keeps one; 300999 sits exactly on the
# Disciple of Naralex escort path (points 14-15), ~210 yd of walking from the instance entrance,
# on dry ground, standing still. Kresh (the other "near" boss) patrols in deep water at z -106
# (a swimming turtle), which the realm's ground-height host and the route graph do not model.
#
# Trash: the ten mobs that stand within a few yards of the escort path between the entrance and
# Anacondra, in route order. Groups (creature_groups) are kept whole: 26220+26224.
# ---------------------------------------------------------------------------------------------
BOSS_GUID = 300999
TRASH_GUIDS = [
    26202,  # Deviate Guardian      first bend north of the entrance (escort 3-4)
    26220,  # Deviate Ravager       pack leader (escort 6-7)
    26224,  # Deviate Ravager       its group member
    26216,  # Deviate Ravager       east turn (escort 9-10)
    26223,  # Deviate Ravager
    26217,  # Deviate Guardian      the low hall (escort 11-12)
    26227,  # Deviate Ravager
    26230,  # Deviate Ravager       approach to Anacondra's room (escort 14)
    26239,  # Druid of the Fang     short patrol in front of the boss
    26252,  # Druid of the Fang     the boss's guard
]
# Escort points used for the route: from the start (0) to Anacondra's room (15).
ESCORT_LAST_POINT = 15
# A spur edge from a mob's spawn to the route is drawn only if it is at most this long (yd).
MAX_SPUR = 20.0
# Patrol paths (creature_movement) whose points are added to the graph as extra walkable chains.
# 26225's Druid of the Fang patrols from the low hall (escort 11) south to Anacondra's other
# spawn room at (-68, 122), so it is a second, tunnel-following corridor.
EXTRA_PATROL_GUIDS = [26225]

# ---------------------------------------------------------------------------------------------
# MPQ (1.12.1 envelope)
# ---------------------------------------------------------------------------------------------


def _crypt_table():
    seed = 0x00100001
    table = [0] * 0x500
    for i1 in range(0x100):
        for i2 in range(5):
            idx = i1 + i2 * 0x100
            seed = (seed * 125 + 3) % 0x2AAAAB
            t1 = (seed & 0xFFFF) << 0x10
            seed = (seed * 125 + 3) % 0x2AAAAB
            t2 = seed & 0xFFFF
            table[idx] = t1 | t2
    return table


CRYPT = _crypt_table()
M32 = 0xFFFFFFFF


def mpq_hash(name, kind):
    seed1, seed2 = 0x7FED7FED, 0xEEEEEEEE
    for b in name.encode('ascii'):
        ch = ord('\\') if b == ord('/') else (b - 32 if 97 <= b <= 122 else b)
        seed1 = (CRYPT[kind + ch] ^ ((seed1 + seed2) & M32)) & M32
        seed2 = (ch + seed1 + seed2 + ((seed2 << 5) & M32) + 3) & M32
    return seed1


def mpq_decrypt(words, key):
    seed = 0xEEEEEEEE
    out = []
    for v in words:
        seed = (seed + CRYPT[0x400 + (key & 0xFF)]) & M32
        ch = v ^ ((key + seed) & M32)
        out.append(ch)
        key = ((((~key) << 0x15) & M32) + 0x11111111 & M32) | (key >> 0x0B)
        seed = (ch + seed + ((seed << 5) & M32) + 3) & M32
    return out


class Mpq:
    def __init__(self, path):
        self.path = path
        with open(path, 'rb') as f:
            data = f.read(1 << 20)
            off = 0
            while True:
                if data[off:off + 4] == b'MPQ\x1a':
                    break
                off += 512
                if off + 32 > len(data):
                    raise ValueError(f'{path}: no MPQ header')
            self.base = off
            hdr = data[off:off + 32]
            (_, _, _, _, shift, ht_pos, bt_pos, ht_n, bt_n) = struct.unpack('<4sIIHHIIII', hdr)
            self.sector = 512 << shift
            f.seek(off + ht_pos)
            ht = mpq_decrypt(struct.unpack(f'<{ht_n * 4}I', f.read(ht_n * 16)),
                             mpq_hash('(hash table)', 0x300))
            f.seek(off + bt_pos)
            bt = mpq_decrypt(struct.unpack(f'<{bt_n * 4}I', f.read(bt_n * 16)),
                             mpq_hash('(block table)', 0x300))
        self.hashes = [tuple(ht[i * 4:i * 4 + 4]) for i in range(ht_n)]
        self.blocks = [tuple(bt[i * 4:i * 4 + 4]) for i in range(bt_n)]

    def _find(self, name):
        n = len(self.hashes)
        i = mpq_hash(name, 0) % n
        a, b = mpq_hash(name, 0x100), mpq_hash(name, 0x200)
        for _ in range(n):
            ha, hb, _loc, bi = self.hashes[i]
            if bi == 0xFFFFFFFF:
                return None
            if ha == a and hb == b and bi != 0xFFFFFFFE:
                return self.blocks[bi]
            i = (i + 1) % n
        return None

    def read(self, name):
        blk = self._find(name)
        if blk is None:
            return None
        pos, csize, size, flags = blk
        if not flags & 0x80000000 or flags & 0x02000000:
            return None
        if flags & 0x00010000 or flags & 0x01000000:
            raise ValueError(f'{name}: encrypted/single-unit file outside the 1.12.1 envelope')
        with open(self.path, 'rb') as f:
            f.seek(self.base + pos)
            raw = f.read(csize)
        if not flags & 0x00000300:
            return raw[:size]
        nsec = (size + self.sector - 1) // self.sector
        offs = struct.unpack(f'<{nsec + 1}I', raw[:(nsec + 1) * 4])
        out = bytearray()
        for i in range(nsec):
            want = min(self.sector, size - i * self.sector)
            s = raw[offs[i]:offs[i + 1]]
            if len(s) >= want:
                out += s[:want]
            elif s[0] == 0x02:
                out += zlib.decompress(s[1:])
            else:
                raise ValueError(f'{name}: sector codec {s[0]:#x} unsupported here')
        return bytes(out)


class Chain:
    """The client's mount order, last wins: patch-2 over patch over the base archives."""
    BASE = ['base.MPQ', 'backup.MPQ', 'dbc.MPQ', 'fonts.MPQ', 'interface.MPQ', 'misc.MPQ',
            'model.MPQ', 'sound.MPQ', 'speech.MPQ', 'terrain.MPQ', 'texture.MPQ', 'wmo.MPQ',
            'patch.MPQ', 'patch-2.MPQ']

    def __init__(self, data_dir):
        present = {n.lower(): n for n in os.listdir(data_dir)}
        self.archives = [Mpq(os.path.join(data_dir, present[n.lower()]))
                         for n in self.BASE if n.lower() in present]
        if not self.archives:
            raise SystemExit(f'no MPQs in {data_dir}')

    def read(self, name):
        for a in reversed(self.archives):
            b = a.read(name)
            if b is not None:
                return b
        raise KeyError(name)


def dbc(chain, name):
    b = chain.read('DBFilesClient\\' + name)
    magic, n, fields, size, ssize = struct.unpack('<4sIIII', b[:20])
    assert magic == b'WDBC', name
    recs = [b[20 + i * size:20 + (i + 1) * size] for i in range(n)]
    strings = b[20 + n * size:]

    def s(off):
        return strings[off:strings.index(b'\0', off)].decode('utf-8', 'replace')
    return recs, s


def u32(r, i):
    return struct.unpack_from('<I', r, i * 4)[0]


def f32(r, i):
    return struct.unpack_from('<f', r, i * 4)[0]


# ---------------------------------------------------------------------------------------------
# Client data
# ---------------------------------------------------------------------------------------------


def client_data(chain):
    out = {}
    recs, _ = dbc(chain, 'AreaTrigger.dbc')
    out['area_triggers'] = {}
    for r in recs:
        tid, m = u32(r, 0), u32(r, 1)
        if tid in (ENTRANCE_TRIGGER, EXIT_TRIGGER) or m == MAP_ID:
            out['area_triggers'][tid] = {
                'id': tid, 'map': m,
                'x': rnd(f32(r, 2)), 'y': rnd(f32(r, 3)), 'z': rnd(f32(r, 4)),
                'radius': rnd(f32(r, 5)),
                'box': [rnd(f32(r, 6)), rnd(f32(r, 7)), rnd(f32(r, 8))],
                'box_yaw': rnd(f32(r, 9)),
            }

    recs, s = dbc(chain, 'Map.dbc')
    for r in recs:
        if u32(r, 0) == MAP_ID:
            out['map'] = {
                'id': MAP_ID, 'directory': s(u32(r, 1)), 'instance_type': u32(r, 2),
                'name': s(u32(r, 4)), 'loading_screen_id': u32(r, 38),
            }
    recs, s = dbc(chain, 'LoadingScreens.dbc')
    for r in recs:
        if u32(r, 0) == out['map']['loading_screen_id']:
            out['map']['loading_screen_name'] = s(u32(r, 1))
            out['map']['loading_screen_blp'] = s(u32(r, 2))

    recs, s = dbc(chain, 'WorldSafeLocs.dbc')
    out['safe_locs'] = {}
    for r in recs:
        out['safe_locs'][u32(r, 0)] = {
            'id': u32(r, 0), 'map': u32(r, 1),
            'x': rnd(f32(r, 2)), 'y': rnd(f32(r, 3)), 'z': rnd(f32(r, 4)), 'name': s(u32(r, 5)),
        }

    d = out['map']['directory']
    wdt = chain.read(f'World\\Maps\\{d}\\{d}.wdt')
    out['wdt'] = parse_wdt(wdt)
    return out


def parse_wdt(b):
    i, res = 0, {'tiles_with_adt': 0}
    while i + 8 <= len(b):
        tag, n = b[i:i + 4][::-1].decode(), struct.unpack_from('<I', b, i + 4)[0]
        body = b[i + 8:i + 8 + n]
        if tag == 'MPHD':
            res['mphd_flags'] = struct.unpack_from('<I', body)[0]
            res['wmo_only'] = bool(res['mphd_flags'] & 1)
        elif tag == 'MAIN':
            res['tiles_with_adt'] = sum(1 for k in range(0, len(body), 8)
                                        if struct.unpack_from('<I', body, k)[0] & 1)
        elif tag == 'MWMO' and n:
            res['wmo'] = body.split(b'\0')[0].decode()
        elif tag == 'MODF' and n >= 64:
            v = struct.unpack_from('<II3f3f3f3fHHHH', body)
            res['modf'] = {
                'position': [rnd(x) for x in v[2:5]], 'rotation': [rnd(x) for x in v[5:8]],
                'extents_min': [rnd(x) for x in v[8:11]], 'extents_max': [rnd(x) for x in v[11:14]],
                'doodad_set': v[15], 'name_set': v[16],
            }
        i += 8 + n
    return res


# ---------------------------------------------------------------------------------------------
# vmangos
# ---------------------------------------------------------------------------------------------

RANKS = {0: 'normal', 1: 'elite', 2: 'rare_elite', 3: 'boss', 4: 'rare'}
BOSS_ENTRIES = {
    3671: 'Lady Anacondra', 3669: 'Lord Cobrahn', 3670: 'Lord Pythas', 3673: 'Lord Serpentis',
    3653: 'Kresh', 3674: 'Skum', 5775: 'Verdan the Everliving', 3654: 'Mutanus the Devourer',
    5912: 'Deviate Faerie Dragon',
}


def rnd(v, n=3):
    return round(float(v), n)


def rows(c, sql, *args):
    cur = c.execute(sql, args)
    cols = [d[0] for d in cur.description]
    return [dict(zip(cols, r)) for r in cur.fetchall()]


def template(c, entry):
    t = rows(c, 'select * from creature_template where entry=? and patch<=? order by patch desc limit 1',
             entry, VMANGOS_PATCH)
    return t[0] if t else None


def class_stats(c, unit_class, level):
    r = rows(c, 'select * from creature_classlevelstats where class=? and level=?', unit_class, level)
    return r[0] if r else None


def creature_record(c, sp, tcache):
    e = sp['id']
    if e not in tcache:
        tcache[e] = template(c, e)
    t = tcache[e]
    displays = [t[f'display_id{i}'] for i in range(1, 5) if t[f'display_id{i}']]
    probs = [t[f'display_probability{i}'] for i in range(1, 5) if t[f'display_id{i}']]
    lv = []
    for level in sorted({t['level_min'], t['level_max']}):
        cs = class_stats(c, t['unit_class'], level)
        if cs:
            lv.append({
                'level': level,
                'health': round(cs['health'] * t['health_multiplier']),
                'mana': round(cs['mana'] * t['mana_multiplier']),
                'armor': round(cs['armor'] * t['armor_multiplier']),
                'melee_damage_avg': rnd(cs['melee_damage'] * t['damage_multiplier'], 2),
                'attack_power': cs['attack_power'],
            })
    moves = rows(c, 'select point, position_x x, position_y y, position_z z, orientation o, waittime '
                    'from creature_movement where id=? order by point', sp['guid'])
    addon = rows(c, 'select * from creature_addon where guid=? order by patch desc limit 1', sp['guid'])
    rec = {
        'guid': sp['guid'], 'entry': e, 'name': t['name'], 'subname': t['subname'] or '',
        'alt_entries': [sp[k] for k in ('id2', 'id3', 'id4', 'id5') if sp[k]],
        'x': rnd(sp['position_x']), 'y': rnd(sp['position_y']), 'z': rnd(sp['position_z']),
        'o': rnd(sp['orientation']),
        'level_min': t['level_min'], 'level_max': t['level_max'],
        'display_ids': displays, 'display_probabilities': probs,
        'scale': rnd(t['display_scale1'] or 1.0),
        'faction': t['faction'], 'npc_flags': t['npc_flags'],
        'static_flags1': t['static_flags1'], 'static_flags2': t['static_flags2'],
        'flags_extra': t['flags_extra'],
        'rank': t['rank'], 'rank_name': RANKS.get(t['rank'], str(t['rank'])),
        'boss': e in BOSS_ENTRIES,
        'type': t['type'], 'unit_class': t['unit_class'],
        'speed_walk': rnd(t['speed_walk']), 'speed_run': rnd(t['speed_run']),
        'detection_range': rnd(t['detection_range']), 'call_for_help_range': rnd(t['call_for_help_range']),
        'leash_range': rnd(t['leash_range']),
        'base_attack_time': t['base_attack_time'], 'damage_school': t['damage_school'],
        'health_multiplier': t['health_multiplier'], 'damage_multiplier': t['damage_multiplier'],
        'stats_by_level': lv,
        'loot_id': t['loot_id'], 'gold_min': t['gold_min'], 'gold_max': t['gold_max'],
        'ai_name': t['ai_name'], 'script_name': t['script_name'],
        'spawn_movement_type': sp['movement_type'], 'wander_distance': rnd(sp['wander_distance']),
        'respawn_secs': sp['spawntimesecsmin'],
        'waypoints': [{k: rnd(v) if isinstance(v, float) else v for k, v in m.items()} for m in moves],
    }
    if addon:
        a = addon[0]
        rec['addon'] = {k: a[k] for k in ('stand_state', 'sheath_state', 'emote_state', 'auras')}
    return rec


def import_spawns(c):
    sp = rows(c, 'select * from creature where map=? and patch_min<=? and patch_max>=? order by guid',
              MAP_ID, VMANGOS_PATCH, VMANGOS_PATCH)
    tcache = {}
    recs = [creature_record(c, s, tcache) for s in sp]
    guids = {r['guid'] for r in recs}
    groups = rows(c, 'select * from creature_groups order by leader_guid, member_guid')
    groups = [g for g in groups if g['leader_guid'] in guids]
    for g in groups:
        g['dist'], g['angle'] = rnd(g['dist']), rnd(g['angle'])
    return recs, groups


# ---------------------------------------------------------------------------------------------
# Route graph
# ---------------------------------------------------------------------------------------------


def dist(a, b):
    return math.dist((a['x'], a['y'], a['z']), (b['x'], b['y'], b['z']))


def build_route(c, by_guid, entrance):
    esc = rows(c, 'select pointid, location_x x, location_y y, location_z z, waittime from script_waypoint '
                  'where entry=? and pointid<=? order by pointid', ESCORT_ENTRY, ESCORT_LAST_POINT)
    nodes, edges = [], []

    def add(nid, p, source, note=None):
        n = {'id': nid, 'x': rnd(p['x'], 2), 'y': rnd(p['y'], 2), 'z': rnd(p['z'], 2), 'source': source}
        if note:
            n['note'] = note
        nodes.append(n)
        return n

    def link(a, b, source):
        edges.append({'a': a['id'], 'b': b['id'], 'length': rnd(dist(a, b), 2), 'source': source})

    start = add('entrance', entrance, 'areatrigger_teleport 228 target', 'instance arrival point')
    prev = start
    chain = []
    for p in esc:
        n = add(f'escort_{p["pointid"]}', p, f'script_waypoint {ESCORT_ENTRY} point {p["pointid"]}')
        link(prev, n, 'entrance->escort' if prev is start else 'escort path')
        chain.append(n)
        prev = n

    # Extra tunnel-following chains from creature patrol paths; each end joins its nearest node.
    for g in EXTRA_PATROL_GUIDS:
        pts = by_guid[g]['waypoints']
        seen, pnodes = set(), []
        for w in pts:
            key = (round(w['x']), round(w['y']))
            if key in seen:
                continue
            seen.add(key)
            pnodes.append(add(f'patrol_{g}_{w["point"]}', w, f'creature_movement guid {g} point {w["point"]}'))
        # A patrol walks back and forth; chain it along its first unique pass.
        pnodes.sort(key=lambda n: n['y'])
        for a, b in zip(pnodes, pnodes[1:]):
            link(a, b, f'patrol {g}')
        for end in (pnodes[0], pnodes[-1]):
            near = min(chain, key=lambda n: dist(n, end))
            if dist(near, end) <= MAX_SPUR:
                link(near, end, 'patrol end -> escort (nearest)')

    graph_core = list(nodes)
    mobs = []
    for i, g in enumerate(TRASH_GUIDS + [BOSS_GUID]):
        r = by_guid[g]
        near = min(graph_core, key=lambda n: dist(n, r))
        d = dist(near, r)
        mn = add(f'mob_{g}', r, f'creature spawn guid {g}')
        if d <= MAX_SPUR:
            link(near, mn, 'spur to spawn (straight line, short)')
        else:
            print(f'warning: guid {g} is {d:.1f} yd from the route; no spur drawn', file=sys.stderr)
        mobs.append((r, mn['id'], near['id'], d))
    return nodes, edges, mobs, chain


def path_length(nodes, edges, src, dst):
    import heapq
    adj = {}
    for e in edges:
        adj.setdefault(e['a'], []).append((e['b'], e['length']))
        adj.setdefault(e['b'], []).append((e['a'], e['length']))
    best, pq = {src: 0.0}, [(0.0, src)]
    while pq:
        d, u = heapq.heappop(pq)
        if u == dst:
            return d
        if d > best.get(u, 1e18):
            continue
        for v, w in adj.get(u, []):
            if d + w < best.get(v, 1e18):
                best[v] = d + w
                heapq.heappush(pq, (d + w, v))
    return None


# ---------------------------------------------------------------------------------------------


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    repo = os.path.dirname(os.path.dirname(here))
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--db', default=os.path.join(os.environ.get('LOCALAPPDATA', ''),
                    'Temp', 'claude', 'twe-cache', 'vmangos', 'sqlite-dump', 'mangos.sqlite'))
    ap.add_argument('--data', default=os.environ.get('WOW_DATA', r'C:\Users\davis\wow-data\Data'))
    ap.add_argument('--out', default=os.path.join(repo, 'data', 'twilights-eve'))
    a = ap.parse_args()

    c = sqlite3.connect(f'file:{a.db}?mode=ro&immutable=1', uri=True)
    chain = Chain(a.data)
    cd = client_data(chain)

    tele = {r['id']: r for r in rows(c, 'select * from areatrigger_teleport where id in (?,?)',
                                     ENTRANCE_TRIGGER, EXIT_TRIGGER)}
    mt = rows(c, 'select * from map_template where entry=? and patch<=? order by patch desc limit 1',
              MAP_ID, VMANGOS_PATCH)[0]
    gy = rows(c, 'select * from game_graveyard_zone where ghost_zone=? and patch_min<=? and patch_max>=?',
              ZONE_ID, VMANGOS_PATCH, VMANGOS_PATCH)
    facing = {r['id']: r['orientation'] for r in rows(c, 'select * from world_safe_locs_facing')}

    spawns, groups = import_spawns(c)
    by_guid = {r['guid']: r for r in spawns}

    ent_t, exit_t = tele[ENTRANCE_TRIGGER], tele[EXIT_TRIGGER]
    entrance_in = {'map': MAP_ID, 'x': rnd(ent_t['target_position_x']), 'y': rnd(ent_t['target_position_y']),
                   'z': rnd(ent_t['target_position_z']), 'o': rnd(ent_t['target_orientation'])}
    outside = {'map': exit_t['target_map'], 'x': rnd(exit_t['target_position_x']),
               'y': rnd(exit_t['target_position_y']), 'z': rnd(exit_t['target_position_z']),
               'o': rnd(exit_t['target_orientation'])}

    graveyards = []
    for g in gy:
        loc = cd['safe_locs'].get(g['id'])
        if loc:
            graveyards.append(dict(loc, faction=g['faction'], o=rnd(facing.get(g['id'], 0.0))))

    os.makedirs(a.out, exist_ok=True)
    full = {
        'source': 'vmangos world DB (creature, creature_template @ patch<=10, creature_movement, '
                  'creature_addon, creature_groups, creature_classlevelstats)',
        'map_id': MAP_ID,
        'count': len(spawns),
        'bosses': sorted({(r['entry'], r['name']) for r in spawns if r['boss']}),
        'notes': [
            'Lady Anacondra (3671) has four authored spawns (27366, 300999, 301000, 301001); '
            'the vmangos instance script keeps one per instance.',
            'Mutanus the Devourer (3654) has no static spawn: he is summoned by the Naralex event.',
            'stats_by_level = creature_classlevelstats x the template multipliers (vmangos formula).',
            'rank: 0 normal, 1 elite, 2 rare elite, 3 boss, 4 rare.',
        ],
        'creature_groups': groups,
        'spawns': spawns,
    }
    full['bosses'] = [{'entry': e, 'name': n, 'guids': [r['guid'] for r in spawns if r['entry'] == e]}
                      for e, n in full['bosses']]
    with open(os.path.join(a.out, 'wailing_caverns_spawns_full.json'), 'w', encoding='utf-8', newline='\n') as f:
        json.dump(full, f, indent=1, ensure_ascii=False)
        f.write('\n')

    nodes, edges, mobs, chain_nodes = build_route(c, by_guid, entrance_in)

    def slim(r, node, near, d, role):
        keep = ('guid', 'entry', 'name', 'subname', 'x', 'y', 'z', 'o', 'level_min', 'level_max',
                'display_ids', 'scale', 'faction', 'rank', 'rank_name', 'type', 'unit_class',
                'speed_walk', 'speed_run', 'detection_range', 'call_for_help_range',
                'base_attack_time', 'stats_by_level', 'spawn_movement_type', 'wander_distance',
                'waypoints', 'ai_name')
        out = {k: r[k] for k in keep}
        grp = [g for g in groups if g['leader_guid'] == r['guid'] or g['member_guid'] == r['guid']]
        out.update(role=role, route_node=node, nearest_route_node=near, spur_length=rnd(d, 2),
                   group_leader=(grp[0]['leader_guid'] if grp else None))
        return out

    run_mobs = [slim(r, n, near, d, 'boss' if r['guid'] == BOSS_GUID else 'trash') for r, n, near, d in mobs]
    boss = run_mobs[-1]
    run = {
        'dungeon': 'The Wailing Caverns',
        'map_id': MAP_ID,
        'zone_id': ZONE_ID,
        'client_map': cd['map'],
        'wdt': cd['wdt'],
        'entrance': {
            'area_trigger': cd['area_triggers'][ENTRANCE_TRIGGER],
            'vmangos_areatrigger_teleport': {k: ent_t[k] for k in ('id', 'name', 'message', 'required_level')},
            'teleport_to': entrance_in,
        },
        'exit': {
            'area_trigger': cd['area_triggers'][EXIT_TRIGGER],
            'vmangos_areatrigger_teleport': {k: exit_t[k] for k in ('id', 'name')},
            'teleport_to': outside,
        },
        'death': {
            'respawn_inside_at': entrance_in,
            'note': 'Twilight\'s Eve rule: dying inside respawns at the instance entrance (teleport_to of '
                    'the entrance trigger). vmangos instead sends the ghost to the zone graveyard below and '
                    'lets the corpse run re-enter via map_template.ghost_entrance.',
            'vmangos_ghost_entrance': {'map': mt['ghost_entrance_map'], 'x': rnd(mt['ghost_entrance_x']),
                                       'y': rnd(mt['ghost_entrance_y'])},
            'vmangos_zone_graveyards': graveyards,
        },
        'boss': boss,
        'mobs': run_mobs,
        'route': {
            'nodes': nodes,
            'edges': edges,
            'entrance_node': 'entrance',
            'boss_node': boss['route_node'],
            'walk_length_entrance_to_boss': rnd(path_length(nodes, edges, 'entrance', boss['route_node']), 1),
            'notes': [
                'Edges are undirected. escort_* nodes are the Disciple of Naralex escort path '
                '(script_waypoint 3678, points 0-%d): a scripted NPC walk through the tunnels, so '
                'consecutive points are known-walkable.' % ESCORT_LAST_POINT,
                'patrol_* nodes are a Druid of the Fang patrol (creature_movement): also walkable.',
                'mob_* nodes are spawn points joined to the nearest route node by a straight spur '
                '(<= %.0f yd). These spurs are the only edges not walked by a vmangos NPC.' % MAX_SPUR,
                'z values are the server\'s; the client re-grounds units on its own collision.',
            ],
        },
    }
    with open(os.path.join(a.out, 'wailing_caverns_run.json'), 'w', encoding='utf-8', newline='\n') as f:
        json.dump(run, f, indent=1, ensure_ascii=False)
        f.write('\n')

    print(f'map {MAP_ID}: {len(spawns)} spawns, {len(groups)} group links')
    print(f'client map: {cd["map"]}')
    print(f'wdt: {cd["wdt"]}')
    print(f'triggers: {cd["area_triggers"]}')
    print(f'graveyards: {graveyards}')
    print(f'route: {len(nodes)} nodes, {len(edges)} edges, entrance->boss {run["route"]["walk_length_entrance_to_boss"]} yd')
    for m in run_mobs:
        print(f'  {m["role"]:5} {m["guid"]:6} {m["name"]:22} L{m["level_min"]}-{m["level_max"]} '
              f'{m["rank_name"]:6} near {m["nearest_route_node"]} ({m["spur_length"]} yd)')


if __name__ == '__main__':
    main()
