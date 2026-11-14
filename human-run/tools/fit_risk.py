#!/usr/bin/env python3
"""Early pass rates of the risk-lane model against the runner's (journal 2026-11-14-22-00 table: 24/60 pass the Brock split, 14/22
the Misty split, 10/14 Rock Tunnel), and the binomial log-likelihood over `risk_foe`.

usage: fit_risk.py PLAN.md [R] [RISK_FOE,RISK_FOE,...] [json patch]

Runs `frlg-human report` at WR+2m (the lenient rung; pace resets are rare in the first hour, so 'reached' ratios are survival)
and prints, per risk_foe, the three window pass rates and the log-likelihood. stdlib only."""
import json, math, os, subprocess, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
BIN = os.environ.get('FRLG_BIN', os.path.join(ROOT, 'target', 'release', 'frlg-human'))
plan = sys.argv[1]
R = sys.argv[2] if len(sys.argv) > 2 else '300000'
rfs = [float(x) for x in sys.argv[3].split(',')] if len(sys.argv) > 3 else [1.0]
patch = json.loads(sys.argv[4]) if len(sys.argv) > 4 else {}
SCR = os.environ.get('FRLG_SCRATCH', '/tmp/frlg-human-scratch')
os.makedirs(SCR, exist_ok=True)
base = json.loads(open(plan).read().split('```json')[1].split('```')[0])


def ll(p, n, k):
    p = min(max(p, 1e-6), 1 - 1e-6)
    return k * math.log(p) + (n - k) * math.log(1 - p)


for rf in rfs:
    q = dict(base)
    q['risk_foe'] = rf
    q.update(patch)
    q['name'] = 'fitrisk'
    path = os.path.join(SCR, 'fitrisk-%d.md' % os.getpid())
    open(path, 'w').write('```json\n' + json.dumps(q) + '\n```\n')
    out = subprocess.run([BIN, 'report', path, '--rung', 'WR+2m', '--R', R], capture_output=True, text=True).stdout
    reach, order = {}, []
    for l in out.splitlines():
        f = l.split()
        if len(f) > 3 and f[1].endswith('%'):
            reach[f[0]] = float(f[1][:-1])
            order.append(f[0])
    s = reach['starter']
    nxt = reach[order[order.index('brock') + 1]]
    w1, w2 = nxt / s, reach['rival-cerulean'] / nxt
    w3 = reach['rock-tunnel'] / reach['rival-cerulean'] if 'rock-tunnel' in reach else float('nan')
    L = ll(w1, 60, 24) + ll(w2 / w1 * w1, 22, 14) + ll(w3, 14, 10)
    print('risk_foe %.2f  Brock split %.3f (runner 24/60)  Misty split %.3f (14/22)  Rock Tunnel %.3f (10/14)  LL %.2f'
          % (rf, w1, w2, w3, L), flush=True)
