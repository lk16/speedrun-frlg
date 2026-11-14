#!/usr/bin/env python3
"""Calibration sensitivity helpers for human-run (ROBUSTNESS.md). stdlib only.

Library: load_plan, rebase (give an older plan the honest-model keys of CURRENT), patch (apply one
calibration level to a plan dict), run_eval (EHT at a rung), run_splits (model vs WR splits).
Commands:
  sens.py tf                      trainer_fit vs the WR splits (CURRENT)
  sens.py grid OUT.json RUNG R SEED [PLAN ...]   the plan x calibration-level grid, written as JSON
"""
import json, math, os, re, subprocess, sys, concurrent.futures

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
BIN = os.environ.get('FRLG_BIN', os.path.join(ROOT, 'target', 'release', 'frlg-human'))
SCRATCH = os.environ.get('FRLG_SCRATCH', '/tmp/frlg-human-scratch')
os.makedirs(SCRATCH, exist_ok=True)
POST = ['lorelei-2', 'bruno-2', 'agatha-2', 'lance-2', 'champion-2', 'icefall-grunt', 'warehouse-admin-1', 'warehouse-admin-2',
        'warehouse-gideon', 'lorelei', 'bruno', 'agatha', 'lance', 'champion']

# (knob, [levels]); the first level of each knob that equals the plan's own value is the "primary".
LEVELS = [
    ('S', [0.10, 0.40, 1.0]),                   # post-door survival (product over the League/post-game hazards); primary 0.183
    ('slow', [0.0, 2.4]),                       # `slow_scale`: execution-loss events; primary 1.0
    ('win_cv', [0.0, 0.066]),                   # primary 0.033
    ('trainer_fit', [0.76, 0.86, 0.91]),        # primary 0.81
    ('mistake', [0.0, 1.0]),                    # primary 0.3
    ('p_hit', [0.55, 0.64, 0.76]),              # per-press starter hit rate; primary 0.692
    ('quiet', [0]),                             # quiet-seed table off; primary on
    ('wall', [1.3]),                            # off-timer starter wall costs x; primary 1.0
    ('prefatal', [0.5, 1.5]),                   # pre-door death hazards, log-survival x; primary 1.0
]


def load_plan(name):
    t = open(os.path.join(ROOT, 'human-run', 'plans', name + '.md')).read()
    return json.loads(t.split('```json')[1].split('```')[0])


def rebase(p, cur):
    """An older plan gets CURRENT's value for every model key it lacks (fatal, slow, win_cv, trainer_fit, ...)."""
    q = dict(p)
    for k, v in cur.items():
        if k not in q:
            q[k] = v
    return q


def survival(fatal, keys):
    s = 1.0
    for k in keys:
        s *= 1 - fatal.get(k, 0.0)
    return s


def scale_surv(fatal, keys, target):
    cur = survival(fatal, keys)
    if cur >= 1.0 or cur <= 0.0:
        return dict(fatal)
    k = math.log(target) / math.log(cur)
    out = dict(fatal)
    for key in keys:
        if key in out:
            out[key] = min(0.999, 1 - (1 - out[key]) ** k)
    return out


def patch(p, knob, val):
    q = json.loads(json.dumps(p))
    m = q.setdefault('manip', {})
    if knob == 'S':
        post = [k for k in POST if k in q.get('fatal', {})]
        q['fatal'] = scale_surv(q['fatal'], post, val)
    elif knob == 'slow':
        q['slow_scale'] = val
    elif knob == 'win_cv':
        q['win_cv'] = val
    elif knob == 'trainer_fit':
        q['trainer_fit'] = val
    elif knob == 'mistake':
        q['mistake_rate'] = val
    elif knob == 'p_hit':
        m['p_hit'] = val
    elif knob == 'quiet':
        m['quiet_max_hits'] = int(val)
    elif knob == 'wall':
        m['s_boot_title'] = m.get('s_boot_title', 45.0) * val
        m['s_reset'] = m.get('s_reset', 100.0) * val
    elif knob == 'prefatal':
        q['fatal'] = {k: (v if k in POST else min(0.999, 1 - (1 - v) ** val)) for k, v in q.get('fatal', {}).items()}
    elif knob == 'none':
        pass
    else:
        raise SystemExit('unknown knob ' + knob)
    return q


def write(q, tag):
    f = os.path.join(SCRATCH, tag + '.md')
    q = dict(q)
    q['name'] = tag
    open(f, 'w').write('```json\n' + json.dumps(q) + '\n```\n')
    return f


def run_eval(q, tag, rung, R, seed):
    f = write(q, tag)
    out = subprocess.run([BIN, 'eval', f, '--rung', rung, '--R', str(R), '--seed', str(seed)], capture_output=True, text=True).stdout.strip().splitlines()
    line = out[-1] if out else ''
    m = re.search(r'finishes (\d+)/(\d+) EHT ([\d.]+) h', line)
    if not m:
        return None, 0
    return float(m.group(3)), int(m.group(1))


