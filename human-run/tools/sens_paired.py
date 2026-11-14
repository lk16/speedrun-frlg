#!/usr/bin/env python3
"""Paired `compare` rows for ROBUSTNESS.md and scores.jsonl. usage: sens_paired.py RUNG R SEED A B [A B ...]
Appends one scores.jsonl row per pair (verdict text says it is a robustness/model-change row). stdlib only."""
import json, os, subprocess, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
BIN = os.environ.get('FRLG_BIN', os.path.join(ROOT, 'target', 'release', 'frlg-human'))
rung, R, seed = sys.argv[1], sys.argv[2], sys.argv[3]
pairs = sys.argv[4:]
for a, b in zip(pairs[::2], pairs[1::2]):
    out = subprocess.run([BIN, 'compare', os.path.join(ROOT, 'human-run', 'plans', a + '.md'), os.path.join(ROOT, 'human-run', 'plans', b + '.md'),
                          '--rung', rung, '--R', R, '--seed', seed], capture_output=True, text=True).stdout
    j = json.loads([l for l in out.splitlines() if l.startswith('JSON ')][0][5:])
    print(a, 'vs', b, rung, 'R', R, 'seed', seed, j, flush=True)
    row = dict(plan=a, parent=b, idea='robustness/model-change paired row (ROBUSTNESS.md)', rung=rung, eht_hours=j['a_eht'], parent_eht=j['b_eht'], R=j['R'],
               ci=[j['ci_lo'], j['ci_hi']], diff_vs_parent=j['diff'], seed=int(seed), accepted=False,
               verdict=f"MODEL SENSITIVITY row, not a plan acceptance: {a} {j['a_eht']} h ({j.get('a_fin')} fin) vs {b} {j['b_eht']} h ({j.get('b_fin')} fin) at {rung}")
    with open(os.path.join(ROOT, 'human-run', 'scores.jsonl'), 'a') as f:
        f.write(json.dumps(row) + '\n')
