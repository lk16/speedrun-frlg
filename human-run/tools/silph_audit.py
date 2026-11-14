#!/usr/bin/env python3
"""Silph Co. forced-fight audit WITH the card-key doors and the elevator modelled.

The older audit (forced.py) walked the layout `map.bin` as it is stored, i.e. every card-key door
open, and found 0 cone-forced fights from 1F to 11F. That is a lower bound: the doors are drawn
closed on map load (`SilphCo_<n>F_OnLoad`: `call_if_unset FLAG_SILPH_<n>F_DOOR_k,
EventScript_Close<n>FDoor<k>`, `decompiled/data/scripts/silphco_doors.inc`) and only open with the
Card Key (`EventScript_TryUnlockDoor`: needs `FLAG_HIDE_SILPH_CO_5F_CARD_KEY`, i.e. the 5F ball
`SilphCo_5F_EventScript_ItemCardKey` taken). So:

  phase 0: 1F entrance -> a tile next to the Card Key ball on 5F, every door barrier closed
  phase 1: -> the 11F Giovanni trigger (5,15)/(6,15), doors passable (opened one script each),
           the 7F rival trigger (2,4)/(2,5) counted as a forced fight like a trainer

Elevator: any floor's elevator warp -> any other floor's `setdynamicwarp ..., 255, x, y`
(`SilphCo_Elevator/scripts.inc`), no key needed.

Sight cones: static `FACE_<dir>` trainers use their one direction, sight range from the object
(`trainer_see.c:146-231`, walls and bodies stop the line). `--rotate` (pessimistic) gives
`FACE_A_AND_B` / `LOOK_AROUND` trainers a cone in every direction they can turn to; wanderers are
bodies only. Reports the minimum number of cone-forced fights and the minimal sets.
usage: silph_audit.py [--rotate] [--no-doors] [--no-elevator]
"""
import heapq
import importlib.machinery
import importlib.util
import json
import os
import re
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
os.environ.setdefault('FRLG_DECOMP', os.path.join(ROOT, 'decompiled'))
DEC = os.environ['FRLG_DECOMP']
loader = importlib.machinery.SourceFileLoader('mg', os.path.join(ROOT, 'bin', 'frlg-mapgraph'))
spec = importlib.util.spec_from_loader('mg', loader)
mg = importlib.util.module_from_spec(spec)
loader.exec_module(mg)

DIRS = {'UP': (0, -1), 'DOWN': (0, 1), 'LEFT': (-1, 0), 'RIGHT': (1, 0)}
FLOORS = ['1F', '2F', '3F', '4F', '5F', '6F', '7F', '8F', '9F', '10F', '11F']
MID = {f: 'MAP_SILPH_CO_' + f for f in FLOORS}


def door_tiles():
    """{map id: set of barrier tiles that are impassable while the door is closed}."""
    txt = open(f'{DEC}/data/scripts/silphco_doors.inc').read()
    out = {}
    for m in re.finditer(r'^EventScript_Close(\d+F)Door\d*::.*?return', txt, re.S | re.M):
        mid = MID[m.group(1)]
        for t in re.finditer(r'setmetatile (\d+), (\d+), \w+, 1\b', m.group(0)):
            out.setdefault(mid, set()).add((int(t.group(1)), int(t.group(2))))
    return out


def door_groups():
    """[(label, map id, barrier tiles)] one per card-key door."""
    txt = open(f'{DEC}/data/scripts/silphco_doors.inc').read()
    out = []
    for m in re.finditer(r'^EventScript_Close(\d+F)Door(\d*)::.*?return', txt, re.S | re.M):
        tl = {(int(t.group(1)), int(t.group(2)))
              for t in re.finditer(r'setmetatile (\d+), (\d+), \w+, 1\b', m.group(0))}
        out.append((m.group(1) + 'D' + m.group(2), MID[m.group(1)], tl))
    return out


def elevator_arrivals():
    txt = open(f'{DEC}/data/maps/SilphCo_Elevator/scripts.inc').read()
    return {m.group(1): (int(m.group(2)), int(m.group(3)))
            for m in re.finditer(r'setdynamicwarp (MAP_SILPH_CO_\w+), 255, (\d+), (\d+)', txt)}


def trainer_cones(rotate):
    """[(name, map, (x,y), cone-tile-sets-by-direction)] plus static body tiles per map."""
    trainers = []
    for f in FLOORS:
        m = mg.Map.get(MID[f])
        for o in m.objects:
            if o.get('trainer_type') != 'TRAINER_TYPE_NORMAL':
                continue
            mv = o['movement_type'].replace('MOVEMENT_TYPE_', '')
            dirs = []
            if mv.startswith('FACE_'):
                dirs = [d for d in DIRS if d in mv.split('_AND_') or mv == 'FACE_' + d
                        or d in mv[5:].split('_AND_')]
                dirs = [d for d in DIRS if d in mv[5:].split('_AND_')]
                if not rotate and '_AND_' in mv:
                    dirs = []  # rotating: bodies only in the optimistic model
            elif mv == 'LOOK_AROUND' and rotate:
                dirs = list(DIRS)
            trainers.append(dict(name=f"{f}:{o['script'].split('EventScript_')[-1]}",
                                 map=MID[f], x=o['x'], y=o['y'], dirs=dirs,
                                 sight=int(o['trainer_sight_or_berry_tree_id']), mv=mv))
    # scripted triggers that start the fight by themselves
    trainers.append(dict(name='7F:RIVAL-trigger', map=MID['7F'], x=-1, y=-1, dirs=[], sight=0,
                         mv='coord', tiles={(2, 4), (2, 5)}))
    trainers.append(dict(name='11F:GIOVANNI-trigger', map=MID['11F'], x=-1, y=-1, dirs=[],
                         sight=0, mv='coord', tiles={(5, 15), (6, 15)}))
    return trainers


