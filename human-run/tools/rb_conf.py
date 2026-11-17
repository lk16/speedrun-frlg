#!/usr/bin/env python3
"""Paired confirmation of a variant against its base in constant corners: rb_conf.py OUT.json VARIANT SCEN,SCEN RUNG,RUNG R SEED,SEED [CAL] [PLAN]
Runs `frlg-human compare base variant --rung --R --seed` (paired, CI of the difference)."""
import json, os, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import rb_lib as L  # noqa: E402
import rb_mc as M  # noqa: E402
import rb_var as V  # noqa: E402

out, var = sys.argv[1], sys.argv[2]
scens, rungs, R, seeds = sys.argv[3].split(','), sys.argv[4].split(','), int(sys.argv[5]), [int(s) for s in sys.argv[6].split(',')]
calp = sys.argv[7] if len(sys.argv) > 7 else os.path.join(HERE, '..', 'data', 'robust-cal-1057.json')
plan = sys.argv[8] if len(sys.argv) > 8 else '1057-calm-sp9-ppup-redo'
base = L.load_plan(plan)
cal = M.Cal(calp, base)
res = json.load(open(out)) if os.path.exists(out) else []
for s in scens:
    kw = V.corner(s, cal)
    fa = L.write(L.patch(base, **kw), 'conf_base_' + s.replace('.', '_'))
    fb = L.write(L.patch(V.variant(var, base), **kw), 'conf_' + var + '_' + s.replace('.', '_'))
    for rg in rungs:
        for sd in seeds:
            o = subprocess.run([L.BIN, 'compare', fa, fb, '--rung', rg, '--R', str(R), '--seed', str(sd)], capture_output=True, text=True).stdout.strip()
            res.append(dict(variant=var, scen=s, rung=rg, R=R, seed=sd, out=o.splitlines()[-1] if o else ''))
            json.dump(res, open(out, 'w'))
            print(var, s, rg, sd, res[-1]['out'], flush=True)
