#!/usr/bin/env python3
"""Per-leg, per-map tile counts and encounter-tile counts along a path, from the decomp's map data.

usage: pathenc.py [leg-name ...]      writes human-run/data/pathenc.json and prints a table.

Uses bin/frlg-mapgraph's tile model and legs (the glitchless route's endpoints). A tile triggers
wild-encounter rolls when its metatile attribute `METATILE_ATTRIBUTE_ENCOUNTER_TYPE` (bits 24-26,
`decompiled/src/fieldmap.c:68`) is `TILE_ENCOUNTER_LAND` (1) or `TILE_ENCOUNTER_WATER` (2)
(`decompiled/include/global.fieldmap.h:40-43`, `decompiled/src/wild_encounter.c:360-420`).
For each leg two paths are found: the shortest in tiles (lambda 0) and one that pays `lambda` extra
tiles per encounter tile (lambda 3), which a human who avoids grass where the detour is cheap would walk.
The tiles are the steps the leg costs; `land` / `water` the steps that roll for an encounter (a map with no wild table never rolls). stdlib only.
"""
import heapq
import importlib.machinery
import importlib.util
import json
import os
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
os.environ.setdefault('FRLG_DECOMP', os.path.join(ROOT, 'decompiled'))
loader = importlib.machinery.SourceFileLoader('mg', os.path.join(ROOT, 'bin', 'frlg-mapgraph'))
spec = importlib.util.spec_from_loader('mg', loader)
mg = importlib.util.module_from_spec(spec)
loader.exec_module(mg)

_attr_cache = {}


def enc_type(m, x, y):
    """0 none, 1 land, 2 water for the tile (x, y) of map m."""
    if m.id not in _attr_cache:
        lay = mg.layouts()[m.j['layout']]
        _attr_cache[m.id] = (mg.tileset_attrs(lay['primary_tileset']), mg.tileset_attrs(lay['secondary_tileset']))
    pri, sec = _attr_cache[m.id]
    mt, _ = m.cell(x, y)
    a = pri[mt] if mt < mg.NUM_PRIMARY else sec[mt - mg.NUM_PRIMARY]
    return (a >> 24) & 7


def search(start, goals, opts, lam):
    """Dijkstra over (map, x, y); a step costs 1 + lam if it ends on an encounter tile."""
    surf = opts.get('surf', False)
    blocked = {}
    dist = {start: 0}
    prev = {}
    pq = [(0, start)]
    goalset = set(goals)
    while pq:
        d, cur = heapq.heappop(pq)
        if d > dist.get(cur, 1e18):
            continue
        if cur in goalset:
            path = [cur]
            while path[-1] in prev:
                path.append(prev[path[-1]])
            return list(reversed(path))
        mid, x, y = cur
        if mid in opts.get('avoid_maps', ()):
            continue
        m = mg.Map.get(mid)
        if mid not in blocked:
            blocked[mid] = m.blocked_tiles(opts)
        nxt = []
        for w in m.warps:
            if (w['x'], w['y']) == (x, y) and w['dest_map'] in mg.map_index() and w['dest_map'] != 'MAP_DYNAMIC':
                dm = mg.Map.get(w['dest_map'])
                wid = int(w['dest_warp_id']) if str(w['dest_warp_id']).isdigit() else 0
                if wid < len(dm.warps):
                    dw = dm.warps[wid]
                    nxt.append((w['dest_map'], dw['x'], dw['y']))
        for nid, nx, ny in mg.conn_exit(m, x, y):
            if mg.Map.get(nid).enterable(nx, ny, surf):
                nxt.append((nid, nx, ny))
        for nx, ny in mg.neighbours(m, x, y, surf, blocked[mid]):
            nxt.append((mid, nx, ny))
        for n in nxt:
            nm = mg.Map.get(n[0])
            c = 1 + (lam if enc_type(nm, n[1], n[2]) in (1, 2) else 0)
            nd = d + c
            if nd < dist.get(n, 1e18):
                dist[n] = nd
                prev[n] = cur
                heapq.heappush(pq, (nd, n))
    return None


def per_map(path):
    out = {}
    for i, (mid, x, y) in enumerate(path):
        if i == 0:
            continue
        m = mg.Map.get(mid)
        e = out.setdefault(mid, [0, 0, 0])
        e[0] += 1
        t = enc_type(m, x, y)
        if t == 1:
            e[1] += 1
        elif t == 2:
            e[2] += 1
    return {k: {'tiles': v[0], 'land': v[1], 'water': v[2]} for k, v in out.items()}


# Legs the glitchless table does not have but the human route walks.
EXTRA_LEGS = [
    ('cerulean-to-lavender-rock-tunnel', ('MAP_CERULEAN_CITY', 22, 19), ('MAP_LAVENDER_TOWN', 6, 5),
     {'cut': True, 'ignore_scripts': ('CeruleanCity_EventScript_Grunt',), 'avoid_maps': ('MAP_SAFFRON_CITY',)}),
    ('lavender-to-tower-7f', ('MAP_LAVENDER_TOWN', 18, 6), ('MAP_POKEMON_TOWER_7F', 11, 16), {}),
    ('cerulean-cave-to-mewtwo', ('MAP_CERULEAN_CITY', 22, 19), ('MAP_CERULEAN_CAVE_B1F', 7, 13), {'cut': True}),
    ('victory-road-1f-to-route23-north', ('MAP_VICTORY_ROAD_1F', 11, 20), ('MAP_ROUTE23', 18, 28), {}),
]


def main(names):
    res = []
    for name, a, b, opts in list(mg.LEGS) + EXTRA_LEGS:
        if names and name not in names:
            continue
        row = {'name': name, 'from': list(a), 'to': list(b)}
        for lam, key in ((0, 'shortest'), (3, 'avoid_grass')):
            p = search(a, [b], opts, lam)
            row[key] = per_map(p) if p else None
        res.append(row)
        s = row['shortest']
        if s:
            print(f"{name:36s} " + '  '.join(f"{k.replace('MAP_', '')}:{v['tiles']}/{v['land']}L{v['water']}W" for k, v in s.items()))
    dest = os.path.join(ROOT, 'human-run', 'data', 'pathenc.json')
    json.dump(res, open(dest, 'w'), indent=1)
    print('written', dest)


if __name__ == '__main__':
    main(sys.argv[1:])
