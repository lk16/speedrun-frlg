#!/usr/bin/env python3
"""Solve the post-door survival S (fidelity fit 2026-11-14) so the attempts per WR finish equal the runner's.

usage: fit_post.py PLAN.md [--target 15811] [--R 3200000] [--seed 4] [--lo 0.05] [--hi 0.6] [--iters 6]

The plan carries the window hazards (`fatal` from `fit_hazards.py`), `slow` and `win_cv`. The post-door
`fatal` entries are replaced by `fit_hazards.post_fatal(S)` and S is bisected (geometric) until the
WR-rung evaluation needs `target` attempts per finish. The runner's 15 811 attempts is one observation
(a geometric first-passage time: the 95% interval for the true rate spans 1.4e-6 .. 3e-4 per attempt), so
S is a calibration residual, not a measurement; the journal reports how the result moves with it. stdlib only.
"""
import json, math, os, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, '..', '..')
BIN = os.environ.get('FRLG_BIN', os.path.join(ROOT, 'target', 'release', 'frlg-human'))
SCRATCH = os.environ.get('FRLG_SCRATCH', '/tmp/frlg-human-scratch')
sys.path.insert(0, HERE)
import fit_hazards as fh  # noqa: E402

os.makedirs(SCRATCH, exist_ok=True)
a = sys.argv[1:]


def opt(n, d):
    if n in a:
        i = a.index(n)
        v = type(d)(a[i + 1])
        del a[i:i + 2]
        return v
    return d


target, R, seed = opt('--target', 15811.0), opt('--R', 3200000), opt('--seed', 4)
lo, hi, iters = opt('--lo', 0.05), opt('--hi', 0.6), opt('--iters', 6)
plan = a[0]
params = json.loads(open(plan).read().split('```json')[1].split('```')[0])
window_fatal = {k: v for k, v in params['fatal'].items() if k not in fh.POST_WEIGHTS}


def attempts_per_finish(S):
    p = dict(params)
    p['fatal'] = dict(window_fatal)
    p['fatal'].update(fh.post_fatal(S))
    path = os.path.join(SCRATCH, '_fit_post.md')
    open(path, 'w').write('```json\n' + json.dumps(p) + '\n```\n')
    out = subprocess.check_output([BIN, 'eval', path, '--rung', 'WR', '--R', str(R), '--seed', str(seed)], text=True)
    fin = int(out.strip().splitlines()[-1].split('finishes ')[1].split('/')[0])
    return R / max(fin, 1), fin


for _ in range(iters):
    mid = math.sqrt(lo * hi)
    apf, fin = attempts_per_finish(mid)
    print(f'S={mid:.4f} attempts/finish {apf:.0f} ({fin} finishes)', flush=True)
    if apf > target:
        lo = mid
    else:
        hi = mid
print(f'S ~ {math.sqrt(lo * hi):.3f}')
