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


def current():
    return open(os.path.join(PLANS, 'CURRENT')).read().strip()


def load(name):
    return json.loads(open(os.path.join(PLANS, name + '.md')).read().split('```json')[1].split('```')[0])


def next_number():
    return max(int(f.split('-')[0]) for f in os.listdir(PLANS) if f[:3].isdigit()) + 1


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
        for f in (0.5, 1.6):
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
    aggr = p.get('reset_aggr_at', {})
    for s in SPLITS:
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
    if only:
        out = [c for c in out if c[0] in only]
    return out


def try_it(parent, label, patch, name, seed):
    r = subprocess.run([sys.executable, os.path.join(HERE, 'try.py'), os.path.join(PLANS, parent + '.md'), label,
                        json.dumps(patch), '--name', name, '--cap', str(cap), '--seed', str(seed)],
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
        subprocess.run(['git', '-C', ROOT, 'add', '-A'], check=True)
        subprocess.run(['git', '-C', ROOT, 'commit', '-qm',
                        f'human-run: plan {pname}: {label}\n\nCo-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>'],
                       check=True)
        print(f'  ==> promoted {pname}', flush=True)
        accepts += 1
        changed = True
        if accepts >= max_accepts:
            break
    if not changed or accepts >= max_accepts:
        break
print('done; accepted', accepts)
