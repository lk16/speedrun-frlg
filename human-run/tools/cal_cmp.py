#!/usr/bin/env python3
"""cal_cmp.py PLAN_A PLAN_B CONSTS_JSON [rung=WR+2m] [R=1600000] [seed=6101] [tag=x]

Paired `frlg-human compare` of two plans at the SAME stand-in constants. CONSTS_JSON is a dict of rb_lib.patch knobs
(lp, mp, cp, flat, lane, set={dotted.key: value}); plan names are files under human-run/plans or JSON "NAME+{dotted:val}"
strings (a plan with dotted-key overrides: `1103-dropfill442+{"fight_tactics.champion.x":["guard","special"]}`).
Prints the compare JSON line. stdlib only."""
import json, os, re, subprocess, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rb_lib as L


def plan_of(spec):
    if '+{' in spec:
        name, ov = spec.split('+', 1)
        return L.patch(L.load_plan(name), set=json.loads(ov))
    return L.load_plan(spec)


a, b, consts = sys.argv[1], sys.argv[2], json.loads(sys.argv[3])
kw = dict(x.split('=') for x in sys.argv[4:])
rung, R, seed, tag = kw.get('rung', 'WR+2m'), kw.get('R', '1600000'), kw.get('seed', '6101'), kw.get('tag', 'x')
pa, pb = L.write(L.patch(plan_of(a), **consts), tag + 'a'), L.write(L.patch(plan_of(b), **consts), tag + 'b')
out = subprocess.run([L.BIN, 'compare', pa, pb, '--rung', rung, '--R', R, '--seed', seed], capture_output=True, text=True).stdout
print(json.dumps({'a': a, 'b': b, 'consts': consts, 'rung': rung, 'R': R, 'seed': seed, 'out': out.strip().splitlines()[-2:]}))
