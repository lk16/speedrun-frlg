#!/usr/bin/env python3
"""cal_eht.py PLAN [lp=..] [mp=..] [cp=..] [D=0.23] [R=1600000] [seed=5301] [rungs=WR+2m,WR] [noflat=1]

EHT of PLAN at the given stand-in constants with the door-to-finish survival D PINNED to the evidence (default .23, the
rb_post posterior mean): the engine-only D (flat post-door `fatal` rows off) is measured at the constants, then the flat
rows are scaled (`flat` knob of rb_lib.patch) so that ln D = ln S_eng + flat * ln S_flat (as rb_mc.Cal.flat_for does).
Prints one JSON line. Scratch plans under $RB_SCRATCH. stdlib only."""
import json, math, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rb_lib as L

plan = sys.argv[1]
kw = dict(a.split('=') for a in sys.argv[2:])
D = float(kw.pop('D', 0.23))
R = int(kw.pop('R', 1600000))
seed = int(kw.pop('seed', 5301))
rungs = kw.pop('rungs', 'WR+2m,WR').split(',')
noflat = int(kw.pop('noflat', 0))
Rcal = int(kw.pop('Rcal', 600000))
pinQ = int(kw.pop('pinQ', 0))
pk = {k: float(v) for k, v in kw.items()}
base = L.load_plan(plan)
q0 = L.patch(base, **pk)
lam = 1.0
if pinQ and 'cp' in pk:
    # pre-door survival Q pinned to the plan's own (cp, lane 1) value by bisection on the risk-lane scale
    Qt = L.QD(L.run_report(base, 'ceht-q%d' % os.getpid(), 'WR+160m', Rcal, 77))[0]
    lo, hi = 0.6, 1.8
    for _ in range(9):
        mid = math.sqrt(lo * hi)
        Qm = L.QD(L.run_report(L.patch(base, **dict(pk, lane=mid)), 'ceht-q%d' % os.getpid(), 'WR+160m', Rcal, 77))[0]
        lo, hi = (mid, hi) if Qm < Qt else (lo, mid)
    lam = math.sqrt(lo * hi)
    q0 = L.patch(base, **dict(pk, lane=lam))
# engine-only D
qe = json.loads(json.dumps(q0))
qe['fatal'] = {k: (0.0 if k in L.POST else v) for k, v in qe.get('fatal', {}).items()}
Q, De = L.QD(L.run_report(qe, 'ceht-d%d' % os.getpid(), 'WR+160m', Rcal, 77))
lnS = L.flat_logsurv(q0)
flat = 0.0 if D >= De else min(3.0, (math.log(D) - math.log(De)) / lnS)
if noflat:
    flat = 1.0
q = L.patch(q0, flat=flat) if flat != 1.0 else q0
res = {'plan': plan, 'consts': pk, 'D_pin': D, 'D_engine': round(De, 4), 'flat': round(flat, 3), 'Q': round(Q, 4)}
for rg in rungs:
    e, f = L.run_eval(q, 'ceht-%d' % os.getpid(), rg, R, seed)
    res[rg] = {'eht': e, 'fin': f}
print(json.dumps(res), flush=True)