def hms(s):
    h, m, x = s.split(':')
    return int(h) * 3600 + int(m) * 60 + int(x)


def run_splits(q, tag, R=20000, seed=1):
    f = write(q, tag)
    out = subprocess.run([BIN, 'splits', f, '--R', str(R), '--seed', str(seed)], capture_output=True, text=True).stdout.splitlines()
    rows = []
    for l in out:
        m = re.match(r'^(.+?)\s+(\d:\d\d:\d\d)\s+(\d:\d\d:\d\d)\s+([+-]?\d+)\s+([+-]?[\d.]+)%\s+(\d:\d\d:\d\d)\s+(\d:\d\d:\d\d)', l)
        if m:
            rows.append((m.group(1).strip(), hms(m.group(2)), hms(m.group(3)), hms(m.group(6)), hms(m.group(7))))
    return rows


def cmd_tf():
    cur = load_plan('608-evo581-habit')
    names = ('Brock', 'Misty', 'Koga', 'Sevii', 'Giovanni 3', 'Start E4R1', 'End')
    print('trainer_fit  ' + '  '.join(f'{n:>10s}' for n in names) + '   window rms %, Brock..Giovanni 3')
    for tf in (0.76, 0.81, 0.86, 0.91):
        rows = run_splits(patch(cur, 'trainer_fit', tf), 'tf', 20000)
        d = {r[0]: r for r in rows}
        errs = [d[n][2] - d[n][1] for n in names]
        win, prev = [], None
        for r in rows[:13]:
            a = r[2] - (prev[2] if prev else 0)
            b = r[1] - (prev[1] if prev else 0)
            win.append(a / b - 1)
            prev = r
        rms = math.sqrt(sum(x * x for x in win) / len(win)) * 100
        print(f'{tf:<12}' + '  '.join(f'{e:>+10d}' for e in errs) + f'   {rms:.1f}')


def cmd_grid(out, rung, R, seed, plans):
    cur = load_plan('608-evo581-habit')
    jobs = []
    for name in plans:
        base = rebase(load_plan(name), cur)
        jobs.append((name, 'none', 0, base))
        for knob, levels in LEVELS:
            for v in levels:
                jobs.append((name, knob, v, patch(base, knob, v)))
    res = []

    def one(j):
        name, knob, v, q = j
        tag = re.sub(r'[^A-Za-z0-9]+', '_', f'g_{name[:3]}_{knob}_{v}')
        eht, fin = run_eval(q, tag, rung, R, seed)
        return dict(plan=name, knob=knob, level=v, eht=eht, fin=fin)

    # the engine is already parallel over all cores; run the grid serially
    for j in jobs:
        r = one(j)
        res.append(r)
        print(r, flush=True)
        json.dump(res, open(out, 'w'))


SCEN = [('none', 0), ('S', 0.10), ('S', 1.0), ('trainer_fit', 0.76), ('trainer_fit', 0.86), ('mistake', 0.0), ('mistake', 1.0),
        ('slow', 2.4), ('prefatal', 0.5), ('prefatal', 1.5), ('p_hit', 0.55)]


def shifted(p, d, skip=('oak-national-dex', 'brock')):
    q = json.loads(json.dumps(p))
    q['reset_aggr_at'] = {k: (v if k in skip else round(min(1.0, max(0.3, v + d)), 3)) for k, v in p['reset_aggr_at'].items()}
    return q


def cmd_cands(out, rung, R, seed, plan, shifts):
    cur = load_plan('608-evo581-habit')
    base = rebase(load_plan(plan), cur)
    res = []
    for d in shifts:
        cand = shifted(base, d)
        for knob, v in SCEN:
            tag = re.sub(r'[^A-Za-z0-9]+', '_', f'c_{d}_{knob}_{v}')
            eht, fin = run_eval(patch(cand, knob, v), tag, rung, R, seed)
            r = dict(plan=f'{plan}@{d:+.2f}', knob=knob, level=v, eht=eht, fin=fin)
            res.append(r)
            print(r, flush=True)
            json.dump(res, open(out, 'w'))


if __name__ == '__main__':
    cmd = sys.argv[1]
    if cmd == 'tf':
        cmd_tf()
    elif cmd == 'grid':
        cmd_grid(sys.argv[2], sys.argv[3], int(sys.argv[4]), int(sys.argv[5]), sys.argv[6:])
    elif cmd == 'cands':
        cmd_cands(sys.argv[2], sys.argv[3], int(sys.argv[4]), int(sys.argv[5]), sys.argv[6], [float(x) for x in sys.argv[7:]])
