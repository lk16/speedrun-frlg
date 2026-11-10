#!/usr/bin/env python3
"""Coordinate descent over the plan's knobs, following human-run/PROMPT.md's comparison rules.

usage: descend.py [--max-accepts N] [--passes N] [--only KIND,...]

Starts from plans/CURRENT. For every candidate patch (one knob changed) runs try.py against the
current plan (seed 1); on ACCEPT it re-runs on seed 2 to filter chance wins (many candidates are
tried, so a lone 95% win is not enough), and only if both accept does it promote the plan
(promote.py), commit and continue from it. Every try is logged in scores.jsonl by try.py.
Stops when a full pass yields no accepted plan. stdlib only.
"""
import json, os, subprocess, sys, copy

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, '..', '..')
PLANS = os.path.join(ROOT, 'human-run', 'plans')
SCRATCH = os.environ.setdefault('FRLG_SCRATCH', '/tmp/frlg-human-scratch')
SPLITS = ['brock', 'misty-badge', 'surge-badge', 'erika-badge', 'koga-badge', 'sabrina-badge',
          'blaine-badge-and-sevii-scene', 'giovanni-badge', 'indigo-heal', 'hall-of-fame-1',
          'oak-national-dex', 'celio-network-machine']

# Extra splits were tried (extra_splits, aggr 0.7: no resets triggered; aggr 1.0: slightly worse), so none here.
EXTRA_SPLIT_CANDIDATES = []

args = sys.argv[1:]
def opt(name, default):
    if name in args:
        i = args.index(name)
        v = args[i + 1]
        return type(default)(v)
    return default
max_accepts = opt('--max-accepts', 50)
max_passes = opt('--passes', 3)
only = opt('--only', '').split(',') if '--only' in args else None
cap = opt('--cap', 400000)
POINTER = opt('--pointer', 'CURRENT')
FIXED_RUNG = opt('--rung', '')
os.environ['FRLG_POINTER'] = POINTER


def current():
    return open(os.path.join(PLANS, POINTER)).read().strip()


def load(name):
    return json.loads(open(os.path.join(PLANS, name + '.md')).read().split('```json')[1].split('```')[0])


def next_number():
    return max(int(f.split('-')[0]) for f in os.listdir(PLANS) if f[:3].isdigit()) + 1


_SEGS = None


def grass_segments():
    global _SEGS
    if _SEGS is None:
        try:
            plan = os.path.join(PLANS, current() + '.md')
            r = subprocess.run([os.path.join(ROOT, 'target', 'release', 'frlg-human'), 'segs', plan],
                               capture_output=True, text=True)
            _SEGS = [l.split()[0] for l in r.stdout.splitlines() if l.strip()]
        except Exception:
            _SEGS = []
    return _SEGS


