#!/usr/bin/env python3
"""EHT interval of a plan over the evidence intervals of trainer_fit and mistake_rate (journal 2026-11-25). stdlib only.

usage: exec_interval.py GRID.json [TF_MU TF_SD MR_MU MR_SDLOG]

GRID.json is a list of {tf, mr, eht, fin} (data/exec-grid-1029-wr2m.json: `sens.py`-style evals at WR+2m, R 3.2M, seed 61).
Each grid cell gets the weight (normal density of trainer_fit) x (lognormal density of mistake_rate) integrated over the
cell's width; EHT quantiles are weighted quantiles over the cells ('no finish' counts as infinity). The defaults are
the fits of `fit_exec.py` for 1029 (trainer_fit 0.729 +- 0.066, mistake_rate 0.162, log-sd 0.426).
"""
import json, math, sys

g = json.load(open(sys.argv[1]))
tf_mu, tf_sd, mr_mu, mr_sd = (float(x) for x in sys.argv[2:6]) if len(sys.argv) >= 6 else (0.729, 0.0662, 0.1617, 0.4259)
tfs = sorted({r['tf'] for r in g})
mrs = sorted({r['mr'] for r in g})


def cells(vs):
    out = {}
    for i, v in enumerate(vs):
        lo = (vs[i - 1] + v) / 2 if i else v - (vs[1] - v) / 2
        hi = (vs[i + 1] + v) / 2 if i < len(vs) - 1 else v + (v - vs[i - 1]) / 2
        out[v] = (lo, hi)
    return out


def ncdf(x):
    return 0.5 * (1 + math.erf(x / 2 ** 0.5))


ct, cm = cells(tfs), cells(mrs)


def wt(v):
    lo, hi = ct[v]
    return ncdf((hi - tf_mu) / tf_sd) - ncdf((lo - tf_mu) / tf_sd)


def wm(v):
    lo, hi = cm[v]
    lo = max(lo, 1e-6)
    return ncdf((math.log(hi / mr_mu)) / mr_sd) - ncdf((math.log(lo / mr_mu)) / mr_sd)


pts = sorted(((wt(r['tf']) * wm(r['mr']), r['eht'] if r['eht'] else float('inf'), r) for r in g), key=lambda x: x[1])
tot = sum(w for w, _, _ in pts)
res = {}
for q in (0.025, 0.16, 0.5, 0.84, 0.975):
    c = 0.0
    for w, e, r in pts:
        c += w / tot
        if c >= q:
            res[str(q)] = dict(eht=e, tf=r['tf'], mr=r['mr'])
            break
res['p_eht_lt_500'] = sum(w for w, e, _ in pts if e < 500) / tot
res['p_eht_lt_1000'] = sum(w for w, e, _ in pts if e < 1000) / tot
res['p_no_finish'] = sum(w for w, e, _ in pts if e == float('inf')) / tot
res['plan_constants_tf_081_cdf'] = ncdf((0.81 - tf_mu) / tf_sd)
res['plan_constants_mr_030_cdf'] = ncdf(math.log(0.3 / mr_mu) / mr_sd)
print(json.dumps(res, indent=1))
