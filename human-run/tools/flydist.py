#!/usr/bin/env python3
"""Walking tiles from the nearest Fly point to the first encounter tile of each fill site.

usage: flydist.py [site ...]     writes human-run/data/flydist.json and prints a table.

Fly arrives on a heal location (`decompiled/src/data/heal_locations.json`). For every encounter site
that has a land table (or a fishing table: the nearest tile next to water that can be surfed or fished
from, found as an enterable tile with a water neighbour), a tile-level search (bin/frlg-mapgraph's model,
Surf and Cut available, trainers ignored) from each fly point gives the tiles to the nearest tile that
rolls for an encounter (`METATILE_ATTRIBUTE_ENCOUNTER_TYPE`, `pathenc.py`). The minimum over fly points is
the one-way walk; `frlg-human` doubles it (`detour_tiles`). stdlib only.
"""
import heapq
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pathenc as P  # noqa: E402

mg = P.mg
ROOT = P.ROOT
heal = json.load(open(os.path.join(ROOT, 'decompiled', 'src', 'data', 'heal_locations.json')))['heal_locations']
# Sevii (One Island and beyond) and the Indigo Plateau count; Fly needs the Town Map flag, assumed set.
FLY = [(h['id'], h['map'], h['x'], h['y']) for h in heal]


def targets(site, area):
    """Tiles of the site map where the human can start hunting."""
    m = mg.Map.get(site)
    out = []
    for y in range(m.h):
        for x in range(m.w):
            if area == 'land':
                if P.enc_type(m, x, y) == 1 and m.enterable(x, y, False):
                    out.append((site, x, y))
            else:
                if m.enterable(x, y, True) and any(
                        0 <= x + dx < m.w and 0 <= y + dy < m.h and m.behavior(x + dx, y + dy) in mg.MB_WATER
                        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))):
                    out.append((site, x, y))
    return out


def dijkstra_all(starts, goalset, opts, limit=900):
    """Multi-source search from one fly point; returns tiles to the first goal reached."""
    dist = {s: 0 for s in starts}
    pq = [(0, s) for s in starts]
    blocked = {}
    surf = opts.get('surf', False)
    while pq:
        d, cur = heapq.heappop(pq)
        if d > dist.get(cur, 1e18) or d > limit:
            continue
        if cur in goalset:
            return d
        mid, x, y = cur
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
            if d + 1 < dist.get(n, 1e18):
                dist[n] = d + 1
                heapq.heappush(pq, (d + 1, n))
    return None


def main(names):
    plan_sites = []
    if names == ['--all']:
        dex = json.load(open(os.path.join(ROOT, 'human-run', 'data', 'dex.json')))
        names = []
        for site, areas in sorted(dex['sites_lg'].items()):
            if site not in mg.map_index():
                continue
            if areas.get('land_mons'):
                names.append((site, 'land'))
            if areas.get('fishing_mons'):
                names.append((site, 'fish'))
    elif not names:
        cur = open(os.path.join(ROOT, 'human-run', 'plans', 'CURRENT')).read().strip()
        p = json.loads(open(os.path.join(ROOT, 'human-run', 'plans', cur + '.md')).read().split('```json')[1].split('```')[0])
        plan_sites = [(h['site'], h.get('area', 'land')) for h in p.get('fill', [])]
        plan_sites += [(h['site'], h.get('area', 'land')) for h in p.get('hunts', [])]
        names = sorted(set(plan_sites))
    else:
        names = [(n, 'land') for n in names]
    if not isinstance(names[0], tuple):
        names = [(n, 'land') for n in names]
    opts = {'surf': True, 'cut': True, 'ignore_objects': True}
    res = {}
    for site, area in names:
        goals = set(targets(site, area))
        if not goals:
            print(f'{site} {area}: no target tiles')
            continue
        best = None
        for fid, fmap, fx, fy in FLY:
            d = dijkstra_all([(fmap, fx, fy)], goals, opts)
            if d is not None and (best is None or d < best[0]):
                best = (d, fid)
        res[f'{site}|{area}'] = {'tiles': best[0] if best else None, 'from': best[1] if best else None}
        print(f'{site:36s} {area:5s} {best}')
    dest = os.path.join(ROOT, 'human-run', 'data', 'flydist.json')
    json.dump(res, open(dest, 'w'), indent=1)
    print('written', dest)


if __name__ == '__main__':
    main(sys.argv[1:])
