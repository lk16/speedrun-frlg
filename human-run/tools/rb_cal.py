#!/usr/bin/env python3
"""Calibration surfaces for rb_mc.py on a given plan (ROBUSTNESS.md section 14/16). stdlib only.

usage: rb_cal.py PLAN OUT.json [--R 400000] [--seed 31] [--rung WR+160m]
Q(cp, lane): pre-door survival (reach indigo-heal / reach rival-lab) on the lenient rung, cp in {1.6, 2.0, 2.4},
lane scale in {0.8, 1.0, 1.25}, and the door-to-finish survival D at the same cell.
D(lp, mp): engine-only door-to-finish survival (the flat post-door `fatal` rows set to 0), lp in {1.0, 1.2, 1.4, 1.7},
mp in {1.0, 1.2, 1.4, 1.6}. Same file layout as data/robust-cal-1057.json.
"""
import json, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import rb_lib as L  # noqa: E402

a = sys.argv[1:]


def opt(name, d):
    if name in a:
        i = a.index(name)
        v = type(d)(a[i + 1])
        del a[i:i + 2]
        return v
    return d


R, seed, rung = opt('--R', 400000), opt('--seed', 31), opt('--rung', 'WR+160m')
plan, out = a[0], a[1]
base = L.load_plan(plan)
res = {'Q': [], 'D': [], 'raw_D': {}}
for cp in (1.6, 2.0, 2.4):
    for lam in (0.8, 1.0, 1.25):
        q = L.patch(base, cp=cp, lane=lam)
        Q, D = L.QD(L.run_report(q, f'cal-q{cp}-{lam}', rung, R, seed))
        res['Q'].append({'cp': cp, 'lam': lam, 'Q': Q, 'D': D})
        print('Q', cp, lam, round(Q, 4), round(D, 3), flush=True)
for lp in (1.0, 1.2, 1.4, 1.7):
    for mp in (1.0, 1.2, 1.4, 1.6):
        q = L.patch(base, lp=lp, mp=mp, flat=1e-9)
        q['fatal'] = {k: (0.0 if k in L.POST else v) for k, v in q.get('fatal', {}).items()}
        Q, D = L.QD(L.run_report(q, f'cal-d{lp}-{mp}', rung, R, seed))
        res['D'].append({'lp': lp, 'mp': mp, 'D': D})
        res['raw_D'][f'{lp},{mp}'] = D
        print('D', lp, mp, round(D, 3), flush=True)
json.dump(res, open(out, 'w'))