def main(argv):
    rotate = '--rotate' in argv
    use_doors = '--no-doors' not in argv
    use_elev = '--no-elevator' not in argv
    opts = {}
    doors = door_tiles() if use_doors else {}
    elev = elevator_arrivals()
    trainers = trainer_cones(rotate)
    bodies = {}
    for f in FLOORS:
        bodies[MID[f]] = mg.Map.get(MID[f]).blocked_tiles(opts)

    def blocked(mid, key):
        b = set(bodies[mid])
        if not key:
            b |= doors.get(mid, set())
        return b

    cone_cache = {}

    def cones(mid, key):
        k = (mid, key)
        if k in cone_cache:
            return cone_cache[k]
        m = mg.Map.get(mid)
        bl = blocked(mid, key)
        c = {}
        for i, t in enumerate(trainers):
            if t['map'] != mid:
                continue
            for xy in t.get('tiles', ()):
                c.setdefault(xy, set()).add(i)
            for d in t['dirs']:
                dx, dy = DIRS[d]
                for s in range(1, t['sight'] + 1):
                    tx, ty = t['x'] + dx * s, t['y'] + dy * s
                    if not m.enterable(tx, ty, False) or (tx, ty) in bl:
                        break
                    c.setdefault((tx, ty), set()).add(i)
        cone_cache[k] = {xy: frozenset(v) for xy, v in c.items()}
        return cone_cache[k]

    ball = [o for o in mg.Map.get(MID['5F']).objects if 'ItemCardKey' in str(o['script'])][0]
    ball_adj = {(ball['x'] + dx, ball['y'] + dy) for dx, dy in DIRS.values()}

    def succ(node, key):
        mid, x, y = node
        m = mg.Map.get(mid)
        out = []
        for w in m.warps:
            if (w['x'], w['y']) == (x, y) and w['dest_map'] in mg.map_index() \
               and w['dest_map'] != 'MAP_DYNAMIC':
                if w['dest_map'] == 'MAP_SILPH_CO_ELEVATOR':
                    if use_elev:
                        out += [(d, ax, ay) for d, (ax, ay) in elev.items()]
                    continue
                if w['dest_map'] not in bodies:
                    continue  # the exits to Saffron
                dm = mg.Map.get(w['dest_map'])
                dw = dm.warps[int(w['dest_warp_id'])]
                out.append((w['dest_map'], dw['x'], dw['y']))
        bl = blocked(mid, key)
        for nx, ny in mg.neighbours(m, x, y, False, bl):
            out.append((mid, nx, ny))
        return out

    start = (MID['1F'], 8, 20)
    pq = [(0, 0, start, False, frozenset(), (start,))]
    labels = {}
    seq = 0
    best = None
    results = {}
    while pq:
        k, _, node, key, ts, path = heapq.heappop(pq)
        if best is not None and k > best:
            break
        lab = labels.setdefault((node, key), [])
        if any(o <= ts for o in lab):
            continue
        lab.append(ts)
        if not key and node[0] == MID['5F'] and (node[1], node[2]) in ball_adj:
            key = True  # picked the Card Key up
            labels.setdefault((node, key), []).append(ts)
        if key and node[0] == MID['11F'] and (node[1], node[2]) in ((5, 15), (6, 15)):
            best = k
            results.setdefault(ts, path)
            continue
        for n in succ(node, key):
            nts = ts | cones(n[0], key).get((n[1], n[2]), frozenset())
            if any(o <= nts for o in labels.get((n, key), ())):
                continue
            seq += 1
            heapq.heappush(pq, (len(nts), seq, n, key, nts, path + (n,)))
    mode = ('rotating trainers pessimistic' if rotate else 'static cones only') + \
        (', doors modelled' if use_doors else ', doors open') + \
        (', elevator' if use_elev else ', no elevator')
    print(f'Silph 1F -> Card Key (5F) -> 11F Giovanni trigger [{mode}]')
    if best is None:
        print('  unreachable')
        return
    print(f'  minimum cone-forced fights (rival + Giovanni triggers included): {best}')
    for ts, path in results.items():
        floors = []
        for p in path:
            f = p[0][9:]
            if not floors or floors[-1] != f:
                floors.append(f)
        crossed = []
        for p_ in path:
            for lab, dm, tl in door_groups():
                if p_[0] == dm and (p_[1], p_[2]) in tl and lab not in crossed:
                    crossed.append(lab)
        print('  set:', sorted(trainers[i]['name'] for i in ts), ' floors:', '>'.join(floors),
              f' steps {len(path)}')
        print('  card-key doors entered after the key:', crossed)


if __name__ == '__main__':
    main(sys.argv[1:])
