#!/usr/bin/env python3
"""h2_score.py CHILD PARENT KIND "idea" [--R 800000] [--seed 7401] [--rung WR+2m]: one paired compare, logged.

Model-honesty-2 scoring (plans 1400-1419): runs `frlg-human compare CHILD PARENT` once at a fixed rung and R and
appends a scores.jsonl line with KIND (MODEL / PLAN / MODEL+PLAN) as the verdict head. A model change is not
accepted or refuted on its score: the line records its effect. stdlib only.
"""
import json, os, subprocess, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
BIN = os.environ.get('FRLG_BIN', os.path.join(ROOT, 'target', 'release', 'frlg-human'))
args = sys.argv[1:]


def opt(name, default):
    if name in args:
        i = args.index(name)
        v = args[i + 1]
        del args[i:i + 2]
        return v
    return default


R, seed, rung = opt('--R', '800000'), opt('--seed', '7401'), opt('--rung', 'WR+2m')
child, parent, kind, idea = args
plan = lambda x: x if os.path.exists(x) else os.path.join(ROOT, 'human-run', 'plans', x + '.md')
load = lambda x: json.loads(open(plan(x)).read().split('```json')[1].split('```')[0])
c, p = load(child), load(parent)
diff = {k: v for k, v in c.items() if k != 'name' and p.get(k) != v}
out = subprocess.run([BIN, 'compare', plan(child), plan(parent), '--rung', rung, '--R', R, '--seed', seed],
                     capture_output=True, text=True, check=True).stdout
j = json.loads([l for l in out.splitlines() if l.startswith('JSON ')][0][5:])
rec = dict(plan=c['name'], parent=p['name'], idea=idea, diff=diff, seed=int(seed), rung=rung, eht_hours=j['a_eht'],
           R=int(R), ci=[j['ci_lo'], j['ci_hi']], diff_vs_parent=j['diff'], parent_eht=j['b_eht'], accepted=False,
           verdict=f"{kind}, paired vs {p['name']}: {j['diff']} h [{j['ci_lo']}, {j['ci_hi']}] ({j['a_fin']} vs {j['b_fin']} finishes)",
           commit=None)
with open(os.path.join(ROOT, 'human-run', 'scores.jsonl'), 'a') as f:
    f.write(json.dumps(rec) + '\n')
print(rec['verdict'])