def candidates(p):
    """(kind, label, patch) for single-knob changes around plan params p."""
    out = []
    fill = p.get('fill', [])
    for i, h in enumerate(fill):
        for f in (0.6, 1.5):
            q = copy.deepcopy(fill)
            q[i]['steps'] = max(8, int(round(h['steps'] * f)))
            if q[i]['steps'] != h['steps']:
                out.append(('fillsteps', f"fill[{i}] {h['site']} {h['area']} steps x{f}", {'fill': q}))
        for f in ((0.5, 1.6) if not p.get('detour_map') else ()):
            q = copy.deepcopy(fill)
            q[i]['detour_tiles'] = int(round(h.get('detour_tiles', 0) * f))
            if q[i]['detour_tiles'] != h.get('detour_tiles', 0):
                out.append(('detour', f"fill[{i}] {h['site']} detour x{f}", {'fill': q}))
    for i in range(len(fill) - 1):
        q = copy.deepcopy(fill)
        q[i], q[i + 1] = q[i + 1], q[i]
        out.append(('order', f"swap fill[{i}]={fill[i]['site']} {fill[i]['area']} with fill[{i+1}]", {'fill': q}))
    for i, h in enumerate(fill):
        q = copy.deepcopy(fill)
        del q[i]
        out.append(('dropfill', f"drop fill[{i}] {h['site']} {h['area']}", {'fill': q}))
    extra = list(p.get('extra_splits', []))
    for name in EXTRA_SPLIT_CANDIDATES:
        new = [x for x in extra if x != name] if name in extra else extra + [name]
        out.append(('split', f"extra split {'-' if name in extra else '+'}{name}", {'extra_splits': new}))
    aggr = p.get('reset_aggr_at', {})
    for s in SPLITS + extra:
        cur = aggr.get(s, p.get('reset_aggr', 0.0))
        for d in (-0.2, 0.2):
            v = round(min(1.0, max(0.0, cur + d)), 2)
            if v != cur:
                q = dict(aggr)
                q[s] = v
                out.append(('aggr', f"aggr at {s} {cur}->{v}", {'reset_aggr_at': q}))
    for d in (-0.1, 0.1):
        out.append(('aggr', f"global aggr {p.get('reset_aggr', 0)}->{round(p.get('reset_aggr', 0) + d, 2)}",
                    {'reset_aggr': round(p.get('reset_aggr', 0) + d, 2)}))
    for d in (-1, 1):
        out.append(('starter', f"starter_min_total {p['starter_min_total']}->{p['starter_min_total'] + d}",
                    {'starter_min_total': p['starter_min_total'] + d}))
        out.append(('fight', f"fight_until_level {p['fight_until_level']}->{p['fight_until_level'] + d}",
                    {'fight_until_level': p['fight_until_level'] + d}))
    sm = p.get('starter_min_stats', [0] * 6)
    for k in range(6):
        for d in (-1, 1):
            if sm[k] + d >= 0:
                q = list(sm)
                q[k] += d
                out.append(('starter', f"starter_min_stats[{k}] {sm[k]}->{q[k]}", {'starter_min_stats': q}))
    for i, h in enumerate(p.get('hunts', [])):
        for f in (0.5, 2.0):
            q = copy.deepcopy(p['hunts'])
            q[i]['steps'] = max(8, int(round(h['steps'] * f)))
            out.append(('hunt', f"hunts[{i}] {h['site']} steps x{f}", {'hunts': q}))
        q = copy.deepcopy(p['hunts'])
        del q[i]
        out.append(('hunt', f"drop hunts[{i}] {h['site']}", {'hunts': q}))
    for i, sp in enumerate(p.get('specials', [])):
        q = copy.deepcopy(p['specials'])
        del q[i]
        out.append(('special', f"drop special {sp['kind']} {sp['species']}", {'specials': q}))
    for d in (-10, 10):
        v = p.get('pass_min_catch', 0) + d
        if v >= 0:
            out.append(('catch', f"pass_min_catch {p.get('pass_min_catch', 0)}->{v}", {'pass_min_catch': v}))
    for ball in ('poke', 'great', 'ultra'):
        if p.get('ball', 'poke') != ball:
            out.append(('ball', f'ball {p.get("ball", "poke")}->{ball}', {'ball': ball}))
    if p.get('money_model'):
        for b in (5, 10, 20, 40):
            if p.get('ball_batch', 10) != b:
                out.append(('ball', f'ball_batch {p.get("ball_batch", 10)}->{b}', {'ball_batch': b}))
    for k in ('weaken', 'mewtwo', 'teleport_abra', 'bike', 'detour_bike', 'hunt_bike'):
        out.append(('toggle', f'{k} {p.get(k)}->{not p.get(k)}', {k: not p.get(k)}))
    if p.get('safari_model'):
        for r in (0, 1):
            if p.get('safari_rocks', 0) != r:
                out.append(('safari', f'safari_rocks {p.get("safari_rocks", 0)}->{r}', {'safari_rocks': r}))
    # Extra mainland fill sites (sitescan.py ranking; Cerulean Cave and the Sevii islands open only
    # after the Oak check, `human-run/journal`), tried at the front and the middle of the list.
    present = {(h['site'], h.get('area', 'land')) for h in fill}
    for site, area in (('MAP_SEAFOAM_ISLANDS_B2F', 'land'), ('MAP_SEAFOAM_ISLANDS_B3F', 'land'),
                       ('MAP_SEAFOAM_ISLANDS_B1F', 'land'), ('MAP_ROUTE18', 'land'), ('MAP_ROUTE21_NORTH', 'land'),
                       ('MAP_ROUTE16', 'land'), ('MAP_ROUTE14', 'land'), ('MAP_POKEMON_TOWER_7F', 'land'),
                       ('MAP_ROUTE22', 'fish'), ('MAP_ROUTE23', 'fish'), ('MAP_ROUTE25', 'fish'),
                       ('MAP_VICTORY_ROAD_2F', 'land'), ('MAP_VIRIDIAN_CITY', 'fish'), ('MAP_ROUTE15', 'land'),
                       ('MAP_FUCHSIA_CITY', 'fish'), ('MAP_VERMILION_CITY', 'fish'), ('MAP_PALLET_TOWN', 'fish')):
        if (site, area) in present:
            continue
        for pos in (0, len(fill) // 2):
            q = copy.deepcopy(fill)
            q.insert(pos, dict(after='', site=site, area=area, steps=80 if area == 'land' else 25, detour_tiles=50))
            out.append(('addsite', f'add {site} {area} at position {pos}', {'fill': q}))
    # In-game trades not yet in the plan (decompiled/src/data/ingame_trades.h; Mr. Mime for an Abra at the
    # Route 2 house, Nidoran F for a Nidoran M at the Underground Path north entrance).
    have_sp = {x['species'][0] for x in p.get('specials', [])}
    for sp, req, after, frames in (('SPECIES_MR_MIME', 'SPECIES_ABRA', 'fly-viridian', 2600),
                                   ('SPECIES_NIDORAN_F', 'SPECIES_NIDORAN_M', 'route5-6', 1500)):
        if sp not in have_sp:
            new = copy.deepcopy(p.get('specials', []))
            new.append({'after': after, 'kind': 'trade', 'species': [sp], 'requires': [req], 'frames': frames})
            out.append(('special', f'trade for {sp} (needs {req}) after {after}', {'specials': new}))
    # In-route hunts at grass the shortest path does not cross (pathenc.py): stand in the grass of a route
    # while passing until its new species are caught or the step budget runs out.
    hs = list(p.get('hunts', []))
    have_h = {(h['after'], h['site']) for h in hs}
    for after, site in (('route3', 'MAP_ROUTE3'), ('route4', 'MAP_ROUTE4'), ('nugget-bridge', 'MAP_ROUTE24'),
                        ('route5-6', 'MAP_ROUTE5'), ('route9', 'MAP_ROUTE9'), ('route10-north', 'MAP_ROUTE10'),
                        ('route8-7', 'MAP_ROUTE8'), ('cycling-road', 'MAP_ROUTE16'), ('cycling-road', 'MAP_ROUTE17'),
                        ('cycling-road', 'MAP_ROUTE18'), ('route22', 'MAP_ROUTE22')):
        if (after, site) in have_h:
            continue
        new = copy.deepcopy(hs)
        new.append(dict(after=after, site=site, area='land', steps=60, detour_tiles=2, skip=[]))
        out.append(('hunt', f'add an in-route hunt at {site} after {after}', {'hunts': new}))
    # Celadon Game Corner prizes (LeafGreen coin prices x 20 money per coin).
    have = {x['species'][0] for x in p.get('specials', []) if x.get('kind') == 'buy'}
    for sp, cost in (('SPECIES_ABRA', 2400), ('SPECIES_CLEFAIRY', 15000), ('SPECIES_PINSIR', 50000)):
        if sp not in have:
            new = copy.deepcopy(p.get('specials', []))
            new.append({'after': 'hall-of-fame-1', 'kind': 'buy', 'species': [sp], 'cost': cost, 'frames': 2400})
            out.append(('special', f'buy {sp} at the Game Corner', {'specials': new}))
    segs = grass_segments()
    cur = list(p.get('repel_on', []))
    for sname in segs:
        if sname in ('route1-encounters', 'route2-encounters', 'forest-encounters'):
            continue  # before the first mart that sells Repels
        new = [x for x in cur if x != sname] if sname in cur else cur + [sname]
        out.append(('repel', f"repel_on {'-' if sname in cur else '+'}{sname}", {'repel_on': new}))
    if only:
        out = [c for c in out if c[0] in only]
    return out


def try_it(parent, label, patch, name, seed):
    r = subprocess.run([sys.executable, os.path.join(HERE, 'try.py'), os.path.join(PLANS, parent + '.md'), label,
                        json.dumps(patch), '--name', name, '--cap', str(cap), '--seed', str(seed)]
                       + (['--rung', FIXED_RUNG] if FIXED_RUNG else []),
                       capture_output=True, text=True)
    lines = [l for l in r.stdout.strip().splitlines() if l.startswith(('ACCEPT', 'refuted'))]
    return (lines[0] if lines else 'error: ' + r.stdout[-200:] + r.stderr[-200:])


accepts = 0
for ps in range(max_passes):
    changed = False
    cur = current()
    p = load(cur)
    cands = candidates(p)
    print(f'pass {ps}: {len(cands)} candidates from {cur}', flush=True)
    for ci, (kind, label, patch) in enumerate(cands):
        cur = current()
        p = load(cur)
        # skip candidates that no longer differ from the current plan
        if all(p.get(k) == v for k, v in patch.items()):
            continue
        name = f'd{next_number()}-{ci}'
        v1 = try_it(cur, f'[{kind}] {label}', patch, name, 1)
        print(f'  {label}: {v1}', flush=True)
        if not v1.startswith('ACCEPT'):
            continue
        v2 = try_it(cur, f'[{kind}] {label} (confirm seed 2)', patch, name + 's2', 2)
        print(f'    confirm: {v2}', flush=True)
        if not v2.startswith('ACCEPT'):
            continue
        num = next_number()
        pname = f"{num:03d}-{kind}{ci}"
        subprocess.run([sys.executable, os.path.join(HERE, 'promote.py'), os.path.join(SCRATCH, name + '.md'), pname,
                        f'{label} (descend.py; confirmed on seeds 1 and 2)'], check=True, capture_output=True)
        import time
        for attempt_ in range(8):  # another process may hold the index lock for a moment
            a = subprocess.run(['git', '-C', ROOT, 'add', '-A'], capture_output=True)
            b = subprocess.run(['git', '-C', ROOT, 'commit', '-qm',
                                f'human-run: plan {pname}: {label}\n\nCo-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>'],
                               capture_output=True)
            if a.returncode == 0 and b.returncode in (0, 1):
                break
            time.sleep(3)
        print(f'  ==> promoted {pname}', flush=True)
        accepts += 1
        changed = True
        if accepts >= max_accepts:
            break
    if not changed or accepts >= max_accepts:
        break
print('done; accepted', accepts)
