#!/usr/bin/env python3
"""Coordinate descent over the starter-phase (manip) knobs only, for one `dials` setting.

usage: manipgrid.py PARENT.md DIALS RUNG R SEED OUT_PREFIX [--passes N]
Knobs: min_spa/min_spe/min_hp/min_spd, s_reset, quiet_max_hits (p_hit stays at the plan's evidence value;
natures stay Rash/Mild). One frlg-human `compare` per candidate against the running best (paired, same seed);
a candidate is taken if variant - parent < 0 and its CI upper bound < 0. Writes OUT_PREFIX-best.md and
OUT_PREFIX.jsonl. stdlib only.
"""
import json, os, subprocess, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
BIN = os.environ.get('FRLG_BIN', os.path.join(ROOT, 'target', 'release', 'frlg-human'))
parent, dials, rung, R, seed, prefix = sys.argv[1:7]
passes = int(sys.argv[sys.argv.index('--passes') + 1]) if '--passes' in sys.argv else 2
text = open(parent).read()
P = json.loads(text.split('```json')[1].split('```')[0])
P['manip']['dials'] = int(dials)
P['name'] = os.path.basename(prefix) + '-base'


def write(p, path):
    open(path, 'w').write('```json\n' + json.dumps(p, indent=1) + '\n```\n')


def steps(k, v):
    if k == 's_reset':
        return [v - 20, v + 20]
    if k == 'quiet_max_hits':
        return [v - 1, v + 1, v + 2]
    return [v - 3, v - 1, v + 1, v + 3, v + 5] if k != 'min_spa' else [v - 1, v + 1, v + 2]


cur = P
curpath = prefix + '-cur.md'
write(cur, curpath)
log = open(prefix + '.jsonl', 'a')
for ps in range(passes):
    moved = False
    for k in ('min_spe', 'min_hp', 'min_spd', 'min_spa', 'quiet_max_hits', 's_reset'):
        for v in steps(k, cur['manip'][k]):
            if v < 0 or (k.startswith('min_') and v > 31):
                continue
            ch = json.loads(json.dumps(cur))
            ch['manip'][k] = v
            ch['name'] = prefix.split('/')[-1] + '-cand'
            cp = prefix + '-cand.md'
            write(ch, cp)
            r = subprocess.run([BIN, 'compare', curpath, cp, '--rung', rung, '--R', R, '--seed', seed],
                               capture_output=True, text=True)
            js = [l for l in r.stdout.splitlines() if l.startswith('JSON ')]
            if not js:
                print(k, v, 'ERROR', r.stderr[-200:], flush=True)
                continue
            j = json.loads(js[0][5:])
            d = None if j['diff'] is None else -j['diff']
            hi = None if j['ci_lo'] is None else -j['ci_lo']
            rec = dict(dials=int(dials), knob=k, to=v, frm=cur['manip'][k], parent=j['a_eht'], variant=j['b_eht'],
                       diff=d, ci_hi=hi, rung=rung, R=int(R), seed=int(seed))
            log.write(json.dumps(rec) + '\n'); log.flush()
            print('d%s %s %s->%s: %s vs %s diff %s hi %s' % (dials, k, cur['manip'][k], v, j['a_eht'], j['b_eht'], d, hi), flush=True)
            if d is not None and hi is not None and hi < 0:
                cur = ch
                write(cur, curpath)
                moved = True
    if not moved:
        break
write(cur, prefix + '-best.md')
print('BEST', json.dumps(cur['manip']), flush=True)
