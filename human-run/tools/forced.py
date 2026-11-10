#!/usr/bin/env python3
"""Which trainers can a leg not avoid? Sight-cone-aware BFS over the decomp's map data.

usage: forced.py MAP_A x,y MAP_B x,y [--surf] [--cut] [--ignore SCRIPTTAG ...] [--avoid MAP ...]
                 [--ignore-boulders]

Reuses bin/frlg-mapgraph's tile model (collision bits, warps, connections, static objects as
bodies). Adds the trainers' line of sight, checked the way the engine does it: a trainer notices
the player when the player stands in the trainer's facing line within its sight range and every
tile between them is free of collision and objects
(`decompiled/src/trainer_see.c:93-231`, `CheckPathBetweenTrainerAndPlayer`).

It then looks for the smallest set of trainers whose cones the route has to cross, by trying
subsets of increasing size: a subset is "allowed" when its cones are walkable, all other cones
are walls. Output: the minimum count and every minimal set found, so the intersection (trainers
in every minimal set) is the truly forced list.

Caveats, same family as docs/glitchless-run/plan.md section 3: wandering and rotating trainers
are bodies only (their cones move; a human can time them), script-driven metatile barriers
(gym puzzles, Silph doors) are not modelled, elevation differences are ignored, and a fought
trainer's body stays where it stood. stdlib only.
"""
import importlib.machinery
import importlib.util
import itertools
import os
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
os.environ.setdefault('FRLG_DECOMP', os.path.join(ROOT, 'decompiled'))
loader = importlib.machinery.SourceFileLoader('mg', os.path.join(ROOT, 'bin', 'frlg-mapgraph'))
spec = importlib.util.spec_from_loader('mg', loader)
mg = importlib.util.module_from_spec(spec)
loader.exec_module(mg)

DIRS = {'UP': (0, -1), 'DOWN': (0, 1), 'LEFT': (-1, 0), 'RIGHT': (1, 0)}
_orig_blocked = mg.Map.blocked_tiles


def _blocked(self, opts):
    t = _orig_blocked(self, opts)
    if opts.get('ignore_boulders'):
        for o in self.objects:
            if 'BOULDER' in o.get('graphics_id', ''):
                t.discard((o['x'], o['y']))
    return t | opts.get('forbid', {}).get(self.id, set())


mg.Map.blocked_tiles = _blocked


def trainers_in(map_ids, opts):
    """[(key, map, x, y, facing-or-None, sight)] for every trainer object on the maps."""
    out = []
    for mid in map_ids:
        m = mg.Map.get(mid)
        for o in m.objects:
            if o.get('trainer_type') != 'TRAINER_TYPE_NORMAL':
                continue
            if any(tag in str(o.get('script')) for tag in opts.get('ignore_scripts', ())):
                continue
            mv = o.get('movement_type', '')
            face = None
            for k in DIRS:
                if mv.endswith('FACE_' + k):
                    face = k
            out.append((str(o['script']), mid, o['x'], o['y'], face,
                        int(o['trainer_sight_or_berry_tree_id'])))
    return out


def cone(trainer, opts):
    key, mid, x, y, face, sight = trainer
    if face is None:
        return set()
    m = mg.Map.get(mid)
    blocked = _orig_blocked(m, opts)
    dx, dy = DIRS[face]
    tiles = set()
    for i in range(1, sight + 1):
        tx, ty = x + dx * i, y + dy * i
        if not m.enterable(tx, ty, opts.get('surf', False)) or (tx, ty) in blocked:
            break
        tiles.add((tx, ty))
    return tiles


def search_sets(start, goal, opts, cones_of_map, trainers_all):
    """Label-setting search over (map, x, y, set of triggered trainers), cheapest set first.
    Returns (k, [minimal sets of size k]) or (None, [])."""
    import heapq
    surf = opts.get('surf', False)
    blocked_cache = {}
    labels = {}  # node -> list of frozensets already settled there
    pq = [(0, 0, start, frozenset())]
    best = None
    sets = set()
    seq = 0
    while pq:
        k, _, node, ts = heapq.heappop(pq)
        if best is not None and k > best:
            break
        if any(old <= ts for old in labels.get(node, ())):
            continue
        labels.setdefault(node, []).append(ts)
        if node == goal:
            best = k
            sets.add(ts)
            continue
        mid, x, y = node
        if mid in opts.get('avoid_maps', ()):
            continue
        m = mg.Map.get(mid)
        if mid not in blocked_cache:
            blocked_cache[mid] = _orig_blocked(m, opts)
        nxt = []
        for w in m.warps:
            if (w['x'], w['y']) == (x, y) and w['dest_map'] in mg.map_index() \
               and w['dest_map'] != 'MAP_DYNAMIC':
                dm = mg.Map.get(w['dest_map'])
                wid = int(w['dest_warp_id']) if str(w['dest_warp_id']).isdigit() else 0
                if wid < len(dm.warps):
                    dw = dm.warps[wid]
                    nxt.append((w['dest_map'], dw['x'], dw['y']))
        for nid, nx, ny in mg.conn_exit(m, x, y):
            if mg.Map.get(nid).enterable(nx, ny, surf):
                nxt.append((nid, nx, ny))
        for nx, ny in mg.neighbours(m, x, y, surf, blocked_cache[mid]):
            nxt.append((mid, nx, ny))
        for n in nxt:
            nts = ts | cones_of_map(n[0]).get((n[1], n[2]), frozenset())
            if any(old <= nts for old in labels.get(n, ())):
                continue
            seq += 1
            heapq.heappush(pq, (len(nts), seq, n, nts))
    return best, sorted(sets, key=sorted)


def main(argv):
    opts = {'surf': '--surf' in argv, 'cut': '--cut' in argv,
            'ignore_boulders': '--ignore-boulders' in argv, 'ignore_scripts': []}
    rest = []
    i = 0
    while i < len(argv):
        a = argv[i]
        if a == '--avoid':
            opts.setdefault('avoid_maps', []).append(argv[i + 1])
            i += 2
            continue
        if a == '--ignore':
            opts['ignore_scripts'].append(argv[i + 1])
            i += 2
            continue
        if not a.startswith('--'):
            rest.append(a)
        i += 1
    ma, xy_a, mb, xy_b = rest
    start = (ma, *map(int, xy_a.split(',')))
    goal = (mb, *map(int, xy_b.split(',')))
    d, legs = mg.search(start, [goal], opts)
    if d is None:
        print('no path even ignoring sight')
        return
    trainers = []
    per_map = {}

    def cones_of_map(mid):
        if mid not in per_map:
            c = {}
            for t in trainers_in([mid], opts):
                trainers.append(t)
                for xy in cone(t, opts):
                    c[xy] = c.get(xy, frozenset()) | {len(trainers) - 1}
            per_map[mid] = c
        return per_map[mid]

    k, sets = search_sets(start, goal, opts, cones_of_map, None)
    print(f'{ma} {xy_a} -> {mb} {xy_b}: {d} tiles unconstrained, {len(trainers)} trainers on '
          f'{len(per_map)} maps searched')
    if k is None:
        print('  unreachable')
        return
    names = lambda ts: sorted(trainers[i][0] for i in ts)
    inter = set(names(sets[0]))
    union = set()
    for s_ in sets:
        inter &= set(names(s_))
        union |= set(names(s_))
    print(f'  minimum forced fights: {k} ({len(sets)} minimal set(s))')
    print(f'  in every minimal set : {sorted(inter)}')
    print(f'  in some minimal set  : {sorted(union)}')


if __name__ == '__main__':
    main(sys.argv[1:])
