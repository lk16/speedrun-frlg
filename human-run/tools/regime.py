#!/usr/bin/env python3
"""Day-regime mixture of EHT (fidelity audit 2026-11-14).

usage: regime.py PLAN.md [--rung WR] [--R 200000] [--seed 1] [--w 0.5]

The plan is the good day. Its optional `regime_bad` block patches the bad day: lower manip hit
probability, more mistakes and jitter, the early-death hazards of the runner's bad days
(`video-analysis/STRATEGY.md` 1.3 and 7: "12 misses in a row", "2/12 and 3/14 past Brock", 1.5 h
lost to drift, fatigue after 6-7 h). Hours are what the mixture weighs, not attempts: with a
share `w` of the runner's hours in the good regime and finishes per hour `1/EHT_g` and `1/EHT_b`,
the all-days EHT is `1 / (w/EHT_g + (1-w)/EHT_b)`. The weight is a guess (the evidence has
good days in T06 and bad ones in T07, T10, T12).

Prints one JSON line. stdlib only.
"""
import json, os, subprocess, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
BIN = os.environ.get('FRLG_BIN', os.path.join(ROOT, 'target', 'release', 'frlg-human'))
SCRATCH = os.environ.get('FRLG_SCRATCH', '/tmp/frlg-human-scratch')
os.makedirs(SCRATCH, exist_ok=True)

args = sys.argv[1:]
def opt(name, default):
    if name in args:
        i = args.index(name)
        v = args[i + 1]
        del args[i:i + 2]
        return type(default)(v)
    return default

rung = opt('--rung', 'WR')
R = opt('--R', 200000)
seed = opt('--seed', 1)
w_arg = opt('--w', -1.0)
path = args[0]
good = json.loads(open(path).read().split('```json')[1].split('```')[0])
bad_patch = good.get('regime_bad', {})
w = w_arg if w_arg >= 0 else bad_patch.get('weight', 0.5)

bad = json.loads(json.dumps(good))
bad['name'] = good['name'] + '-bad'
m = dict(bad.get('manip', {}))
if 'manip_p_hit' in bad_patch:
    m['p_hit'] = bad_patch['manip_p_hit']
bad['manip'] = m
for k in ('mistake_rate', 'jitter_n', 'react_lo', 'react_hi', 'hazard'):
    if k in bad_patch:
        bad[k] = bad_patch[k]
bpath = os.path.join(SCRATCH, bad['name'] + '.md')
open(bpath, 'w').write('```json\n' + json.dumps(bad, indent=1) + '\n```\n')


def eht(p):
    out = subprocess.check_output([BIN, 'eval', p, '--rung', rung, '--R', str(R), '--seed', str(seed)], text=True)
    line = out.strip().splitlines()[-1]
    fin = int(line.split('finishes ')[1].split('/')[0])
    e = line.split('EHT ')[1].split()[0]
    return fin, (float('inf') if e == 'inf' else float(e))

fg, eg = eht(path)
fb, eb = eht(bpath)
rate = (w / eg if eg != float('inf') else 0.0) + ((1 - w) / eb if eb != float('inf') else 0.0)
mix = 1 / rate if rate > 0 else float('inf')
print(json.dumps(dict(plan=good['name'], rung=rung, R=R, seed=seed, w_good=w,
                      good_eht=eg, good_fin=fg, bad_eht=eb, bad_fin=fb, mix_eht=mix)))
