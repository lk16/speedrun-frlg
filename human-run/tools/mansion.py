#!/usr/bin/env python3
"""Sight-cone audit of the Pokémon Mansion with its statue switch modelled.

`forced.py` cannot see the Mansion because every floor's barriers depend on one global switch state
(`FLAG_POKEMON_MANSION_SWITCH_STATE`, `decompiled/data/scripts/pokemon_mansion.inc`). This tool searches
over (map, x, y, switch state, triggered trainers):

* state OFF is each floor's `map.bin` as authored; state ON applies that floor's `PressSwitch_<F>` list of
  `setmetatile x, y, METATILE, collision` (collision 1 = barrier, 0 = floor), which `OnLoad` runs when the
  flag is set (`PokemonMansion_1F/scripts.inc:6-8`);
* pressing a statue (a `bg_event` whose script ends in `_Statue`, adjacent to it) flips the state.

usage: mansion.py        (start: 1F entrance from Cinnabar; goal: the Secret Key ball on B1F)
stdlib only; reuses forced.py's tile and sight model.
"""
import heapq
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import forced as F  # noqa: E402

mg = F.mg
ROOT = F.ROOT
FLOORS = {'1F': 'MAP_POKEMON_MANSION_1F', '2F': 'MAP_POKEMON_MANSION_2F', '3F': 'MAP_POKEMON_MANSION_3F',
          'B1F': 'MAP_POKEMON_MANSION_B1F'}
src = open(os.path.join(os.environ['FRLG_DECOMP'] if 'FRLG_DECOMP' in os.environ else os.path.join(ROOT, 'decompiled'),
                        'data', 'scripts', 'pokemon_mansion.inc')).read()


def section(name):
    m = re.search(r'^' + name + r'::\n(.*?)^\s+return', src, re.S | re.M)
    return m.group(1) if m else ''


ON = {}  # map id -> {(x, y): blocked?}
for fl, mid in FLOORS.items():
    d = {}
    for x, y, col in re.findall(r'setmetatile\s+(\d+),\s*(\d+),\s*\w+,\s*([01])', section(f'PokemonMansion_EventScript_PressSwitch_{fl}')):
        d[(int(x), int(y))] = col == '1'
    ON[mid] = d

state = [0]  # current switch state while expanding a node
_orig_enterable = F._orig_enterable


def enterable(self, x, y, surf):
    if state[0] == 1:
        ov = ON.get(self.id, {})
        if (x, y) in ov:
            return not ov[(x, y)]
    return _orig_enterable(self, x, y, surf)


mg.Map.enterable = enterable
F.OPEN.clear()

statues = {}
for fl, mid in FLOORS.items():
    j = json.load(open(os.path.join(ROOT, 'decompiled', 'data', 'maps', 'PokemonMansion_' + fl, 'map.json')))
    for b in j.get('bg_events', []):
        if 'Statue' in str(b.get('script')):
            statues.setdefault(mid, []).append((b['x'], b['y']))

opts = {'surf': False, 'cut': False, 'ignore_scripts': ['ItemSecretKey']}
trainers = []
cone_cache = {}


def cones(mid, st):
    key = (mid, st)
    if key not in cone_cache:
        state[0] = st
        c = {}
        for t in F.trainers_in([mid], opts):
            trainers.append(t)
            for xy in F.cone(t, opts):
                c[xy] = c.get(xy, frozenset()) | {len(trainers) - 1}
        cone_cache[key] = c
    return cone_cache[key]


def blocked_for(mid, st):
    state[0] = st
    return _orig_blocked(mg.Map.get(mid), opts)


_orig_blocked = F._orig_blocked
start = ('MAP_POKEMON_MANSION_1F', 8, 33)
goal = ('MAP_POKEMON_MANSION_B1F', 5, 6)

pq = [(0, 0, start, 0, frozenset())]
labels = {}
best = None
sets = set()
seq = 0
while pq:
    k, _, node, st, ts = heapq.heappop(pq)
    if best is not None and k > best:
        break
    key = (node, st)
    if any(old <= ts for old in labels.get(key, ())):
        continue
    labels.setdefault(key, []).append(ts)
    if node == goal:
        best = k
        sets.add(ts)
        continue
    mid, x, y = node
    m = mg.Map.get(mid)
    state[0] = st
    nxt = []
    for w in m.warps:
        if (w['x'], w['y']) == (x, y) and w['dest_map'] in mg.map_index() and w['dest_map'] != 'MAP_DYNAMIC':
            dm = mg.Map.get(w['dest_map'])
            wid = int(w['dest_warp_id']) if str(w['dest_warp_id']).isdigit() else 0
            if wid < len(dm.warps):
                dw = dm.warps[wid]
                nxt.append(((w['dest_map'], dw['x'], dw['y']), st))
    if mid.startswith('MAP_POKEMON_MANSION') or mid == 'MAP_CINNABAR_ISLAND':
        blk = _orig_blocked(m, opts)
        for nx, ny in mg.neighbours(m, x, y, False, blk):
            nxt.append(((mid, nx, ny), st))
        # Press a statue from an adjacent tile.
        for sx, sy in statues.get(mid, []):
            if abs(sx - x) + abs(sy - y) == 1:
                nxt.append(((mid, x, y), 1 - st))
    for n, nst in nxt:
        if n[0] not in FLOORS.values():
            continue
        nts = ts | cones(n[0], nst).get((n[1], n[2]), frozenset())
        if any(old <= nts for old in labels.get((n, nst), ())):
            continue
        seq += 1
        heapq.heappush(pq, (len(nts), seq, n, nst, nts))

if best is None:
    print('unreachable with the model')
else:
    names = lambda ts: sorted(trainers[i][0].split('EventScript_')[-1] for i in ts)
    inter = set(names(next(iter(sets))))
    union = set()
    for s_ in sets:
        inter &= set(names(s_))
        union |= set(names(s_))
    print(f'Mansion 1F entrance -> Secret Key (B1F): minimum forced fights {best} ({len(sets)} minimal set(s))')
    print('  in every minimal set:', sorted(inter))
    print('  in some minimal set :', sorted(union))
