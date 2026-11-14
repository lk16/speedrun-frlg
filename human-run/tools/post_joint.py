#!/usr/bin/env python3
"""Joint posterior of the post-door survival S: the door evidence x the runner's 15 811 attempts per WR finish.

usage: post_joint.py PLAN.md OUT.json [--R 3200000] [--seed 4] [--S 0.04,0.07,0.1,0.14,0.183,0.25,0.35,0.5]

For each S (the product of the post-door `fatal` survivals, scaled with sens.patch) runs the WR rung and
records finishes, attempts per finish A and EHT. Posterior over S (flat prior on log S):
  L_door(S)  = binomial 1 finish in 4 door attempts that fought, at total survival T = S * engine product
  L_15811(S) = geometric first passage: r exp(-r * 15811), r = 1 / A(S)
Prints the grid, the normalised posteriors (door only, 15 811 only, both) and their 90% intervals. stdlib only.
"""
import json, math, os, re, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import sens  # noqa: E402

a = sys.argv[1:]


def opt(n, d):
    if n in a:
        i = a.index(n)
        v = type(d)(a[i + 1])
        del a[i:i + 2]
        return v
    return d


R, seed = opt('--R', 3200000), opt('--seed', 4)
grid = [float(x) for x in opt('--S', '0.04,0.07,0.1,0.14,0.183,0.25,0.35,0.5').split(',')]
ENG = opt('--eng', 0.838)  # engine-only product over the post-door fights (fit_post_evidence.py)
plan, outp = a[0], a[1]
p = json.loads(open(plan).read().split('```json')[1].split('```')[0])
res = []
for S in grid:
    q = sens.patch(p, 'S', S)
    eht, fin = sens.run_eval(q, f'pj_{S}', 'WR', R, seed)
    apf = R / max(fin, 1)
    res.append(dict(S=S, fin=fin, apf=apf, eht=eht))
    print(res[-1], flush=True)
    json.dump(res, open(outp, 'w'))


def post(ll):
    m = max(ll)
    w = [math.exp(x - m) for x in ll]
    s = sum(w)
    return [x / s for x in w]


def interval(ws, xs):
    c, lo, med, hi = 0.0, None, None, None
    for w, x in zip(ws, xs):
        c += w
        if lo is None and c >= 0.05:
            lo = x
        if med is None and c >= 0.5:
            med = x
        if hi is None and c >= 0.95:
            hi = x
    return lo, med, hi


S = [r['S'] for r in res]
ld = [math.log(S_ * ENG) + 3 * math.log(1 - S_ * ENG) for S_ in S]  # 1 success, 3 failures at T = S * ENG
lg = [(-math.log(r['apf']) - 15811 / r['apf']) for r in res]
for name, ll in (('door only', ld), ('15811 only', lg), ('both', [x + y for x, y in zip(ld, lg)])):
    w = post(ll)
    lo, med, hi = interval(w, S)
    print(f'{name:>11}: ' + ' '.join(f'{s:.3g}:{x:.2f}' for s, x in zip(S, w)) + f'   5/50/95% grid points {lo}/{med}/{hi}')
